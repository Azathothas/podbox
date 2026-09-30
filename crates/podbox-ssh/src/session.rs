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
//! their own ANSI sequences, while word and line jumps stay silent until
//! the next redraw. A redraw is `\r`, the prompt, the buffer, EL
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
//! `podbox_probe::sys::kill`, the tree's sanctioned syscall wrapper, with
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

/// The prompt, printed before every line. Static on purpose: the shell
/// runs non-interactive and prints none of its own. See the catalogue.
pub const PROMPT: &[u8] = b"$ ";
/// Lines of history kept per session, in memory. Past this the oldest
/// line leaves; a bound, not a tuning knob (see the catalogue: no file).
pub const HISTORY_CAP: usize = 100;
/// Longest single line, in bytes. Past this bytes drop with a bell: an
/// unbounded line buffer is the defect this cap exists to prevent.
pub const LINE_CAP: usize = 65536;
/// How long one supervisor turn waits on the client before polling the
/// shell, and how long the teardown drain lingers for pipe stragglers.
const POLL: Duration = Duration::from_millis(50);
const DRAIN_LINGER: Duration = Duration::from_secs(2);
/// Erase in Line: clear from the cursor to the end, ECMA-48.
const EL: &[u8] = b"\x1b[K";
const BELL: &[u8] = b"\x07";
/// `SIGQUIT` is 3 wherever `SIGINT` is 2; the probe set names only the
/// latter, so it is named here, once.
const SIGQUIT: i64 = 3;

/// A signal character translated for the supervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sig {
    Int,
    Quit,
}

/// One consequence of one client byte. A byte can echo and submit at
/// once (Enter echoes `\r\n` and emits the line), so the discipline
/// returns a list, never a single.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Bytes for the shell's stdin, including the terminating newline.
    ToShell(Vec<u8>),
    /// Bytes for the client: echo, prompt, redraw, or bell.
    ToClient(Vec<u8>),
    /// A signal for the whole process group.
    Signal(Sig),
    /// Input ended: Ctrl-D on an empty line, or client EOF.
    Eof,
}

/// Escape-sequence parser state. Only `ESC [` sequences are read; anything
/// else after ESC is refused with a bell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Esc {
    #[default]
    None,
    Escape,
    Bracket,
}

/// The line under edit. Pure state: every method below is deterministic
/// in its arguments, which is why the unit suite can pin exact bytes.
#[derive(Debug, Default)]
pub struct Discipline {
    line: Vec<u8>,
    cursor: usize,
    history: Vec<String>,
    /// The uncommitted line set aside while browsing history, restored
    /// when the browse returns past the newest entry.
    saved: Option<Vec<u8>>,
    /// History index under view, or `None` when editing a fresh line.
    hpos: Option<usize>,
    esc: Esc,
    /// A `\r` just submitted: a `\n` arriving next is its pair, not a
    /// second line.
    last_was_cr: bool,
}

impl Discipline {
    pub fn new() -> Self {
        Discipline::default()
    }

    /// Redraw the line: carriage return, prompt, buffer, clear the rest,
    /// and the cursor back where the edit left it.
    fn redraw(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(PROMPT.len() + self.line.len() * 2 + 16);
        out.extend_from_slice(b"\r");
        out.extend_from_slice(PROMPT);
        out.extend_from_slice(&self.line);
        out.extend_from_slice(EL);
        out.extend_from_slice(b"\r");
        out.extend_from_slice(PROMPT);
        out.extend_from_slice(&self.line[..self.cursor]);
        out
    }

    /// The `\b`, space, `\b` triple rubbed out `n` cells. Terminals move
    /// the cursor back, blank the cell, and move back again: three bytes,
    /// no ANSI needed.
    fn rubout(n: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(n * 3);
        for _ in 0..n {
            out.extend_from_slice(b"\x08 \x08");
        }
        out
    }

    /// Store the line in history. Empty lines and repeats of the newest
    /// entry are not history: Up must reach a command, not blank air.
    fn commit(&mut self) {
        if self.line.is_empty() {
            return;
        }
        let line = String::from_utf8_lossy(&self.line).to_string();
        if self.history.last().is_some_and(|h| *h == line) {
            return;
        }
        self.history.push(line);
        if self.history.len() > HISTORY_CAP {
            self.history.remove(0);
        }
    }

