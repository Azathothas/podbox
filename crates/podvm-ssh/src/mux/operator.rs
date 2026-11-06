use super::*;

/// Split a received node frame into its session id and payload. The relay
/// prepends the real id to the operator's bare bytes, so the prefix is
/// always there; a frame without one is a relay violation, failed loud.
pub(super) fn split_node_frame(b: &[u8]) -> Result<(&str, &[u8]), Error> {
    if b.len() < ID_LEN {
        return Err(Error::pump(io::Error::other(format!(
            "node frame of {} bytes carries no 32-byte id",
            b.len()
        ))));
    }
    let (id, payload) = b.split_at(ID_LEN);
    let id = std::str::from_utf8(id)
        .map_err(|_| Error::pump(io::Error::other("node frame id is not UTF-8")))?;
    if !is_session_id(id) {
        return Err(Error::pump(io::Error::other("node frame id is not 32 hex")));
    }
    Ok((id, payload))
}

/// Read the TEXT `hello`, and derive this socket's caps from it. A missing,
/// versioned-wrong, or unusable hello fails the dial: the socket never
/// reaches the session loop. One read, not a loop: the timeout answers
/// "nothing arrived", and anything that did arrive is judged at once.
pub(super) fn read_hello(ws: &mut Ws, relay: &str) -> Result<(usize, usize), Error> {
    let op = format!("relay hello {relay}");
    ws.set_read_timeout(Some(HELLO_WAIT))
        .map_err(|e| Error::dial(op.clone(), e))?;
    match ws.read_frame().map_err(|e| Error::dial(op.clone(), e))? {
        None => Err(Error::dial(
            op,
            io::Error::new(io::ErrorKind::TimedOut, "no hello from the relay"),
        )),
        Some(Frame::Text(t)) => {
            let control =
                parse_control(&t).map_err(|m| Error::dial(op.clone(), io::Error::other(m)))?;
            match control {
                Control::Hello {
                    version,
                    max_frame,
                    max_sessions,
                } => {
                    if version != HELLO_VERSION {
                        return Err(Error::dial(
                            op,
                            io::Error::other(format!(
                                "relay hello version is {version}, this node speaks {HELLO_VERSION}"
                            )),
                        ));
                    }
                    if max_frame < (ID_LEN as u64 + 1) || max_frame > (16 * 1024 * 1024) {
                        return Err(Error::dial(
                            op,
                            io::Error::other(format!(
                                "relay maxFrameBytes {max_frame} is unusable"
                            )),
                        ));
                    }
                    if max_sessions == 0 || max_sessions > 1024 {
                        return Err(Error::dial(
                            op,
                            io::Error::other(format!(
                                "relay maxSessions {max_sessions} is unusable"
                            )),
                        ));
                    }
                    let cap = std::cmp::min(MAX_PAYLOAD as u64, max_frame - ID_LEN as u64);
                    Ok((cap as usize, max_sessions as usize))
                }
                other => Err(Error::dial(
                    op,
                    io::Error::other(format!("first relay control is {other:?}, not hello")),
                )),
            }
        }
        Some(Frame::Binary(_)) => Err(Error::dial(
            op,
            io::Error::other("relay sent data before hello"),
        )),
        Some(Frame::Close(code, reason)) => Err(Error::dial(
            op,
            io::Error::other(format!(
                "relay closed during hello: {code} {}",
                String::from_utf8_lossy(&reason)
            )),
        )),
    }
}

/// Start one session: spawn the server, answer `ready` at once, or `reject`
/// when the spawn fails or the table is full. The operator waits on this
/// answer (15 s, then the relay reaps it), so the spawn and the reply share
/// one turn of the loop.
pub(super) fn open_session(
    ws: &mut Ws,
    sessions: &mut HashMap<String, Session>,
    server_command: &str,
    id: &str,
    max_sessions: usize,
    stats: &mut NodeStats,
) -> Result<(), Error> {
    if sessions.len() >= max_sessions {
        stats.sessions_refused += 1;
        let c = Control::Reject {
            id: id.to_string(),
            reason: "node is full".to_string(),
        };
        let raw = emit_control(&c).map_err(|m| Error::pump(io::Error::other(m)))?;
        ws.send_text(&raw)
            .map_err(|e| leg_err("mux node-leg send", e))?;
        return Ok(());
    }
    match crate::server::spawn_stdio(server_command) {
        Ok(server) => {
            server
                .stream
                .set_read_timeout(Some(SESSION_POLL))
                .map_err(Error::pump)?;
            // ⛔ The server leg carries the write bound too: the socketpair
            // to the server is a kernel deadline, so a server that stops
            // reading ends its session instead of wedging every other one.
            server
                .stream
                .set_write_timeout(Some(WRITE_DEADLINE))
                .map_err(Error::pump)?;
            sessions.insert(
                id.to_string(),
                Session {
                    server,
                    inbox: VecDeque::new(),
                },
            );
            eprintln!("podvm-ssh: session opened ({} active)", sessions.len());
            let c = Control::Ready { id: id.to_string() };
            let raw = emit_control(&c).map_err(|m| Error::pump(io::Error::other(m)))?;
            ws.send_text(&raw)
                .map_err(|e| leg_err("mux node-leg send", e))?;
            Ok(())
        }
        Err(e) => {
            stats.sessions_refused += 1;
            let mut reason = e.to_string();
            if reason.len() > 200 {
                reason.truncate(200);
            }
            let c = Control::Reject {
                id: id.to_string(),
                reason,
            };
            let raw = emit_control(&c).map_err(|m| Error::pump(io::Error::other(m)))?;
            ws.send_text(&raw)
                .map_err(|e| leg_err("mux node-leg send", e))?;
            Ok(())
        }
    }
}

