//! RFC 6455 framing with the opcode travelling with its frame.
//!
//! ⛔ This codec returns frames, not bytes. The multiplexed relay protocol
//! (see [`crate::mux`]) routes a TEXT frame to the control parser and a
//! BINARY frame to a session by its id prefix. A byte-pipe websocket that
//! merges both opcodes into one stream destroys the message boundary the
//! relay protocol depends on, and a control message parsed with session data
//! still attached to it was one of the defects behind the reference
//! implementation's multiplexing design
//! (`references/talaria0101__dropssh/tree/docs/multiplexing.md:50-51`).
//!
//! ⛔ Masking is applied on sends when this side is the client, because the
//! RFC requires it and a conforming server drops an unmasked client frame.
//! Receives are unmasked whenever the mask bit is set, whichever side sent
//! them: a tolerant reader on a connection whose writer is exact.
//!
//! SHA-1 here proves nothing about identity: the `Sec-WebSocket-Accept`
//! value only shows the peer parsed the HTTP upgrade. Every byte that
//! matters is carried by SSH above this layer. Base64 comes from the
//! workspace `base64` pin (`Cargo.toml:107-113` carries the version and
//! the marginal-crate accounting); randomness comes from
//! `podbox_probe::sys::getrandom`, the only entropy source this project
//! uses. SHA-1 is hand-rolled because no crate in the tree computes one and
//! its only use is the accept value, pinned by the RFC 6455 test vector
//! below.

use std::io::{self, Read, Write};
use std::time::Duration;

use base64::Engine as _;

use crate::transport::Stream;

/// The relay's accept key derivation namespace.
const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
/// Largest single frame accepted. The relay never sends above 64 KiB plus a
/// 32-byte id; sixteen times that headroom still bounds memory per frame.
const MAX_FRAME: usize = 1024 * 1024;
/// Largest HTTP head accepted during the upgrade.
const MAX_HEAD: usize = 16 * 1024;

/// One complete websocket message. Control frames the codec answers itself
/// (ping) or drops (pong) never surface here; a close does, once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Text(Vec<u8>),
    Binary(Vec<u8>),
    Close(u16, Vec<u8>),
}

/// What the accept side read: the request path and headers.
pub struct ServerRequest {
    pub path: String,
    pub headers: Vec<(String, String)>,
}

/// One websocket connection. The read side yields frames; the write side
/// sends them. Ping is answered inside `read_frame`; anything else control
/// is either dropped (pong) or returned (close).
pub struct Ws {
    inner: Box<dyn Stream>,
    is_client: bool,
    rbuf: Vec<u8>,
    fragment: Vec<u8>,
    fragment_text: bool,
    fragment_open: bool,
    /// The relay's version header from the 101 response, when it sent one.
    /// Logged by the caller; never gated on (see [`crate::mux`]).
    pub version_header: Option<String>,
}

