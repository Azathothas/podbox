//! Two concurrent sessions on one multiplexed node connection.
//!
//! A fake relay speaks the relay side of `reverse-v1` on loopback TCP (plain
//! `ws`, no TLS, over the frame codec of `pod-ws`): the 403 of a wrong token
//! and the 409 of a second node at the upgrade, hello, open, ready
//! forwarding, id prefixing and stripping,
//! per-side closes, and the r12 refusal rules it can trigger without a
//! network, including the 1009 oversize closes that mirror the relay's frame
//! rules on both legs (R12:163, R12:165). Neither close arm is a tripwire
//! any driver reaches (both shipped legs size their reads at the cap),
//! so they are spec, and the entry names the guards that do trip. The
//! node and operator under test are the real binaries, which run the client
//! of `pod-relay` (PodUtils/podcrates) through its blocking facade. One test drives
//! session mechanics through `cat`; the next drives three real,
//! authenticated SSH sessions (two concurrent at a time) through real
//! servers the node spawns.
//!
//! ⛔ The fake relay is a high-fidelity mock, not a second implementation to
//! trust: it enforces the same rules the live relay documents (ready gating,
//! bare-frame, text-frame, and oversize closes, id validation, hello caps),
//! and the live drive in `experiments/387-mux-two-client.sh` is the
//! acceptance gate against the real relay.
//!
//! ⛔ Every wait here is bounded. Readers run on threads feeding channels,
//! conditions are polled with deadlines, and child processes are reaped by a
//! drop guard so a failure cannot wedge the suite.

mod common;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{
    current_user, ensure_privsep, fresh_tmp, make_keys, require_binary, run_ssh, wait_for,
};

#[path = "mux_two_client/ws.rs"]
mod ws;
use ws::{Frame, Ws};

const DEADLINE: Duration = Duration::from_secs(30);
const IO_DEADLINE: Duration = Duration::from_secs(20);

// ------------------------------------------------------------ process guard

/// A child that is killed and reaped on drop, so a failed assertion cannot
/// leave a node or operator behind to hold a port or a session.
struct Proc {
    child: Option<Child>,
}

impl Proc {
    fn spawn(mut cmd: Command) -> Self {
        Proc {
            child: Some(cmd.spawn().expect("could not spawn child")),
        }
    }

    fn stdin(&mut self) -> std::process::ChildStdin {
        self.child.as_mut().unwrap().stdin.take().unwrap()
    }

