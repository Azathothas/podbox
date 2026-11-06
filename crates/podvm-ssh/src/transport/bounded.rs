use super::*;

/// The loud end of a stalled lookup: names the host it waited for and the
/// bound it waited out. Built once so the dial and the tests name the same
/// failure.
pub(super) fn dns_timeout(host: &str, deadline: Duration) -> io::Error {
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
pub(super) fn resolve_with<R>(
    deadline: Duration,
    resolve: impl FnOnce() -> io::Result<R> + Send + 'static,
) -> io::Result<R>
where
    R: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let worker = std::thread::Builder::new()
        .name("podvm-ssh-resolve".to_string())
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
        .name("podvm-ssh-pipewrite".to_string())
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
    pub(super) fn spawn(program: &str, args: &[String]) -> io::Result<Box<dyn Stream>> {
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
