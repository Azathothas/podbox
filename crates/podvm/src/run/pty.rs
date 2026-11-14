//! `run -t`: a pty for the payload, TODO/cli.md T-1760.
//!
//! ⭐ docker's `-t` gives the payload a terminal also where the caller has
//! none: `docker run -t alpine sh -c 'test -t 1'` exits 0 in a pipe. podvm
//! opens `/dev/ptmx` of the host, hands the slave to the payload as its
//! descriptors 0, 1 and 2, and relays: the master to podvm's stdout (the
//! tee of the log, T-1421), and podvm's stdin to the master. Where podvm's
//! stdin is a terminal, the pty takes its window size, and that terminal is
//! raw for the run, so each key goes to the payload once and its echo comes
//! from the pty.
//!
//! ⚠ The slave is not the controlling terminal of the payload: the entry of
//! `pod-enter` makes no session. `isatty`, `ttyname` and the line discipline
//! work; job control of an interactive shell does not.

use std::ffi::{c_int, c_ulong};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::time::Duration;

unsafe extern "C" {
    fn ioctl(fd: c_int, req: c_ulong, ...) -> c_int;
}

use crate::machine::guest::tty::Raw;

const TIOCGPTN: c_ulong = 0x8004_5430;
const TIOCSPTLCK: c_ulong = 0x4004_5431;
const TIOCGWINSZ: c_ulong = 0x5413;
const TIOCSWINSZ: c_ulong = 0x5414;
const O_NOCTTY: i32 = 0o400;

/// The master and the slave of one pty.
pub struct Pty {
    master: std::fs::File,
    slave: OwnedFd,
}

impl Pty {
    /// Open a pty, unlocked, with the window size of podvm's terminal.
    pub fn open() -> std::io::Result<Pty> {
        let master = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NOCTTY)
            .open("/dev/ptmx")?;
        let fd = master.as_raw_fd();
        let unlock: c_int = 0;
        let mut n: u32 = 0;
        // SAFETY: both requests take a pointer to an int of the kernel.
        if unsafe { ioctl(fd, TIOCSPTLCK, &unlock) } != 0
            || unsafe { ioctl(fd, TIOCGPTN, &mut n) } != 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let slave = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NOCTTY)
            .open(format!("/dev/pts/{n}"))?;
        let mut size = [0u16; 4];
        // SAFETY: `struct winsize` is four unsigned shorts.
        if unsafe { ioctl(0, TIOCGWINSZ, size.as_mut_ptr()) } == 0 {
            unsafe { ioctl(fd, TIOCSWINSZ, size.as_ptr()) };
        }
        Ok(Pty {
            master,
            slave: slave.into(),
        })
    }

    /// Three descriptors of the slave, for the payload's 0, 1 and 2: the
    /// child closes each host descriptor after its `dup2`, so each needs
    /// its own.
    pub fn slaves(&self) -> std::io::Result<[OwnedFd; 3]> {
        Ok([
            self.slave.try_clone()?,
            self.slave.try_clone()?,
            self.slave.try_clone()?,
        ])
    }

    /// Start the relay. The caller drops the slaves that it passed after
    /// the payload exits, then calls [`Relay::finish`].
    pub fn relay(self) -> std::io::Result<Relay> {
        let mut from = self.master.try_clone()?;
        let mut to = self.master;
        let raw = Raw::new();
        let (done, wait) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            let mut out = std::io::stdout().lock();
            // EIO is the end: no slave is open any more.
            while let Ok(n) = from.read(&mut buf) {
                if n == 0 || out.write_all(&buf[..n]).is_err() {
                    break;
                }
                let _ = out.flush();
            }
            let _ = done.send(());
        });
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut input = std::io::stdin().lock();
            let mut last: Option<u8> = None;
            while let Ok(n) = input.read(&mut buf) {
                if n == 0 {
                    // TODO/cli.md T-1817: a pty has no half-close, and the
                    // reader keeps a clone of the master, so the payload
                    // never read 0. The end of input of a terminal is VEOF:
                    // one at the start of a line, and one more after a
                    // partial line, which the first only delivers. A fresh
                    // pty's VEOF is 0x04.
                    let eof: &[u8] = match last {
                        Some(b'\n') | None => &[VEOF],
                        Some(_) => &[VEOF, VEOF],
                    };
                    let _ = to.write_all(eof);
                    break;
                }
                if to.write_all(&buf[..n]).is_err() {
                    break;
                }
                last = Some(buf[n - 1]);
            }
        });
        Ok(Relay {
            slave: self.slave,
            wait,
            raw,
        })
    }
}

/// The VEOF character of a fresh pty (`^D`).
const VEOF: u8 = 0x04;

/// The relay of a running payload.
pub struct Relay {
    slave: OwnedFd,
    wait: std::sync::mpsc::Receiver<()>,
    raw: Raw,
}

impl Relay {
    /// Drain the output of the payload after it exited, for at most two
    /// seconds: a process of the payload's tree that still holds the slave
    /// keeps the master open, and podvm does not wait for it.
    pub fn finish(self) {
        drop(self.slave);
        let _ = self.wait.recv_timeout(Duration::from_secs(2));
        drop(self.raw);
    }
}

/// A descriptor of the slave as the raw number that `Fds::pass` takes; the
/// caller keeps the `OwnedFd` until the payload exits.
pub fn raw(fd: &OwnedFd) -> i64 {
    i64::from(fd.as_raw_fd())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::FromRawFd;

    #[test]
    fn a_pty_gives_a_terminal_to_its_slave() {
        let Ok(p) = Pty::open() else {
            eprintln!("T-1760: /dev/ptmx does not open here; skipped");
            return;
        };
        let [a, b, c] = p.slaves().unwrap();
        unsafe extern "C" {
            fn isatty(fd: c_int) -> c_int;
        }
        for fd in [&a, &b, &c] {
            assert_eq!(unsafe { isatty(fd.as_raw_fd()) }, 1);
        }
        let mut f = unsafe { std::fs::File::from_raw_fd(std::os::fd::IntoRawFd::into_raw_fd(a)) };
        f.write_all(b"hi\n").unwrap();
        let mut m = p.master.try_clone().unwrap();
        let mut buf = [0u8; 16];
        let n = m.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"hi\r\n", "the line discipline maps NL to CR NL");
    }

    /// TODO/cli.md T-1817: VEOF written to the master is the end of input
    /// of the slave: after a full line one gives a read of 0, and after a
    /// partial line the first delivers the line and the second gives 0.
    #[test]
    fn veof_ends_the_input_of_the_slave() {
        let Ok(p) = Pty::open() else {
            eprintln!("T-1760: /dev/ptmx does not open here; skipped");
            return;
        };
        let [a, _b, _c] = p.slaves().unwrap();
        let mut slave =
            unsafe { std::fs::File::from_raw_fd(std::os::fd::IntoRawFd::into_raw_fd(a)) };
        let mut m = p.master.try_clone().unwrap();
        let mut buf = [0u8; 64];
        m.write_all(b"line\n").unwrap();
        m.write_all(&[VEOF]).unwrap();
        assert_eq!(slave.read(&mut buf).unwrap(), 5, "the line");
        assert_eq!(
            slave.read(&mut buf).unwrap(),
            0,
            "VEOF at the start of a line"
        );
        m.write_all(b"part").unwrap();
        m.write_all(&[VEOF, VEOF]).unwrap();
        assert_eq!(slave.read(&mut buf).unwrap(), 4, "the partial line");
        assert_eq!(slave.read(&mut buf).unwrap(), 0, "the second VEOF");
    }
}
