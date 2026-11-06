//! A server-side line discipline for interactive SSH with no pty.
//!
//! What this is: the bytes between an SSH session and a shell, with the
//! terminal's half of the work done here instead of in a device that does
//! not exist. The shell runs non-interactive on pipes and reads complete
//! lines; this module echoes input, edits the line, recalls history,
//! delivers signal characters to the shell's process group, and prints a
//! static prompt. Every byte on both legs is deterministic, which is what
//! the tests pin.
//!
//! ## The five asserts this proves (TODO/podssh.md T-1402)
//!
//! Echo, editing, history, state across commands, and a signal sent to the
//! command process group. State across commands is the long-lived shell
//! itself: variables, directory, and functions persist in the process this
//! module supervises.
//!
//! ## Byte rules
//!
//! Client bytes in, four destinations out: back to the client (echo),
//! forward to the shell (submitted lines), a signal to the process group,
//! or the end of input. `\r` and `\n` both submit; the `\n` of a `\r\n`
//! pair is swallowed. Erase-left is DEL and backspace; erase-line is
//! Ctrl-U; erase-word is Ctrl-W; home and end are Ctrl-A and Ctrl-E.
//! Up and Down recall history; Left and Right move the cursor and echo
//! their own ANSI sequences; Home and End (Ctrl-A, Ctrl-E) move the
//! screen's cursor with the line's, so the next key lands where it shows.
//! A redraw is `\r`, the prompt, the buffer, EL
//! (`\x1b[K`), and the cursor restored: ECMA-48 sequences a terminal from
//! the last fifty years answers, and fixed bytes a pipe can assert.
//!
//! The cursor counts bytes, like a terminal without IUTF8: erasing half
//! of a multibyte character sends the other half to the shell. Lines are
//! capped at 64 KiB; past the cap bytes are dropped with a bell. History
//! holds 100 lines in memory, consecutive duplicates skipped, and dies
//! with the session: no file, no cross-session recall.
//!
//! ## Refusal catalogue
//!
//! Refusals answer with a bell and change nothing, never with silence
//! that could read as acceptance. Two entries are not refusals: they
//! pass through to the shell, and are named here so no one mistakes the
//! shell's answer for this module's.
//!
//! - Suspend (Ctrl-Z): no job control without a terminal, and a stopped
//!   shell with no foreground to return to is a wedged session.
//! - Flow control (Ctrl-S, Ctrl-Q): no IXON below, so nothing to stop.
//! - Window size (pass-through): size changes arrive as SSH channel
//!   requests, which `sshd` consumes before any byte reaches this
//!   module, so there is no refusal path here to ring. Programs that
//!   need a size read `$LINES` and `$COLUMNS`; the module sets neither.
//! - Job-control UI (pass-through): `fg`, `bg`, and `jobs` lines reach
//!   the shell, which answers them without monitor mode. The module
//!   neither refuses nor implements them.
//! - Cursor addressing: any escape sequence outside arrows, Home, and End
//!   is dropped.
//! - One-shot commands (including `scp`, which travels as an exec
//!   request): the `shell` binary refuses them naming
//!   `SSH_ORIGINAL_COMMAND`, because a session server serves sessions.
//! - Subsystem requests (such as `sftp`): these never reach the `shell`
//!   binary at all. The generated server configuration defines no
//!   subsystems, so `sshd` itself refuses them (`ssh -s sftp` fails with
//!   "subsystem request failed", exit 255, measured 2026-09-28).
//! - Dynamic prompts: the prompt is the static `$ ` below. The shell's
//!   own `PS1` never reaches the client, because the shell runs
//!   non-interactive on a pipe.
//!
//! ## Signals
//!
//! Signal characters go to the whole process group, never to one pid: the
//! shell's children are the command, and the command is what must die.
//! Ctrl-C sends `SIGINT`, Ctrl-\ sends `SIGQUIT`. Delivery runs through
//! `podvm_probe::sys::kill`, the tree's sanctioned syscall wrapper, with
//! the negative group id. `ESRCH` means the group already died and the
//! death notice is one poll away, so it continues; any other errno fails
//! the session loud. One honest limit travels with this: ignored
//! dispositions are inherited across fork and exec, so a command started
//! under a trap-ignoring shell ignores the signal too, and no inner
//! `trap -` undoes an ignore inherited on entry. Killing the command
//! while the shell survives needs a trap handler: trapped signals reset
//! to the default in children while the shell itself runs the handler.
//! The session tests trap a handler for exactly this.
//!
//! ## Teardown
//!
//! The shell's death ends the session. After input ends the supervisor
//! drops the shell's stdin (the EOF the shell reads), waits for the
//! shell to exit, kills the process group with `SIGKILL` so background
//! children cannot hold the pipes open past the session, drains what
//! the pipes still hold inside a bound, and exits with the shell's own
//! code, or `128+N` where signal `N` ended it. Client EOF submits a
//! partial line first, then ends input the same way Ctrl-D on an empty
//! line does.

