use super::*;

pub(super) fn is_hex32(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Parse a TEXT control into (type, id), or refuse it. The fake relay needs
/// the shape, not the full codec: the codec is the unit under test only on
/// the node and operator sides.
pub(super) fn text_control(t: &[u8]) -> Result<(String, String), ()> {
    let v: serde_json::Value = serde_json::from_slice(t).map_err(|_| ())?;
    let kind = v
        .get("type")
        .and_then(|k| k.as_str())
        .ok_or(())?
        .to_string();
    let id = v
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();
    if matches!(kind.as_str(), "ready" | "reject" | "close") && !is_hex32(&id) {
        return Err(());
    }
    Ok((kind, id))
}

pub(super) fn mark_completed(records: &Arc<Mutex<Records>>, id: &str) {
    for s in records.lock().unwrap().sessions.iter_mut() {
        if s.id == id {
            s.completed = true;
        }
    }
}

pub(super) fn mark_rejected(records: &Arc<Mutex<Records>>, id: &str) {
    for s in records.lock().unwrap().sessions.iter_mut() {
        if s.id == id {
            s.rejected = true;
        }
    }
}

pub(super) fn add_bytes(records: &Arc<Mutex<Records>>, id: &str, op_bytes: u64, node_bytes: u64) {
    for s in records.lock().unwrap().sessions.iter_mut() {
        if s.id == id {
            s.op_bytes += op_bytes;
            s.node_bytes += node_bytes;
        }
    }
}

// ------------------------------------------------------------ drivers

pub(super) struct Operator {
    pub(super) proc_: Proc,
    pub(super) stdin: Option<std::process::ChildStdin>,
    pub(super) stdout: Bytes,
    pub(super) stderr: Lines,
}

pub(super) fn spawn_operator(origin: &str, name: &str, token: &str, extra: &[&str]) -> Operator {
    let exe = env!("CARGO_BIN_EXE_operator");
    let mut cmd = Command::new(exe);
    cmd.args(["--relay", origin, "--name", name, "--connect-token", token])
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut proc_ = Proc::spawn(cmd);
    let stdin = Some(proc_.stdin());
    let child = proc_.child.as_mut().unwrap();
    let stdout = Bytes::spawn(child.stdout.take().unwrap());
    let stderr = Lines::spawn(child.stderr.take().unwrap());
    Operator {
        proc_,
        stdin,
        stdout,
        stderr,
    }
}

pub(super) fn spawn_node(
    origin: &str,
    name: &str,
    token: &str,
    extra: &[&str],
    envs: &[(String, String)],
) -> (Proc, Lines) {
    let exe = env!("CARGO_BIN_EXE_node");
    let mut cmd = Command::new(exe);
    cmd.args(["--relay", origin, "--name", name, "--node-token", token])
        .args(extra)
        .envs(envs.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut proc_ = Proc::spawn(cmd);
    let child = proc_.child.as_mut().unwrap();
    let stderr = Lines::spawn(child.stderr.take().unwrap());
    (proc_, stderr)
}

fn big_payload(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i % 251) as u8).collect()
}

/// Failure evidence for the relay tests: on panic the stack unwinds through
/// this guard, which prints the relay's records and the node's last lines
/// into the captured test output. A hang with no evidence is undebuggable.
struct Evidence {
    records: Arc<Mutex<Records>>,
    node_err: Lines,
    done: bool,
}

impl Drop for Evidence {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        let r = self.records.lock().unwrap();
        eprintln!(
            "EVIDENCE node_conns={} sessions={:?} closes={:?} wrong={}",
            r.node_conns, r.sessions, r.closes, r.wrong_tokens
        );
        eprintln!("EVIDENCE node stderr: {:?}", self.node_err.drain());
    }
}

// ------------------------------------------------------------ test A: mechanics

