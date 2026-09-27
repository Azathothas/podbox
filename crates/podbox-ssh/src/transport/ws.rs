//! Minimal RFC 6455 websocket framing, enough for ssh bytes and nothing else.
//!
//! Both ends are ours or a compat relay's, so the codec implements binary
//! data, ping/pong, close and continuation. It is a byte pipe with a framing
//! header, not a general websocket stack.
//!
//! ⛔ Fragmentation is handled because a CDN may split a frame; masking is
//! applied on the client side because the RFC requires it and a
//! spec-following server will drop an unmasked client frame.

use std::io::{self, Read, Write};
use std::time::Duration;

use super::{read_headers, Stream};
use crate::url::Spec;
use crate::util::{b64_encode, random_bytes, sha1};

const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const MAX_FRAME: usize = 16 * 1024 * 1024;

/// One websocket connection reduced to a byte pipe.
pub struct Ws {
    inner: Box<dyn Stream>,
    is_client: bool,
    rbuf: Vec<u8>,
    pending: Vec<u8>,
    pending_at: usize,
    fragment: Vec<u8>,
    fragment_open: bool,
    closed: bool,
}

impl Ws {
    /// Dial side. `spec.path` is the request path; empty becomes `/`.
    pub fn client(mut base: Box<dyn Stream>, spec: &Spec) -> io::Result<Self> {
        let host = spec.host.clone().unwrap_or_default();
        let port = spec.port_or_default();
        let host_header = match port {
            Some(p) => format!("{host}:{p}"),
            None => host,
        };
        let path = if spec.path.is_empty() {
            "/".to_string()
        } else {
            spec.path.clone()
        };
        let key = b64_encode(&random_bytes(16)?);
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {host_header}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n\
             Sec-WebSocket-Version: 13\r\nUser-Agent: podssh/1\r\n\r\n"
        );
        base.write_all(req.as_bytes())?;
        base.flush()?;
        let head = read_headers(&mut *base, 16 * 1024, Duration::from_secs(15))?;
        let text = String::from_utf8_lossy(&head);
        let status = text.lines().next().unwrap_or("");
        if !status.contains(" 101") {
            return Err(io::Error::other(format!(
                "websocket upgrade refused: {status}"
            )));
        }
        let expect = b64_encode(&sha1(format!("{key}{GUID}").as_bytes()));
        if !text.contains(&expect) {
            return Err(io::Error::other(
                "websocket upgrade: Sec-WebSocket-Accept does not match",
            ));
        }
        let mut ws = Ws::new(base, true);
        // Anything already buffered after the headers is framing, and the
        // byte-at-a-time header read means there is normally none.
        ws.rbuf.clear();
        Ok(ws)
    }

    /// Accept side. Returns the request path.
    pub fn server(mut base: Box<dyn Stream>) -> io::Result<Self> {
        let head = read_headers(&mut *base, 16 * 1024, Duration::from_secs(15))?;
        let text = String::from_utf8_lossy(&head);
        let request = text.lines().next().unwrap_or("");
        let mut parts = request.split_whitespace();
        let _method = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("/").to_string();
        let mut key = None;
        let mut upgrade = false;
        for line in text.lines().skip(1) {
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim().to_ascii_lowercase();
                let v = v.trim();
                if k == "sec-websocket-key" {
                    key = Some(v.to_string());
                }
                if k == "upgrade" && v.eq_ignore_ascii_case("websocket") {
                    upgrade = true;
                }
            }
        }
        let key = key
            .ok_or_else(|| io::Error::other("websocket upgrade: no Sec-WebSocket-Key"))?;
        if !upgrade {
            return Err(io::Error::other("websocket upgrade: no Upgrade header"));
        }
        let accept = b64_encode(&sha1(format!("{key}{GUID}").as_bytes()));
        let resp = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        base.write_all(resp.as_bytes())?;
        base.flush()?;
        let _ = path;
        Ok(Ws::new(base, false))
    }

    fn new(inner: Box<dyn Stream>, is_client: bool) -> Self {
        Ws {
            inner,
            is_client,
            rbuf: Vec::new(),
            pending: Vec::new(),
            pending_at: 0,
            fragment: Vec::new(),
            fragment_open: false,
            closed: false,
        }
    }

    /// Read more raw bytes from the transport. `Ok(0)` is a close.
    fn fill(&mut self) -> io::Result<usize> {
        let mut tmp = [0u8; 65536];
        let n = self.inner.read(&mut tmp)?;
        if n == 0 {
            return Ok(0);
        }
        self.rbuf.extend_from_slice(&tmp[..n]);
        Ok(n)
    }

    /// Try to decode one frame from `rbuf`. Returns `Ok(Some(wait))` where
    /// `wait` is true when more bytes are needed.
    fn decode_frame(&mut self) -> io::Result<Option<bool>> {
        if self.rbuf.len() < 2 {
            return Ok(Some(true));
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
                return Ok(Some(true));
            }
            len = u16::from_be_bytes([self.rbuf[2], self.rbuf[3]]) as usize;
            idx = 4;
        } else if len == 127 {
            if self.rbuf.len() < 10 {
                return Ok(Some(true));
            }
            let mut v = [0u8; 8];
            v.copy_from_slice(&self.rbuf[2..10]);
            let l = u64::from_be_bytes(v);
            if l > MAX_FRAME as u64 {
                return Err(io::Error::other("websocket frame larger than 16 MiB"));
            }
            len = l as usize;
            idx = 10;
        }
        let mask: [u8; 4] = if masked {
            if self.rbuf.len() < idx + 4 {
                return Ok(Some(true));
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
            return Ok(Some(true));
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
                // continuation
                if !self.fragment_open {
                    return Err(io::Error::other("websocket continuation with no start"));
                }
                self.fragment.extend_from_slice(&payload);
                if fin {
                    self.fragment_open = false;
                    self.pending = std::mem::take(&mut self.fragment);
                    self.pending_at = 0;
                }
            }
            0x1 | 0x2 => {
                if fin {
                    self.pending = payload;
                    self.pending_at = 0;
                } else {
                    self.fragment = payload;
                    self.fragment_open = true;
                }
            }
            0x8 => {
                // close
                let _ = self.send_frame(0x8, &payload);
                self.closed = true;
            }
            0x9 => {
                let _ = self.send_frame(0xA, &payload);
            }
            0xA => {}
            other => {
                return Err(io::Error::other(format!("websocket opcode {other:#x}")));
            }
        }
        Ok(Some(false))
    }

    fn send_frame(&mut self, opcode: u8, payload: &[u8]) -> io::Result<()> {
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
}