use std::io::Read;
use std::time::{Duration, Instant};

use crate::error::Error;
use crate::transport::{write_pipe_bounded, write_stall, Stream, WRITE_DEADLINE};

/// The prompt, printed before every line (static: the shell runs
/// non-interactive and prints none of its own); the lines of history kept
/// per session, in memory; the longest line, in bytes.
pub use pod_term::echo::{HISTORY_CAP, LINE_CAP, PROMPT};
/// How long one supervisor turn waits on the client before polling the
/// shell, and how long the teardown drain lingers for pipe stragglers.
const POLL: Duration = Duration::from_millis(50);
const DRAIN_LINGER: Duration = Duration::from_secs(2);
/// `SIGQUIT` is 3 wherever `SIGINT` is 2; the probe set names only the
/// latter, so it is named here, once.
const SIGQUIT: i64 = 3;

// The discipline is `pod_term::echo` of PodUtils/podcrates: the byte rules
// and the refusal catalogue above are its rules.
pub use pod_term::echo::{Discipline, Event, Sig};

// ------------------------------------------------------------ supervisor

/// What one interactive session needs. The shell is a command line for
/// `/bin/sh -c` semantics handled by the shell itself: it runs with no
/// arguments and reads commands from its stdin.
pub struct SessionConfig {
    pub shell: String,
}

/// Which shell to supervise. `$SHELL` wins when it names something;
/// otherwise `sh`, which every Linux here answers to.
pub fn detect_shell() -> String {
    match std::env::var("SHELL") {
        Ok(s) if !s.trim().is_empty() => s,
        _ => "sh".to_string(),
    }
}

/// One chunk of shell output on its way to the client. Standard output
/// and standard error travel one channel; each stream keeps its own
/// order, and the merge order between the two is the scheduler's, like
/// any two writers to one terminal.
enum ShellOut {
    Bytes(Vec<u8>),
}