impl Ws {
    /// Dial side. Sends the upgrade for `path` with `headers` appended, then
    /// checks the 101 status and the accept value derived from our key.
    /// Anything already buffered after the head is framing, and the
    /// byte-at-a-time head read below means there normally is none.
    pub fn client(
        mut base: Box<dyn Stream>,
        host: &str,
        path: &str,
        headers: &[(String, String)],
    ) -> io::Result<Self> {
        let key = base64::engine::general_purpose::STANDARD.encode(random_bytes(16)?);
        // ⛔ Headers are validated at the guarded action, not just in the
        // callers: a name or value carrying a control byte would inject a
        // second header line into the upgrade.
        for (name, value) in headers {
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                return Err(io::Error::other(format!(
                    "websocket header name {name:?} is not a token"
                )));
            }
            if value.bytes().any(|b| b.is_ascii_control() || !b.is_ascii()) {
                return Err(io::Error::other(format!(
                    "websocket header {name:?} carries a control or non-ASCII byte"
                )));
            }
        }
        let mut extra = String::new();
        for (name, value) in headers {
            extra.push_str(&format!("{name}: {value}\r\n"));
        }
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n\
             Sec-WebSocket-Version: 13\r\nUser-Agent: podssh/{ver}\r\n{extra}\r\n",
            ver = env!("CARGO_PKG_VERSION"),
        );
        base.write_all(req.as_bytes())?;
        base.flush()?;
        let (status, resp_headers) = read_head(&mut *base)?;
        if !status.contains(" 101") {
            return Err(io::Error::other(format!(
                "websocket upgrade refused: {status}"
            )));
        }
        let expect = base64::engine::general_purpose::STANDARD
            .encode(sha1(format!("{key}{GUID}").as_bytes()));
        let ok = resp_headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("sec-websocket-accept") && v.trim() == expect);
        if !ok {
            return Err(io::Error::other(
                "websocket upgrade: Sec-WebSocket-Accept does not match the key sent",
            ));
        }
        let version_header = resp_headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("x-relay-version"))
            .map(|(_, v)| v.trim().to_string());
        Ok(Ws {
            inner: base,
            is_client: true,
            rbuf: Vec::new(),
            fragment: Vec::new(),
            fragment_text: false,
            fragment_open: false,
            version_header,
        })
    }

    /// Accept side. Reads the upgrade, requires the key and the upgrade
    /// header, answers 101, and returns the connection with the request path
    /// and headers. The integration tests serve this side; no shipped caller
    /// listens (the node and the operator only ever dial out).
    pub fn server(mut base: Box<dyn Stream>) -> io::Result<(Self, ServerRequest)> {
        let (request, headers) = read_head(&mut *base)?;
        let mut parts = request.split_whitespace();
        let _method = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("/").to_string();
        let mut key = None;
        let mut upgrade = false;
        for (k, v) in &headers {
            if k.eq_ignore_ascii_case("sec-websocket-key") {
                key = Some(v.clone());
            }
            if k.eq_ignore_ascii_case("upgrade") && v.eq_ignore_ascii_case("websocket") {
                upgrade = true;
            }
        }
        let key = key.ok_or_else(|| io::Error::other("websocket upgrade: no Sec-WebSocket-Key"))?;
        if !upgrade {
            return Err(io::Error::other("websocket upgrade: no Upgrade header"));
        }
        let accept = base64::engine::general_purpose::STANDARD
            .encode(sha1(format!("{key}{GUID}").as_bytes()));
        let resp = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        base.write_all(resp.as_bytes())?;
        base.flush()?;
        Ok((
            Ws {
                inner: base,
                is_client: false,
                rbuf: Vec::new(),
                fragment: Vec::new(),
                fragment_text: false,
                fragment_open: false,
                version_header: None,
            },
            ServerRequest { path, headers },
        ))
    }

    /// Read one frame. `Ok(None)` means the inner read timed out: nothing
    /// arrived inside its window, which the caller treats as "try the other
    /// direction" rather than as a close. A peer that goes away without a
    /// close frame is an error, not a clean end: silent loss of the socket
    /// must not read as an orderly shutdown.
    pub fn read_frame(&mut self) -> io::Result<Option<Frame>> {
        loop {
            match self.decode_one()? {
                Some(frame) => return Ok(Some(frame)),
                None => {
                    if self.rbuf.len() >= MAX_FRAME + 16 {
                        return Err(io::Error::other("websocket frame exceeds the 1 MiB cap"));
                    }
                    let mut tmp = [0u8; 65536];
                    match self.inner.read(&mut tmp) {
                        Ok(0) => {
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "peer went away without a websocket close frame",
                            ));
                        }
                        Ok(n) => self.rbuf.extend_from_slice(&tmp[..n]),
                        Err(e)
                            if e.kind() == io::ErrorKind::WouldBlock
                                || e.kind() == io::ErrorKind::TimedOut =>
                        {
                            return Ok(None)
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }
    }

    /// Decode one frame from the buffer, or `Ok(None)` when more bytes are
    /// needed. Ping is answered inline; pong is dropped; a close is echoed
    /// once and then returned.
    fn decode_one(&mut self) -> io::Result<Option<Frame>> {
        if self.rbuf.len() < 2 {
            return Ok(None);
        }
        let b0 = self.rbuf[0];
        let b1 = self.rbuf[1];
        let fin = b0 & 0x80 != 0;
        let opcode = b0 & 0x0f;
        let masked = b1 & 0x80 != 0;
        let mut len = (b1 & 0x7f) as usize;
        let mut idx = 2usize;
        if len == 126 {
            if self.rbuf.len() < 4 {
                return Ok(None);
            }
            len = u16::from_be_bytes([self.rbuf[2], self.rbuf[3]]) as usize;
            idx = 4;
        } else if len == 127 {
            if self.rbuf.len() < 10 {
                return Ok(None);
            }
            let mut v = [0u8; 8];
            v.copy_from_slice(&self.rbuf[2..10]);
            let l = u64::from_be_bytes(v);
            if l > MAX_FRAME as u64 {
                return Err(io::Error::other("websocket frame exceeds the 1 MiB cap"));
            }
            len = l as usize;
            idx = 10;
        }
        // ⛔ The length is checked against what actually arrived, never
        // trusted on its own: a declared length longer than the buffer means
        // "wait", not "read past it".
        let mask: [u8; 4] = if masked {
            if self.rbuf.len() < idx + 4 {
                return Ok(None);
            }
            let m = [
                self.rbuf[idx],
                self.rbuf[idx + 1],
                self.rbuf[idx + 2],
                self.rbuf[idx + 3],
            ];
            idx += 4;
            m
        } else {
            [0; 4]
        };
        if self.rbuf.len() < idx + len {
            return Ok(None);
        }
        let mut payload = self.rbuf[idx..idx + len].to_vec();
        self.rbuf.drain(..idx + len);
        if masked {
            for (i, b) in payload.iter_mut().enumerate() {
                *b ^= mask[i % 4];
            }
        }
        match opcode {
            0x0 => {
                if !self.fragment_open {
                    return Err(io::Error::other("websocket continuation with no start"));
                }
                self.fragment.extend_from_slice(&payload);
                if self.fragment.len() > MAX_FRAME {
                    return Err(io::Error::other("websocket fragments exceed the 1 MiB cap"));
                }
                if fin {
                    self.fragment_open = false;
                    let payload = std::mem::take(&mut self.fragment);
                    return Ok(Some(if self.fragment_text {
                        Frame::Text(payload)
                    } else {
                        Frame::Binary(payload)
                    }));
                }
                Ok(None)
            }
            0x1 | 0x2 => {
                let text = opcode == 0x1;
                if fin {
                    return Ok(Some(if text {
                        Frame::Text(payload)
                    } else {
                        Frame::Binary(payload)
                    }));
                }
                // ⛔ A new start frame while one is open discards the partial:
                // erroring would let one bad peer wedge the connection, and
                // keeping both would mix two messages into one.
                self.fragment = payload;
                self.fragment_text = text;
                self.fragment_open = true;
                Ok(None)
            }
            0x8 => {
                let (code, reason) = split_close(&payload);
                let _ = self.send_frame(0x8, &payload);
                Ok(Some(Frame::Close(code, reason)))
            }
            0x9 => {
                let _ = self.send_frame(0xA, &payload);
                Ok(None)
            }
            0xA => Ok(None),
            other => Err(io::Error::other(format!("websocket opcode {other:#x}"))),
        }
    }

    /// Send one unfragmented frame. Everything this side sends fits in one
    /// frame: session payloads are chunked by the caller first.
    fn send_frame(&mut self, opcode: u8, payload: &[u8]) -> io::Result<()> {
        if payload.len() > MAX_FRAME {
            return Err(io::Error::other("websocket send exceeds the 1 MiB cap"));
        }
        let mut frame = Vec::with_capacity(payload.len() + 14);
        frame.push(0x80 | opcode);
        let mask_bit = if self.is_client { 0x80 } else { 0x00 };
        let len = payload.len();
        if len < 126 {
            frame.push(mask_bit | len as u8);
        } else if len < 65536 {
            frame.push(mask_bit | 126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push(mask_bit | 127);
            frame.extend_from_slice(&(len as u64).to_be_bytes());
        }
        if self.is_client {
            let mask = random_bytes(4)?;
            frame.extend_from_slice(&mask);
            frame.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
        } else {
            frame.extend_from_slice(payload);
        }
        self.inner.write_all(&frame)?;
        self.inner.flush()
    }

    pub fn send_text(&mut self, payload: &[u8]) -> io::Result<()> {
        self.send_frame(0x1, payload)
    }

    pub fn send_binary(&mut self, payload: &[u8]) -> io::Result<()> {
        self.send_frame(0x2, payload)
    }

    pub fn send_close(&mut self, code: u16, reason: &[u8]) -> io::Result<()> {
        let mut payload = Vec::with_capacity(reason.len() + 2);
        payload.extend_from_slice(&code.to_be_bytes());
        payload.extend_from_slice(reason);
        self.send_frame(0x8, &payload)
    }

    /// Bound how long `read_frame` waits for bytes before reporting `Ok(None)`.
    pub fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.inner.set_read_timeout(d)
    }

    pub fn describe(&self) -> &'static str {
        "ws"
    }
}

