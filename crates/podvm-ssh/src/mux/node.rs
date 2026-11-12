//! The node's side of podvm: the SSH server of each session, the lines of
//! the binary, and its counts.

use super::*;

/// The counts of a node, and the sessions whose server still runs.
#[derive(Default)]
pub(super) struct Tally {
    state: Mutex<State>,
    ended: Condvar,
}

#[derive(Default)]
struct State {
    stats: NodeStats,
    active: u64,
    /// With `--once`: the line of the try that gave no socket.
    failure: Option<String>,
}

impl Tally {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(super) fn stats(&self) -> NodeStats {
        self.lock().stats
    }

    pub(super) fn failure(&self) -> Option<String> {
        self.lock().failure.clone()
    }

    /// Wait up to `limit` for each session's server to end.
    pub(super) fn wait_for_sessions(&self, limit: Duration) {
        let deadline = Instant::now() + limit;
        let mut state = self.lock();
        while state.active > 0 {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                return;
            };
            state = match self.ended.wait_timeout(state, left) {
                Ok((state, _)) => state,
                Err(e) => e.into_inner().0,
            };
        }
    }
}

/// The node's lines, with podvm's prefix. The crate says `online` at each
/// socket that the relay took; each other line follows a try that gave no
/// socket, or the end of one. With `--once`, the first such line stops the
/// node: an end after `online`, or a failure before it.
pub(super) fn say(
    cfg: &NodeConfig,
    tally: Arc<Tally>,
    stopper: Stopper,
) -> pod_relay::blocking::Say {
    let (name, relay, once) = (cfg.name.clone(), cfg.relay.clone(), cfg.once);
    Arc::new(move |line: String| {
        if line.starts_with("online") {
            tally.lock().stats.sockets += 1;
            eprintln!(
                "podvm-ssh: registered as {name} via {relay} (node speaks {PROTOCOL_VERSION})"
            );
            return;
        }
        eprintln!("podvm-ssh: node: {line}");
        if once {
            let mut state = tally.lock();
            if state.stats.sockets == 0 && state.failure.is_none() {
                state.failure = Some(line);
            }
            stopper.stop();
        }
    })
}

/// Start the server of one session on a socketpair: the crate reads the
/// socket on one thread and writes it on another. The server is killed and
/// reaped when the session ends.
pub(super) fn open_server(command: &str, tally: &Arc<Tally>) -> Result<Local, String> {
    let (socket, mut child) = match crate::server::spawn_socket(command) {
        Ok(parts) => parts,
        Err(e) => {
            tally.lock().stats.sessions_refused += 1;
            return Err(e.to_string());
        }
    };
    let clones = socket
        .try_clone()
        .and_then(|r| Ok((r, socket.try_clone()?)));
    let (reader, closer) = match clones {
        Ok(clones) => clones,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            tally.lock().stats.sessions_refused += 1;
            return Err(format!("the server's socket: {e}"));
        }
    };
    // ⛔ A server that stops reading its stdin ends its own session inside
    // the bound, instead of holding the relay's bytes for ever.
    let _ = socket.set_write_timeout(Some(WRITE_DEADLINE));
    let active = {
        let mut state = tally.lock();
        state.active += 1;
        state.active
    };
    eprintln!("podvm-ssh: session opened ({active} active)");
    let tally = tally.clone();
    Ok(Local::new(reader, socket).on_finish(move || {
        let _ = closer.shutdown(Shutdown::Both);
        let _ = child.kill();
        let _ = child.wait();
        let active = {
            let mut state = tally.lock();
            state.active = state.active.saturating_sub(1);
            state.stats.sessions_completed += 1;
            state.active
        };
        tally.ended.notify_all();
        eprintln!("podvm-ssh: session closed ({active} active)");
    }))
}

/// Why the relay ended a node for good.
pub(super) fn exit_words(exit: &Exit) -> String {
    match exit {
        Exit::Stopped => "the node was stopped".to_string(),
        Exit::NameInUse => "another node holds the pair name on the relay (409)".to_string(),
        Exit::PairStopped => "the relay says that the pair was stopped".to_string(),
        Exit::PairExpired => "the pair expired".to_string(),
        Exit::Forbidden => {
            "the relay refused the node token (403): a wrong token, or a stopped or expired pair"
                .to_string()
        }
        Exit::Unusable(why) => why.clone(),
    }
}
