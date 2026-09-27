//! Transports. A transport turns a scheme into a bidirectional byte stream.
//!
//! ⛔ Nothing above this module knows which scheme it is using. `serve`,
//! `connect`, `relay` and `probe` all take a `&dyn Stream`, so adding a
//! transport is a new arm in [`Dialer::dial`] and one file, not a change to
//! the protocol.

pub mod exec;
pub mod proxy;
pub mod tls;
pub mod ws;

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;

use crate::url::{self, Spec};

/// A bidirectional byte stream with a read timeout. The timeout is what lets
/// a single-threaded [`crate::util::pump`] poll one direction, try the other,
/// and never deadlock.
pub trait Stream: Read + Write + Send {
    fn set_read_timeout(&self, _d: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
    /// Half-close the write side so the peer sees EOF and can finish. A blind
    /// pipe that never does this deadlocks the moment one side stops talking.
    fn shutdown_write(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn describe(&self) -> &'static str {
        "stream"
    }
}

impl std::fmt::Debug for dyn Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stream({})", self.describe())
    }
}

/// Plain TCP.
pub struct Tcp(pub TcpStream);

impl Read for Tcp {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}
impl Write for Tcp {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl Stream for Tcp {
    fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.0.set_read_timeout(d)
    }
    fn shutdown_write(&mut self) -> io::Result<()> {
        self.0.shutdown(std::net::Shutdown::Write)
    }
    fn describe(&self) -> &'static str {
        "tcp"
    }
}

/// An `AF_UNIX` stream. This is the transport the bindless cages can carry,
/// because `bind(2)` on TCP is denied there while a unix socket is not.
pub struct Unix(pub UnixStream);

impl Read for Unix {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}
impl Write for Unix {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl Stream for Unix {
    fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.0.set_read_timeout(d)
    }
    fn shutdown_write(&mut self) -> io::Result<()> {
        self.0.shutdown(std::net::Shutdown::Write)
    }
    fn describe(&self) -> &'static str {
        "unix"
    }
}

/// How podssh reaches every transport that is not `unix` or `exec`. The
/// optional proxy is the *egress* hop a sealed cage must take: `http-connect`
/// and `socks5` are dialed first and asked to CONNECT to the real target.
pub struct Dialer {
    pub proxy: Option<proxy::Proxy>,
    pub tls: Arc<tls::ClientConfig>,
    pub timeout: Duration,
}

impl Dialer {
    pub fn new(
        proxy: Option<proxy::Proxy>,
        tls: Arc<tls::ClientConfig>,
        timeout: Duration,
    ) -> Self {
        Dialer {
            proxy,
            tls,
            timeout,
        }
    }

    /// Dial one transport spec.
    pub fn dial(&self, raw: &str) -> io::Result<Box<dyn Stream>> {
        let spec = url::parse(raw)?;
        match spec.scheme.as_str() {
            "tcp" => self.dial_tcp(&spec),
            "unix" => {
                let path = spec.path.clone();
                Ok(Box::new(Unix(UnixStream::connect(path)?)))
            }
            "tls" => {
                let base = self.dial_tcp(&spec)?;
                Ok(Box::new(tls::Tls::connect(
                    base,
                    spec.host.as_deref().unwrap_or_default(),
                    self.tls.clone(),
                )?))
            }
            "ws" => {
                let base = self.dial_tcp(&spec)?;
                Ok(Box::new(ws::Ws::client(base, &spec)?))
            }
            "ws+unix" => {
                let base: Box<dyn Stream> = Box::new(Unix(UnixStream::connect(&spec.path)?));
                Ok(Box::new(ws::Ws::client(base, &spec)?))
            }
            "tls+unix" => {
                let base: Box<dyn Stream> = Box::new(Unix(UnixStream::connect(&spec.path)?));
                Ok(Box::new(tls::Tls::connect(
                    base,
                    "localhost",
                    self.tls.clone(),
                )?))
            }
            "wss+unix" => {
                let base: Box<dyn Stream> = Box::new(Unix(UnixStream::connect(&spec.path)?));
                let base: Box<dyn Stream> =
                    Box::new(tls::Tls::connect(base, "localhost", self.tls.clone())?);
                Ok(Box::new(ws::Ws::client(base, &spec)?))
            }
            "wss" => {
                let base = self.dial_tcp(&spec)?;
                let host = spec.host.clone().unwrap_or_default();
                let base: Box<dyn Stream> =
                    Box::new(tls::Tls::connect(base, &host, self.tls.clone())?);
                Ok(Box::new(ws::Ws::client(base, &spec)?))
            }
            "exec" => exec::Exec::spawn(&spec.path),
            other => Err(io::Error::other(format!(
                "unknown transport scheme {other:?} in {raw:?}; \
                 podssh knows tcp, unix, tls, ws, wss and exec"
            ))),
        }
    }

    /// Dial `host:port`, through the egress proxy when one is configured.
    pub fn dial_tcp(&self, spec: &Spec) -> io::Result<Box<dyn Stream>> {
        let host = spec
            .host
            .clone()
            .ok_or_else(|| io::Error::other(format!("{} needs a host", spec.raw)))?;
        let port = spec
            .port_or_default()
            .ok_or_else(|| io::Error::other(format!("{} needs a port", spec.raw)))?;
        self.dial_addr(&host, port)
    }

