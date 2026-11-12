//! The emulator of one guest run: spawn it, read its console to the exit
//! marker, and end it within a bound (TODO/machine.md T-1656).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use podvm_image::error::EXIT_RUNTIME_ERROR;

use super::{init, stderr, tty, Request, DEFAULT_TIMEOUT};

/// Spawn the emulator and read its console to the exit marker.
pub(super) fn drive(
    r: &Request,
    qemu: &Path,
    argv: &[String],
    nonce: &str,
    dir: &Path,
    sock: &Path,
) -> i32 {
    use std::os::unix::process::CommandExt;
    let errlog = dir.join("qemu.err");
    let mut child = match Command::new(qemu)
        .args(argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(
            std::fs::File::create(&errlog)
                .map(Stdio::from)
                .unwrap_or(Stdio::null()),
        )
        .process_group(0)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("podvm {}: machine tier: {}: {e}", r.verb, qemu.display());
            return EXIT_RUNTIME_ERROR;
        }
    };
    let pid = child.id() as i32;
    let deadline = Instant::now() + r.timeout.unwrap_or(DEFAULT_TIMEOUT);
    let gone = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let errs = stderr::pump(sock.to_path_buf(), deadline, gone.clone());
    // TODO/machine.md T-1723: the terminal of -t, raw on the host until the
    // end of the run.
    let (_raw, term) = if r.tty {
        (
            Some(tty::Raw::new()),
            Some(tty::pump(tty::socket_path(sock), deadline)),
        )
    } else {
        (None, None)
    };
    let out = child.stdout.take().expect("piped");
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in BufReader::new(out).split(b'\n').map_while(Result::ok) {
            if tx
                .send(String::from_utf8_lossy(&line).into_owned())
                .is_err()
            {
                break;
            }
        }
    });
    let mut console = init::Console::new(nonce);
    let mut blanks = 0usize;
    let mut code: Option<i32> = None;
    let stdout = std::io::stdout();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left.max(Duration::from_millis(1))) {
            Ok(line) => match console.line(&line) {
                init::Line::Before | init::Line::Ready => {}
                init::Line::Out("") => blanks += 1,
                init::Line::Out(s) => {
                    let mut o = stdout.lock();
                    for _ in 0..blanks {
                        let _ = writeln!(o);
                    }
                    blanks = 0;
                    let _ = writeln!(o, "{s}");
                }
                init::Line::Exit(c, before) => {
                    // The script's `echo` before the marker ends a last line
                    // without a newline; that one blank line is not output.
                    let mut o = stdout.lock();
                    for _ in 0..blanks.saturating_sub(1) {
                        let _ = writeln!(o);
                    }
                    if !before.is_empty() {
                        let _ = writeln!(o, "{before}");
                    }
                    code = Some(c);
                    break;
                }
            },
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
        }
        if Instant::now() >= deadline {
            break;
        }
    }
    // The guest resets after the marker and the emulator exits; a guest
    // that does not, and each child of the emulator, goes with SIGKILL.
    let ended = wait_bounded(&mut child, Duration::from_secs(10));
    if !ended {
        // SAFETY: a plain system call on the process group that `spawn`
        // made for the emulator.
        unsafe { kill(-pid, 9) };
        let _ = child.wait();
    }
    // The port closed with the emulator: the pump has each byte now.
    gone.store(true, std::sync::atomic::Ordering::Release);
    let _ = errs.join();
    if let Some(t) = term {
        let _ = t.join();
    }
    match code {
        Some(c) => c,
        None => {
            let tail = std::fs::read_to_string(dir.join("qemu.err")).unwrap_or_default();
            let tail = tail.lines().last().unwrap_or("");
            eprintln!(
                "podvm {}: machine tier: the guest gave no exit marker{}{}",
                r.verb,
                if Instant::now() >= deadline {
                    " before the timeout"
                } else {
                    ""
                },
                if tail.is_empty() {
                    String::new()
                } else {
                    format!("; qemu: {tail}")
                }
            );
            EXIT_RUNTIME_ERROR
        }
    }
}

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

fn wait_bounded(child: &mut std::process::Child, d: Duration) -> bool {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if let Ok(Some(_)) = child.try_wait() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}
