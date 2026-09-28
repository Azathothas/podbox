//! An interactive session with no pty, through a real SSH client.
//!
//! Each test starts a real `sshd -i` whose `ForceCommand` is the real
//! `shell` binary, then drives it with a real `ssh` whose standard
//! descriptors are pipes. The five asserts of TODO/podssh.md T-1402 run
//! here: echo, editing, history, state across commands, and a signal to
//! the command process group. A sixth pins the premise itself: standard
//! descriptors inside the session are not a terminal.
//!
//! ⛔ Every wait here is bounded. Reads poll with deadlines, children are
//! reaped by a drop guard, and pipes drain on threads so a full buffer
//! in either direction cannot wedge the runner.
//!
//! ⛔ Needles must be shell-output-only. The discipline echoes every typed
//! byte and every submit carries `\r\n$ `, so a needle present in the
//! input echo proves nothing: `echo alive-after-signal` typed at the
//! prompt is followed by the submit echo, which already contains
//! `alive-after-signal\r\n` before the shell runs anything. Every output
//! needle below is therefore a computed marker (`out=$(...)`) whose
//! expanded form never occurs in the typed bytes.

mod common;

use std::io::Write;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{current_user, ensure_privsep, fresh_tmp, make_keys, require_binary, run_ssh};

const ACCEPT_DEADLINE: Duration = Duration::from_secs(20);
const IO_DEADLINE: Duration = Duration::from_secs(20);

struct Fixture {
    tmp: PathBuf,
    config: PathBuf,
    user: String,
    user_key: PathBuf,
    force_command: String,
}

fn setup() -> Fixture {
    require_binary("sshd");
    require_binary("ssh");
    let keygen = require_binary("ssh-keygen");
    let tmp = fresh_tmp("podbox-ssh-shell");
    ensure_privsep();
    let keys = make_keys(&keygen, &tmp);
    let config = podbox_ssh::write_sshd_config(&tmp, &keys.host, &keys.authorized).unwrap();
    let shell = env!("CARGO_BIN_EXE_shell");
    Fixture {
        tmp,
        config,
        user: current_user(),
        user_key: keys.user,
        // ⛔ The session server is chosen per connection, not per user:
        // no account changes, no config rewrite, one flag on the daemon.
        force_command: format!("{shell} --shell sh"),
    }
}

/// One `sshd -i` on the accepted end of `listener`, running the session
/// server. Returns the server thread and the sender that stops it.
fn start_server(
    listener: UnixListener,
    config: PathBuf,
    force_command: String,
) -> (std::thread::JoinHandle<()>, std::sync::mpsc::Sender<()>) {
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let handle = std::thread::spawn(move || {
        let deadline = Instant::now() + ACCEPT_DEADLINE;
        let accepted = loop {
            match listener.accept() {
                Ok((s, _)) => break Some(s),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        break None;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => panic!("accept failed: {e}"),
            }
        };
        let accepted = accepted.expect("no client connected inside the accept deadline");
        let fd: OwnedFd = accepted.into();
        let stdin_fd: OwnedFd = fd.try_clone().unwrap();
        let mut child = Command::new("sshd")
            .args(["-i", "-e", "-f"])
            .arg(&config)
            .args(["-o", &format!("ForceCommand={force_command}")])
            .stdin(Stdio::from(stdin_fd))
            .stdout(Stdio::from(fd))
            .spawn()
            .unwrap();
        let _ = done_rx.recv_timeout(IO_DEADLINE);
        let _ = child.kill();
        let _ = child.wait();
    });
    (handle, done_tx)
}

/// A child that is killed and reaped on drop, so a failed assertion
/// cannot leave an ssh client behind holding the session.
struct Proc {
    child: Option<Child>,
}

