use super::*;
use std::os::unix::net::UnixStream;

fn pair() -> (Box<dyn Stream>, Box<dyn Stream>) {
    let (a, b) = UnixStream::pair().unwrap();
    (
        Box::new(crate::transport::Unix(a)),
        Box::new(crate::transport::Unix(b)),
    )
}

#[test]
fn sha1_matches_the_rfc3174_vectors() {
    // ⛔ A digest this file trusts is checked against the RFC's vectors,
    // not against itself.
    assert_eq!(
        sha1(b"abc").to_vec(),
        b"\xa9\x99\x3e\x36\x47\x06\x81\x6a\xba\x3e\x25\x71\x78\x50\xc2\x6c\x9c\xd0\xd8\x9d"
            .to_vec()
    );
    assert_eq!(
        sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").to_vec(),
        b"\x84\x98\x3e\x44\x1c\x3b\xd2\x6e\xba\xae\x4a\xa1\xf9\x51\x29\xe5\xe5\x46\x70\xf1"
            .to_vec()
    );
    assert_eq!(
        sha1(b"").to_vec(),
        b"\xda\x39\xa3\xee\x5e\x6b\x4b\x0d\x32\x55\xbf\xef\x95\x60\x18\x90\xaf\xd8\x07\x09"
            .to_vec()
    );
}

#[test]
fn accept_matches_the_rfc6455_vector() {
    // The RFC's worked example: key `dGhlIHNhbXBsZSBub25jZQ==` accepts
    // as `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`.
    let key = "dGhlIHNhbXBsZSBub25jZQ==";
    let accept =
        base64::engine::general_purpose::STANDARD.encode(sha1(format!("{key}{GUID}").as_bytes()));
    assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
}

#[test]
fn upgrade_refusal_names_the_status() {
    let (a, b) = pair();
    let server = std::thread::spawn(move || {
        let mut b = b;
        use std::io::Write as _;
        let mut head = Vec::new();
        let mut one = [0u8; 1];
        // Read the request head, then refuse without upgrading.
        loop {
            b.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            b.read_exact(&mut one).unwrap();
            head.push(one[0]);
            if head.len() >= 4 && head[head.len() - 4..] == *b"\r\n\r\n" {
                break;
            }
        }
        b.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
        b.flush().unwrap();
    });
    let e = match Ws::client(a, "example.com", "/", &[]) {
        Ok(_) => panic!("a refused upgrade must not succeed"),
        Err(e) => e,
    };
    assert!(e.to_string().contains("403"), "got {e}");
    server.join().unwrap();
}

#[test]
fn upgrade_accept_mismatch_fails() {
    let (a, b) = pair();
    let server = std::thread::spawn(move || {
        let mut b = b;
        use std::io::Write as _;
        let mut head = Vec::new();
        let mut one = [0u8; 1];
        loop {
            b.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            b.read_exact(&mut one).unwrap();
            head.push(one[0]);
            if head.len() >= 4 && head[head.len() - 4..] == *b"\r\n\r\n" {
                break;
            }
        }
        // A 101 with the wrong accept value is not a websocket peer.
        b.write_all(
            b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
                  Connection: Upgrade\r\nSec-WebSocket-Accept: bogus\r\n\r\n",
        )
        .unwrap();
        b.flush().unwrap();
    });
    let e = match Ws::client(a, "example.com", "/", &[]) {
        Ok(_) => panic!("a refused upgrade must not succeed"),
        Err(e) => e,
    };
    assert!(e.to_string().contains("Sec-WebSocket-Accept"), "got {e}");
    server.join().unwrap();
}

#[test]
fn server_reports_path_and_headers() {
    // ⛔ Nothing is sent after the assertions: the server half returns
    // from its thread and drops its socket, so a trailing send here
    // races the close and fails with a broken pipe exactly as observed.
    let (a, b) = pair();
    let server = std::thread::spawn(move || Ws::server(b).unwrap());
    let headers = vec![("X-Relay-Token".to_string(), "tok".to_string())];
    let _ws = Ws::client(a, "example.com", "/v1/node/n1", &headers).unwrap();
    let (_, req) = server.join().unwrap();
    assert_eq!(req.path, "/v1/node/n1");
    assert!(req
        .headers
        .iter()
        .any(|(k, v)| k == "X-Relay-Token" && v == "tok"));
}