impl Read for Ws {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.pending_at < self.pending.len() {
                let n = std::cmp::min(buf.len(), self.pending.len() - self.pending_at);
                buf[..n].copy_from_slice(&self.pending[self.pending_at..self.pending_at + n]);
                self.pending_at += n;
                if self.pending_at == self.pending.len() {
                    self.pending.clear();
                    self.pending_at = 0;
                }
                return Ok(n);
            }
            if self.closed {
                return Ok(0);
            }
            match self.decode_frame() {
                Ok(Some(false)) => continue,
                Ok(Some(true)) => {
                    let n = self.fill()?;
                    if n == 0 {
                        self.closed = true;
                        return Ok(0);
                    }
                }
                Ok(None) => continue,
                Err(e) => return Err(e),
            }
        }
    }
}

impl Write for Ws {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.send_frame(0x2, buf)?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Stream for Ws {
    fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.inner.set_read_timeout(d)
    }
    fn shutdown_write(&mut self) -> io::Result<()> {
        let _ = self.send_frame(0x8, b"");
        self.inner.shutdown_write()
    }
    fn describe(&self) -> &'static str {
        "ws"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream;

    fn pair() -> (Box<dyn Stream>, Box<dyn Stream>) {
        let (a, b) = UnixStream::pair().unwrap();
        (Box::new(super::super::Unix(a)), Box::new(super::super::Unix(b)))
    }

    #[test]
    fn client_server_frame_round_trip_across_lengths() {
        let (a, b) = pair();
        let server = std::thread::spawn(move || {
            let b = b;
            let mut ws = Ws::server(b).unwrap();
            // Echo everything until close.
            let mut out = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                match ws.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => out.extend_from_slice(&buf[..n]),
                    Err(e) if crate::util::is_would_block(&e) => continue,
                    Err(e) => panic!("server read: {e}"),
                }
            }
            out
        });
        let spec = crate::url::parse("ws://test/").unwrap();
        let mut ws = Ws::client(a, &spec).unwrap();
        for n in [0usize, 1, 125, 126, 65535, 65536, 200_000] {
            let data: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
            ws.write_all(&data).unwrap();
            ws.flush().unwrap();
        }
        ws.send_frame(0x8, b"").unwrap();
        let out = server.join().unwrap();
        let mut expect = Vec::new();
        for n in [0usize, 1, 125, 126, 65535, 65536, 200_000] {
            expect.extend((0..n).map(|i| (i % 251) as u8));
        }
        assert_eq!(out, expect);
    }

}
