//! `run -t --podvm-tier=machine`: a terminal on a virtio console,
//! TODO/machine.md T-1723.
//!
//! The emulator gets a `virtconsole` on a unix socket of the run
//! ([`argv`]). In the guest, `podvm guest-tty CMD...` ([`guest_main`])
//! starts a session, takes `/dev/hvc0` as its controlling terminal on
//! descriptors 0, 1 and 2, and execs the command, so the guest's line
//! discipline echoes and edits as a terminal does. On the host podvm puts
//! its own terminal in raw mode ([`Raw`]), copies stdin to the socket and
//! the socket to stdout ([`pump`]), and restores the terminal at the end.
//! The exit code still comes back on the first serial port.

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

/// The console device of the guest.
pub const GUEST_DEV: &str = "/dev/hvc0";

/// The socket of the terminal of one run, beside the stderr socket (a
/// short path: a unix socket path has at most 108 bytes).
pub fn socket_path(err_sock: &Path) -> PathBuf {
    err_sock.with_extension("tty")
}

/// The emulator's arguments of the terminal.
pub fn argv(sock: &Path) -> Vec<String> {
    vec![
        "-device".into(),
        "virtio-serial-pci,id=podvmvs".into(),
        "-chardev".into(),
        format!(
            "socket,id=podvmtty,path={},server=on,wait=on",
            sock.display()
        ),
        "-device".into(),
        "virtconsole,chardev=podvmtty,name=podvm.tty".into(),
    ]
}

/// `struct termios` of Linux on x86_64 and aarch64 (60 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    iflag: u32,
    oflag: u32,
    cflag: u32,
    lflag: u32,
    line: u8,
    cc: [u8; 32],
    ispeed: u32,
    ospeed: u32,
}

unsafe extern "C" {
    fn tcgetattr(fd: i32, t: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, act: i32, t: *const Termios) -> i32;
    fn cfmakeraw(t: *mut Termios);
    fn isatty(fd: i32) -> i32;
    fn setsid() -> i32;
    fn ioctl(fd: i32, req: u64, ...) -> i32;
    fn dup2(a: i32, b: i32) -> i32;
}

const TCSANOW: i32 = 0;
const TIOCSCTTY: u64 = 0x540e;
const TIOCGWINSZ: u64 = 0x5413;
const TIOCSWINSZ: u64 = 0x5414;

/// Whether descriptor 0 is a terminal.
pub fn stdin_is_tty() -> bool {
    // SAFETY: isatty reads no memory.
    unsafe { isatty(0) == 1 }
}

/// The host terminal in raw mode until drop; nothing where stdin is no
/// terminal.
pub struct Raw(Option<Termios>);

impl Raw {
    pub fn new() -> Raw {
        if !stdin_is_tty() {
            return Raw(None);
        }
        let mut t = std::mem::MaybeUninit::<Termios>::zeroed();
        // SAFETY: tcgetattr fills the termios of this frame.
        if unsafe { tcgetattr(0, t.as_mut_ptr()) } != 0 {
            return Raw(None);
        }
        // SAFETY: tcgetattr succeeded, so the struct is filled.
        let old = unsafe { t.assume_init() };
        let mut raw = old;
        // SAFETY: cfmakeraw edits the copy; tcsetattr reads it.
        unsafe {
            cfmakeraw(&mut raw);
            tcsetattr(0, TCSANOW, &raw);
        }
        Raw(Some(old))
    }
}

impl Drop for Raw {
    fn drop(&mut self) {
        if let Some(old) = &self.0 {
            // SAFETY: the termios that tcgetattr gave.
            unsafe { tcsetattr(0, TCSANOW, old) };
        }
    }
}

/// The rows and columns of the host terminal, where stdin is one.
pub fn size() -> Option<(u16, u16)> {
    let mut ws = [0u16; 4];
    // SAFETY: TIOCGWINSZ fills a struct winsize of four u16.
    let r = unsafe { ioctl(0, TIOCGWINSZ, ws.as_mut_ptr()) };
    (r == 0 && ws[0] > 0 && ws[1] > 0).then_some((ws[0], ws[1]))
}

/// Connect to the socket as soon as the emulator listens, then copy stdin
/// to it and it to stdout. The thread of stdout ends when the emulator
/// closes the port; the thread of stdin is left to the end of podvm.
pub fn pump(sock: PathBuf, deadline: std::time::Instant) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let conn = loop {
            match std::os::unix::net::UnixStream::connect(&sock) {
                Ok(c) => break c,
                Err(_) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(20))
                }
                Err(_) => return,
            }
        };
        if let Ok(mut w) = conn.try_clone() {
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let mut stdin = std::io::stdin();
                let mut last: Option<u8> = None;
                while let Ok(n) = stdin.read(&mut buf) {
                    if n == 0 {
                        // TODO/cli.md T-1817: the end of input of the guest's
                        // terminal is VEOF (`^D`), as in the pty relay of the
                        // rootfs rungs: the socket stays open for the output.
                        let eof: &[u8] = match last {
                            Some(b'\n') | None => &[0x04],
                            Some(_) => &[0x04, 0x04],
                        };
                        let _ = w.write_all(eof);
                        break;
                    }
                    if w.write_all(&buf[..n]).is_err() {
                        break;
                    }
                    last = Some(buf[n - 1]);
                }
            });
        }
        let mut r = conn;
        let mut buf = [0u8; 4096];
        let mut out = std::io::stdout();
        while let Ok(n) = r.read(&mut buf) {
            if n == 0 || out.write_all(&buf[..n]).and_then(|()| out.flush()).is_err() {
                break;
            }
        }
    })
}

/// `podvm guest-tty [ROWSxCOLS] -- CMD...` in a guest: a session with
/// `/dev/hvc0` as its terminal, then the command. Exit 127 where it does not
/// start.
pub fn guest_main(args: &[String]) -> i32 {
    let (size, cmd) = match args.iter().position(|a| a == "--") {
        Some(i) => (args.first().filter(|_| i == 1), &args[i + 1..]),
        None => (None, args),
    };
    let Some((prog, rest)) = cmd.split_first() else {
        eprintln!("podvm guest-tty: no command");
        return 127;
    };
    let tty = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(GUEST_DEV)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("podvm guest-tty: {GUEST_DEV}: {e}");
            return 127;
        }
    };
    let fd = tty.as_raw_fd();
    // SAFETY: plain system calls on the descriptor of this frame.
    unsafe {
        setsid();
        ioctl(fd, TIOCSCTTY, 0);
        if let Some((r, c)) = size
            .and_then(|s| s.split_once('x'))
            .and_then(|(r, c)| Some((r.parse::<u16>().ok()?, c.parse::<u16>().ok()?)))
        {
            let ws = [r, c, 0u16, 0u16];
            ioctl(fd, TIOCSWINSZ, ws.as_ptr());
        }
        for t in 0..3 {
            dup2(fd, t);
        }
    }
    use std::os::unix::process::CommandExt;
    let e = std::process::Command::new(prog).args(rest).exec();
    eprintln!("podvm guest-tty: {prog}: {e}");
    127
}