#[test]
fn header_injection_is_refused_before_any_byte() {
    // ⛔ The validation lives at the guarded action: each case runs
    // against a fresh pair with no server behind it, because the
    // refusal must precede the first byte, not the relay's answer.
    // ⛔ The non-ASCII case is spelled as an escape: the committed
    // source stays ASCII while the runtime value stays non-ASCII.
    for headers in [
        vec![("X Relay".to_string(), "tok".to_string())],
        vec![("X-Relay:Token".to_string(), "tok".to_string())],
        vec![("".to_string(), "tok".to_string())],
        vec![("X-Relay-Token".to_string(), "to\r\nk".to_string())],
        vec![("X-Relay-Token".to_string(), "t\u{f6}k".to_string())],
    ] {
        let (a, _b) = pair();
        let e = match Ws::client(a, "example.com", "/", &headers) {
            Ok(_) => panic!("injection passed: {headers:?}"),
            Err(e) => e,
        };
        assert!(e.to_string().contains("header"), "wrong refusal: {e}");
    }
}

#[test]
fn client_server_round_trip_keeps_text_and_binary_apart() {
    // ⛔ The property this whole module exists for: a text frame never
    // arrives as binary and a binary frame never arrives as text, across
    // every length class including the 16-bit and 64-bit boundaries.
    let (a, b) = pair();
    let server = std::thread::spawn(move || {
        let (mut ws, _) = Ws::server(b).unwrap();
        ws.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let mut frames = Vec::new();
        loop {
            match ws.read_frame().unwrap() {
                Some(Frame::Close(_, _)) => break,
                Some(f) => frames.push(f),
                None => continue,
            }
        }
        frames
    });
    let mut ws = Ws::client(a, "example.com", "/", &[]).unwrap();
    ws.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut expect = Vec::new();
    for n in [0usize, 1, 125, 126, 127, 1000, 65535, 65536, 200_000] {
        let data: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
        ws.send_binary(&data).unwrap();
        expect.push(Frame::Binary(data.clone()));
        ws.send_text(&data).unwrap();
        expect.push(Frame::Text(data));
    }
    ws.send_close(1000, b"done").unwrap();
    let frames = server.join().unwrap();
    assert_eq!(frames, expect);
}

#[test]
fn ping_is_answered_and_pong_is_dropped() {
    // The client half answers a ping on the wire and never surfaces it:
    // a ping followed by a binary frame reads back as just the binary.
    let (a, b) = pair();
    let server = std::thread::spawn(move || {
        let (mut ws, _) = Ws::server(b).unwrap();
        ws.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let mut frames = Vec::new();
        loop {
            match ws.read_frame().unwrap() {
                Some(Frame::Close(_, _)) => break,
                Some(f) => frames.push(f),
                None => continue,
            }
        }
        frames
    });
    let mut ws = Ws::client(a, "example.com", "/", &[]).unwrap();
    ws.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    ws.send_frame(0x9, b"are-you-there").unwrap();
    ws.send_binary(b"after-ping").unwrap();
    ws.send_close(1000, b"").unwrap();
    let frames = server.join().unwrap();
    assert_eq!(frames, vec![Frame::Binary(b"after-ping".to_vec())]);
}