impl Proc {
    fn wait_deadline(mut self, what: &str) -> std::process::ExitStatus {
        let start = Instant::now();
        loop {
            match self.child.as_mut().unwrap().try_wait().unwrap() {
                Some(st) => {
                    self.child.take();
                    return st;
                }
                None => {
                    if start.elapsed() >= IO_DEADLINE {
                        panic!("{what} outlived its deadline");
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// An interactive client: stdin to type into, stdout to expect from.
/// Output drains on a thread into shared bytes; `expect` polls those
/// bytes with a deadline instead of blocking. It searches past the
/// previous match, so a prompt matched once cannot satisfy the next
/// wait: every expectation proves NEW output arrived.
struct Term {
    proc_: Proc,
    stdin: Option<ChildStdin>,
    out: Arc<Mutex<Vec<u8>>>,
    pos: usize,
}

impl Term {
    fn send(&mut self, bytes: &[u8]) {
        let stdin = self.stdin.as_mut().expect("stdin closed");
        stdin.write_all(bytes).unwrap();
        stdin.flush().unwrap();
    }

    fn expect(&mut self, what: &str, needle: &[u8]) {
        let start = Instant::now();
        loop {
            let out = self.out.lock().unwrap();
            if let Some(at) = out
                .get(self.pos..)
                .and_then(|fresh| fresh.windows(needle.len().max(1)).position(|w| w == needle))
            {
                self.pos += at + needle.len().max(1);
                return;
            }
            if start.elapsed() >= IO_DEADLINE {
                panic!(
                    "{what}: never saw {needle:?}; saw fresh: {:?}",
                    String::from_utf8_lossy(out.get(self.pos..).unwrap_or_default())
                );
            }
            drop(out);
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

fn spawn_ssh(
    fx: &Fixture,
) -> (
    Term,
    std::thread::JoinHandle<()>,
    std::sync::mpsc::Sender<()>,
    PathBuf,
) {
    let listener = UnixListener::bind(fx.sock_path()).unwrap();
    listener.set_nonblocking(true).unwrap();
    let (server, done) = start_server(listener, fx.config.clone(), fx.force_command.clone());
    let proxy = env!("CARGO_BIN_EXE_proxy");
    let proxy_cmd = format!("'{proxy}' unix '{}'", fx.sock_path().display());
    let mut cmd = Command::new("ssh");
    cmd.args(["-o", &format!("ProxyCommand={proxy_cmd}")])
        .args(["-o", "BatchMode=yes"])
        .args(["-o", "StrictHostKeyChecking=no"])
        .args(["-o", "UserKnownHostsFile=/dev/null"])
        .args(["-o", "ConnectTimeout=10"])
        .args(["-o", "LogLevel=ERROR"])
        .args(["-i"])
        .arg(&fx.user_key)
        .args(["-l", &fx.user, "localhost"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let stdin = child.stdin.take().unwrap();
    let out = Arc::new(Mutex::new(Vec::new()));
    let out_bg = out.clone();
    let mut stdout = child.stdout.take().unwrap();
    std::thread::spawn(move || {
        let mut buf = [0u8; 65536];
        use std::io::Read as _;
        loop {
            match stdout.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => out_bg.lock().unwrap().extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
    });
    // Server stderr drains nowhere: the client asked for ERROR only, and
    // the session server is silent on success. Reaped with the child.
    let term = Term {
        proc_: Proc { child: Some(child) },
        stdin: Some(stdin),
        out,
        pos: 0,
    };
    (term, server, done, fx.tmp.clone())
}

impl Fixture {
    fn sock_path(&self) -> PathBuf {
        self.tmp.join("ssh.sock")
    }
}

fn finish(
    mut term: Term,
    server: std::thread::JoinHandle<()>,
    done: std::sync::mpsc::Sender<()>,
    tmp: PathBuf,
) {
    drop(term.stdin.take());
    drop(term);
    let _ = done.send(());
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn prompt_echo_command_and_no_terminal() {
    // ⛔ The premise first: descriptors 0 and 1 inside the session are
    // pipes, not a terminal. Everything after this line depends on it.
    let fx = setup();
    let (mut term, server, done, tmp) = spawn_ssh(&fx);
    term.expect("prompt", b"$ ");
    term.send(b"test -t 0; echo stdin_tty=$?\n");
    term.expect("stdin check", b"$ stdin_tty=1\r\n");
    term.send(b"test -t 1; echo stdout_tty=$?\n");
    term.expect("stdout check", b"$ stdout_tty=1\r\n");
    term.send(b"echo hello-session\n");
    term.expect("echo", b"echo hello-session");
    // The shell ran: only expansion produces `out=hello-session`. The
    // bare word already occurs in the input echo above, so it proves
    // nothing on its own.
    term.send(b"echo out=$(echo hello-session)\n");
    term.expect("output", b"out=hello-session\r\n");
    finish(term, server, done, tmp);
}

#[test]
fn editing_repairs_a_typo() {
    // Type "hello" with a tripled letter, erase three times, retype:
    // the shell runs "help", proving the discipline edited, not echoed.
    let fx = setup();
    let (mut term, server, done, tmp) = spawn_ssh(&fx);
    term.expect("prompt", b"$ ");
    term.send(b"echo helllo\x7f\x7f\x7fp\n");
    term.expect("output", b"help\r\n");
    finish(term, server, done, tmp);
}

#[test]
fn history_recalls_the_previous_command() {
    let fx = setup();
    let (mut term, server, done, tmp) = spawn_ssh(&fx);
    term.expect("prompt", b"$ ");
    // A computed marker: the expanded `recall-2` never occurs in the
    // typed bytes, so each sighting is one shell run. Up brings the line
    // back; Enter runs it again, for two runs in all.
    term.send(b"echo recall-$((1+1))\n");
    term.expect("first", b"recall-2\r\n");
    term.send(b"\x1b[A\n");
    term.expect("recalled echo", b"echo recall-$((1+1))");
    term.expect("recalled output", b"recall-2\r\n");
    finish(term, server, done, tmp);
}

#[test]
fn state_persists_across_commands() {
    // Variables and directory live in the supervised shell process, so
    // they survive from one submitted line to the next.
    let fx = setup();
    let (mut term, server, done, tmp) = spawn_ssh(&fx);
    term.expect("prompt", b"$ ");
    term.send(b"podbox_mark=42\n");
    term.expect("prompt again", b"$ ");
    term.send(b"echo got-$podbox_mark\n");
    term.expect("variable", b"got-42\r\n");
    term.send(b"cd /tmp\n");
    term.expect("prompt again", b"$ ");
    // Computed: the typed bytes hold `$(pwd)`, never `/tmp` after `=`.
    term.send(b"echo dir=$(pwd)\n");
    term.expect("directory", b"dir=/tmp\r\n");
    finish(term, server, done, tmp);
}

#[test]
fn signal_kills_the_command_not_the_shell() {
    // The outer shell traps INT with a handler, so it survives; the
    // foreground sleep is reset to the default in the child, so the
    // group SIGINT kills the command and nothing else. The handler (not
    // an ignore) is load-bearing: ignored dispositions are inherited
    // across fork and exec, so under `trap '' INT` the sleep would
    // ignore the signal too and run its full course, and POSIX lets no
    // inner `trap - INT` undo an ignore inherited on entry. Trapped
    // signals reset to default in children; that reset is the whole of
    // this proof. The alive marker is computed, so only the shell
    // running the line produces it; the 20 s read deadline against the
    // 60 s sleep is the bound that convicts a surviving sleep.
    let fx = setup();
    let (mut term, server, done, tmp) = spawn_ssh(&fx);
    term.expect("prompt", b"$ ");
    term.send(b"trap 'echo outer-trapped' INT\n");
    term.expect("prompt again", b"$ ");
    term.send(b"sleep 60\n");
    std::thread::sleep(Duration::from_secs(5));
    term.send(b"\x03");
    term.expect("SIGINT caret and fresh prompt", b"^C\r\n$ ");
    term.expect("outer handler ran", b"outer-trapped\r\n");
    term.send(b"echo alive-$((40+2))\n");
    term.expect("shell alive after SIGINT", b"alive-42\r\n");
    finish(term, server, done, tmp);
}

#[test]
fn signal_without_trap_ends_the_session_with_130() {
    // No trap anywhere: the group SIGINT kills the shell and its
    // foreground sleep together, and the session reports 128+2. Delivery
    // to the group is what this proves; selectivity is the test above.
    let fx = setup();
    let (mut term, server, done, tmp) = spawn_ssh(&fx);
    term.expect("prompt", b"$ ");
    term.send(b"sleep 60\n");
    std::thread::sleep(Duration::from_secs(5));
    term.send(b"\x03");
    let stdin = term.stdin.take();
    drop(stdin);
    // `wait_deadline` waits 20 s: a surviving 60 s sleep would outlive
    // it and panic there, so reaching the code assert proves the group
    // died by the signal.
    let st = term.proc_.wait_deadline("ssh");
    assert_eq!(st.code(), Some(130));
    let _ = done.send(());
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn shell_exit_code_passes_through() {
    let fx = setup();
    let (mut term, server, done, tmp) = spawn_ssh(&fx);
    term.expect("prompt", b"$ ");
    term.send(b"exit 7\n");
    let stdin = term.stdin.take();
    drop(stdin);
    let st = term.proc_.wait_deadline("ssh");
    assert_eq!(st.code(), Some(7));
    let _ = done.send(());
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn one_shot_command_is_refused_naming_the_variable() {
    // ⛔ A session server serves sessions. An exec request names
    // `SSH_ORIGINAL_COMMAND` and dies with the runtime code, never by
    // running the command.
    let fx = setup();
    let listener = UnixListener::bind(fx.sock_path()).unwrap();
    listener.set_nonblocking(true).unwrap();
    let (server, done) = start_server(listener, fx.config.clone(), fx.force_command.clone());
    let proxy = env!("CARGO_BIN_EXE_proxy");
    let proxy_cmd = format!("'{proxy}' unix '{}'", fx.sock_path().display());
    let out = run_ssh(&proxy_cmd, &fx.user, &fx.user_key, &["echo", "hi"], None);
    assert_eq!(out.status.code(), Some(125));
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(err.contains("SSH_ORIGINAL_COMMAND"), "{err}");
    assert!(out.stdout.is_empty(), "command ran: {:?}", out.stdout);
    let _ = done.send(());
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&fx.tmp);
}

#[test]
fn subsystem_sftp_never_reaches_the_shell() {
    // ⛔ The narrowed half of the refusal catalogue: the generated daemon
    // configuration defines no subsystems, so `sshd` itself refuses the
    // request. The shell binary never runs: stdout stays empty and the
    // exit is nonzero. The stderr wording stays unpinned; it is the
    // client's, not this tree's.
    let fx = setup();
    let listener = UnixListener::bind(fx.sock_path()).unwrap();
    listener.set_nonblocking(true).unwrap();
    let (server, done) = start_server(listener, fx.config.clone(), fx.force_command.clone());
    let proxy = env!("CARGO_BIN_EXE_proxy");
    let proxy_cmd = format!("'{proxy}' unix '{}'", fx.sock_path().display());
    let out = run_ssh(&proxy_cmd, &fx.user, &fx.user_key, &["-s", "sftp"], None);
    assert_ne!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty(), "subsystem ran: {:?}", out.stdout);
    let _ = done.send(());
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&fx.tmp);
}

#[test]
fn ctrl_d_on_empty_line_ends_the_session() {
    // Ctrl-D on an empty line ends input: the shell reads EOF on a pipe
    // and exits 0, and the session reports that code. This walks the
    // input-ended teardown (stdin drop, wait, group kill, bounded drain)
    // that the prompt tests never reach.
    let fx = setup();
    let (mut term, server, done, tmp) = spawn_ssh(&fx);
    term.expect("prompt", b"$ ");
    term.send(b"\x04");
    let stdin = term.stdin.take();
    drop(stdin);
    let st = term.proc_.wait_deadline("ssh");
    assert_eq!(st.code(), Some(0));
    let _ = done.send(());
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn client_eof_submits_the_partial_line_then_ends() {
    // EOF with a half-typed line submits it first: the computed marker
    // below reaches the shell only through that submit, and the session
    // still ends 0 afterwards.
    let fx = setup();
    let (mut term, server, done, tmp) = spawn_ssh(&fx);
    term.expect("prompt", b"$ ");
    term.send(b"echo partial-$((20+2))");
    let stdin = term.stdin.take();
    drop(stdin);
    // The marker arrives before the EOF the shell then reads: only the
    // EOF submit runs the line. The exit wait after it proves the
    // session still ends cleanly.
    term.expect("partial ran", b"partial-22\r\n");
    let st = term.proc_.wait_deadline("ssh");
    assert_eq!(st.code(), Some(0));
    let _ = done.send(());
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn large_output_does_not_wedge_the_session() {
    // Two hundred kilobytes of shell output exceed every pipe buffer on
    // the path. One reader thread per pipe is what keeps the shell from
    // blocking on a full buffer while the supervisor waits for its exit;
    // the computed marker after the flood proves the session lived.
    let fx = setup();
    let (mut term, server, done, tmp) = spawn_ssh(&fx);
    term.expect("prompt", b"$ ");
    term.send(b"i=0; while [ \"$i\" -lt 20000 ]; do echo 0123456789; i=$((i+1)); done\n");
    term.send(b"echo flood-done-$((20+2))\n");
    term.expect("past the flood", b"flood-done-22\r\n");
    finish(term, server, done, tmp);
}