    fn kill(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    fn wait_deadline(mut self, what: &str, deadline: Duration) -> std::process::ExitStatus {
        let start = Instant::now();
        loop {
            match self.child.as_mut().unwrap().try_wait().unwrap() {
                Some(st) => {
                    self.child.take();
                    return st;
                }
                None => {
                    if start.elapsed() >= deadline {
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
        self.kill();
    }
}

// ------------------------------------------------------------ pipe readers

/// Lines arriving on a pipe, read on a thread so the test thread can wait
/// with a deadline instead of blocking.
struct Lines {
    rx: std::sync::mpsc::Receiver<String>,
}

impl Lines {
    fn spawn(pipe: impl Read + Send + 'static) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(pipe);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        if tx.send(line.clone()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Lines { rx }
    }

    fn wait_contains(&self, what: &str, needle: &str, deadline: Duration) {
        let start = Instant::now();
        let mut seen = Vec::new();
        loop {
            match self.rx.recv_timeout(Duration::from_millis(100)) {
                Ok(line) => {
                    if line.contains(needle) {
                        return;
                    }
                    seen.push(line);
                }
                Err(_) => {
                    if start.elapsed() >= deadline {
                        panic!("{what}: never saw {needle:?}; saw: {seen:?}");
                    }
                }
            }
        }
    }

    /// Drain every line still queued. What arrives after this call is a
    /// failure of sequence, not of timing.
    fn drain(&self) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(l) = self.rx.try_recv() {
            out.push(l);
        }
        out
    }
}

/// Raw bytes arriving on a pipe, same shape as [`Lines`].
struct Bytes {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
}

impl Bytes {
    fn spawn(pipe: impl Read + Send + 'static) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = [0u8; 65536];
            let mut pipe = pipe;
            loop {
                match pipe.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Bytes { rx }
    }

    fn read_exact(&self, what: &str, mut want: &[u8], deadline: Duration) {
        let start = Instant::now();
        while !want.is_empty() {
            let left = start.elapsed();
            if left >= deadline {
                panic!("{what}: timed out with {} bytes still wanted", want.len());
            }
            match self.rx.recv_timeout(deadline - left) {
                Ok(chunk) => {
                    assert!(
                        chunk.len() <= want.len() && chunk.as_slice() == &want[..chunk.len()],
                        "{what}: expected {want:?}, got {chunk:?}"
                    );
                    want = &want[chunk.len()..];
                }
                Err(_) => panic!("{what}: pipe ended with {} bytes still wanted", want.len()),
            }
        }
    }

    fn read_timeout_is_quiet(&self, what: &str, quiet_for: Duration) {
        if let Ok(chunk) = self.rx.recv_timeout(quiet_for) {
            panic!("{what}: stray bytes arrived: {chunk:?}")
        }
    }
}

// ------------------------------------------------------------ the fake relay

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionRec {
    id: String,
    node_bytes: u64,
    op_bytes: u64,
    completed: bool,
    /// Refused with `reject`, never served. Kept apart from `completed`:
    /// a refusal ends the entry, and counting it as completed would let a
    /// table-full test pass while serving nothing.
    rejected: bool,
}

#[derive(Debug, Default)]
struct Records {
    node_conns: u32,
    wrong_tokens: u32,
    sessions: Vec<SessionRec>,
    /// (side, code) for every close frame the relay sent.
    closes: Vec<(String, u16)>,
}

/// How the fake relay treats its peers. Every fault-injection shape the
/// suite needs: the live relay is the only other implementation, and it
/// cannot be told to misbehave on demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Full multiplexed behaviour per the r12 rules.
    Normal,
    /// Refuse every operator with a `reject` naming the reason.
    RejectAll,
    /// Accept operators and then never ready them.
    Silent,
    /// Send this TEXT frame instead of `hello` on the node socket.
    CustomHello(&'static str),
    /// Send a BINARY frame instead of `hello` on the node socket.
    BinaryFirst,
    /// Accept operators, immediately end them with `bye`, and hold the
    /// socket.
    Bye,
    /// Accept the node and then end a sessionless socket after its grace:
    /// the `--once` exit-1 path needs a clean end with zero sessions.
    Lonely,
    /// Abort the first node socket after its first completed session, with
    /// no goodbye: the mid-service drop a live reconnect must survive. The
    /// node redials the same name; later sockets serve to the test's end.
    /// The first socket's sessions already completed, so the drop strands
    /// no bytes: a mid-frame kill would strand the first session and could
    /// never show two completions.
    Reconnect,
}

struct FakeRelay {
    port: u16,
    records: Arc<Mutex<Records>>,
    running: Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl FakeRelay {
    fn start(node_token: &str, connect_token: &str, mode: Mode) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let records = Arc::new(Mutex::new(Records::default()));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let node_token = node_token.to_string();
        let connect_token = connect_token.to_string();
        let records_bg = records.clone();
        let running_bg = running.clone();
        let handle = std::thread::spawn(move || {
            serve(
                listener,
                &node_token,
                &connect_token,
                mode,
                records_bg,
                running_bg,
            );
        });
        FakeRelay {
            port,
            records,
            running,
            handle: Some(handle),
        }
    }

    fn origin(&self) -> String {
        format!("ws://127.0.0.1:{}", self.port)
    }

    fn stop(mut self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[path = "mux_two_client/relay.rs"]
mod relay;
use relay::*;

#[path = "mux_two_client/sessions.rs"]
mod sessions;
use sessions::*;

#[path = "mux_two_client/refusals.rs"]
mod refusals;