#[test]
fn ping_gets_a_pong_on_the_wire() {
    // Raw driver against the accept side: a ping frame earns a pong
    // carrying the same payload, observed as bytes, not as frames.
    let (a, b) = pair();
    let server = std::thread::spawn(move || {
        let (mut ws, _) = Ws::server(b).unwrap();
        ws.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        // No data frame ever arrives; the ping must not surface either.
        // One queued close ends the read; anything else is a failure.
        loop {
            match ws.read_frame().unwrap() {
                Some(Frame::Close(_, _)) => break,
                Some(f) => panic!("control surfaced as {f:?}"),
                None => continue,
            }
        }
    });
    // Handshake by hand with the RFC's example key, then check the
    // accept value the server derived from it.
    let mut a = a;
    use std::io::Write as _;
    a.write_all(
        b"GET / HTTP/1.1\r\nHost: example.com\r\nUpgrade: websocket\r\n\
              Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
              Sec-WebSocket-Version: 13\r\n\r\n",
    )
    .unwrap();
    a.flush().unwrap();
    a.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut head = Vec::new();
    let mut one = [0u8; 1];
    loop {
        use std::io::Read as _;
        a.read_exact(&mut one).unwrap();
        head.push(one[0]);
        if head.len() >= 4 && head[head.len() - 4..] == *b"\r\n\r\n" {
            break;
        }
    }
    let head = String::from_utf8_lossy(&head).to_string();
    assert!(head.contains(" 101"), "{head}");
    assert!(head.contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="), "{head}");
    // Unmasked ping, as a server sends it.
    a.write_all(&[0x89, 0x0d]).unwrap();
    a.write_all(b"are-you-there").unwrap();
    a.flush().unwrap();
    // The pong arrives unmasked with the same payload.
    let mut hdr = [0u8; 2];
    use std::io::Read as _;
    a.read_exact(&mut hdr).unwrap();
    assert_eq!(hdr[0], 0x8A, "expected a pong, got {hdr:02x?}");
    assert_eq!(hdr[1], 0x0d, "expected 13 pong bytes, got {hdr:02x?}");
    let mut payload = [0u8; 13];
    a.read_exact(&mut payload).unwrap();
    assert_eq!(&payload, b"are-you-there");
    // A forged close ends the server read cleanly.
    a.write_all(&[0x88, 0x00]).unwrap();
    a.flush().unwrap();
    server.join().unwrap();
}

#[test]
fn send_bigger_than_the_cap_is_refused() {
    let (a, b) = pair();
    let server = std::thread::spawn(move || Ws::server(b).unwrap());
    let mut ws = Ws::client(a, "example.com", "/", &[]).unwrap();
    let e = ws.send_binary(&vec![0u8; MAX_FRAME + 1]).unwrap_err();
    assert!(e.to_string().contains("1 MiB"), "got {e}");
    server.join().unwrap();
}

#[test]
fn oversize_frame_is_refused_before_its_payload() {
    // A header declaring 2 MiB errors without the payload ever being
    // read: the cap guards memory, not just the message.
    let (a, b) = pair();
    let server = std::thread::spawn(move || {
        let (mut ws, _) = Ws::server(b).unwrap();
        ws.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        ws.read_frame().unwrap_err()
    });
    let mut a = a;
    use std::io::Write as _;
    a.write_all(
        b"GET / HTTP/1.1\r\nHost: example.com\r\nUpgrade: websocket\r\n\
              Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
              Sec-WebSocket-Version: 13\r\n\r\n",
    )
    .unwrap();
    a.flush().unwrap();
    a.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut head = Vec::new();
    let mut one = [0u8; 1];
    loop {
        use std::io::Read as _;
        a.read_exact(&mut one).unwrap();
        head.push(one[0]);
        if head.len() >= 4 && head[head.len() - 4..] == *b"\r\n\r\n" {
            break;
        }
    }
    // 127-class length of 2 MiB, then nothing: the refusal must not wait
    // for payload bytes that never come.
    let mut forged = vec![0x82u8, 127];
    forged.extend_from_slice(&(2u64 * 1024 * 1024).to_be_bytes());
    a.write_all(&forged).unwrap();
    a.flush().unwrap();
    let e = server.join().unwrap();
    assert!(e.to_string().contains("1 MiB"), "got {e}");
}

/// Complete the upgrade by hand against the accept side, with `key`,
/// and return the raw stream positioned at the first frame byte. The
/// 101 and the accept value are asserted here, so a failure is a
/// handshake failure rather than the test's.
fn raw_upgrade(mut a: Box<dyn Stream>, head: &[u8], key: &str) -> Box<dyn Stream> {
    use std::io::{Read as _, Write as _};
    a.write_all(head).unwrap();
    a.flush().unwrap();
    a.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut got = Vec::new();
    let mut one = [0u8; 1];
    loop {
        a.read_exact(&mut one).unwrap();
        got.push(one[0]);
        if got.len() >= 4 && got[got.len() - 4..] == *b"\r\n\r\n" {
            break;
        }
    }
    let text = String::from_utf8_lossy(&got).to_string();
    assert!(text.contains(" 101"), "{text}");
    let accept =
        base64::engine::general_purpose::STANDARD.encode(sha1(format!("{key}{GUID}").as_bytes()));
    assert!(text.contains(&accept), "{text}");
    a
}

#[test]
fn accept_without_key_is_refused() {
    let (a, b) = pair();
    let server = std::thread::spawn(move || match Ws::server(b) {
        Ok(_) => panic!("a keyless upgrade must not succeed"),
        Err(e) => e,
    });
    let mut a = a;
    use std::io::Write as _;
    // No Sec-WebSocket-Key line. The socket is then closed: the
    // refusal is the server's whole answer.
    a.write_all(
        b"GET / HTTP/1.1\r\nHost: example.com\r\nUpgrade: websocket\r\n\
              Connection: Upgrade\r\nSec-WebSocket-Version: 13\r\n\r\n",
    )
    .unwrap();
    a.flush().unwrap();
    drop(a);
    let e = server.join().unwrap();
    assert!(e.to_string().contains("Sec-WebSocket-Key"), "got {e}");
}

#[test]
fn reserved_opcode_is_refused() {
    let (a, b) = pair();
    let server = std::thread::spawn(move || {
        let (mut ws, _) = Ws::server(b).unwrap();
        ws.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        loop {
            match ws.read_frame() {
                Ok(_) => continue,
                Err(e) => return e,
            }
        }
    });
    let mut a = raw_upgrade(
        a,
        b"GET / HTTP/1.1\r\nHost: example.com\r\nUpgrade: websocket\r\n\
              Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
              Sec-WebSocket-Version: 13\r\n\r\n",
        "dGhlIHNhbXBsZSBub25jZQ==",
    );
    use std::io::Write as _;
    // FIN + reserved opcode 0x3, one payload byte, unmasked.
    a.write_all(&[0x83, 0x01, b'x']).unwrap();
    a.flush().unwrap();
    let e = server.join().unwrap();
    assert!(e.to_string().contains("opcode"), "got {e}");
}

#[test]
fn continuation_without_start_is_refused() {
    let (a, b) = pair();
    let server = std::thread::spawn(move || {
        let (mut ws, _) = Ws::server(b).unwrap();
        ws.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        loop {
            match ws.read_frame() {
                Ok(_) => continue,
                Err(e) => return e,
            }
        }
    });
    let mut a = raw_upgrade(
        a,
        b"GET / HTTP/1.1\r\nHost: example.com\r\nUpgrade: websocket\r\n\
              Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
              Sec-WebSocket-Version: 13\r\n\r\n",
        "dGhlIHNhbXBsZSBub25jZQ==",
    );
    use std::io::Write as _;
    // FIN + continuation with no start frame before it.
    a.write_all(&[0x80, 0x01, b'x']).unwrap();
    a.flush().unwrap();
    let e = server.join().unwrap();
    assert!(e.to_string().contains("continuation"), "got {e}");
}

#[test]
fn unexpected_eof_is_an_error_not_a_clean_close() {
    // ⛔ The distinction this pins: a socket that goes quiet without a
    // close frame reads as a fault, never as an orderly end.
    let (a, b) = pair();
    let server = std::thread::spawn(move || {
        let (mut ws, _) = Ws::server(b).unwrap();
        ws.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        // The client half completes the handshake, then vanishes.
        loop {
            match ws.read_frame() {
                Ok(Some(f)) => panic!("expected EOF, got {f:?}"),
                Ok(None) => continue,
                Err(e) => return e,
            }
        }
    });
    {
        let _ws = Ws::client(a, "example.com", "/", &[]).unwrap();
        // Drop the whole client without a close frame.
    }
    let e = server.join().unwrap();
    assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof, "got {e}");
}