#[test]
fn two_sessions_share_one_node_connection_through_cat() {
    // ⛔ The assertion this pins: one node socket, two completed sessions,
    // isolated in both directions, close-one-continues, and a 200000-byte
    // transfer that must cross as several frames.
    let relay = FakeRelay::start("node-tok", "conn-tok", Mode::Normal);
    let name = "mux-a";
    let (node, node_err) = spawn_node(
        &relay.origin(),
        name,
        "node-tok",
        &["--server", "cat", "--once"],
        &[],
    );
    node_err.wait_contains("node", "registered as mux-a", DEADLINE);

    let mut op1 = spawn_operator(&relay.origin(), name, "conn-tok", &[]);
    let mut op2 = spawn_operator(&relay.origin(), name, "conn-tok", &[]);
    // Both sessions opened on the node: each line means a server spawned
    // and its `ready` answered, so bytes written next cannot race it.
    node_err.wait_contains("node", "session opened (1 active)", DEADLINE);
    node_err.wait_contains("node", "session opened (2 active)", DEADLINE);

    // Data both ways, distinct markers per session.
    let mut s1 = op1.stdin.take().unwrap();
    let mut s2 = op2.stdin.take().unwrap();
    s1.write_all(b"op1-marker").unwrap();
    s1.flush().unwrap();
    op1.stdout
        .read_exact("op1 echo", b"op1-marker", IO_DEADLINE);
    s2.write_all(b"op2-marker").unwrap();
    s2.flush().unwrap();
    op2.stdout
        .read_exact("op2 echo", b"op2-marker", IO_DEADLINE);

    // Isolation: neither side holds the other's bytes.
    op1.stdout
        .read_timeout_is_quiet("op1 after op2", Duration::from_millis(500));
    op2.stdout
        .read_timeout_is_quiet("op2 after op1", Duration::from_millis(500));

    // A 200000-byte transfer must cross as several 65504-byte frames.
    let big = big_payload(200_000);
    s1.write_all(&big).unwrap();
    s1.flush().unwrap();
    op1.stdout.read_exact("op1 big transfer", &big, IO_DEADLINE);

    // The keepalive shape the live relay sends: empty frames change nothing.
    op1.stdout
        .read_timeout_is_quiet("op1 keepalive window", Duration::from_millis(300));

    // Close one: it exits 0, and the other continues on the same socket.
    drop(s1);
    let st1 = op1.proc_.wait_deadline("op1", DEADLINE);
    assert_eq!(st1.code(), Some(0));
    wait_for("first session completed", DEADLINE, || {
        let r = relay.records.lock().unwrap();
        (r.sessions.iter().filter(|s| s.completed).count() == 1).then_some(())
    });
    s2.write_all(b"op2-still-here").unwrap();
    s2.flush().unwrap();
    op2.stdout
        .read_exact("op2 after op1 closed", b"op2-still-here", IO_DEADLINE);

    // Close the other: the relay ends the socket, the `--once` node exits 0.
    drop(s2);
    let st2 = op2.proc_.wait_deadline("op2", DEADLINE);
    assert_eq!(st2.code(), Some(0));
    let stn = node.wait_deadline("node", DEADLINE);
    assert_eq!(stn.code(), Some(0));
    node_err.wait_contains("node end", "2 completed", DEADLINE);

    {
        let r = relay.records.lock().unwrap();
        assert_eq!(r.node_conns, 1, "one node socket, got {:?}", r);
        assert_eq!(r.sessions.len(), 2, "two sessions, got {:?}", r);
        assert!(r.sessions.iter().all(|s| s.completed), "{r:?}");
        assert!(
            r.sessions
                .iter()
                .all(|s| s.node_bytes > 0 && s.op_bytes > 0),
            "{r:?}"
        );
        assert_ne!(r.sessions[0].id, r.sessions[1].id);
        assert_eq!(r.wrong_tokens, 0);
    }
    relay.stop();
}

// ------------------------------------------------------------ test A2: reconnect