    /// Submit the line: history, the shell bytes, and the echo. The echo
    /// carries the next prompt with it: the shell runs non-interactive
    /// and prints none of its own, so a prompt travels with every
    /// submitted line rather than after its output. The order reads
    /// prompt-first on a busy session; the bytes stay deterministic,
    /// which a readiness heuristic could never promise.
    fn submit(&mut self) -> Vec<Event> {
        self.commit();
        let mut shell = std::mem::take(&mut self.line);
        shell.push(b'\n');
        self.cursor = 0;
        self.saved = None;
        self.hpos = None;
        let mut echo = b"\r\n".to_vec();
        echo.extend_from_slice(PROMPT);
        vec![Event::ToClient(echo), Event::ToShell(shell)]
    }

    /// Recall history entry `idx`: replace the buffer, park the cursor
    /// at the end, and redraw.
    fn recall(&mut self, idx: usize) -> Vec<Event> {
        self.line = self.history[idx].clone().into_bytes();
        self.cursor = self.line.len();
        self.hpos = Some(idx);
        vec![Event::ToClient(self.redraw())]
    }

    /// One client byte in, its consequences out.
    pub fn key(&mut self, b: u8) -> Vec<Event> {
        // An escape in progress owns the byte before anything else does.
        match self.esc {
            Esc::Escape => {
                self.esc = if b == b'[' { Esc::Bracket } else { Esc::None };
                if b != b'[' {
                    return vec![Event::ToClient(BELL.to_vec())];
                }
                return vec![];
            }
            Esc::Bracket => {
                self.esc = Esc::None;
                return self.escape(b);
            }
            Esc::None => {}
        }
        // The second half of a `\r\n` pair is the pair, not a line.
        if self.last_was_cr {
            self.last_was_cr = false;
            if b == b'\n' {
                return vec![];
            }
        }
        match b {
            0x03 => self.signal_key(Sig::Int, b"^C\r\n"),
            0x1c => self.signal_key(Sig::Quit, b"^\\\r\n"),
            // Refused, loudly: the catalogue names each one.
            0x1a | 0x11 | 0x13 => vec![Event::ToClient(BELL.to_vec())],
            0x04 => self.eof_or_delete(),
            0x0d => {
                self.last_was_cr = true;
                self.submit()
            }
            0x0a => self.submit(),
            0x7f | 0x08 => self.erase_left(),
            0x15 => self.erase_line(),
            0x17 => self.erase_word(),
            0x01 => {
                self.cursor = 0;
                vec![]
            }
            0x05 => {
                self.cursor = self.line.len();
                vec![]
            }
            0x1b => {
                self.esc = Esc::Escape;
                vec![]
            }
            _ => self.insert(b),
        }
    }

    /// A signal character: drop the line being edited, echo the caret
    /// notation a terminal shows, print a fresh prompt, and report the
    /// signal. The interrupted line never ran, so history never saw it.
    fn signal_key(&mut self, sig: Sig, caret: &[u8]) -> Vec<Event> {
        self.line.clear();
        self.cursor = 0;
        self.saved = None;
        self.hpos = None;
        let mut echo = caret.to_vec();
        echo.extend_from_slice(PROMPT);
        vec![Event::ToClient(echo), Event::Signal(sig)]
    }

    /// Ctrl-D: end of input on an empty line, delete under the cursor on
    /// a live one, and nothing at all at the very end.
    fn eof_or_delete(&mut self) -> Vec<Event> {
        if self.line.is_empty() {
            return vec![Event::Eof];
        }
        if self.cursor < self.line.len() {
            self.line.remove(self.cursor);
            return vec![Event::ToClient(self.redraw())];
        }
        vec![]
    }

    /// Erase one cell left of the cursor. At the very start there is
    /// nothing to erase, and the bell says so.
    fn erase_left(&mut self) -> Vec<Event> {
        if self.cursor == 0 {
            return vec![Event::ToClient(BELL.to_vec())];
        }
        self.cursor -= 1;
        self.line.remove(self.cursor);
        // At the end the triple suffices; mid-line the tail shifted and
        // only a redraw puts every cell right.
        if self.cursor == self.line.len() {
            vec![Event::ToClient(Self::rubout(1))]
        } else {
            vec![Event::ToClient(self.redraw())]
        }
    }

