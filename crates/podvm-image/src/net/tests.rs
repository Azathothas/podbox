use super::*;

use std::collections::HashMap;
use std::io::Write;
use std::net::TcpListener;

/// The proxy settings of a made-up environment.
fn env_of(pairs: &[(&str, &str)]) -> Environment {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    environment(&|n| map.get(n).map(OsString::from))
}

fn proxied(env: Environment, host: &str, port: u16) -> bool {
    let d = dialer(env).expect("the dialer starts");
    d.proxy_at(host, port)
        .expect("the proxy is usable")
        .is_some()
}

/// ⛔ Measured: with a proxy in the environment the plain-HTTP request to a
/// loopback registry came back HTTP 405, which is a proxy refusing a
/// non-CONNECT request and reads as a broken registry.
#[test]
fn loopback_never_goes_through_a_proxy() {
    let env = env_of(&[("HTTPS_PROXY", "http://proxy.example:3128")]);
    for host in [
        "localhost",
        "127.0.0.1",
        "127.1.2.3",
        "::1",
        "[::1]",
        "ip6-localhost",
    ] {
        assert!(
            !proxied(env.clone(), host, 5000),
            "{host} would go through a proxy"
        );
    }
    for host in ["ghcr.io", "registry-1.docker.io", "127.example.com"] {
        assert!(
            proxied(env.clone(), host, 443),
            "{host} would not take the proxy"
        );
    }
}

/// ⚠ Both spellings of `NO_PROXY` count, joined, as podvm read them before
/// `pod-proxy`; a parent domain matches its hosts.
#[test]
fn both_spellings_of_no_proxy_are_honoured() {
    let env = env_of(&[
        ("HTTPS_PROXY", "http://proxy.example:3128"),
        ("NO_PROXY", "registry.internal"),
        ("no_proxy", "example.org"),
    ]);
    assert!(!proxied(env.clone(), "registry.internal", 5000));
    assert!(!proxied(env.clone(), "mirror.example.org", 443));
    assert!(proxied(env.clone(), "ghcr.io", 443));
    let star = env_of(&[
        ("ALL_PROXY", "http://proxy.example:3128"),
        ("no_proxy", "*"),
    ]);
    assert!(!proxied(star, "ghcr.io", 443));
}

/// ⛔ A variable cannot name a proxy at a command.
#[test]
fn a_proxy_variable_cannot_name_a_command() {
    let env = env_of(&[("HTTPS_PROXY", "http+exec:nc proxy.example 3128")]);
    let d = dialer(env).expect("the dialer starts");
    assert!(d.proxy_at("ghcr.io", 443).is_err());
}

#[test]
fn a_location_joins_its_base() {
    let base = "https://r.example:5000/v2/a/blobs/sha256:00?x=1";
    assert_eq!(
        join(base, "https://cdn.example/b?sig=1"),
        "https://cdn.example/b?sig=1"
    );
    assert_eq!(join(base, "//cdn.example/b"), "https://cdn.example/b");
    assert_eq!(join(base, "/v2/b"), "https://r.example:5000/v2/b");
    assert_eq!(join(base, "c"), "https://r.example:5000/v2/a/blobs/c");
    assert_eq!(authority(base), "r.example:5000");
}

#[test]
fn a_query_value_is_escaped() {
    assert_eq!(
        query_value("repository:library/alpine:pull"),
        "repository%3Alibrary%2Falpine%3Apull"
    );
    assert_eq!(query_value("registry.docker.io"), "registry.docker.io");
}

/// A loopback origin that answers each connection with the next of
/// `answers`, and keeps the head of each request.
fn origin(answers: Vec<String>) -> (u16, std::sync::mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback binds where tests run");
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for answer in answers {
            let (mut s, _) = listener.accept().unwrap();
            let mut req = [0u8; 4096];
            let n = s.read(&mut req).unwrap_or(0);
            let _ = tx.send(String::from_utf8_lossy(&req[..n]).into_owned());
            let _ = s.write_all(answer.replace("{port}", &port.to_string()).as_bytes());
        }
    });
    (port, rx)
}

fn net() -> Net {
    let (config, _) = crate::tls::client_config();
    Net::with_environment(config, Environment::default()).expect("the transport starts")
}

const LIMITS: Limits = Limits {
    head: Duration::from_secs(5),
    idle: Duration::from_secs(5),
};

/// ⚠ `pod-proxy` follows no redirect: `net` does, and the credential does
/// not go to another host.
#[test]
fn a_redirect_is_followed_and_the_credential_stays_with_its_host() {
    let (cdn, cdn_seen) = origin(vec![
        "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nblob".into(),
    ]);
    let (port, seen) = origin(vec![
        "HTTP/1.1 307 Temporary Redirect\r\nLocation: /moved\r\nContent-Length: 0\r\n\r\n".into(),
        format!(
            "HTTP/1.1 302 Found\r\nLocation: http://localhost:{cdn}/b?sig=1\r\nContent-Length: 0\r\n\r\n"
        ),
    ]);
    let headers = [("Authorization", "Bearer made-up".to_string())];
    let url = format!("http://127.0.0.1:{port}/v2/x");
    let mut body = match net().send("GET", &url, &headers, true, LIMITS) {
        Ok(b) => b,
        Err(e) => panic!("the redirects are followed: {e}"),
    };
    assert_eq!(text(&mut body, 64).unwrap(), "blob");
    let first = seen.recv().unwrap();
    let second = seen.recv().unwrap();
    assert!(first.contains("Authorization: Bearer made-up"), "{first}");
    assert!(second.starts_with("GET /moved "), "{second}");
    assert!(second.contains("Authorization: Bearer made-up"), "{second}");
    let third = cdn_seen.recv().unwrap();
    assert!(third.starts_with("GET /b?sig=1 "), "{third}");
    assert!(!third.contains("Authorization"), "{third}");
    assert!(third.contains(USER_AGENT), "{third}");
}

#[test]
fn a_status_of_400_or_more_is_a_refusal() {
    let (port, _seen) = origin(vec![
        "HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\n\r\nno".into(),
    ]);
    match net().send(
        "GET",
        &format!("http://127.0.0.1:{port}/"),
        &[],
        true,
        LIMITS,
    ) {
        Err(Refusal::Status(b)) => assert_eq!(b.status(), 404),
        Err(e) => panic!("a 404 is a status, not {e}"),
        Ok(_) => panic!("a 404 is a refusal"),
    }
}