#[test]
fn node_redial_pairs_a_second_session_after_the_first_socket_drops() {
    // ⛔ The assertion this pins: one dropped node socket, one redial of the
    // same name, and a second operator session pairing on the new socket.
    // Both sessions complete with exact bytes; the connection count proves
    // exactly one redial, and the port rebind proves no listener is left.
    // The drop lands after the first session completed: a mid-frame kill
    // would strand that session, and two completions could never follow.
    let relay = FakeRelay::start("node-tok-z", "conn-tok-z", Mode::Reconnect);
    let port = relay.port;
    let name = "mux-z";
    // ⛔ No `--once`: the node under test must redial on loss, which exits
    // only when killed. `cat` stands in for the server, as in test A.
    let (mut node, node_err) = spawn_node(
        &relay.origin(),
        name,
        "node-tok-z",
        &["--server", "cat"],
        &[],
    );
    node_err.wait_contains("node", "registered as mux-z", DEADLINE);

    // Session 1 completes fully on the first socket.
    let mut op1 = spawn_operator(&relay.origin(), name, "conn-tok-z", &[]);
    node_err.wait_contains("node", "session opened (1 active)", DEADLINE);
    let mut s1 = op1.stdin.take().unwrap();
    s1.write_all(b"before-drop").unwrap();
    s1.flush().unwrap();
    op1.stdout
        .read_exact("op1 echo", b"before-drop", IO_DEADLINE);
    drop(s1);
    let st1 = op1.proc_.wait_deadline("op1", DEADLINE);
    assert_eq!(st1.code(), Some(0));

    // The relay aborted the first socket after that completion. The node
    // redials the same name: the second registration line is the proof, and
    // the count below pins it to exactly one redial.
    wait_for("node redial", DEADLINE, || {
        (relay.records.lock().unwrap().node_conns == 2).then_some(())
    });
    node_err.wait_contains("node redial", "registered as mux-z", DEADLINE);

    // Session 2 pairs on the new socket and completes with exact bytes.
    let mut op2 = spawn_operator(&relay.origin(), name, "conn-tok-z", &[]);
    node_err.wait_contains("node", "session opened (1 active)", DEADLINE);
    let mut s2 = op2.stdin.take().unwrap();
    s2.write_all(b"after-redial").unwrap();
    s2.flush().unwrap();
    op2.stdout
        .read_exact("op2 echo", b"after-redial", IO_DEADLINE);
    drop(s2);
    let st2 = op2.proc_.wait_deadline("op2", DEADLINE);
    assert_eq!(st2.code(), Some(0));

    // A non-`--once` node runs until killed: end it, then judge the records.
    node.kill();
    {
        let r = relay.records.lock().unwrap();
        assert_eq!(r.node_conns, 2, "one drop and one redial, got {:?}", r);
        assert_eq!(r.sessions.len(), 2, "two sessions, got {:?}", r);
        assert!(r.sessions.iter().all(|s| s.completed), "{r:?}");
        assert!(
            r.sessions
                .iter()
                .all(|s| s.node_bytes > 0 && s.op_bytes > 0),
            "{r:?}"
        );
        assert_ne!(r.sessions[0].id, r.sessions[1].id);
        assert_eq!(r.wrong_tokens, 0);
    }
    relay.stop();
    // No listener left behind: the relay's port rebinds at once.
    assert!(
        TcpListener::bind(("127.0.0.1", port)).is_ok(),
        "relay port {port} still held after stop"
    );
}

// ------------------------------------------------------------ test B: real SSH

