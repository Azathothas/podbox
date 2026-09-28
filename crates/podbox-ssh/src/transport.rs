//! Transports. A transport turns a target into a bidirectional byte stream.
//!
//! ⛔ Nothing above this module knows which target it is using. The proxy
//! binary and the pump both take a `Box<dyn Stream>`, so adding a transport
//! is a new arm in [`Dialer::dial`] and one type, not a change to the pump.
//!
//! ⛔ One read path and one write path. Every byte in comes through `Read`
//! and every byte out through `Write`. The drain thread below exists for one
//! reason: a pipe carries no read timeout, so a reader that handed a pipe to
//! the pump would stall it. The thread moves those bytes into a channel and
//! `read` pulls whatever has arrived, or reports `WouldBlock` when nothing
//! has. The pump itself stays single-threaded.

use std::fs::File;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio as ProcStdio};
use std::time::Duration;

use crate::error::{Error, Kind};

/// A bidirectional byte stream with a read timeout. The timeout is what lets
/// the single-threaded [`crate::pump`] poll one direction, try the other,
/// and never deadlock.
pub trait Stream: Read + Write + Send {
    /// Bound how long `read` waits for a byte. The pump sets this itself.
    fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()>;
    /// Half-close the write side so the peer sees EOF and can finish. A
    /// blind pipe that never does this deadlocks the moment one side stops
    /// talking.
    fn shutdown_write(&mut self) -> io::Result<()>;
    /// The transport name for logs and error operations.
    fn describe(&self) -> &'static str {
        "stream"
    }
}

impl std::fmt::Debug for dyn Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stream({})", self.describe())
    }
}

/// What to dial. Three shapes, and no fourth hiding in a string: TCP for a
/// network hop, a Unix socket for a local server, a child process for
/// anything that carries a byte pipe on its standard input and output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Tcp { host: String, port: u16 },
    Unix { path: PathBuf },
    Exec { program: String, args: Vec<String> },
}

impl Target {
    /// Parse `host:port`, `[v6]:port`, or a bare `host` with `default_port`.
    ///
    /// ⛔ A bare host gets `default_port` and never a guessed port. A name
    /// with no dot in it is a name: `localhost` reaches the loopback
    /// resolver, and a parser that demands a dot sends it nowhere.
    pub fn parse_tcp(spec: &str, default_port: u16) -> Result<Target, Error> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err(Error::new(
                Kind::Usage,
                "target",
                "the destination is empty",
            ));
        }
        // Bracketed IPv6, with or without a port: [::1] or [::1]:22.
        if let Some(rest) = spec.strip_prefix('[') {
            let (host, tail) = rest.split_once(']').ok_or_else(|| {
                Error::new(
                    Kind::Usage,
                    "target",
                    format!("{spec:?} opens [ and never closes it"),
                )
            })?;
            if host.is_empty() {
                return Err(Error::new(
                    Kind::Usage,
                    "target",
                    format!("{spec:?} has an empty host"),
                ));
            }
            // The tail after `]` is `""` or `":port"`, so the leading colon
            // belongs to the separator, not to the port.
            let port = match tail.strip_prefix(':') {
                None if tail.is_empty() => default_port,
                None => {
                    return Err(Error::new(
                        Kind::Usage,
                        "target",
                        format!("{spec:?} has {tail:?} after ] where :port or nothing was meant"),
                    ));
                }
                Some(p) => parse_port(p)?,
            };
            return Ok(Target::Tcp {
                host: host.to_string(),
                port,
            });
        }
        // More than one colon and no brackets is a bare IPv6 literal, which
        // carries no port. `::1` is an address, not `host:port`.
        if spec.matches(':').count() > 1 {
            return Ok(Target::Tcp {
                host: spec.to_string(),
                port: default_port,
            });
        }
        match spec.split_once(':') {
            Some((host, p)) => {
                if host.is_empty() {
                    return Err(Error::new(
                        Kind::Usage,
                        "target",
                        format!("{spec:?} has no host"),
                    ));
                }
                Ok(Target::Tcp {
                    host: host.to_string(),
                    port: parse_port(p)?,
                })
            }
            None => Ok(Target::Tcp {
                host: spec.to_string(),
                port: default_port,
            }),
        }
    }

    /// The `host:port` spelling for logs and error operations.
    pub fn plain(&self) -> String {
        match self {
            Target::Tcp { host, port } => format!("{host}:{port}"),
            Target::Unix { path } => format!("unix {}", path.display()),
            Target::Exec { program, args } => {
                let mut s = program.clone();
                for a in args {
                    s.push(' ');
                    s.push_str(a);
                }
                s
            }
        }
    }
}