fn split_close(payload: &[u8]) -> (u16, Vec<u8>) {
    if payload.len() >= 2 {
        (
            u16::from_be_bytes([payload[0], payload[1]]),
            payload[2..].to_vec(),
        )
    } else {
        (1005, Vec::new())
    }
}

/// Read one HTTP head, one byte at a time so the byte after the blank line
/// (the first frame) is never consumed by the header read. Returns the first
/// line and the `name: value` pairs below it.
fn read_head(s: &mut dyn Stream) -> io::Result<(String, Vec<(String, String)>)> {
    s.set_read_timeout(Some(Duration::from_secs(15)))?;
    let mut raw = Vec::new();
    let mut b = [0u8; 1];
    loop {
        if raw.len() >= MAX_HEAD {
            return Err(io::Error::other("http head exceeds 16 KiB"));
        }
        match s.read(&mut b) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "peer closed during the websocket upgrade",
                ))
            }
            Ok(_) => {
                raw.push(b[0]);
                if raw.len() >= 4 && raw[raw.len() - 4..] == *b"\r\n\r\n" {
                    break;
                }
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut
                    || e.kind() == io::ErrorKind::Interrupted =>
            {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "websocket upgrade read timed out",
                ))
            }
            Err(e) => return Err(e),
        }
    }
    let text = String::from_utf8_lossy(&raw).to_string();
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("").to_string();
    let mut headers = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    Ok((first, headers))
}

