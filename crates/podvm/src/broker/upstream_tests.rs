//! TODO/net.md T-1820: the broker dials an allowed host through the
//! upstream proxy of the host. A fixture upstream on the loopback answers
//! the CONNECT that the broker sends: with 200, the client of the broker
//! gets 200; with a refusal, the client gets 502, and the text names the
//! status of the upstream (podcrates `v0.45.0`, podcrates@T-609).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn b64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in input.chunks(3) {
        let n = c.iter().fold(0u32, |a, b| (a << 8) | *b as u32) << (8 * (3 - c.len()));
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// One upstream proxy that takes one request and answers `reply`. The
/// first line of the request comes back on the channel.
fn upstream(reply: &'static str) -> (u16, std::sync::mpsc::Receiver<String>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        if let Ok((s, _)) = l.accept() {
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut first = String::new();
            let _ = r.read_line(&mut first);
            loop {
                let mut h = String::new();
                if r.read_line(&mut h).unwrap_or(0) == 0 || h == "\r\n" {
                    break;
                }
            }
            let _ = tx.send(first.trim_end().to_string());
            let mut s = s;
            let _ = s.write_all(reply.as_bytes());
            std::thread::sleep(Duration::from_millis(200));
        }
    });
    (port, rx)
}

/// The status line that the broker answers to a CONNECT of `host:443`.
fn connect_through_broker(upstream_port: u16, host: &str) -> String {
    let proxy = pod_proxy::env::proxy_from_vars(host, &|n| {
        (n == "HTTPS_PROXY").then(|| format!("http://127.0.0.1:{upstream_port}").into())
    })
    .unwrap();
    assert!(proxy.is_some(), "the fixture is the upstream");
    let started = super::start_with(&[host.to_string()], &[], proxy).unwrap();
    let mut c = TcpStream::connect(("127.0.0.1", started.port)).unwrap();
    c.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let auth = b64(format!("run:{}", started.token).as_bytes());
    write!(
        c,
        "CONNECT {host}:443 HTTP/1.1\r\nHost: {host}:443\r\nProxy-Authorization: Basic {auth}\r\n\r\n"
    )
    .unwrap();
    let mut buf = [0u8; 2048];
    let n = c.read(&mut buf).unwrap_or(0);
    String::from_utf8_lossy(&buf[..n]).to_string()
}

#[test]
fn broker_upstream_answers_200_for_an_allowed_host() {
    let (port, seen) = upstream("HTTP/1.1 200 Connection Established\r\n\r\n");
    let reply = connect_through_broker(port, "example.com");
    assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
    let first = seen.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(first, "CONNECT example.com:443 HTTP/1.1");
}

#[test]
fn broker_upstream_refusal_names_its_status() {
    let (port, _seen) = upstream("HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n");
    let reply = connect_through_broker(port, "example.com");
    assert!(reply.starts_with("HTTP/1.1 502"), "{reply}");
    assert!(
        reply.contains("403"),
        "the 502 names no upstream status: {reply}"
    );
}