/// Parse a port string. Refused values are usage failures, not panics:
/// a caller that passes a non-numeric port must get an error, not an abort.
/// Port 0 is refused: a destination on port 0 is never what a caller meant.
pub fn parse_port(s: &str) -> Result<u16, Error> {
    let n: u32 = s
        .parse()
        .map_err(|_| Error::new(Kind::Usage, "target", format!("{s:?} is not a port")))?;
    if n == 0 {
        return Err(Error::new(
            Kind::Usage,
            "target",
            format!("{s:?}: port 0 names no service"),
        ));
    }
    if n > u16::MAX as u32 {
        return Err(Error::new(
            Kind::Usage,
            "target",
            format!("{s:?}: port {n} is over 65535"),
        ));
    }
    Ok(n as u16)
}

/// Dials targets. One dispatch, per-dial timeouts.
pub struct Dialer {
    /// How long one dial waits. Every address a name resolves to gets this
    /// long, so a dial is always bounded.
    pub timeout: Duration,
}

impl Dialer {
    pub fn new(timeout: Duration) -> Self {
        Dialer { timeout }
    }

    /// Dial one target. This is the one dispatch: every caller reaches every
    /// transport through here.
    pub fn dial(&self, target: &Target) -> Result<Box<dyn Stream>, Error> {
        match target {
            Target::Tcp { host, port } => self.dial_tcp(host, *port),
            Target::Unix { path } => self.dial_unix(path),
            Target::Exec { program, args } => self.dial_exec(program, args),
        }
    }

    fn dial_tcp(&self, host: &str, port: u16) -> Result<Box<dyn Stream>, Error> {
        let op = format!("dial tcp {host}:{port}");
        let mut last = io::Error::new(
            io::ErrorKind::NotFound,
            format!("no address for {host}:{port}"),
        );
        // ⛔ Every resolved address is tried, in order. The first address is
        // not always the reachable one.
        let addrs: Vec<_> = (host, port)
            .to_socket_addrs()
            .map_err(|e| Error::dial(op.clone(), e))?
            .collect();
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, self.timeout) {
                Ok(s) => {
                    // ⛔ Best-effort latency hint. A stream that cannot set
                    // it still carries bytes, so this must not fail the dial.
                    let _ = s.set_nodelay(true);
                    return Ok(Box::new(Tcp(s)));
                }
                Err(e) => last = e,
            }
        }
        Err(Error::dial(op, last))
    }

    fn dial_unix(&self, path: &Path) -> Result<Box<dyn Stream>, Error> {
        let op = format!("dial unix {}", path.display());
        match UnixStream::connect(path) {
            Ok(s) => Ok(Box::new(Unix(s))),
            Err(e) => Err(Error::dial(op, e)),
        }
    }

    fn dial_exec(&self, program: &str, args: &[String]) -> Result<Box<dyn Stream>, Error> {
        let op = format!("dial exec {program}");
        match Exec::spawn(program, args) {
            Ok(s) => Ok(s),
            Err(e) => Err(Error::dial(op, e)),
        }
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

/// An `AF_UNIX` stream. This is the transport the local server speaks,
/// because it needs no listener and no port.
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

/// A reader that never blocks the pump: a helper thread drains the real
/// descriptor into a channel and `read` pulls whatever has arrived. This is
/// the one read path for descriptors that carry no read timeout, which is
/// child stdio pipes and process standard input.
struct PipeReader {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    buf: Vec<u8>,
    pos: usize,
    closed: bool,
}

impl PipeReader {
    fn from_read<R: Read + Send + 'static>(mut r: R) -> Self {
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
        PipeReader {
            rx,
            buf: Vec::new(),
            pos: 0,
            closed: false,
        }
    }

    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
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
                    return Err(io::Error::new(io::ErrorKind::WouldBlock, "no bytes yet"));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.closed = true;
                    return Ok(0);
                }
            }
        }
    }
}