#[test]
fn three_authenticated_ssh_sessions_share_one_node_connection() {
    // ⛔ The assertion this pins: three authenticated SSH sessions through
    // one registered node connection, two concurrent at a time, exact bytes
    // and exit codes in both directions, and the first session surviving
    // the others' close. Three sessions exceed T-1403's two-session proof;
    // the count is exact everywhere it is asserted.
    require_binary("sshd");
    require_binary("ssh");
    let keygen = require_binary("ssh-keygen");
    let tmp = fresh_tmp("podvm-ssh-mux");
    ensure_privsep();
    let keys = make_keys(&keygen, &tmp);
    let user = current_user();

    let relay = FakeRelay::start("node-tok-b", "conn-tok-b", Mode::Normal);
    let name = "mux-b";
    // ⛔ No `--server`: the node detects and probes sshd itself, which is
    // the production default this test must exercise rather than inject.
    let envs = vec![
        (
            "PODSSH_RUNTIME".to_string(),
            tmp.join("rt").display().to_string(),
        ),
        (
            "PODSSH_AUTHORIZED_KEYS".to_string(),
            keys.authorized.display().to_string(),
        ),
    ];
    // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
    unsafe { std::env::remove_var("PODSSH_SERVER") };
    let (node, node_err) = spawn_node(&relay.origin(), name, "node-tok-b", &["--once"], &envs);
    node_err.wait_contains("node", "registered as mux-b", DEADLINE);
    // ⛔ `node_err` moves into the evidence guard: later waits go through
    // it, so a failure still prints everything the node said.
    let mut evidence = Evidence {
        records: relay.records.clone(),
        node_err,
        done: false,
    };

    let operator = env!("CARGO_BIN_EXE_operator");
    let proxy_cmd = format!(
        "'{operator}' --relay '{}' --name '{name}' --connect-token 'conn-tok-b'",
        relay.origin()
    );

    // Client A stays connected across client B's whole session: its second
    // half proves the first session survived the second's close.
    let user_key_a = keys.user.clone();
    let user_a = user.clone();
    let proxy_a = proxy_cmd.clone();
    let handle_a = std::thread::spawn(move || {
        run_ssh(
            &proxy_a,
            &user_a,
            &user_key_a,
            &["printf first; sleep 3; printf second"],
            None,
        )
    });
    // Client B exits 42 while A sleeps; the code must pass through intact.
    std::thread::sleep(Duration::from_millis(500));
    eprintln!("MARKER launching exit-42 client");
    let out_b = run_ssh(&proxy_cmd, &user, &keys.user, &["exit", "42"], None);
    eprintln!("MARKER exit-42 client done");
    assert_eq!(out_b.status.code(), Some(42), "stderr: {:?}", out_b.stderr);

    // A 200000-byte round trip through `cat` on the surviving session: both
    // directions at once, compared exactly.
    let big = big_payload(200_000);
    eprintln!("MARKER launching 200000-byte client");
    let out_cat = run_ssh(&proxy_cmd, &user, &keys.user, &["cat"], Some(big.clone()));
    eprintln!("MARKER 200000-byte client done");
    assert_eq!(
        out_cat.status.code(),
        Some(0),
        "stderr: {:?}",
        out_cat.stderr
    );
    assert_eq!(out_cat.stdout, big);

    let out_a = handle_a.join().unwrap();
    eprintln!("MARKER client A done");
    assert_eq!(out_a.stdout, b"firstsecond", "stderr: {:?}", out_a.stderr);
    assert_eq!(out_a.status.code(), Some(0));

    let stn = node.wait_deadline("node", DEADLINE);
    assert_eq!(stn.code(), Some(0));
    evidence
        .node_err
        .wait_contains("node end", "3 completed", DEADLINE);

    {
        let r = relay.records.lock().unwrap();
        assert_eq!(r.node_conns, 1, "one node socket, got {:?}", r);
        assert_eq!(r.sessions.len(), 3, "three sessions, got {:?}", r);
        assert!(r.sessions.iter().all(|s| s.completed), "{r:?}");
    }
    evidence.done = true;
    let _ = std::fs::remove_dir_all(&tmp);
    relay.stop();
}

// ------------------------------------------------------------ test C: refusals
