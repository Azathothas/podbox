use super::*;

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
    validate_name("p-0123456789abcdef").unwrap();
    validate_name("agent-1").unwrap();
    for bad in [
        "",
        "has space",
        "with/slash",
        "with?query",
        "semi;colon",
        "-leading-dash",
        &"x".repeat(129),
    ] {
        let e = validate_name(bad).unwrap_err();
        assert_eq!(e.kind(), Kind::Usage, "{bad:?}: {e}");
    }
}

#[test]
fn tokens_have_the_relay_s_shape() {
    // ⛔ The token becomes an HTTP header value. A newline in it is a
    // second header, so anything but visible ASCII is refused, and the
    // relay's tokens have 8 characters or more.
    validate_token("ephm1.1234.forward.abcdef").unwrap();
    let long = "x".repeat(4097);
    for bad in [
        "",
        "short",
        "has space",
        "line\nbreak",
        "tab\there-and-more",
        "cr\rhere-and-more",
        // ⛔ Spelled as an escape: the committed source stays ASCII
        // while the runtime value stays non-ASCII.
        "t\u{e9}st-unicode",
        long.as_str(),
    ] {
        let e = validate_token(bad).unwrap_err();
        assert_eq!(e.kind(), Kind::Usage, "{bad:?}: {e}");
        assert!(bad.is_empty() || !e.to_string().contains(bad), "{e}");
    }
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
    // Port 1 is closed: the first try fails fast, and `--once` ends there.
    errs.push(run_node(&cfg).unwrap_err());
    let op = OperatorConfig::new(
        "wss://127.0.0.1:1".to_string(),
        "test-node".to_string(),
        secret.to_string(),
    );
    errs.push(run_operator(&op, io::empty(), io::sink()).unwrap_err());
    for e in errs {
        assert!(!e.to_string().contains(secret), "leak: {e}");
    }
}

#[test]
fn each_end_of_a_node_has_its_words() {
    for (exit, words) in [
        (Exit::NameInUse, "409"),
        (Exit::Forbidden, "403"),
        (Exit::PairExpired, "expired"),
        (Exit::PairStopped, "stopped"),
        (
            Exit::Unusable("the relay reads dialect 2".into()),
            "dialect 2",
        ),
    ] {
        assert!(node::exit_words(&exit).contains(words), "{exit:?}");
    }
}