/// Tear down one session. Returns true when a session actually ended.
pub(super) fn close_session(sessions: &mut HashMap<String, Session>, id: &str) -> bool {
    if sessions.remove(id).is_some() {
        eprintln!("podvm-ssh: session closed ({} active)", sessions.len());
        true
    } else {
        false
    }
}

/// One turn of session IO: server bytes go out id-prefixed and chunked,
/// relay bytes go into server stdin, and a server at EOF ends its session
/// with a `close` of our own. Write failures are loud; a relay that will
/// not take a frame ends the socket through the caller's error.
pub(super) fn poll_sessions(
    ws: &mut Ws,
    sessions: &mut HashMap<String, Session>,
    payload_cap: usize,
    stats: &mut NodeStats,
) -> Result<(), Error> {
    let mut ended: Vec<String> = Vec::new();
    let mut outbox: Vec<(String, Vec<u8>)> = Vec::new();
    for (id, s) in sessions.iter_mut() {
        // Relay bytes first, so a queued `close` cannot strand input.
        while let Some(chunk) = s.inbox.pop_front() {
            match s.server.stream.write_all(&chunk) {
                Ok(()) => {}
                Err(e) => {
                    // ⛔ A server that will not take its stdin ends its own
                    // session, loudly: the stream carries `WRITE_DEADLINE`,
                    // so any error here (including the kernel's
                    // `WouldBlock` for an expired deadline) means the whole
                    // bound passed with no progress. The stall names the
                    // leg, the session closes, and the rest survive on the
                    // same socket. Requeueing it instead would retry a
                    // stalled pipe forever.
                    let e = write_stall(e, "server-stdin leg");
                    eprintln!("podvm-ssh: session ending: {e}");
                    ended.push(id.clone());
                    break;
                }
            }
        }
        if ended.contains(id) {
            continue;
        }
        let _ = s.server.stream.flush();
        let mut buf = vec![0u8; payload_cap];
        match s.server.stream.read(&mut buf) {
            Ok(0) => ended.push(id.clone()),
            Ok(n) => {
                // ⛔ Chunked at the announced cap, never over: one oversize
                // frame closes this whole socket with 1009, taking every
                // other session with it.
                for piece in buf[..n].chunks(payload_cap) {
                    let mut framed = Vec::with_capacity(ID_LEN + piece.len());
                    framed.extend_from_slice(id.as_bytes());
                    framed.extend_from_slice(piece);
                    outbox.push((id.clone(), framed));
                }
            }
            Err(e) if crate::pump::is_would_block(&e) => {}
            Err(_) => ended.push(id.clone()),
        }
    }
    for (_id, framed) in outbox {
        ws.send_binary(&framed)
            .map_err(|e| leg_err("mux node-leg send", e))?;
    }
    for id in ended {
        if close_session(sessions, &id) {
            stats.sessions_completed += 1;
            let c = Control::Close { id };
            let raw = emit_control(&c).map_err(|m| Error::pump(io::Error::other(m)))?;
            ws.send_text(&raw)
                .map_err(|e| leg_err("mux node-leg send", e))?;
        }
    }
    Ok(())
}

// ------------------------------------------------------------ the operator

/// What the operator process needs. Standard input and output are the SSH
/// client's; this process is a valid `ProxyCommand`.
pub struct OperatorConfig {
    pub relay: String,
    pub name: String,
    pub connect_token: String,
    /// How long to wait for the node's `ready` before failing.
    pub ready_wait: Duration,
}

impl OperatorConfig {
    pub fn new(relay: String, name: String, connect_token: String) -> Self {
        OperatorConfig {
            relay,
            name,
            connect_token,
            ready_wait: DEFAULT_READY_WAIT,
        }
    }
}