/// A child process whose standard input and output are the byte pipe. Any
/// command that carries bytes on descriptors 0 and 1 is a transport.
pub struct Exec {
    child: Child,
    stdin: Option<ChildStdin>,
    reader: PipeReader,
}

impl Exec {
    fn spawn(program: &str, args: &[String]) -> io::Result<Box<dyn Stream>> {
        // ⛔ No shell. The program runs with exactly the arguments given, so
        // a path with a space in it stays one argument.
        let mut child = Command::new(program)
            .args(args)
            .stdin(ProcStdio::piped())
            .stdout(ProcStdio::piped())
            .stderr(ProcStdio::inherit())
            .spawn()
            .map_err(|e| io::Error::other(format!("exec {program:?}: {e}")))?;
        let stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("exec: no stdout pipe"))?;
        Ok(Box::new(Exec {
            child,
            stdin,
            reader: PipeReader::from_read(stdout),
        }))
    }
}

impl Read for Exec {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}
impl Write for Exec {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.stdin {
            Some(s) => s.write(buf),
            None => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "exec: stdin already closed",
            )),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match &mut self.stdin {
            Some(s) => s.flush(),
            None => Ok(()),
        }
    }
}
impl Stream for Exec {
    fn set_read_timeout(&self, _d: Option<Duration>) -> io::Result<()> {
        // ⛔ A pipe carries no read timeout. The drain thread reports
        // `WouldBlock` instead, so accepting a timeout here would be a lie.
        Ok(())
    }
    fn shutdown_write(&mut self) -> io::Result<()> {
        self.stdin.take();
        Ok(())
    }
    fn describe(&self) -> &'static str {
        "exec"
    }
}

impl Drop for Exec {
    fn drop(&mut self) {
        // Closing stdin lets a well-behaved command exit; the kill is the
        // backstop for one that does not.
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The caller's own standard input and output, as one stream. This is what
/// makes the proxy binary a valid `ProxyCommand`: ssh writes and reads this
/// process's descriptors, and the pump moves those bytes to the target.
pub struct Stdio {
    reader: PipeReader,
    output: File,
}

impl Stdio {
    pub fn new() -> io::Result<Self> {
        let input = File::from(io::stdin().as_fd().try_clone_to_owned()?);
        let output = File::from(io::stdout().as_fd().try_clone_to_owned()?);
        Ok(Stdio {
            reader: PipeReader::from_read(input),
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
    fn set_read_timeout(&self, _d: Option<Duration>) -> io::Result<()> {
        // ⛔ Standard input may be a pipe, which carries no read timeout.
        // The drain thread reports `WouldBlock` instead.
        Ok(())
    }
    fn shutdown_write(&mut self) -> io::Result<()> {
        // ⛔ Standard output has no half-close. The process exiting is the
        // EOF signal, so there is nothing to do here.
        Ok(())
    }
    fn describe(&self) -> &'static str {
        "stdio"
    }
}

#[cfg(test)]
mod tests {
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
            path: PathBuf::from("/tmp/podbox-ssh-test-no-such-socket-xyz"),
        };
        let e = dialer.dial(&target).unwrap_err();
        assert_eq!(e.kind(), Kind::Dial);
        assert!(e.to_string().contains("no-such-socket"), "{e}");
    }

    #[test]
    fn dial_tcp_refused_is_a_dial_error_naming_the_target() {
        // Bind a listener, read its port, then drop it: the port is closed
        // on this machine whatever else runs here.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let dialer = Dialer::new(Duration::from_secs(5));
        let target = Target::Tcp {
            host: "127.0.0.1".to_string(),
            port,
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
                program: "podbox-ssh-no-such-program-xyz".to_string(),
                args: Vec::new(),
            })
            .unwrap_err();
        assert_eq!(e.kind(), Kind::Dial);
    }
}
