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
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio as ProcStdio};
use std::time::Duration;

use crate::error::{Error, Kind};

/// How long one DNS resolution may take. The dial timeout bounds the
/// connects; without this the lookup before them is unbounded, and a
/// stalled resolver wedges the whole dial.
pub const DNS_DEADLINE: Duration = Duration::from_secs(10);
/// How long one relay-leg write waits for the kernel to take it. A peer
/// that stops reading ends the op inside this bound instead of wedging it.
pub const WRITE_DEADLINE: Duration = Duration::from_secs(10);

/// A bidirectional byte stream with a read timeout. The timeout is what lets
/// the single-threaded [`crate::pump`] poll one direction, try the other,
/// and never deadlock.
pub trait Stream: Read + Write + Send {
    /// Bound how long `read` waits for a byte. The pump sets this itself.
    fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()>;
    /// Bound how long `write` waits for the kernel to take a byte. The
    /// relay legs set this at dial; a peer that stops reading ends the op
    /// inside the bound instead of wedging it.
    fn set_write_timeout(&self, d: Option<Duration>) -> io::Result<()>;
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
        // ⛔ The lookup itself is bounded: `to_socket_addrs` carries no
        // deadline, so it runs on one worker and `recv_timeout` ends the
        // dial when the resolver stalls. The bound is the tighter of the
        // dial timeout and `DNS_DEADLINE`, and the failure names the host.
        let deadline = std::cmp::min(self.timeout, DNS_DEADLINE);
        let addrs =
            resolve_bounded(host, port, deadline).map_err(|e| Error::dial(op.clone(), e))?;
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

/// The loud end of a stalled lookup: names the host it waited for and the
/// bound it waited out. Built once so the dial and the tests name the same
/// failure.
fn dns_timeout(host: &str, deadline: Duration) -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        format!(
            "DNS resolution of {host} timed out after {}s",
            deadline.as_secs()
        ),
    )
}

/// Resolve `host:port` inside `deadline`. A stalled resolver fails loud
/// with the host named, as [`Kind::Dial`] at the caller.
pub fn resolve_bounded(host: &str, port: u16, deadline: Duration) -> io::Result<Vec<SocketAddr>> {
    let owned = host.to_string();
    let for_closure = owned.clone();
    resolve_with(deadline, move || {
        (for_closure.as_str(), port)
            .to_socket_addrs()
            .map(|addrs| addrs.collect())
    })
    .map_err(|e| {
        if e.kind() == io::ErrorKind::TimedOut {
            dns_timeout(&owned, deadline)
        } else {
            e
        }
    })
}

/// Run `resolve` on one worker and wait at most `deadline` for it. The
/// worker joins on success and detaches on timeout: a stuck lookup owns no
/// handle the caller must reap, so the detached thread ends with its own
/// call and workers never accumulate past one per dial. The closure is the
/// seam the fault tests stall through; production passes the real lookup.
fn resolve_with<R>(
    deadline: Duration,
    resolve: impl FnOnce() -> io::Result<R> + Send + 'static,
) -> io::Result<R>
where
    R: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let worker = std::thread::Builder::new()
        .name("podbox-ssh-resolve".to_string())
        .spawn(move || {
            let _ = tx.send(resolve());
        });
    // ⛔ A resolver thread that cannot spawn is a failure, not a wait: the
    // dial fails at once naming the spawn instead of hanging on a channel
    // nobody will ever answer.
    let worker = worker.map_err(io::Error::other)?;
    match rx.recv_timeout(deadline) {
        Ok(r) => {
            let _ = worker.join();
            r
        }
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("resolution timed out after {}s", deadline.as_secs()),
        )),
    }
}

/// Normalize an expired send deadline. On a blocking socket the kernel
/// reports it as `WouldBlock` (EAGAIN), not `TimedOut`: with a write
/// deadline set that error means the whole wait passed with no progress,
/// never a momentary full buffer. Callers that set the deadline convert
/// through here so a stall reads as a timeout naming the leg. The message
/// carries no seconds: the bound in force is whatever the socket holds,
/// and the constant beside it would be a guess.
pub fn write_stall(e: io::Error, leg: &str) -> io::Error {
    if e.kind() == io::ErrorKind::WouldBlock {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!("{leg} stalled past its write bound"),
        )
    } else {
        e
    }
}

/// Write `buf` to a pipe-backed `w` inside `deadline`, naming `leg` on a
/// stall. Pipes carry no kernel write deadline (`set_write_timeout` is a
/// no-op on them by design), so the write runs on one detached worker and
/// the caller waits bounded. A stall fails the op loud; the worker's late
/// bytes land in a pipe whose session already ended. Callers pass at most
/// one line (64 KiB plus the newline), never a stream.
pub fn write_pipe_bounded<W>(w: &mut W, buf: &[u8], deadline: Duration, leg: &str) -> io::Result<()>
where
    W: Write + AsFd,
{
    let owned = std::fs::File::from(w.as_fd().try_clone_to_owned()?);
    let bytes = buf.to_vec();
    let leg = leg.to_string();
    let (tx, rx) = std::sync::mpsc::channel::<io::Result<()>>();
    std::thread::Builder::new()
        .name("podbox-ssh-pipewrite".to_string())
        .spawn(move || {
            let mut f = owned;
            let r = f.write_all(&bytes).and_then(|()| f.flush());
            let _ = tx.send(r);
        })
        .map_err(io::Error::other)?;
    match rx.recv_timeout(deadline) {
        Ok(r) => r,
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("{leg}: pipe write stalled over {}s", deadline.as_secs()),
        )),
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
    fn set_write_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.0.set_write_timeout(d)
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
    fn set_write_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.0.set_write_timeout(d)
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
    fn set_write_timeout(&self, _d: Option<Duration>) -> io::Result<()> {
        // ⛔ A pipe carries no write timeout either. Callers that must bound
        // a pipe write use `write_pipe_bounded`, which waits on a worker
        // instead of the kernel.
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
    fn set_write_timeout(&self, _d: Option<Duration>) -> io::Result<()> {
        // ⛔ Standard output may be a pipe, which carries no write timeout.
        // A stalled console here is outside the relay legs' contract.
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
                program: "podbox-ssh-no-such-program-xyz".to_string(),
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
                if let Ok(name) = std::fs::read_to_string(e.path().join("comm")) {
                    if name.trim().starts_with(prefix) {
                        n += 1;
                    }
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
        let e = dns_timeout("no-such-podbox-host-xyz", DNS_DEADLINE);
        assert!(e.to_string().contains("no-such-podbox-host-xyz"), "{e}");
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
            if named_threads("podbox-ssh-") == 0 {
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
}