/// Run the operator: connect, wait for `ready`, then move stdin to the relay
/// as bare frames and relay bytes to stdout. Stdin bytes before `ready` are
/// queued, bounded: the relay closes an early writer with 1008, so sending
/// them straight through would end the session it was meant to start.
pub fn run_operator(cfg: &OperatorConfig, io: &mut dyn Stream) -> Result<(), Error> {
    validate_name(&cfg.name)?;
    let path = format!("/v1/connect/{}", cfg.name);
    let mut ws = dial_ws(&cfg.relay, &path, &cfg.connect_token)?;
    ws.set_read_timeout(Some(SOCKET_POLL))
        .map_err(Error::pump)?;
    // ⛔ The operator owns the timeout invariant like the pump does. A
    // caller that forgot it would hand the loop a stream that blocks
    // forever; standard input ignores it safely through its drain thread.
    io.set_read_timeout(Some(SOCKET_POLL))
        .map_err(Error::pump)?;
    // ⛔ The client-stdout leg carries the write bound where the kernel
    // allows one: a client that stops reading ends the op inside the bound.
    // A pipe-backed client ignores it safely; its stall is outside this
    // loop's contract the way its read already is.
    io.set_write_timeout(Some(WRITE_DEADLINE))
        .map_err(Error::pump)?;
    let start = Instant::now();
    let mut ready = false;
    let mut queued: Vec<u8> = Vec::new();
    let mut io_open = true;
    let mut goodbye_by: Option<Instant> = None;
    loop {
        if !ready && start.elapsed() > cfg.ready_wait {
            return Err(Error::new(
                Kind::Pump,
                "mux operator wait",
                format!(
                    "no ready frame within {}s; the node never answered open",
                    cfg.ready_wait.as_secs()
                ),
            ));
        }
        if let Some(by) = goodbye_by
            && Instant::now() >= by
        {
            break;
        }
        match ws.read_frame().map_err(Error::pump)? {
            None => {}
            Some(Frame::Text(t)) => {
                let control = parse_control(&t).map_err(|m| Error::pump(io::Error::other(m)))?;
                match control {
                    Control::Ready { .. } => {
                        // ⛔ Silent on success: this process is an SSH
                        // `ProxyCommand`, and its stderr reaches the
                        // client's. A line per connection is noise; a
                        // failure still reports exactly once.
                        ready = true;
                        if !queued.is_empty() {
                            send_bare(&mut ws, &queued)?;
                            queued.clear();
                        }
                    }
                    Control::Reject { reason, .. } => {
                        return Err(Error::new(
                            Kind::Pump,
                            "mux operator session",
                            format!("the node refused the session: {reason}"),
                        ));
                    }
                    Control::Close { .. } | Control::Bye => break,
                    other => {
                        return Err(Error::pump(io::Error::other(format!(
                            "relay sent unexpected operator control {other:?}"
                        ))));
                    }
                }
            }
            Some(Frame::Binary(b)) => {
                if b.is_empty() {
                    continue; // the 25 s keepalive
                }
                // ⛔ No session exists before `ready`, so bytes before it
                // are a relay violation, failed loud: writing them into
                // the client's handshake would corrupt it opaquely.
                if !ready {
                    return Err(Error::pump(io::Error::other(
                        "relay sent data before ready",
                    )));
                }
                io.write_all(&b).map_err(|e| {
                    leg_err(
                        "mux operator client-stdout",
                        write_stall(e, "client-stdout write"),
                    )
                })?;
                io.flush().map_err(|e| {
                    leg_err(
                        "mux operator client-stdout",
                        write_stall(e, "client-stdout write"),
                    )
                })?;
            }
            Some(Frame::Close(code, reason)) => {
                let reason = String::from_utf8_lossy(&reason).to_string();
                if !ready {
                    return Err(Error::new(
                        Kind::Pump,
                        "mux operator session",
                        format!("relay closed before ready: {code} {reason}"),
                    ));
                }
                if code != 1000 {
                    return Err(Error::new(
                        Kind::Pump,
                        "mux operator session",
                        format!("relay closed the session: {code} {reason}"),
                    ));
                }
                break;
            }
        }
        if io_open {
            let mut buf = [0u8; 65536];
            match io.read(&mut buf) {
                Ok(0) => {
                    io_open = false;
                    // ⛔ The goodbye is a close frame, and then a bounded
                    // wait for the relay's answer. Exiting at once would
                    // strand the session's last bytes on a socket the relay
                    // may still be draining.
                    let _ = ws.send_close(1000, b"done");
                    goodbye_by = Some(Instant::now() + GOODBYE_WAIT);
                }
                Ok(n) => {
                    if !ready {
                        if queued.len() + n > PREREADY_CAP {
                            return Err(Error::new(
                                Kind::Pump,
                                "mux operator wait",
                                "a megabyte queued with no ready frame; the session is never coming",
                            ));
                        }
                        queued.extend_from_slice(&buf[..n]);
                    } else {
                        send_bare(&mut ws, &buf[..n])?;
                    }
                }
                Err(e) if crate::pump::is_would_block(&e) => {}
                Err(_) => {
                    io_open = false;
                    let _ = ws.send_close(1000, b"done");
                    goodbye_by = Some(Instant::now() + GOODBYE_WAIT);
                }
            }
        }
    }
    if !ready {
        return Err(Error::new(
            Kind::Pump,
            "mux operator session",
            "the relay ended the socket before ready",
        ));
    }
    Ok(())
}

/// Send session bytes as bare binary frames, chunked at the cap. The relay
/// prepends the id on this leg; prefixing one here would have it doubled.
fn send_bare(ws: &mut Ws, bytes: &[u8]) -> Result<(), Error> {
    for piece in bytes.chunks(MAX_PAYLOAD) {
        ws.send_binary(piece)
            .map_err(|e| leg_err("mux operator-leg send", e))?;
    }
    Ok(())
}
