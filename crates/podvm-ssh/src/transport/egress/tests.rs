//! The proxy of a TCP dial: a stand-in HTTP CONNECT proxy on the loopback
//! carries the dial that the variables send to it, and `NO_PROXY` and a
//! loopback target go around it. The variables come through a reader, not
//! the process environment, so the tests run in parallel.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use pod_proxy::Environment;

use crate::error::Kind;
use crate::transport::{Dialer, Target};

const LIMIT: Duration = Duration::from_secs(5);

/// The environment of `pairs`, as `Environment::read` reads the process's.
fn environment(pairs: &[(&str, String)]) -> Environment {
    let vars: HashMap<String, OsString> = pairs
        .iter()
        .map(|(name, value)| (name.to_string(), OsString::from(value)))
        .collect();
    Environment::read(&|name| vars.get(name).cloned())
}

fn tcp(host: &str, port: u16) -> Target {
    Target::Tcp {
        host: host.to_string(),
        port,
    }
}

/// A stand-in HTTP proxy on the loopback: it answers 200 to one CONNECT
/// and then gives back each byte. The thread returns the head that it got.
fn connect_proxy() -> (u16, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = thread::spawn(move || {
        let (sock, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(sock.try_clone().unwrap());
        let mut head = String::new();
        while !head.ends_with("\r\n\r\n") && reader.read_line(&mut head).unwrap() > 0 {}
        let mut sock = sock;
        sock.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .unwrap();
        let mut far = sock.try_clone().unwrap();
        let _ = std::io::copy(&mut reader, &mut far);
        head
    });
    (port, seen)
}

/// A listener that must get no connection: the proxy that a dial went
/// around.
fn unused_proxy() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

fn assert_unused(listener: &TcpListener) {
    match listener.accept() {
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        other => panic!("the proxy got a connection: {other:?}"),
    }
}

/// A server on the loopback that gives back each byte of one connection.
fn echo() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        let (mut sock, _) = listener.accept().unwrap();
        let mut far = sock.try_clone().unwrap();
        let _ = std::io::copy(&mut sock, &mut far);
    });
    port
}

fn round_trip(s: &mut dyn crate::transport::Stream) {
    s.set_read_timeout(Some(LIMIT)).unwrap();
    s.write_all(b"ping").unwrap();
    s.flush().unwrap();
    let mut got = [0u8; 4];
    s.read_exact(&mut got).unwrap();
    assert_eq!(&got, b"ping");
}

#[test]
fn https_proxy_carries_the_dial_through_connect() {
    let (port, seen) = connect_proxy();
    let env = environment(&[("HTTPS_PROXY", format!("http://127.0.0.1:{port}"))]);
    let dialer = Dialer::with_environment(LIMIT, env).unwrap();
    let mut s = dialer.dial(&tcp("relay.invalid", 443)).unwrap();
    assert_eq!(s.describe(), "tcp");
    round_trip(&mut *s);
    drop(s);
    let head = seen.join().unwrap();
    assert!(
        head.starts_with("CONNECT relay.invalid:443 HTTP/1.1\r\n"),
        "{head}"
    );
}

#[test]
fn all_proxy_carries_the_dial_when_https_proxy_is_unset() {
    let (port, seen) = connect_proxy();
    let env = environment(&[("ALL_PROXY", format!("http://127.0.0.1:{port}"))]);
    let dialer = Dialer::with_environment(LIMIT, env).unwrap();
    let mut s = dialer.dial(&tcp("relay.invalid", 8443)).unwrap();
    round_trip(&mut *s);
    drop(s);
    let head = seen.join().unwrap();
    assert!(head.starts_with("CONNECT relay.invalid:8443 "), "{head}");
}

#[test]
fn a_loopback_target_goes_around_the_proxy() {
    let (proxy, port) = unused_proxy();
    let env = environment(&[("HTTPS_PROXY", format!("http://127.0.0.1:{port}"))]);
    let dialer = Dialer::with_environment(LIMIT, env).unwrap();
    let mut s = dialer.dial(&tcp("127.0.0.1", echo())).unwrap();
    round_trip(&mut *s);
    assert_unused(&proxy);
}

#[test]
fn no_proxy_sends_the_target_directly() {
    let (proxy, port) = unused_proxy();
    let env = environment(&[
        ("HTTPS_PROXY", format!("http://127.0.0.1:{port}")),
        ("NO_PROXY", ".invalid".to_string()),
    ]);
    // A short limit: the direct dial resolves a name that has no address,
    // and fails with the error of the direct dial.
    let dialer = Dialer::with_environment(Duration::from_secs(2), env).unwrap();
    let e = dialer.dial(&tcp("relay.invalid", 443)).unwrap_err();
    assert_eq!(e.kind(), Kind::Dial);
    assert!(e.to_string().contains("relay.invalid"), "{e}");
    assert!(!e.to_string().contains("proxy"), "{e}");
    assert_unused(&proxy);
}

#[test]
fn an_unusable_proxy_setting_fails_the_dial() {
    let env = environment(&[("HTTPS_PROXY", "ftp://proxy.invalid:21".to_string())]);
    let dialer = Dialer::with_environment(LIMIT, env).unwrap();
    let e = dialer.dial(&tcp("relay.invalid", 443)).unwrap_err();
    assert_eq!(e.kind(), Kind::Dial);
    assert!(e.to_string().contains("proxy setting is not usable"), "{e}");
}

#[test]
fn an_environment_with_no_proxy_starts_no_runtime() {
    let env = environment(&[("NO_PROXY", "*".to_string())]);
    let dialer = Dialer::with_environment(LIMIT, env).unwrap();
    assert!(dialer.egress.is_none());
    let mut s = dialer.dial(&tcp("127.0.0.1", echo())).unwrap();
    round_trip(&mut *s);
}
