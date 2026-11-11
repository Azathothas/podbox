//! The second serial port of a guest: the command's stderr, apart from its
//! stdout, TODO/machine.md T-1656.
//!
//! The first serial port carries the console, the markers and the
//! command's stdout. The second (`ttyS1` in the guest) is a socket
//! chardev with `server=on,wait=on`: QEMU starts the guest only when podvm
//! has connected, so no early byte of stderr is lost. `/podvm-init` sends
//! the command's stderr there, and podvm copies it to its own stderr as it
//! comes, without the `\r` that the tty puts before each `\n`.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The socket of a run, under the temporary directory: a path under a
/// deep store can pass the 108 bytes of `sun_path`.
pub fn socket_path(nonce: &str) -> PathBuf {
    std::env::temp_dir().join(format!("podvm-guest-{}.err", &nonce[..nonce.len().min(16)]))
}

/// The emulator's arguments for the second port.
pub fn argv(sock: &Path) -> Vec<String> {
    vec![
        "-chardev".to_string(),
        format!(
            "socket,id=podvmerr,path={},server=on,wait=on",
            sock.display()
        ),
        "-serial".to_string(),
        "chardev:podvmerr".to_string(),
    ]
}

/// Bytes of the guest's stderr for the host: each `\r` goes.
pub fn strip_cr(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied().filter(|&b| b != b'\r').collect()
}

/// Connect to `sock` as soon as QEMU listens there, then copy each byte to
/// podvm's stderr until the emulator closes the port. `gone` stops the
/// wait for a connection where the emulator ended before it listened.
pub fn pump(
    sock: PathBuf,
    deadline: Instant,
    gone: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> std::thread::JoinHandle<()> {
    use std::sync::atomic::Ordering;
    std::thread::spawn(move || {
        let mut s = loop {
            match UnixStream::connect(&sock) {
                Ok(s) => break s,
                Err(_) if Instant::now() < deadline && !gone.load(Ordering::Acquire) => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => return,
            }
        };
        let mut buf = [0u8; 4096];
        loop {
            match s.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let mut e = std::io::stderr().lock();
                    let _ = e.write_all(&strip_cr(&buf[..n]));
                    let _ = e.flush();
                }
            }
        }
        let _ = std::fs::remove_file(&sock);
    })
}
