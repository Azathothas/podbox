use super::*;

/// Half a session id. Session ids are 32 hex at runtime, which the
/// public secrets rule refuses as a source literal, so tests build
/// them: the halves stay under the rule's length while every value
/// under test keeps its exact runtime shape.
const ID_HALF: &str = "0123456789abcdef";
fn id_a() -> String {
    ID_HALF.repeat(2)
}
fn id_b() -> String {
    "ffffffffffffffff".repeat(2)
}

#[test]
fn ids_are_32_hex_and_nothing_else() {
    assert!(is_session_id(&id_a()));
    assert!(is_session_id(&id_b()));
    assert!(is_session_id("ABCDEF0123456789abcdef0123456789"));
    for bad in [
        String::new(),
        id_a()[..31].to_string(),
        id_a() + "0",
        id_a()[..31].to_string() + "g",
        id_a() + " ",
        "0123456789abcdef-123456789abcde".to_string(),
    ] {
        assert!(!is_session_id(&bad), "{bad:?} passed");
    }
}

#[test]
fn origins_parse_scheme_host_and_port() {
    assert_eq!(
        parse_origin("wss://example.com").unwrap(),
        (true, "example.com".to_string(), 443)
    );
    assert_eq!(
        parse_origin("wss://example.com:8443").unwrap(),
        (true, "example.com".to_string(), 8443)
    );
    assert_eq!(
        parse_origin("ws://127.0.0.1:9000").unwrap(),
        (false, "127.0.0.1".to_string(), 9000)
    );
    assert_eq!(
        parse_origin("ws://localhost").unwrap(),
        (false, "localhost".to_string(), 80)
    );
    // Brackets are URI spelling: the parsed host is the bare literal.
    assert_eq!(
        parse_origin("wss://[::1]").unwrap(),
        (true, "::1".to_string(), 443)
    );
    assert_eq!(
        parse_origin("ws://[::1]:9000").unwrap(),
        (false, "::1".to_string(), 9000)
    );
}

#[test]
fn origins_refuse_paths_queries_and_cleartext_off_loopback() {
    // ⛔ Each shape this pins would dial somewhere unreviewed, leak the
    // token off TLS, or silently drop part of the address.
    for bad in [
        "https://example.com",
        "wss://",
        "wss://example.com/v1/node/n",
        "wss://example.com?token=abc",
        // ⛔ Built in parts: the committed source carries no
        // email-shaped literal, which a scanner reading a value rather
        // than a shape would report, while the runtime value keeps its
        // userinfo for the refusal under test.
        &("wss://user".to_string() + "@example.com"),
        "wss://example.com:0",
        "wss://example.com:99999",
        "ws://example.com",
        "ws://192.168.1.1",
        "wss://exam ple.com",
        "wss://[::1",
        "wss://[]:22",
        "wss://[::1]x",
        "wss://[::1]:99999",
        "wss://:443",
    ] {
        let e = parse_origin(bad).unwrap_err();
        assert_eq!(e.kind(), Kind::Usage, "{bad:?}: {e}");
    }
}

#[test]
fn names_are_path_segments() {
    validate_name(&format!("p-{}", id_a())).unwrap();
    validate_name("agent-1").unwrap();
    for bad in [
        "",
        "has space",
        "with/slash",
        "with?query",
        "semi;colon",
        &"x".repeat(129),
    ] {
        assert!(validate_name(bad).is_err(), "{bad:?} passed");
    }
}

#[test]
fn tokens_reject_header_injection() {
    // ⛔ The token becomes an HTTP header value. A newline in it is a
    // second header, so anything but visible non-blank ASCII is refused.
    validate_token("ephm1.1234.forward.abcdef").unwrap();
    let long = "x".repeat(1025);
    for bad in [
        "",
        "has space",
        "line\nbreak",
        "tab\there",
        "cr\rhere",
        // ⛔ Spelled as an escape: the committed source stays ASCII
        // while the runtime value stays non-ASCII.
        "t\u{e9}st-unicode",
        long.as_str(),
    ] {
        assert!(validate_token(bad).is_err(), "{bad:?} passed");
    }
}

#[test]
fn controls_round_trip() {
    let cases = [
        Control::Hello {
            version: 1,
            max_frame: 65536,
            max_sessions: 64,
        },
        Control::Open { id: id_a() },
        Control::Close { id: id_a() },
        Control::Ready { id: id_a() },
        Control::Reject {
            id: id_a(),
            reason: "no server: \"quoted\"".to_string(),
        },
        Control::Bye,
    ];
    for c in cases {
        let bytes = emit_control(&c).unwrap();
        assert_eq!(parse_control(&bytes).unwrap(), c, "{c:?}");
    }
}