    /// Erase the whole line, wherever the cursor sits: carriage return,
    /// prompt, and clear, which is one fixed shape for every length.
    fn erase_line(&mut self) -> Vec<Event> {
        if self.line.is_empty() {
            return vec![Event::ToClient(BELL.to_vec())];
        }
        self.line.clear();
        self.cursor = 0;
        let mut out = b"\r".to_vec();
        out.extend_from_slice(PROMPT);
        out.extend_from_slice(EL);
        vec![Event::ToClient(out)]
    }

    /// Erase back over one word: trailing blanks, then non-blanks. The
    /// boundary rule is spaces, matching what a shell word means here.
    fn erase_word(&mut self) -> Vec<Event> {
        if self.cursor == 0 {
            return vec![Event::ToClient(BELL.to_vec())];
        }
        let mut n = 0;
        while self.cursor > 0 && self.line[self.cursor - 1] == b' ' {
            self.cursor -= 1;
            self.line.remove(self.cursor);
            n += 1;
        }
        while self.cursor > 0 && self.line[self.cursor - 1] != b' ' {
            self.cursor -= 1;
            self.line.remove(self.cursor);
            n += 1;
        }
        if self.cursor == self.line.len() {
            vec![Event::ToClient(Self::rubout(n))]
        } else {
            vec![Event::ToClient(self.redraw())]
        }
    }

    /// The final byte of an `ESC [` sequence. Arrows echo their own
    /// sequences, which move a real terminal's cursor exactly where the
    /// local cursor went; anything else is refused with a bell.
    fn escape(&mut self, b: u8) -> Vec<Event> {
        match b {
            b'A' => self.history_up(),
            b'B' => self.history_down(),
            b'C' if self.cursor < self.line.len() => {
                self.cursor += 1;
                vec![Event::ToClient(b"\x1b[C".to_vec())]
            }
            b'D' if self.cursor > 0 => {
                self.cursor -= 1;
                vec![Event::ToClient(b"\x1b[D".to_vec())]
            }
            b'H' => {
                self.cursor = 0;
                vec![]
            }
            b'F' => {
                self.cursor = self.line.len();
                vec![]
            }
            _ => vec![Event::ToClient(BELL.to_vec())],
        }
    }

    /// History up: set the uncommitted line aside on first browse, then
    /// walk older. Both ends answer with a bell.
    fn history_up(&mut self) -> Vec<Event> {
        if self.history.is_empty() {
            return vec![Event::ToClient(BELL.to_vec())];
        }
        match self.hpos {
            None => {
                self.saved = Some(std::mem::take(&mut self.line));
                self.cursor = 0;
                self.recall(self.history.len() - 1)
            }
            Some(0) => vec![Event::ToClient(BELL.to_vec())],
            Some(i) => self.recall(i - 1),
        }
    }

    /// History down: walk newer, and past the newest restore the line
    /// the browse set aside.
    fn history_down(&mut self) -> Vec<Event> {
        match self.hpos {
            None => vec![Event::ToClient(BELL.to_vec())],
            Some(i) if i + 1 < self.history.len() => self.recall(i + 1),
            Some(_) => {
                self.line = self.saved.take().unwrap_or_default();
                self.cursor = self.line.len();
                self.hpos = None;
                vec![Event::ToClient(self.redraw())]
            }
        }
    }

