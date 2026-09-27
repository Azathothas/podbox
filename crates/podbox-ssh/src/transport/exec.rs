//! `exec://<command>`: the escape hatch. Any command that carries a byte pipe
//! on its standard input and output is a transport, so a deployment can put
//! TURN, gsocket, cloudflared, `ssh -W`, or a one-off script underneath
//! podssh without podssh learning a new protocol.
//!
//! ⛔ This is the reason the "prediscovered relay" list never has to be
//! complete. A missing transport is a command line, not a patch.
//!
//! ⛔ The child's output is drained by a helper thread, so `read` never blocks
//! the pump on a pipe that `SO_RCVTIMEO` cannot time out.

use std::io::{self, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio as ProcStdio};

use super::{ChannelReader, Stream};

pub struct Exec {
    child: Child,
    stdin: Option<ChildStdin>,
    reader: ChannelReader,
}

impl Exec {
    pub fn spawn(command: &str) -> io::Result<Box<dyn Stream>> {
        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .stdin(ProcStdio::piped())
            .stdout(ProcStdio::piped())
            .stderr(ProcStdio::inherit())
            .spawn()
            .map_err(|e| io::Error::other(format!("exec {command:?}: {e}")))?;
        let stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("exec: no stdout pipe"))?;
        Ok(Box::new(Exec {
            child,
            stdin,
            reader: ChannelReader::from_read(stdout),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn exec_carries_bytes_through_cat() {
        let mut s = Exec::spawn("cat").unwrap();
        s.write_all(b"through-the-pipe").unwrap();
        s.flush().unwrap();
        let mut buf = [0u8; 16];
        let mut got = 0;
        while got < buf.len() {
            match s.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) if crate::util::is_would_block(&e) => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(e) => panic!("read: {e}"),
            }
        }
        assert_eq!(&buf, b"through-the-pipe");
    }
}