#[test]
fn stalled_frame_send_ends_within_the_write_bound_naming_the_leg() {
    // ⛔ The fault this pins at the frame layer: the far side finishes
    // the upgrade and then never reads, so the send buffer fills and
    // the next frame must fail loud inside the write bound, naming the
    // relay-leg send, rather than wedge the socket.
    let (a, b) = pair();
    let server = std::thread::spawn(move || {
        let (mut ws, _) = Ws::server(b).unwrap();
        ws.set_write_timeout(Some(Duration::from_secs(1))).unwrap();
        let payload = vec![0x5Au8; 1_000_000];
        loop {
            match ws.send_binary(&payload) {
                Ok(()) => continue,
                Err(e) => return e,
            }
        }
    });
    // Complete the upgrade by hand, then hold the socket open and read
    // nothing: the peer that stopped reading, without closing.
    let mut a = raw_upgrade(
        a,
        b"GET / HTTP/1.1\r\nHost: example.com\r\nUpgrade: websocket\r\n\
              Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
              Sec-WebSocket-Version: 13\r\n\r\n",
        "dGhlIHNhbXBsZSBub25jZQ==",
    );
    // One small frame fits the empty buffer and proves the leg is live;
    // the stall comes after, never before.
    use std::io::Write as _;
    a.write_all(&[0x82, 0x01, b'x']).unwrap();
    a.flush().unwrap();
    let start = std::time::Instant::now();
    let e = server.join().unwrap();
    let elapsed = start.elapsed();
    assert_eq!(e.kind(), std::io::ErrorKind::TimedOut, "got {e}");
    assert!(e.to_string().contains("relay-leg send"), "{e}");
    assert!(
        elapsed >= Duration::from_millis(900),
        "failed after {elapsed:?}: the buffer never filled, so no stall was proved"
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "frame send escaped its 1 s bound: {elapsed:?}"
    );
    drop(a);
}
