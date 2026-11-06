use super::*;
use std::net::TcpListener;

#[test]
fn dotless_host_parses_as_a_name() {
    // ⛔ The defect class this pins: a parser that demands a dot in the
    // host sends `localhost` nowhere. A name with no dot is a name.
    assert_eq!(
        Target::parse_tcp("localhost:22", 22).unwrap(),
        Target::Tcp {
            host: "localhost".to_string(),
            port: 22,
        }
    );
    assert_eq!(
        Target::parse_tcp("localhost", 22).unwrap(),
        Target::Tcp {
            host: "localhost".to_string(),
            port: 22,
        }
    );
    assert!(Target::parse_tcp("localhost", 22)
        .unwrap()
        .plain()
        .contains("localhost"));
}

#[test]
fn tcp_targets_parse_host_port_and_ipv6() {
    assert_eq!(
        Target::parse_tcp("example.com:2222", 22).unwrap(),
        Target::Tcp {
            host: "example.com".to_string(),
            port: 2222,
        }
    );
    assert_eq!(
        Target::parse_tcp("[::1]:22", 22).unwrap(),
        Target::Tcp {
            host: "::1".to_string(),
            port: 22,
        }
    );
    assert_eq!(
        Target::parse_tcp("[::1]", 22).unwrap(),
        Target::Tcp {
            host: "::1".to_string(),
            port: 22,
        }
    );
    assert_eq!(
        Target::parse_tcp("::1", 22).unwrap(),
        Target::Tcp {
            host: "::1".to_string(),
            port: 22,
        }
    );
    assert_eq!(
        Target::parse_tcp("  example.com:22  ", 22).unwrap(),
        Target::Tcp {
            host: "example.com".to_string(),
            port: 22,
        }
    );
}

#[test]
fn bad_targets_are_usage_errors() {
    for bad in [
        "",
        "   ",
        ":22",
        "[::1",
        "[]:22",
        "host:0",
        "host:99999",
        "host:http",
        "[::1]x",
    ] {
        let e = Target::parse_tcp(bad, 22).unwrap_err();
        assert_eq!(e.kind(), Kind::Usage, "{bad:?}: {e}");
    }
}

#[test]
fn dial_unix_reports_the_missing_path() {
    let dialer = Dialer::new(Duration::from_secs(2));
    let target = Target::Unix {
        path: PathBuf::from("/tmp/podvm-ssh-test-no-such-socket-xyz"),
    };
    let e = dialer.dial(&target).unwrap_err();
    assert_eq!(e.kind(), Kind::Dial);
    assert!(e.to_string().contains("no-such-socket"), "{e}");
}

#[test]
fn dial_tcp_refused_is_a_dial_error_naming_the_target() {
    // ⛔ Port 1, not a just-freed ephemeral port: the old shape bound
    // :0, dropped it, and dialed the freed port, which a parallel test
    // binary's listener could reclaim first, and did once this suite
    // grew a relay. Nothing binds port 1 by accident, so refusal here
    // is structural rather than a scheduling outcome.
    let dialer = Dialer::new(Duration::from_secs(5));
    let target = Target::Tcp {
        host: "127.0.0.1".to_string(),
        port: 1,
    };
    let e = dialer.dial(&target).unwrap_err();
    assert_eq!(e.kind(), Kind::Dial);
    assert!(e.to_string().contains("127.0.0.1"), "{e}");
}

#[test]
fn dial_tcp_carries_bytes_through_a_loopback_listener() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4];
        s.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"ping");
        s.write_all(b"pong").unwrap();
    });
    let dialer = Dialer::new(Duration::from_secs(5));
    let mut s = dialer
        .dial(&Target::Tcp {
            host: "127.0.0.1".to_string(),
            port,
        })
        .unwrap();
    assert_eq!(s.describe(), "tcp");
    s.write_all(b"ping").unwrap();
    s.flush().unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut buf = [0u8; 4];
    s.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"pong");
    server.join().unwrap();
}

#[test]
fn exec_carries_bytes_through_cat() {
    let dialer = Dialer::new(Duration::from_secs(5));
    let mut s = dialer
        .dial(&Target::Exec {
            program: "cat".to_string(),
            args: Vec::new(),
        })
        .unwrap();
    assert_eq!(s.describe(), "exec");
    s.write_all(b"through-the-pipe").unwrap();
    s.flush().unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut buf = [0u8; 16];
    let mut got = 0;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while got < buf.len() && std::time::Instant::now() < deadline {
        match s.read(&mut buf[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) if crate::pump::is_would_block(&e) => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => panic!("read: {e}"),
        }
    }
    assert_eq!(&buf, b"through-the-pipe");
}

