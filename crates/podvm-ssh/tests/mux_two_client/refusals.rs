use super::*;

#[test]
fn operator_reports_a_reject_with_the_node_reason() {
    let relay = FakeRelay::start("node-tok-c", "conn-tok-c", Mode::RejectAll);
    let op = spawn_operator(&relay.origin(), "mux-c", "conn-tok-c", &[]);
    let st = op.proc_.wait_deadline("op", DEADLINE);
    assert_eq!(st.code(), Some(125));
    // The refusal names the reason and never the token.
    let start = Instant::now();
    loop {
        let lines = op.stderr.drain().join("\n");
        if lines.contains("no such server") {
            assert!(!lines.contains("conn-tok-c"), "token in stderr: {lines}");
            break;
        }
        if start.elapsed() >= DEADLINE {
            panic!("refusal never reported the reason; saw: {lines:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    relay.stop();
}

#[test]
fn operator_fails_loud_when_ready_never_comes() {
    // ⛔ "Loud" is asserted, not just the exit code: the stderr names the
    // wait that expired.
    let relay = FakeRelay::start("node-tok-d", "conn-tok-d", Mode::Silent);
    let op = spawn_operator(
        &relay.origin(),
        "mux-d",
        "conn-tok-d",
        &["--ready-wait", "2"],
    );
    let st = op.proc_.wait_deadline("op", DEADLINE);
    assert_eq!(st.code(), Some(125));
    let start = Instant::now();
    loop {
        let text = op.stderr.drain().join("\n");
        if text.contains("no ready frame within 2s") {
            break;
        }
        if start.elapsed() >= DEADLINE {
            panic!("ready-wait failure never named the wait");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    relay.stop();
}

#[test]
fn wrong_token_is_a_125_not_a_session() {
    // Both legs gate on the token: a wrong one fails the upgrade, and no
    // session is ever registered.
    let relay = FakeRelay::start("node-tok-e", "conn-tok-e", Mode::Normal);
    let op = spawn_operator(&relay.origin(), "mux-e", "wrong-tok", &[]);
    let st = op.proc_.wait_deadline("op", DEADLINE);
    assert_eq!(st.code(), Some(125));
    let (node, _) = spawn_node(
        &relay.origin(),
        "mux-e",
        "wrong-tok",
        &["--server", "cat", "--once"],
        &[],
    );
    let stn = node.wait_deadline("node", DEADLINE);
    assert_eq!(stn.code(), Some(125));
    {
        let r = relay.records.lock().unwrap();
        assert_eq!(r.wrong_tokens, 2, "{r:?}");
        assert_eq!(r.node_conns, 0, "{r:?}");
    }
    relay.stop();
}

#[test]
fn bad_invocations_exit_125_with_usage_before_any_byte() {
    // No relay is dialed: a bad origin, name, or token is a usage failure
    // naming its field before any byte.
    let node = env!("CARGO_BIN_EXE_node");
    for argv in [
        vec![
            "--relay",
            "http://example.com",
            "--name",
            "n",
            "--node-token",
            "t",
        ],
        vec![
            "--relay",
            "wss://example.com",
            "--name",
            "has space",
            "--node-token",
            "t",
        ],
        vec![
            "--relay",
            "wss://example.com",
            "--name",
            "n",
            "--node-token",
            "has space",
        ],
    ] {
        let out = Command::new(node).args(&argv).output().unwrap();
        assert_eq!(out.status.code(), Some(125), "{argv:?}");
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        assert!(err.contains("usage: node"), "{argv:?}: {err}");
    }
}

// ------------------------------------------------------------ hello refusals

#[test]
fn node_refuses_a_hello_it_does_not_speak() {
    // The hello version and caps are the dial gate: a wrong one fails the
    // dial with the relay named, and the socket never reaches sessions.
    for (hello, needle) in [
        (
            r#"{"type":"hello","version":2,"maxFrameBytes":65536,"maxSessions":64}"#,
            "version is 2",
        ),
        (
            r#"{"type":"hello","version":1,"maxFrameBytes":10,"maxSessions":64}"#,
            "maxFrameBytes 10",
        ),
        (
            r#"{"type":"hello","version":1,"maxFrameBytes":65536,"maxSessions":0}"#,
            "maxSessions 0",
        ),
        (
            // ⛔ Built at runtime and leaked for the 'static the relay mode
            // holds: the committed source carries no 32-hex literal,
            // which a scanner reading a value rather than a shape would
            // report, while the value stays a valid id,
            // which the "not hello" diagnosis needs.
            &*Box::leak(
                format!(r#"{{"type":"open","id":"{id}"}}"#, id = mint_id()).into_boxed_str(),
            ),
            "not hello",
        ),
    ] {
        let relay = FakeRelay::start("node-tok-h", "conn-tok-h", Mode::CustomHello(hello));
        let (node, node_err) = spawn_node(
            &relay.origin(),
            "mux-h",
            "node-tok-h",
            &["--server", "cat", "--once"],
            &[],
        );
        let st = node.wait_deadline("node", DEADLINE);
        assert_eq!(st.code(), Some(125));
        node_err.wait_contains("node", needle, DEADLINE);
        relay.stop();
    }
}

#[test]
fn node_refuses_data_before_hello() {
    let relay = FakeRelay::start("node-tok-hb", "conn-tok-hb", Mode::BinaryFirst);
    let (node, node_err) = spawn_node(
        &relay.origin(),
        "mux-hb",
        "node-tok-hb",
        &["--server", "cat", "--once"],
        &[],
    );
    let st = node.wait_deadline("node", DEADLINE);
    assert_eq!(st.code(), Some(125));
    node_err.wait_contains("node", "data before hello", DEADLINE);
    relay.stop();
}

#[test]
fn once_with_no_session_exits_1_not_0() {
    // ⛔ The `--once` contract: a socket that ends with no completed
    // session is exit 1. Exit 0 with zero sessions would read as a proved
    // relay that proved nothing.
    let relay = FakeRelay::start("node-tok-o", "conn-tok-o", Mode::Lonely);
    let (node, node_err) = spawn_node(
        &relay.origin(),
        "mux-o",
        "node-tok-o",
        &["--server", "cat", "--once"],
        &[],
    );
    node_err.wait_contains("node", "registered as mux-o", DEADLINE);
    let st = node.wait_deadline("node", DEADLINE);
    assert_eq!(st.code(), Some(1));
    node_err.wait_contains("node end", "0 completed", DEADLINE);
    relay.stop();
}

// ------------------------------------------------------------ table pressure

#[test]
fn third_session_past_the_table_is_refused_not_wedged() {
    // The relay announced two sessions: the third open earns a `reject`,
    // and the first two keep working on the same socket.
    let hello = r#"{"type":"hello","version":1,"maxFrameBytes":65536,"maxSessions":2}"#;
    let relay = FakeRelay::start("node-tok-t", "conn-tok-t", Mode::CustomHello(hello));
    let (node, node_err) = spawn_node(
        &relay.origin(),
        "mux-t",
        "node-tok-t",
        &["--server", "cat", "--once"],
        &[],
    );
    node_err.wait_contains("node", "registered as mux-t", DEADLINE);
    let mut op1 = spawn_operator(&relay.origin(), "mux-t", "conn-tok-t", &[]);
    let mut op2 = spawn_operator(&relay.origin(), "mux-t", "conn-tok-t", &[]);
    node_err.wait_contains("node", "session opened (2 active)", DEADLINE);
    let op3 = spawn_operator(&relay.origin(), "mux-t", "conn-tok-t", &[]);
    let st3 = op3.proc_.wait_deadline("op3", DEADLINE);
    assert_eq!(st3.code(), Some(125));
    // The first two still echo: the refusal took no session with it.
    let mut s1 = op1.stdin.take().unwrap();
    s1.write_all(b"still-here").unwrap();
    s1.flush().unwrap();
    op1.stdout
        .read_exact("op1 after refusal", b"still-here", IO_DEADLINE);
    drop(s1);
    drop(op2.stdin.take());
    assert_eq!(op1.proc_.wait_deadline("op1", DEADLINE).code(), Some(0));
    assert_eq!(op2.proc_.wait_deadline("op2", DEADLINE).code(), Some(0));
    assert_eq!(node.wait_deadline("node", DEADLINE).code(), Some(0));
    node_err.wait_contains("node end", "2 completed", DEADLINE);
    {
        let r = relay.records.lock().unwrap();
        assert_eq!(r.node_conns, 1, "{r:?}");
        assert_eq!(r.sessions.len(), 3, "{r:?}");
        assert_eq!(
            r.sessions.iter().filter(|s| s.completed).count(),
            2,
            "{r:?}"
        );
        assert_eq!(r.sessions.iter().filter(|s| s.rejected).count(), 1, "{r:?}");
        assert!(
            r.sessions.iter().all(|s| s.completed != s.rejected),
            "{r:?}"
        );
    }
    relay.stop();
}

// ------------------------------------------------------------ operator edges

#[test]
fn megabyte_queued_with_no_ready_fails_loud() {
    // The pre-ready queue is bounded: past a megabyte with no `ready` the
    // session is never coming, and the operator says so instead of
    // buffering forever.
    let relay = FakeRelay::start("node-tok-q", "conn-tok-q", Mode::Silent);
    let mut op = spawn_operator(
        &relay.origin(),
        "mux-q",
        "conn-tok-q",
        &["--ready-wait", "30"],
    );
    let mut stdin = op.stdin.take().unwrap();
    stdin.write_all(&vec![0u8; 1024 * 1024 + 1]).unwrap();
    stdin.flush().unwrap();
    let st = op.proc_.wait_deadline("op", DEADLINE);
    assert_eq!(st.code(), Some(125));
    relay.stop();
}

#[test]
fn bye_before_ready_is_a_failure_not_a_session() {
    let relay = FakeRelay::start("node-tok-y", "conn-tok-y", Mode::Bye);
    let op = spawn_operator(&relay.origin(), "mux-y", "conn-tok-y", &[]);
    let st = op.proc_.wait_deadline("op", DEADLINE);
    assert_eq!(st.code(), Some(125));
    relay.stop();
}

#[test]
fn node_death_mid_session_closes_the_operator_loud() {
    // The relay answers a dead node with 1011 on the operator: the session
    // ends with the code and the reason, never with silence.
    let relay = FakeRelay::start("node-tok-k", "conn-tok-k", Mode::Normal);
    let (mut node, node_err) = spawn_node(
        &relay.origin(),
        "mux-k",
        "node-tok-k",
        &["--server", "cat", "--once"],
        &[],
    );
    node_err.wait_contains("node", "registered as mux-k", DEADLINE);
    let mut op = spawn_operator(&relay.origin(), "mux-k", "conn-tok-k", &[]);
    node_err.wait_contains("node", "session opened (1 active)", DEADLINE);
    let mut stdin = op.stdin.take().unwrap();
    stdin.write_all(b"alive").unwrap();
    stdin.flush().unwrap();
    op.stdout.read_exact("op echo", b"alive", IO_DEADLINE);
    node.kill();
    let st = op.proc_.wait_deadline("op", DEADLINE);
    assert_eq!(st.code(), Some(125));
    // The close carries its code: 1011 names the dead node, and a bare
    // "connection closed" would not.
    let start = Instant::now();
    loop {
        let text = op.stderr.drain().join("\n");
        if text.contains("1011") {
            break;
        }
        if start.elapsed() >= DEADLINE {
            panic!("abnormal close never carried its code");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    relay.stop();
}

#[test]
fn bad_server_command_ends_the_session_at_once() {
    // ⛔ What this pins, against the old promise of a `reject`: the server
    // runs through `/bin/sh -c` (`server.rs`), so a bad command still
    // spawns the shell, which dies at once. The node answers `ready` at
    // `open` (the relay reaps a slow answer), sees EOF on the next poll,
    // and ends the session with a clean `close`. The operator exits 0,
    // and no diagnosis is invented: nothing was refused.
    let relay = FakeRelay::start("node-tok-r", "conn-tok-r", Mode::Normal);
    let (node, node_err) = spawn_node(
        &relay.origin(),
        "mux-r",
        "node-tok-r",
        &["--server", "podvm-ssh-no-such-server-xyz", "--once"],
        &[],
    );
    node_err.wait_contains("node", "registered as mux-r", DEADLINE);
    let op = spawn_operator(&relay.origin(), "mux-r", "conn-tok-r", &[]);
    let st = op.proc_.wait_deadline("op", DEADLINE);
    assert_eq!(st.code(), Some(0));
    assert_eq!(node.wait_deadline("node", DEADLINE).code(), Some(0));
    node_err.wait_contains("node end", "1 completed", DEADLINE);
    // A clean end carries no token on either stderr.
    for line in op.stderr.drain() {
        assert!(!line.contains("conn-tok-r"), "token in stderr: {line}");
    }
    for line in node_err.drain() {
        assert!(!line.contains("node-tok-r"), "token in stderr: {line}");
    }
    {
        let r = relay.records.lock().unwrap();
        assert_eq!(r.node_conns, 1, "{r:?}");
        assert_eq!(r.sessions.len(), 1, "{r:?}");
        assert!(r.sessions.iter().all(|s| s.completed), "{r:?}");
        assert!(r.sessions.iter().all(|s| !s.rejected), "{r:?}");
    }
    relay.stop();
}