    /// An ordinary byte: append at the end with a one-byte echo, or
    /// insert mid-line with a redraw. Past the cap the byte drops with
    /// a bell, and the bell is the whole answer.
    fn insert(&mut self, b: u8) -> Vec<Event> {
        if self.line.len() >= LINE_CAP {
            return vec![Event::ToClient(BELL.to_vec())];
        }
        if self.cursor == self.line.len() {
            self.line.push(b);
            self.cursor += 1;
            vec![Event::ToClient(vec![b])]
        } else {
            self.line.insert(self.cursor, b);
            self.cursor += 1;
            vec![Event::ToClient(self.redraw())]
        }
    }
}

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
                if !disc.line.is_empty() {
                    for e in disc.submit() {
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
            Err(e) if crate::pump::is_would_block(&e) => {}
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
    if let Event::ToShell(b) = e {
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
    if let Event::ToClient(b) = e {
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
            Sig::Int => podbox_probe::sys::SIGINT as i64,
            Sig::Quit => SIGQUIT,
        };
        // ⛔ A negative pid is the group: `kill(-pgid, sig)` reaches the
        // shell and every child it started. That minus sign is the whole
        // of the process-group contract.
        match podbox_probe::sys::kill(-pgid, num) {
            Ok(_) => {}
            Err(e) if e == podbox_probe::sys::ESRCH => {}
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
    let _ = podbox_probe::sys::kill(-pgid, SIGKILL);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feed bytes, collect consequences: client bytes, shell bytes,
    /// signals, and whether input ended.
    fn feed(d: &mut Discipline, bytes: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<Sig>, bool) {
        let mut client = Vec::new();
        let mut shell = Vec::new();
        let mut sigs = Vec::new();
        let mut eof = false;
        for b in bytes {
            for e in d.key(*b) {
                match e {
                    Event::ToClient(c) => client.extend_from_slice(&c),
                    Event::ToShell(s) => shell.extend_from_slice(&s),
                    Event::Signal(s) => sigs.push(s),
                    Event::Eof => eof = true,
                }
            }
        }
        (client, shell, sigs, eof)
    }

    /// True when `needle` occurs in `hay`. `Vec::contains` takes an
    /// element, not a slice, so subsequence checks go through here.
    fn has(hay: &[u8], needle: &[u8]) -> bool {
        !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
    }

    fn typed(d: &mut Discipline, s: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<Sig>, bool) {
        feed(d, s)
    }

    #[test]
    fn typing_echoes_and_enter_submits() {
        let mut d = Discipline::new();
        let (client, shell, sigs, eof) = typed(&mut d, b"hi\n");
        assert_eq!(client, b"hi\r\n$ ");
        assert_eq!(shell, b"hi\n");
        assert!(sigs.is_empty() && !eof);
    }

    #[test]
    fn crlf_submits_one_line() {
        // ⛔ The pair is one line: a client speaking CRLF must not run
        // every command twice.
        let mut d = Discipline::new();
        let (client, shell, _, _) = typed(&mut d, b"a\r\n");
        assert_eq!(client, b"a\r\n$ ");
        assert_eq!(shell, b"a\n");
    }

    #[test]
    fn lone_cr_submits_and_lone_lf_too() {
        // ⛔ Each alone, in a fresh discipline: back to back they are a
        // `\r\n` pair, which the pair test above already covers.
        let mut d = Discipline::new();
        let (_, shell, _, _) = typed(&mut d, b"\r");
        assert_eq!(shell, b"\n");
        let mut d = Discipline::new();
        let (_, shell, _, _) = typed(&mut d, b"\n");
        assert_eq!(shell, b"\n");
    }

    #[test]
    fn backspace_erases_one_cell() {
        let mut d = Discipline::new();
        let (client, shell, _, _) = typed(&mut d, b"ab\x7f\n");
        assert_eq!(client, b"ab\x08 \x08\r\n$ ");
        assert_eq!(shell, b"a\n");
    }

    #[test]
    fn backspace_at_start_bells() {
        let mut d = Discipline::new();
        let (client, shell, _, _) = typed(&mut d, b"\x7f");
        assert_eq!(client, BELL);
        assert!(shell.is_empty());
    }

    #[test]
    fn ctrl_u_erases_the_line() {
        let mut d = Discipline::new();
        let (client, shell, _, _) = typed(&mut d, b"hello\x15");
        let mut expect = b"\r".to_vec();
        expect.extend_from_slice(PROMPT);
        expect.extend_from_slice(EL);
        assert_eq!(client, [b"hello".to_vec(), expect].concat());
        assert!(shell.is_empty());
        // The line is gone: Enter submits an empty line.
        let (_, shell, _, _) = typed(&mut d, b"\n");
        assert_eq!(shell, b"\n");
    }

    #[test]
    fn ctrl_w_erases_one_word() {
        let mut d = Discipline::new();
        let (_, shell, _, _) = typed(&mut d, b"foo bar\x17\n");
        assert_eq!(shell, b"foo \n");
    }

    #[test]
    fn ctrl_c_signals_and_discards() {
        let mut d = Discipline::new();
        let (client, shell, sigs, _) = typed(&mut d, b"abc\x03");
        assert_eq!(sigs, vec![Sig::Int]);
        assert_eq!(client, b"abc^C\r\n$ ");
        assert!(shell.is_empty());
        // The interrupted line never ran, so history never saw it.
        let (client, _, _, _) = typed(&mut d, b"\x1b[A");
        assert_eq!(client, BELL);
    }

    #[test]
    fn ctrl_backslash_signals_quit() {
        let mut d = Discipline::new();
        let (client, _, sigs, _) = typed(&mut d, b"\x1c");
        assert_eq!(sigs, vec![Sig::Quit]);
        assert_eq!(client, b"^\\\r\n$ ");
    }

    #[test]
    fn suspend_and_flow_control_bell_and_change_nothing() {
        // The catalogue's refused half, pinned as refused: a bell, no
        // signal, and the line intact underneath.
        let mut d = Discipline::new();
        typed(&mut d, b"ab");
        for b in [0x1au8, 0x11, 0x13] {
            let (client, shell, sigs, _) = typed(&mut d, &[b]);
            assert_eq!(client, BELL, "byte {b:#x}");
            assert!(shell.is_empty() && sigs.is_empty(), "byte {b:#x}");
        }
        let (_, shell, _, _) = typed(&mut d, b"\n");
        assert_eq!(shell, b"ab\n");
    }

    #[test]
    fn ctrl_d_on_empty_is_eof_inside_is_delete() {
        let mut d = Discipline::new();
        let (_, _, _, eof) = typed(&mut d, b"\x04");
        assert!(eof);
        // Mid-line it deletes under the cursor with a redraw instead.
        let mut d = Discipline::new();
        typed(&mut d, b"ab\x1b[D");
        let (client, shell, _, eof) = typed(&mut d, b"\x04\n");
        assert!(!eof);
        assert_eq!(shell, b"a\n");
        assert!(has(&client, b"\x1b[K"));
    }

    #[test]
    fn history_browses_and_reruns() {
        let mut d = Discipline::new();
        let (_, shell, _, _) = typed(&mut d, b"one\ntwo\n");
        assert_eq!(shell, b"one\ntwo\n");
        // Up recalls newest first.
        let (client, _, _, _) = typed(&mut d, b"\x1b[A");
        assert!(has(&client, b"two"));
        let (client, _, _, _) = typed(&mut d, b"\x1b[A");
        assert!(has(&client, b"one"));
        // Past the oldest is a bell, and the line stays.
        let (client, _, _, _) = typed(&mut d, b"\x1b[A");
        assert_eq!(client, BELL);
        // Down walks back to the newest; Enter runs it again.
        typed(&mut d, b"\x1b[B");
        let (_, shell, _, _) = typed(&mut d, b"\n");
        assert_eq!(shell, b"two\n");
    }

    #[test]
    fn history_skips_empty_and_repeats() {
        let mut d = Discipline::new();
        typed(&mut d, b"\nx\nx\n");
        let (client, _, _, _) = typed(&mut d, b"\x1b[A");
        assert!(client.ends_with(b"x"));
        // One entry, not three: the repeat and the blank never stored.
        let (client, _, _, _) = typed(&mut d, b"\x1b[A");
        assert_eq!(client, BELL);
    }

    #[test]
    fn arrows_move_and_insert_midline() {
        let mut d = Discipline::new();
        let (client, _, _, _) = typed(&mut d, b"ac\x1b[D");
        assert!(has(&client, b"\x1b[D"));
        let (client, shell, _, _) = typed(&mut d, b"b\n");
        assert!(has(&client, b"\x1b[K"));
        assert_eq!(shell, b"abc\n");
    }

    #[test]
    fn home_and_end_are_silent_moves() {
        // The jump itself echoes nothing; the insert after it redraws,
        // which is where the moved cursor shows.
        let mut d = Discipline::new();
        let (client, _, _, _) = typed(&mut d, b"ab\x01");
        assert_eq!(client, b"ab");
        let (client, shell, _, _) = typed(&mut d, b"X\n");
        assert!(has(&client, b"\x1b[K"));
        assert_eq!(shell, b"Xab\n");
    }

    #[test]
    fn unknown_escape_bells_and_drops() {
        let mut d = Discipline::new();
        let (client, shell, _, _) = typed(&mut d, b"\x1bxa");
        assert_eq!(client, [BELL.to_vec(), b"a".to_vec()].concat());
        assert!(shell.is_empty());
    }

    #[test]
    fn line_cap_drops_with_bell() {
        let mut d = Discipline::new();
        let big = vec![b'y'; LINE_CAP + 5];
        let (client, _, _, _) = typed(&mut d, &big);
        // Every accepted byte echoes itself; every byte past the cap is
        // one bell and nothing else.
        assert_eq!(client.len(), LINE_CAP + 5);
        assert_eq!(client.iter().filter(|b| **b == b'\x07').count(), 5);
        let (_, shell, _, _) = typed(&mut d, b"\n");
        assert_eq!(shell.len(), LINE_CAP + 1);
    }

    #[test]
    fn midline_erase_redraws() {
        // "abc", two lefts, erase: removes `a`, leaves "bc".
        let mut d = Discipline::new();
        typed(&mut d, b"abc\x1b[D\x1b[D");
        let (client, shell, _, _) = typed(&mut d, b"\x7f\n");
        assert!(has(&client, b"\x1b[K"));
        assert_eq!(shell, b"bc\n");
    }

    #[test]
    fn right_arrow_moves_and_retypes() {
        // "ac", left twice, right once, "b": the insert lands between,
        // "abc".
        let mut d = Discipline::new();
        let (client, _, _, _) = typed(&mut d, b"ac\x1b[D\x1b[D\x1b[C");
        assert!(has(&client, b"\x1b[C"));
        let (client, shell, _, _) = typed(&mut d, b"b\n");
        assert!(has(&client, b"\x1b[K"));
        assert_eq!(shell, b"abc\n");
    }

    #[test]
    fn esc_home_end_jump_silently() {
        // `ESC [ H` and `ESC [ F` move like Ctrl-A and Ctrl-E: the jump
        // echoes nothing, the insert after it redraws.
        let mut d = Discipline::new();
        let (client, _, _, _) = typed(&mut d, b"ab\x1b[H");
        assert_eq!(client, b"ab");
        let (client, shell, _, _) = typed(&mut d, b"X\n");
        assert!(has(&client, b"\x1b[K"));
        assert_eq!(shell, b"Xab\n");
        let mut d = Discipline::new();
        let (client, _, _, _) = typed(&mut d, b"ab\x1b[H\x1b[F");
        assert_eq!(client, b"ab");
        let (_, shell, _, _) = typed(&mut d, b"X\n");
        assert_eq!(shell, b"abX\n");
    }

    #[test]
    fn unknown_bracket_sequence_bells() {
        // `ESC [ Z` is refused: one bell, the line intact underneath.
        let mut d = Discipline::new();
        typed(&mut d, b"ab");
        let (client, shell, _, _) = typed(&mut d, b"\x1b[Z");
        assert_eq!(client, BELL);
        assert!(shell.is_empty());
        let (_, shell, _, _) = typed(&mut d, b"\n");
        assert_eq!(shell, b"ab\n");
    }

    #[test]
    fn history_down_from_fresh_bells() {
        // Down with no browse in progress refuses; the line stays.
        let mut d = Discipline::new();
        typed(&mut d, b"one\n");
        typed(&mut d, b"tw");
        let (client, shell, _, _) = typed(&mut d, b"\x1b[B");
        assert_eq!(client, BELL);
        assert!(shell.is_empty());
        let (_, shell, _, _) = typed(&mut d, b"\n");
        assert_eq!(shell, b"tw\n");
    }

    #[test]
    fn ctrl_d_at_end_of_line_is_silent() {
        // Past the last cell there is nothing to delete: no bell, no
        // redraw, no shell bytes.
        let mut d = Discipline::new();
        typed(&mut d, b"ab");
        let (client, shell, _, eof) = typed(&mut d, b"\x04");
        assert!(!eof);
        assert!(client.is_empty());
        assert!(shell.is_empty());
    }

    #[test]
    fn empty_erases_bell() {
        // Ctrl-U and Ctrl-W on an empty line refuse with a bell each.
        let mut d = Discipline::new();
        for b in [0x15u8, 0x17] {
            let (client, shell, _, _) = typed(&mut d, &[b]);
            assert_eq!(client, BELL, "byte {b:#x}");
            assert!(shell.is_empty(), "byte {b:#x}");
        }
    }

    #[test]
    fn midline_ctrl_w_erases_word_and_redraws() {
        // "foo bar", two lefts, Ctrl-W: erases back over the word
        // fragment before the cursor ("b"), leaves "foo ar", and only a
        // redraw puts the shifted tail right.
        let mut d = Discipline::new();
        typed(&mut d, b"foo bar\x1b[D\x1b[D");
        let (client, shell, _, _) = typed(&mut d, b"\x17\n");
        assert!(has(&client, b"\x1b[K"));
        assert_eq!(shell, b"foo ar\n");
    }

    #[test]
    fn history_evicts_past_the_cap() {
        // Past `HISTORY_CAP` lines the oldest leaves: Up from a full
        // history reaches back exactly the cap, then bells.
        let mut d = Discipline::new();
        for i in 0..HISTORY_CAP + 5 {
            let line = format!("cmd{i}\n");
            typed(&mut d, line.as_bytes());
        }
        assert_eq!(d.history.len(), HISTORY_CAP);
        assert_eq!(d.history[0], format!("cmd{}", 5));
        for _ in 0..HISTORY_CAP {
            let (client, _, _, _) = typed(&mut d, b"\x1b[A");
            assert_ne!(client, BELL);
        }
        let (client, _, _, _) = typed(&mut d, b"\x1b[A");
        assert_eq!(client, BELL);
    }

    #[test]
    fn history_down_past_newest_restores_the_parked_line() {
        // Type a partial line, browse up, browse back past the newest:
        // the uncommitted line set aside on first browse returns, and
        // Enter runs it instead of the recalled entry.
        let mut d = Discipline::new();
        typed(&mut d, b"one\n");
        typed(&mut d, b"tw");
        let (client, _, _, _) = typed(&mut d, b"\x1b[A");
        assert!(has(&client, b"one"));
        let (client, shell, _, _) = typed(&mut d, b"\x1b[B");
        assert!(has(&client, b"tw"));
        assert!(shell.is_empty());
        let (_, shell, _, _) = typed(&mut d, b"\n");
        assert_eq!(shell, b"tw\n");
    }

    #[test]
    fn arrows_at_either_bound_bell() {
        // Left at column zero and Right at end of line refuse with a
        // bell each; the line underneath is untouched.
        let mut d = Discipline::new();
        let (client, shell, _, _) = typed(&mut d, b"\x1b[D");
        assert_eq!(client, BELL);
        assert!(shell.is_empty());
        let (client, shell, _, _) = typed(&mut d, b"a\x1b[C");
        assert_eq!(client, [b"a".to_vec(), BELL.to_vec()].concat());
        assert!(shell.is_empty());
        let (_, shell, _, _) = typed(&mut d, b"\n");
        assert_eq!(shell, b"a\n");
    }

    #[test]
    fn ctrl_w_eats_blanks_then_the_word() {
        // Trailing blanks go first, then the word itself: the whole
        // five cells rub out and the submit that follows is empty.
        let mut d = Discipline::new();
        typed(&mut d, b"foo  ");
        let (client, shell, _, _) = typed(&mut d, b"\x17");
        assert_eq!(client, Discipline::rubout(5));
        assert!(shell.is_empty());
        let (_, shell, _, _) = typed(&mut d, b"\n");
        assert_eq!(shell, b"\n");
    }
}