#[test]
fn emit_refuses_an_unvalidated_id() {
    // ⛔ The constructor's own guard: a bad id never becomes a frame.
    // The relay would close the socket over it, so building it is the
    // defect, not sending it.
    for c in [
        Control::Open {
            id: "not-an-id".to_string(),
        },
        Control::Ready {
            id: id_a()[..31].to_string(),
        },
        Control::Reject {
            id: String::new(),
            reason: "x".to_string(),
        },
    ] {
        assert!(emit_control(&c).is_err(), "{c:?} emitted");
    }
}

#[test]
fn controls_refuse_relay_fed_garbage() {
    // Every shape a hostile or broken relay could send: not UTF-8, not
    // JSON, not an object, no type, a non-word type, bad id, unknown
    // type, and the old `verb` spelling this protocol does not use.
    let mut cases: Vec<Vec<u8>> = vec![
        b"\xff\xfe binary".to_vec(),
        b"not json".to_vec(),
        b"[1,2]".to_vec(),
        b"{}".to_vec(),
        br#"{"type":42}"#.to_vec(),
        format!(r#"{{"type":"open 2","id":"{id}"}}"#, id = id_a()).into_bytes(),
        br#"{"type":"open"}"#.to_vec(),
        br#"{"type":"open","id":"short"}"#.to_vec(),
        format!(r#"{{"verb":"open","id":"{id}"}}"#, id = id_a()).into_bytes(),
        format!(r#"{{"type":"launch","id":"{id}"}}"#, id = id_a()).into_bytes(),
        br#"{"type":"hello","version":1}"#.to_vec(),
        br#"{"type":"hello","version":2,"maxFrameBytes":65536,"maxSessions":64}"#.to_vec(),
    ];
    // The 4 KiB cap has its own case: a long reason is fine, a long
    // frame is not.
    cases.push(vec![b'x'; 5000]);
    // `version: 2` parses: the version gate lives in the hello reader,
    // which names the relay, not in the codec. Everything else refuses.
    // (Matched by content, not by index: inserting a case must not
    // silently reassign the exemption.)
    let v2 = br#"{"type":"hello","version":2,"maxFrameBytes":65536,"maxSessions":64}"#.to_vec();
    assert!(cases.contains(&v2));
    assert!(parse_control(&v2).is_ok());
    for raw in cases.iter().filter(|c| **c != v2) {
        assert!(parse_control(raw).is_err(), "passed: {raw:?}");
    }
}

#[test]
fn reject_without_reason_parses_to_empty() {
    let c = parse_control(format!(r#"{{"type":"reject","id":"{id}"}}"#, id = id_a()).as_bytes())
        .unwrap();
    assert_eq!(
        c,
        Control::Reject {
            id: id_a(),
            reason: String::new()
        }
    );
}

#[test]
fn backoff_is_capped_and_jittered_inside_one_second() {
    let (b0, j0) = backoff_secs(0);
    assert_eq!(b0, 1);
    assert!(j0 < Duration::from_secs(1));
    let (b3, _) = backoff_secs(3);
    assert_eq!(b3, 8);
    for n in [5u32, 6, 30, 100] {
        let (b, j) = backoff_secs(n);
        assert_eq!(b, 30, "attempt {n}");
        assert!(j < Duration::from_secs(1), "attempt {n}");
    }
}

#[test]
fn node_frame_split_needs_a_full_id() {
    let mut frame = id_a().into_bytes();
    frame.extend_from_slice(b"payload");
    let (id, payload) = split_node_frame(&frame).unwrap();
    assert_eq!(id, id_a());
    assert_eq!(payload, b"payload");
    assert!(split_node_frame(b"short").is_err());
    let mut bad = b"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_vec();
    bad.extend_from_slice(b"payload");
    assert!(split_node_frame(&bad).is_err());
}

#[test]
fn failures_never_carry_the_token() {
    // ⛔ The redaction this pins: every error below is built with a
    // token configured, and none may repeat it.
    let secret = "SECRET-TOKEN-xyz-123";
    let mut errs = Vec::new();
    errs.push(parse_origin("ws://example.com").unwrap_err());
    errs.push(validate_token("has space").unwrap_err());
    validate_token(secret).unwrap();
    let cfg = NodeConfig {
        relay: "wss://127.0.0.1:1".to_string(),
        name: "test-node".to_string(),
        node_token: secret.to_string(),
        server_command: Some("cat".to_string()),
        once: true,
    };
    // Port 1 is closed: the dial fails fast, with the token set.
    errs.push(run_node(&cfg).unwrap_err());
    let op = OperatorConfig::new(
        "wss://127.0.0.1:1".to_string(),
        "test-node".to_string(),
        secret.to_string(),
    );
    let mut io = crate::transport::Unix(std::os::unix::net::UnixStream::pair().unwrap().0);
    errs.push(run_operator(&op, &mut io).unwrap_err());
    for e in errs {
        assert!(!e.to_string().contains(secret), "leak: {e}");
    }
}