/// Run the session: supervise the shell, drive the discipline, and
/// return the shell's exit code (or `128+N` where signal `N` ended it).
/// Standard input and output are the SSH client's; this process is what
/// `sshd` runs under `ForceCommand`.
pub fn run_session(cfg: &SessionConfig, io: &mut dyn Stream) -> Result<i32, Error> {
    use std::os::unix::process::CommandExt as _;

    let mut child = std::process::Command::new(&cfg.shell)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        // ⛔ The shell leads its own process group, so one signal reaches
        // the command and its children together. `process_group` is safe
        // `std`, not a raw `setpgid`: the only raw call in this path is
        // the delivery below, through the tree's sanctioned wrapper.
        .process_group(0)
        .spawn()
        .map_err(|e| Error::server("spawn shell", e))?;
    // The group leader's pid is the group id. Read once: the child is
    // ours, and nobody else in this process touches it.
    let pgid = child.id() as i64;
    let mut stdin = child.stdin.take().expect("piped shell stdin");
    let (tx, rx) = std::sync::mpsc::channel::<ShellOut>();
    // ⛔ One thread per pipe, like the transport drain: a shell whose
    // output exceeds the pipe buffer would otherwise block writing it
    // while this loop waits for the exit that never comes.
    drain_to(child.stdout.take().expect("piped shell stdout"), tx.clone());
    drain_to(child.stderr.take().expect("piped shell stderr"), tx.clone());
    drop(tx);
    io.set_read_timeout(Some(POLL)).map_err(Error::pump)?;
    // ⛔ The client leg carries the write bound where the kernel allows
    // one: a client that stops reading ends the session inside the bound.
    // A pipe-backed client ignores it safely, the way its read already is.
    io.set_write_timeout(Some(WRITE_DEADLINE))
        .map_err(Error::pump)?;
    io.write_all(PROMPT).map_err(Error::pump)?;
    io.flush().map_err(Error::pump)?;
    let mut disc = Discipline::new();
    loop {
        // Client bytes first, so a queued Ctrl-C cannot strand input.
        // A turn that moved nothing sleeps one poll: the client leg has
        // no read timeout (see `transport::Stdio`), so an idle session
        // would otherwise spin on `WouldBlock`.
        let mut busy = false;
        let mut buf = [0u8; 65536];
        match io.read(&mut buf) {
            Ok(0) => {
                // Client EOF submits a partial line, then ends input the
                // same way Ctrl-D on an empty line does.
                if !disc.line().is_empty() {
                    for e in disc.submit_and_end() {
                        apply_to_shell(&e, &mut stdin)?;
                        apply_to_client(&e, io)?;
                    }
                }
                break end_session(child, stdin, pgid, &rx, io);
            }
            Ok(n) => {
                busy = true;
                let mut ended = false;
                for b in &buf[..n] {
                    for e in disc.key(*b) {
                        if matches!(e, Event::Eof) {
                            ended = true;
                            break;
                        }
                        deliver(&e, pgid)?;
                        apply_to_shell(&e, &mut stdin)?;
                        apply_to_client(&e, io)?;
                    }
                    if ended {
                        break;
                    }
                }
                if ended {
                    break end_session(child, stdin, pgid, &rx, io);
                }
            }
            // A key still open after a quiet poll (a lone ESC) ends here.
            Err(e) if crate::pump::is_would_block(&e) => {
                if disc.waiting() {
                    for e in disc.idle() {
                        apply_to_client(&e, io)?;
                    }
                }
            }
            Err(e) => return Err(Error::pump(e)),
        }
        // Shell output, whatever arrived since the last turn. Newlines
        // expand on the way out (see `onlcr`).
        while let Ok(ShellOut::Bytes(b)) = rx.try_recv() {
            busy = true;
            io.write_all(&onlcr(&b)).map_err(Error::pump)?;
        }
        io.flush().map_err(Error::pump)?;
        // Shell death ends the session with the shell's own code.
        if let Some(status) = child
            .try_wait()
            .map_err(|e| Error::server("wait shell", e))?
        {
            break end_session_dead(child, pgid, &rx, io, status);
        }
        if !busy {
            std::thread::sleep(POLL);
        }
    }
}

fn apply_to_shell(e: &Event, stdin: &mut std::process::ChildStdin) -> Result<(), Error> {
    if let Event::ToRemote(b) = e {
        // ⛔ A bounded write, not a blocking one: the shell's stdin is a
        // pipe with no kernel write deadline, so the line waits on a worker
        // instead. Lines are capped at 64 KiB and the shell reads stdin
        // continuously; a shell that stopped reading while alive ends the
        // session loud inside the bound naming this leg.
        write_pipe_bounded(stdin, b, WRITE_DEADLINE, "session shell-stdin leg")
            .map_err(Error::pump)?;
    }
    Ok(())
}

fn apply_to_client(e: &Event, io: &mut dyn Stream) -> Result<(), Error> {
    if let Event::ToLocal(b) = e {
        io.write_all(b).map_err(Error::pump)?;
    }
    Ok(())
}

