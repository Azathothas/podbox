//! The fake relay's side of a WebSocket: the upgrade, which it answers with
//! 101 or refuses with a status as the live relay does, and whole frames
//! over the frame codec of `pod-ws` (PodUtils/podcrates). A ping is
//! answered, a pong dropped, and a Close echoed once and returned.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use pod_ws::frame::{self, Role};

/// Largest HTTP head read during the upgrade.
const MAX_HEAD: usize = 16 * 1024;

/// One complete message from the peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Text(Vec<u8>),
    Binary(Vec<u8>),
    Close(u16, Vec<u8>),
}

/// What the peer asked for: the path and the headers.
pub struct ServerRequest {
    pub path: String,
    pub headers: Vec<(String, String)>,
}

/// An upgrade read and not yet answered.
pub struct Upgrade {
    tcp: TcpStream,
    key: String,
}

/// One accepted WebSocket.
pub struct Ws {
    tcp: TcpStream,
    rbuf: Vec<u8>,
    close_sent: bool,
}

impl Ws {
    /// Read the upgrade of `tcp`: the answer is the caller's.
    pub fn server(mut tcp: TcpStream) -> io::Result<(Upgrade, ServerRequest)> {
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            if head.len() > MAX_HEAD {
                return Err(io::Error::other("the upgrade head is too long"));
            }
            tcp.read_exact(&mut byte)?;
            head.push(byte[0]);
        }
        let head =
            String::from_utf8(head).map_err(|_| io::Error::other("a head that is not UTF-8"))?;
        let mut lines = head.split("\r\n");
        let path = lines
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("/")
            .to_string();
        let headers: Vec<(String, String)> = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .collect();
        let key = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("sec-websocket-key"))
            .map(|(_, v)| v.clone())
            .ok_or_else(|| io::Error::other("no Sec-WebSocket-Key"))?;
        Ok((Upgrade { tcp, key }, ServerRequest { path, headers }))
    }

    /// Read one message. `Ok(None)`: nothing came within the read timeout.
    /// The end of the connection with no Close is an error.
    pub fn read_frame(&mut self) -> io::Result<Option<Frame>> {
        loop {
            let decoded = frame::decode(&self.rbuf, Role::Client)
                .map_err(|e| io::Error::other(e.to_string()))?;
            if let Some((f, used)) = decoded {
                self.rbuf.drain(..used);
                match f.opcode {
                    frame::OPCODE_TEXT => return Ok(Some(Frame::Text(f.payload))),
                    frame::OPCODE_BINARY => return Ok(Some(Frame::Binary(f.payload))),
                    frame::OPCODE_CLOSE => {
                        let code = f
                            .payload
                            .get(..2)
                            .map_or(1005, |c| u16::from_be_bytes([c[0], c[1]]));
                        let reason = f.payload.get(2..).unwrap_or_default().to_vec();
                        if !self.close_sent {
                            let _ = self.send(frame::OPCODE_CLOSE, &f.payload);
                        }
                        return Ok(Some(Frame::Close(code, reason)));
                    }
                    frame::OPCODE_PING => self.send(frame::OPCODE_PONG, &f.payload)?,
                    _ => {}
                }
                continue;
            }
            let mut chunk = [0u8; 64 * 1024];
            match self.tcp.read(&mut chunk) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "the peer went away with no Close",
                    ))
                }
                Ok(n) => self.rbuf.extend_from_slice(&chunk[..n]),
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    return Ok(None)
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn send(&mut self, opcode: u8, payload: &[u8]) -> io::Result<()> {
        if opcode == frame::OPCODE_CLOSE {
            self.close_sent = true;
        }
        let f = frame::Frame {
            fin: true,
            opcode,
            payload: payload.to_vec(),
        };
        self.tcp.write_all(&frame::encode(&f, Role::Server, [0; 4]))
    }

    pub fn send_text(&mut self, payload: &[u8]) -> io::Result<()> {
        self.send(frame::OPCODE_TEXT, payload)
    }

    pub fn send_binary(&mut self, payload: &[u8]) -> io::Result<()> {
        self.send(frame::OPCODE_BINARY, payload)
    }

    pub fn send_close(&mut self, code: u16, reason: &[u8]) -> io::Result<()> {
        let payload = [&code.to_be_bytes()[..], reason].concat();
        self.send(frame::OPCODE_CLOSE, &payload)
    }

    pub fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.tcp.set_read_timeout(d)
    }
}

impl Upgrade {
    /// Answer 101: the socket is a WebSocket from now on.
    pub fn accept(mut self) -> io::Result<Ws> {
        let accept = pod_ws::handshake::accept_key(&self.key);
        write!(
            self.tcp,
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        )?;
        Ok(Ws {
            tcp: self.tcp,
            rbuf: Vec::new(),
            close_sent: false,
        })
    }

    /// Refuse the upgrade with `status`, as the relay refuses a wrong token
    /// (403) or a second node on a name (409).
    pub fn refuse(mut self, status: u16, body: &str) {
        let _ = write!(
            self.tcp,
            "HTTP/1.1 {status} Refused\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    }
}
