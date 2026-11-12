use super::*;

impl Drop for FakeRelay {
    fn drop(&mut self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

pub(super) fn mint_id() -> String {
    let mut f = std::fs::File::open("/dev/urandom").unwrap();
    let mut b = [0u8; 16];
    f.read_exact(&mut b).unwrap();
    b.iter().map(|x| format!("{x:02x}")).collect()
}

struct OpSide {
    ws: Ws,
    id: String,
    ready: bool,
    done: bool,
}

/// Serve one node socket and its operators until `running` clears. One
/// thread polls every socket, like the node under test: the frame boundary
/// is the message boundary, and each frame has exactly one destination.
pub(super) fn serve(
    listener: TcpListener,
    node_token: &str,
    connect_token: &str,
    mode: Mode,
    records: Arc<Mutex<Records>>,
    running: Arc<std::sync::atomic::AtomicBool>,
) {
    let mut node: Option<Ws> = None;
    let mut node_since: Option<Instant> = None;
    let mut ops: Vec<OpSide> = Vec::new();
    let mut sessions: HashMap<String, usize> = HashMap::new();
    // Reconnect mode drops the first socket once; later sockets persist.
    let mut aborted = false;
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        // Accept new sockets without blocking the poll.
        match listener.accept() {
            Ok((s, _)) => {
                s.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
                let Ok((upgrade, req)) = Ws::server(s) else {
                    continue;
                };
                let token = req
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("x-relay-token"))
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                if let Some(name) = req.path.strip_prefix("/v1/node/") {
                    let _ = name;
                    if token != node_token {
                        // ⛔ The real relay refuses a wrong token at
                        // the HTTP upgrade with 403, which ends the
                        // node of `pod-relay` for good (exit 125).
                        records.lock().unwrap().wrong_tokens += 1;
                        upgrade.refuse(403, "reverse: forbidden");
                        continue;
                    }
                    if node.is_some() {
                        // One name holds one node socket: the second
                        // is refused, and the live one is undisturbed.
                        upgrade.refuse(409, "reverse: name in use");
                        continue;
                    }
                    let Ok(mut ws) = upgrade.accept() else {
                        continue;
                    };
                    records.lock().unwrap().node_conns += 1;
                    node_since = Some(Instant::now());
                    match mode {
                        Mode::CustomHello(text) => {
                            ws.send_text(text.as_bytes()).unwrap();
                        }
                        Mode::BinaryFirst => {
                            ws.send_binary(b"data-before-hello").unwrap();
                        }
                        _ => {
                            ws.send_text(
                                br#"{"type":"hello","version":1,"maxFrameBytes":65536,"maxSessions":64}"#,
                            )
                            .unwrap();
                        }
                    }
                    ws.set_read_timeout(Some(Duration::from_millis(50)))
                        .unwrap();
                    node = Some(ws);
                } else if req.path.strip_prefix("/v1/connect/").is_some() {
                    if token != connect_token {
                        records.lock().unwrap().wrong_tokens += 1;
                        upgrade.refuse(403, "reverse: forbidden");
                        continue;
                    }
                    let Ok(mut ws) = upgrade.accept() else {
                        continue;
                    };
                    match mode {
                        Mode::RejectAll => {
                            let id = mint_id();
                            let msg = format!(
                                r#"{{"type":"reject","id":"{id}","reason":"no such server"}}"#
                            );
                            ws.send_text(msg.as_bytes()).unwrap();
                            let _ = ws.send_close(1011, b"no such server");
                            records
                                .lock()
                                .unwrap()
                                .closes
                                .push(("op".to_string(), 1011));
                            continue;
                        }
                        Mode::Silent => {
                            ws.set_read_timeout(Some(Duration::from_millis(50)))
                                .unwrap();
                            // No session, no ready: the operator's
                            // own wait is what this mode tests.
                            ops.push(OpSide {
                                ws,
                                id: String::new(),
                                ready: false,
                                done: true,
                            });
                            continue;
                        }
                        Mode::Bye => {
                            ws.set_read_timeout(Some(Duration::from_millis(50)))
                                .unwrap();
                            // Ended before any ready: the operator must
                            // fail, not hang.
                            ws.send_text(br#"{"type":"bye"}"#).unwrap();
                            ops.push(OpSide {
                                ws,
                                id: String::new(),
                                ready: false,
                                done: true,
                            });
                            continue;
                        }
                        Mode::Normal
                        | Mode::CustomHello(_)
                        | Mode::BinaryFirst
                        | Mode::Lonely
                        | Mode::Reconnect => {}
                    }
                    let id = mint_id();
                    ws.set_read_timeout(Some(Duration::from_millis(50)))
                        .unwrap();
                    ops.push(OpSide {
                        ws,
                        id: id.clone(),
                        ready: false,
                        done: false,
                    });
                    records.lock().unwrap().sessions.push(SessionRec {
                        id: id.clone(),
                        node_bytes: 0,
                        op_bytes: 0,
                        completed: false,
                        rejected: false,
                    });
                    let open = format!(r#"{{"type":"open","id":"{id}"}}"#);
                    if let Some(n) = node.as_mut() {
                        n.send_text(open.as_bytes()).unwrap();
                    }
                    sessions.insert(id.clone(), ops.len() - 1);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }
        if node.is_none() && ops.iter().all(|o| o.done) {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }
        // Node socket first, then each operator: one poll turn.
        let mut node_gone = false;
        if let Some(n) = node.as_mut() {
            match n.read_frame() {
                Ok(None) => {}
                Ok(Some(Frame::Text(t))) => {
                    match text_control(&t) {
                        Ok((kind, id)) if kind == "ready" && sessions.contains_key(&id) => {
                            let idx = sessions[&id];
                            ops[idx].ready = true;
                            let msg = format!(r#"{{"type":"ready","id":"{id}"}}"#);
                            ops[idx].ws.send_text(msg.as_bytes()).unwrap();
                        }
                        Ok((kind, _)) if kind == "ready" => {
                            let _ = n.send_close(1003, b"unknown session id");
                            records
                                .lock()
                                .unwrap()
                                .closes
                                .push(("node".to_string(), 1003));
                            node_gone = true;
                        }
                        Ok((kind, id)) if kind == "reject" => {
                            if let Some(&idx) = sessions.get(&id) {
                                let msg = format!(
                                    r#"{{"type":"reject","id":"{id}","reason":"node refused"}}"#
                                );
                                ops[idx].ws.send_text(msg.as_bytes()).unwrap();
                                let _ = ops[idx].ws.send_close(1011, b"node refused");
                                ops[idx].done = true;
                            }
                            sessions.remove(&id);
                            mark_rejected(&records, &id);
                        }
                        Ok((kind, id)) if kind == "close" => {
                            if let Some(&idx) = sessions.get(&id) {
                                let _ = ops[idx].ws.send_close(1000, b"session ended");
                                records
                                    .lock()
                                    .unwrap()
                                    .closes
                                    .push(("op".to_string(), 1000));
                                ops[idx].done = true;
                            }
                            sessions.remove(&id);
                            mark_completed(&records, &id);
                        }
                        _ => {
                            // Malformed control or unknown type: the r12
                            // 1003 answer to a node-side fault.
                            let _ = n.send_close(1003, b"invalid control");
                            records
                                .lock()
                                .unwrap()
                                .closes
                                .push(("node".to_string(), 1003));
                            node_gone = true;
                        }
                    }
                }
                Ok(Some(Frame::Binary(b))) => {
                    // ⛔ The live relay's frame rule
                    // (`docs/history/references/relay-index-2026-09-28-r12.md:163`):
                    // under 32 bytes or over 65568 closes the node with
                    // 1009. Mirrored here as spec: the shipped node sizes
                    // every server read at the cap, so no driver reaches
                    // this arm. The 200000-byte transfers cross as several
                    // in-cap frames across poll turns, not through the
                    // chunking loop.
                    if b.len() < 32 || b.len() > 65568 {
                        let _ = n.send_close(1009, b"bad multiplex frame");
                        records
                            .lock()
                            .unwrap()
                            .closes
                            .push(("node".to_string(), 1009));
                        for op in ops.iter_mut() {
                            let _ = op.ws.send_close(1011, b"node disconnected");
                            op.done = true;
                        }
                        records
                            .lock()
                            .unwrap()
                            .closes
                            .push(("op".to_string(), 1011));
                        node_gone = true;
                    } else {
                        let (id, payload) = b.split_at(32);
                        let id = String::from_utf8_lossy(id).to_string();
                        if !is_hex32(&id) || !sessions.contains_key(&id) {
                            let _ = n.send_close(1003, b"unknown session id");
                            records
                                .lock()
                                .unwrap()
                                .closes
                                .push(("node".to_string(), 1003));
                            node_gone = true;
                        } else {
                            let idx = sessions[&id];
                            let op = &mut ops[idx];
                            if op.ready && !op.done {
                                op.ws.send_binary(payload).unwrap();
                                add_bytes(&records, &id, 0, payload.len() as u64);
                            } else {
                                let _ = n.send_close(1003, b"data before ready");
                                records
                                    .lock()
                                    .unwrap()
                                    .closes
                                    .push(("node".to_string(), 1003));
                                node_gone = true;
                            }
                        }
                    }
                }
                Ok(Some(Frame::Close(_, _))) | Err(_) => node_gone = true,
            }
        }
        if node_gone {
            node = None;
            // The node socket ended: every operator sees 1011, and sessions
            // left open are incomplete, never completed.
            for op in ops.iter_mut().filter(|o| !o.done) {
                let _ = op.ws.send_close(1011, b"node disconnected");
                op.done = true;
            }
            records
                .lock()
                .unwrap()
                .closes
                .push(("op".to_string(), 1011));
        }
        // Operators: binary is bare payload (the relay prepends the id on
        // this leg); text is a 1003; data before ready is a 1008.
        let mut op_gone: Vec<usize> = Vec::new();
        for (i, op) in ops.iter_mut().enumerate() {
            if op.done && op.id.is_empty() {
                continue; // Silent mode: hold the socket, send nothing.
            }
            if op.done {
                continue;
            }
            match op.ws.read_frame() {
                Ok(None) => {}
                Ok(Some(Frame::Text(_))) => {
                    let _ = op.ws.send_close(1003, b"binary frames required");
                    records
                        .lock()
                        .unwrap()
                        .closes
                        .push(("op".to_string(), 1003));
                    op.done = true;
                    op_gone.push(i);
                }
                Ok(Some(Frame::Binary(b))) => {
                    if !op.ready {
                        let _ = op.ws.send_close(1008, b"wait for ready");
                        records
                            .lock()
                            .unwrap()
                            .closes
                            .push(("op".to_string(), 1008));
                        op.done = true;
                        op_gone.push(i);
                        continue;
                    }
                    if b.is_empty() {
                        continue;
                    }
                    // ⛔ The live relay's operator frame rule (R12:165):
                    // one frame over 65536 payload bytes closes the
                    // operator with 1009. Mirrored here as spec: the
                    // shipped operator sizes every stdin read at 64 KiB,
                    // so no driver reaches this arm. The entry carries
                    // the analysis.
                    if b.len() > 65536 {
                        let _ = op.ws.send_close(1009, b"frame byte cap");
                        records
                            .lock()
                            .unwrap()
                            .closes
                            .push(("op".to_string(), 1009));
                        op.done = true;
                        op_gone.push(i);
                        continue;
                    }
                    if let Some(n) = node.as_mut() {
                        let mut framed = Vec::with_capacity(32 + b.len());
                        framed.extend_from_slice(op.id.as_bytes());
                        framed.extend_from_slice(&b);
                        n.send_binary(&framed).unwrap();
                        add_bytes(&records, &op.id, b.len() as u64, 0);
                    }
                }
                Ok(Some(Frame::Close(_, _))) | Err(_) => {
                    op.done = true;
                    op_gone.push(i);
                }
            }
        }
        // Tell the node about operators that went away, then drop fully
        // closed sessions. When no live session is left after at least one
        // completed, end the node socket so a `--once` node exits instead
        // of idling. The count comes from the records, not from the map:
        // the map is already empty by the time the last session ends.
        let mut ended_ids: Vec<String> = Vec::new();
        for &i in &op_gone {
            if !ops[i].id.is_empty() {
                ended_ids.push(ops[i].id.clone());
            }
        }
        for id in ended_ids {
            if let Some(n) = node.as_mut() {
                let msg = format!(r#"{{"type":"close","id":"{id}"}}"#);
                let _ = n.send_text(msg.as_bytes());
            }
            sessions.remove(&id);
            mark_completed(&records, &id);
        }
        // Reconnect mode drops the first socket after its first completed
        // session, without a goodbye: the node sees the socket end and
        // redials the same name. Only the first socket drops; the
        // replacement serves to the test's end.
        if mode == Mode::Reconnect && !aborted {
            let any_completed = records.lock().unwrap().sessions.iter().any(|s| s.completed);
            if any_completed {
                node = None;
                node_since = None;
                aborted = true;
            }
        }
        let live = ops.iter().filter(|o| !o.id.is_empty() && !o.done).count();
        let any_completed = records.lock().unwrap().sessions.iter().any(|s| s.completed);
        // Reconnect mode never idles a socket out with a goodbye: the
        // replacement socket stays until the test kills the node, so the
        // connection count proves exactly one redial.
        if mode != Mode::Reconnect && live == 0 && any_completed && node.is_some() {
            sessions.clear();
            ops.retain(|o| o.id.is_empty());
            if let Some(n) = node.as_mut() {
                let _ = n.send_text(br#"{"type":"bye"}"#);
                let _ = n.send_close(1000, b"no sessions left");
            }
            node = None;
            node_since = None;
        }
        // A sessionless socket in Lonely mode ends after its grace: the
        // `--once` exit-1 path needs a clean end with zero sessions, and
        // no live relay idles a socket that never serves.
        if mode == Mode::Lonely
            && let Some(since) = node_since
            && node.is_some()
            && since.elapsed() > Duration::from_secs(1)
        {
            if let Some(n) = node.as_mut() {
                let _ = n.send_text(br#"{"type":"bye"}"#);
                let _ = n.send_close(1000, b"no sessions");
            }
            node = None;
            node_since = None;
        }
    }
}