#[test]
fn dial_exec_missing_program_is_a_dial_error() {
    let dialer = Dialer::new(Duration::from_secs(2));
    let e = dialer
        .dial(&Target::Exec {
            program: "podvm-ssh-no-such-program-xyz".to_string(),
            args: Vec::new(),
        })
        .unwrap_err();
    assert_eq!(e.kind(), Kind::Dial);
}

/// Serialises the two tests that observe resolver worker threads: the
/// stall test's detached worker outlives its bound by design, and the
/// cleanup test must not count it.
static RESOLVE_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Threads of ours alive right now, by name prefix. Observed through
/// `/proc/self/task`, so this is the kernel's count, not a counter the
/// code under test maintains.
fn named_threads(prefix: &str) -> usize {
    let mut n = 0;
    if let Ok(dir) = std::fs::read_dir("/proc/self/task") {
        for e in dir.flatten() {
            if let Ok(name) = std::fs::read_to_string(e.path().join("comm"))
                && name.trim().starts_with(prefix)
            {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn stalled_resolver_ends_within_its_bound() {
    // ⛔ The fault this pins: a resolver that never answers must end the
    // dial inside the bound, not wedge it. The stalled closure stands in
    // for the resolver; production passes the real lookup through the
    // same seam, and no test may depend on a real DNS stall (loopback
    // fixtures cannot produce one).
    // ⛔ The worker blocks on a channel the test owns, so it dies with
    // the test instead of outliving it into the worker-cleanup test.
    let _guard = RESOLVE_GUARD.lock().unwrap();
    let (hold_tx, hold_rx) = std::sync::mpsc::channel::<()>();
    let bound = Duration::from_secs(1);
    let start = std::time::Instant::now();
    let e = resolve_with(bound, move || {
        let _ = hold_rx.recv();
        Ok::<Vec<SocketAddr>, io::Error>(Vec::new())
    })
    .unwrap_err();
    let elapsed = start.elapsed();
    drop(hold_tx);
    assert_eq!(e.kind(), io::ErrorKind::TimedOut, "got {e}");
    // The bound was exercised, not skipped: the wait lasted the second,
    // and ended well before any stall behind it could outlive the test.
    assert!(
        elapsed >= Duration::from_millis(900),
        "ended after {elapsed:?}: the bound was not waited out"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "ended after {elapsed:?}: the 1 s bound did not hold"
    );
}

#[test]
fn bounded_resolution_names_a_nothing_host_and_carries_loopback() {
    // A numeric loopback address resolves with no DNS at all: fast, and
    // loopback-only.
    let start = std::time::Instant::now();
    let addrs = resolve_bounded("127.0.0.1", 22, Duration::from_secs(10)).unwrap();
    assert!(!addrs.is_empty());
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "loopback resolution escaped its bound"
    );
    // The timeout sentence names the host it waited for: this is the
    // constructor `resolve_bounded` itself emits, so a rename that
    // drops the host fails here rather than in a wedged dial.
    let e = dns_timeout("no-such-podvm-host-xyz", DNS_DEADLINE);
    assert!(e.to_string().contains("no-such-podvm-host-xyz"), "{e}");
    // And the dialer's own op names the host too: a timed-out dial is
    // `Kind::Dial` on `dial tcp host:port`, never a bare timeout.
    let dialer = Dialer::new(Duration::from_secs(2));
    let target = Target::Tcp {
        host: "127.0.0.1".to_string(),
        port: 1,
    };
    let e = dialer.dial(&target).unwrap_err();
    assert_eq!(e.kind(), Kind::Dial);
    assert!(e.to_string().contains("127.0.0.1"), "{e}");
}

#[test]
fn resolver_workers_do_not_accumulate_across_iterations() {
    // ⛔ Twenty fast resolutions must leave no resolver thread behind:
    // each worker joins on success, so the kernel count settles to zero.
    let _guard = RESOLVE_GUARD.lock().unwrap();
    for _ in 0..20 {
        let addrs = resolve_bounded("127.0.0.1", 22, Duration::from_secs(10)).unwrap();
        assert!(!addrs.is_empty());
    }
    // One bounded pipe write exercises the pipe worker the same way.
    let (mut a, mut b) = std::os::unix::net::UnixStream::pair().unwrap();
    write_pipe_bounded(
        &mut a,
        b"cleanup-probe\n",
        Duration::from_secs(5),
        "test leg",
    )
    .unwrap();
    let mut one = [0u8; 14];
    use std::io::Read as _;
    b.read_exact(&mut one).unwrap();
    assert_eq!(&one, b"cleanup-probe\n");
    let start = std::time::Instant::now();
    loop {
        if named_threads("podvm-ssh-") == 0 {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "resolver or pipe workers still alive 10 s after their ops ended"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn non_reading_unix_peer_ends_writes_within_the_bound() {
    // ⛔ The fault this pins: a peer that stops reading must end the
    // write inside the deadline, not wedge the leg. The far end stays
    // open and never reads, so the send buffer fills and the kernel
    // stops taking bytes.
    // ⛔ The kernel contract this pins with it: an expired send deadline
    // on a blocking socket surfaces as `WouldBlock` (EAGAIN), never
    // `TimedOut`. Either kind here proves the stall ended the write;
    // the guarded actions above convert it to a timeout naming the leg.
    let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut w = Unix(a);
    w.set_write_timeout(Some(Duration::from_secs(1))).unwrap();
    let chunk = vec![0xABu8; 65536];
    let start = std::time::Instant::now();
    let e = loop {
        match w.write_all(&chunk) {
            Ok(()) => continue,
            Err(e) => break e,
        }
    };
    let elapsed = start.elapsed();
    assert!(
        matches!(
            e.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
        ),
        "got {e}"
    );
    assert!(
        elapsed >= Duration::from_millis(900),
        "failed after {elapsed:?}: the buffer never filled, so no stall was proved"
    );
    assert!(
        elapsed < Duration::from_secs(15),
        "failed after {elapsed:?}: the 1 s write bound did not hold"
    );
}

#[test]
fn non_reading_tcp_peer_ends_writes_within_the_bound() {
    // The same stall over loopback TCP: the listener accepts and then
    // never reads, which is the relay-leg shape (TCP under TLS or ws).
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let held = std::thread::spawn(move || {
        let (conn, _) = listener.accept().unwrap();
        // Held open and never read until the test ends.
        std::thread::sleep(Duration::from_secs(30));
        drop(conn);
    });
    let dialer = Dialer::new(Duration::from_secs(5));
    let mut s = dialer
        .dial(&Target::Tcp {
            host: "127.0.0.1".to_string(),
            port,
        })
        .unwrap();
    s.set_write_timeout(Some(Duration::from_secs(1))).unwrap();
    let chunk = vec![0xCDu8; 65536];
    let start = std::time::Instant::now();
    let e = loop {
        match s.write_all(&chunk) {
            Ok(()) => continue,
            Err(e) => break e,
        }
    };
    let elapsed = start.elapsed();
    assert!(
        matches!(
            e.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
        ),
        "got {e}"
    );
    assert!(
        elapsed >= Duration::from_millis(900),
        "failed after {elapsed:?}: the buffer never filled, so no stall was proved"
    );
    assert!(
        elapsed < Duration::from_secs(15),
        "failed after {elapsed:?}: the 1 s write bound did not hold"
    );
    drop(s);
    let _ = held.join();
}

#[test]
fn pipe_write_names_its_leg_on_a_stall() {
    // The pipe helper's loud end: a full pipe with no reader fails with
    // the leg named, inside the bound.
    use std::os::unix::net::UnixStream as US;
    let (mut a, _b) = US::pair().unwrap();
    // Fill the pipe first with the kernel deadline as the backstop.
    a.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
    let chunk = vec![0u8; 65536];
    loop {
        if a.write_all(&chunk).is_err() {
            break;
        }
    }
    let start = std::time::Instant::now();
    let e = write_pipe_bounded(&mut a, b"x", Duration::from_secs(1), "test shell-stdin leg")
        .unwrap_err();
    assert_eq!(e.kind(), io::ErrorKind::TimedOut, "got {e}");
    assert!(e.to_string().contains("test shell-stdin leg"), "{e}");
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "pipe bound did not hold: {:?}",
        start.elapsed()
    );
}