/// Fill `n` bytes from the kernel CSPRNG, in a loop: the kernel may return
/// short, and the loop is the contract, not a precaution. Mirrors the reader
/// in `podbox-complete`'s device shim, which is the tree's other caller of
/// this syscall.
fn random_bytes(n: usize) -> io::Result<Vec<u8>> {
    use podbox_probe::sys;
    let mut out = vec![0u8; n];
    let mut off = 0;
    while off < n {
        match sys::getrandom(&mut out[off..]) {
            Ok(0) => break,
            Ok(k) => off += k as usize,
            Err(e) if e == sys::EINTR => continue,
            Err(e) => {
                return Err(io::Error::other(format!(
                    "getrandom(2) for the websocket mask: {}",
                    e.name()
                )))
            }
        }
    }
    if off < n {
        return Err(io::Error::other("getrandom(2) returned no bytes"));
    }
    Ok(out)
}

/// SHA-1 (RFC 3174), for the accept value only: not for signatures, not for
/// passwords, not for anything adversarial.
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    let mut w = [0u32; 80];
    for chunk in msg.chunks_exact(64) {
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let tmp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
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
        let accept = base64::engine::general_purpose::STANDARD
            .encode(sha1(format!("{key}{GUID}").as_bytes()));
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
        let accept = base64::engine::general_purpose::STANDARD
            .encode(sha1(format!("{key}{GUID}").as_bytes()));
        assert!(text.contains(&accept), "{text}");
        a
    }

    #[test]
    fn accept_without_key_is_refused() {
        let (a, b) = pair();
        let server = std::thread::spawn(move || {
            let e = match Ws::server(b) {
                Ok(_) => panic!("a keyless upgrade must not succeed"),
                Err(e) => e,
            };
            e
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
}