/// Drain one shell pipe into the channel on its own thread. Generic
/// over the pipe type so standard output and standard error share the
/// one path instead of growing a second copy.
fn drain_to(pipe: impl Read + Send + 'static, tx: std::sync::mpsc::Sender<ShellOut>) {
    std::thread::spawn(move || {
        let mut pipe = pipe;
        let mut buf = [0u8; 65536];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(ShellOut::Bytes(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
}

/// Expand lone newlines for display, the way OPOST with ONLCR does on a
/// terminal: the client may be a real terminal, and a bare `\n` would
/// stair-step down its screen. A `\r\n` the shell printed becomes
/// `\r\r\n`, exactly as a terminal would show it: binary transparency
/// is what raw mode is for, and this session is cooked.
fn onlcr(bytes: &[u8]) -> Vec<u8> {
    if !bytes.contains(&b'\n') {
        return bytes.to_vec();
    }
    let mut out = Vec::with_capacity(bytes.len() + 8);
    for b in bytes {
        if *b == b'\n' {
            out.extend_from_slice(b"\r\n");
        } else {
            out.push(*b);
        }
    }
    out
}

/// Deliver a signal event to the whole process group. `ESRCH` means the
/// group already died and the death notice is one poll away, so it
/// continues; any other errno fails the session loud, naming the call.
fn deliver(e: &Event, pgid: i64) -> Result<(), Error> {
    if let Event::Signal(sig) = e {
        let num = match sig {
            Sig::Int => podvm_probe::sys::SIGINT as i64,
            Sig::Quit => SIGQUIT,
        };
        // ⛔ A negative pid is the group: `kill(-pgid, sig)` reaches the
        // shell and every child it started. That minus sign is the whole
        // of the process-group contract.
        match podvm_probe::sys::kill(-pgid, num) {
            Ok(_) => {}
            Err(e) if e == podvm_probe::sys::ESRCH => {}
            Err(e) => {
                return Err(Error::server(
                    "session signal",
                    std::io::Error::from_raw_os_error(e.0),
                ))
            }
        }
    }
    Ok(())
}

/// The shell's exit code, or `128+N` where signal `N` ended it: the
/// convention shells and `docker wait` share, which the probe's exit
/// module also records.
fn exit_code(status: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt as _;
    status
        .code()
        .unwrap_or_else(|| status.signal().map(|s| 128 + s).unwrap_or(128))
}

/// End the session after input ended: drop the live stdin first so the
/// shell reads EOF, wait for it to exit, kill the group, drain the
/// pipes inside the bound, and report the code. The stdin handle must
/// arrive here alive: the child handle's own copy left at spawn, so
/// only the caller still holds the descriptor the shell reads.
fn end_session(
    mut child: std::process::Child,
    stdin: std::process::ChildStdin,
    pgid: i64,
    rx: &std::sync::mpsc::Receiver<ShellOut>,
    io: &mut dyn Stream,
) -> Result<i32, Error> {
    drop(stdin);
    let status = child.wait().map_err(|e| Error::server("wait shell", e))?;
    kill_group(pgid);
    drain(rx, io)?;
    Ok(exit_code(status))
}

/// End the session after the shell died on its own: drain what the pipes
/// still hold, hang up the group, and report. The child handle is
/// already reaped by the `try_wait` that saw the death; dropping it
/// closes the last held descriptor.
fn end_session_dead(
    child: std::process::Child,
    pgid: i64,
    rx: &std::sync::mpsc::Receiver<ShellOut>,
    io: &mut dyn Stream,
    status: std::process::ExitStatus,
) -> Result<i32, Error> {
    drop(child);
    kill_group(pgid);
    drain(rx, io)?;
    Ok(exit_code(status))
}

/// Drain the shell pipes into the client, bounded on both axes: two
/// seconds total, and out at the first 100 ms of quiet. The reader
/// threads end at EOF, which the group kill brings about; detached threads
/// die with the process, so nothing here joins one. Pipe bytes that
/// arrive after the window ends are dropped silently with the session.
fn drain(rx: &std::sync::mpsc::Receiver<ShellOut>, io: &mut dyn Stream) -> Result<(), Error> {
    let until = Instant::now() + DRAIN_LINGER;
    loop {
        if Instant::now() >= until {
            break;
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ShellOut::Bytes(b)) => io
                .write_all(&onlcr(&b))
                .map_err(|e| Error::pump(write_stall(e, "session client write")))?,
            Err(_) => break,
        }
    }
    io.flush().map_err(Error::pump)
}

/// Kill the process group outright. `SIGHUP` first would emulate a
/// terminal hangup more closely, but a child that ignores it holds the
/// pipes open and the drain never ends; the group kill is the bound.
/// Named for what it kills, not for the hangup it replaces.
fn kill_group(pgid: i64) {
    const SIGKILL: i64 = 9;
    let _ = podvm_probe::sys::kill(-pgid, SIGKILL);
}

#[cfg(test)]
mod tests;