    pub fn dial_addr(&self, host: &str, port: u16) -> io::Result<Box<dyn Stream>> {
        if let Some(p) = &self.proxy {
            return p.connect(host, port, self.timeout);
        }
        Ok(Box::new(Tcp(connect_tcp(host, port, self.timeout)?)))
    }
}

pub fn connect_tcp(host: &str, port: u16, timeout: Duration) -> io::Result<TcpStream> {
    let addrs: Vec<std::net::SocketAddr> =
        std::net::ToSocketAddrs::to_socket_addrs(&(host, port))?.collect();
    let mut last = io::Error::other(format!("no address for {host}:{port}"));
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(s) => {
                let _ = s.set_nodelay(true);
                return Ok(s);
            }
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// Wrap a raw [`TcpStream`] in the [`Stream`] trait.
pub fn boxed_tcp(s: TcpStream) -> Box<dyn Stream> {
    Box::new(Tcp(s))
}

/// The caller's own standard input and output, as one stream. This is what
/// makes `podssh connect` a valid `ProxyCommand`: ssh writes and reads this
/// process's descriptors, and podssh moves those bytes to the relay.
/// A stream whose reads never block: a helper thread drains the real
/// descriptor into a channel and `read` pulls whatever has arrived. This is
/// what lets the single-threaded pump never stall on a pipe or a terminal,
/// which `SO_RCVTIMEO` cannot time out.
///
/// ⛔ This exists because the first version blocked in `stdio.read` before it
/// ever tried the relay, which deadlocked every ssh session started by a real
/// `ssh` process. A manual `printf | podssh connect` did not reproduce it,
/// because a pipe reaches EOF and a ProxyCommand's socketpair does not.
pub(crate) struct ChannelReader {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    buf: Vec<u8>,
    pos: usize,
    closed: bool,
}

impl ChannelReader {
    pub fn from_read<R: Read + Send + 'static>(mut r: R) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut b = [0u8; 65536];
            loop {
                match r.read(&mut b) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(b[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        });
        ChannelReader {
            rx,
            buf: Vec::new(),
            pos: 0,
            closed: false,
        }
    }

    pub fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.pos < self.buf.len() {
                let n = std::cmp::min(out.len(), self.buf.len() - self.pos);
                out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
                self.pos += n;
                if self.pos == self.buf.len() {
                    self.buf.clear();
                    self.pos = 0;
                }
                return Ok(n);
            }
            if self.closed {
                return Ok(0);
            }
            match self.rx.try_recv() {
                Ok(v) => {
                    self.buf = v;
                    self.pos = 0;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    return Err(io::Error::new(io::ErrorKind::WouldBlock, "no bytes yet"))
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.closed = true;
                    return Ok(0);
                }
            }
        }
    }
}

/// The caller's own standard input and output, as one stream. This is what
/// makes `podssh connect` a valid `ProxyCommand`: ssh writes and reads this
/// process's descriptors, and podssh moves those bytes to the relay.
pub struct Stdio {
    reader: ChannelReader,
    output: std::fs::File,
}

impl Stdio {
    pub fn new() -> io::Result<Self> {
        use std::os::fd::AsFd;
        let input = std::fs::File::from(std::io::stdin().as_fd().try_clone_to_owned()?);
        let output = std::fs::File::from(std::io::stdout().as_fd().try_clone_to_owned()?);
        Ok(Stdio {
            reader: ChannelReader::from_read(input),
            output,
        })
    }
}

impl Read for Stdio {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}
impl Write for Stdio {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}
impl Stream for Stdio {
    fn describe(&self) -> &'static str {
        "stdio"
    }
}

/// A read that returns `Ok(0)` on timeout, for handshakes whose bytes must
/// not overrun the message they belong to.
pub fn read_byte_timeout(s: &mut dyn Stream, deadline: Duration) -> io::Result<Option<u8>> {
    s.set_read_timeout(Some(deadline))?;
    let mut b = [0u8; 1];
    match s.read(&mut b) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(b[0])),
        Err(e) if crate::util::is_would_block(&e) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Read exactly one CRLF-terminated header block, one byte at a time so a
/// transport can never lose bytes that arrive after `\r\n\r\n`.
pub fn read_headers(s: &mut dyn Stream, limit: usize, timeout: Duration) -> io::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(256);
    let mut b = [0u8; 1];
    s.set_read_timeout(Some(timeout))?;
    while out.len() < limit {
        match s.read(&mut b) {
            Ok(0) => break,
            Ok(_) => {
                out.push(b[0]);
                if out.ends_with(b"\r\n\r\n") {
                    return Ok(out);
                }
            }
            Err(e) if crate::util::is_would_block(&e) => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "header read timed out",
                ))
            }
            Err(e) => return Err(e),
        }
    }
    if out.ends_with(b"\r\n\r\n") {
        Ok(out)
    } else {
        Err(io::Error::other("header block was not terminated"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_and_unix_satisfy_stream() {
        let (a, b) = UnixStream::pair().unwrap();
        let mut a: Box<dyn Stream> = Box::new(Unix(a));
        let mut b: Box<dyn Stream> = Box::new(Unix(b));
        a.write_all(b"ping").unwrap();
        a.flush().unwrap();
        let mut buf = [0u8; 4];
        b.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"ping");
    }
}
