//! The multiplexed reverse relay protocol, `reverse-v1`.
//!
//! ## The specification, as measured
//!
//! The authority is the relay's own document, captured at
//! `docs/history/references/relay-index-2026-09-28-r12.md` (fetched
//! 2026-09-28 from `https://tcp.ssh.relay.ajam.dev/index.md`; the `r12`
//! version is the close table's own heading at line 140). `R12:` below
//! cites that file's lines. The cross-checks are the captured dropssh tree
//! (`references/talaria0101__dropssh/tree`, commit `0aafa21d`): the
//! `references/talaria0101__dropssh/tree/docs/reverse-relay.md` four-way
//! frame measurement, the
//! `references/talaria0101__dropssh/tree/docs/relay-issues.md` close-code
//! findings, and the tree's `tests/mux-probe.py` framing rules. Where the
//! two disagree, the disagreement is recorded
//! below and the live relay wins.
//!
//! One name holds one node socket. Both peers dial out; nothing here listens.
//! Pair, tokens, and expiry are R12:117-118; the second-node 409 is R12:134:
//!
//! | role | address | token |
//! | --- | --- | --- |
//! | node | `wss://HOST/v1/node/{name}` | `node_token` in `X-Relay-Token` |
//! | operator | `wss://HOST/v1/connect/{name}` | `connect_token` in `X-Relay-Token` |
//! | status | `GET /v1/status/{name}` | `connect_token` only; the other two get 403 |
//! | stop | `POST /v1/stop/{name}` | `stop_token` |
//! | pair | `POST /v1/pair` with `{}` | returns `{name, node_token, connect_token, stop_token, expires}` |
//!
//! `expires` is ms since epoch, within 72 h of creation (R12:117-118). A
//! second node on a connected name is refused HTTP 409 before accept
//! (R12:134).
//!
//! Control messages are TEXT frames carrying JSON with a `type` field (not
//! `verb`: the codec refuses the old spelling, and the codec tests pin a
//! `verb` frame refused):
//!
//! | direction | message | when |
//! | --- | --- | --- |
//! | relay -> node | `{"type":"hello","version":1,"maxFrameBytes":N,"maxSessions":M}` | immediately after the upgrade |
//! | relay -> node | `{"type":"open","id":"<32 hex>"}` | an operator arrived |
//! | node -> relay | `{"type":"ready","id":"<32 hex>"}` | the local server is up; answer promptly (R12:171: no `ready` within 15 s of `open` reaps the operator with 1013) |
//! | node -> relay | `{"type":"reject","id":"<32 hex>","reason":"..."}` | the session could not be started |
//! | relay -> node | `{"type":"close","id":"<32 hex>"}` | that session is over; the rest survive |
//! | either | `{"type":"bye"}` | that side is finished |
//!
//! Data frames are BINARY, and the two directions are NOT symmetric
//! (R12:124-125; measured four ways in
//! `references/talaria0101__dropssh/tree/docs/reverse-relay.md`):
//!
//! | direction | the peer sends | the relay does | the far side receives |
//! | --- | --- | --- | --- |
//! | operator -> node | bare payload, no id | prepends the session id | `id + payload` |
//! | node -> operator | `id + payload`, one frame | strips the id | bare payload |
//!
//! The operator sends raw session bytes and the relay frames them; an id an
//! operator sends is rewritten, not honoured, so it is never addressing. The
//! node must prefix the full id in the same frame. The relay's closes, one
//! per violation (R12:140-174):
//!
//! - node frame under 32 bytes or over 65568: 1009 `bad multiplex frame`
//!   (R12:163);
//! - non-hex 32-byte prefix: 1003 `bad multiplex id` (R12:157);
//! - well-formed prefix naming no session: 1003, logged by the relay
//!   (R12:133);
//! - data for a session never readied: 1003 `data before ready` (R12:158);
//! - operator text frame: 1003 `binary frames required` (R12:159);
//! - operator frame over 65536 payload bytes: 1009 `frame byte cap`
//!   (R12:165);
//! - session over 64 MiB in both directions: 1009 `session byte cap`
//!   (R12:164);
//! - operator data before the node's `ready`: 1008 `wait for ready`
//!   (R12:161).
//!
//! A node `close` reaches the operator as 1000, a node `reject` as 1011,
//! both carrying the node's own reason truncated to 100 characters: parse
//! the reason, not the code (R12:173). When the relay itself closed the node
//! for cause, that cause is forwarded to the operator's sessions instead of
//! the 1011 default (R12:144). A zero-length frame every 25 s is a keepalive;
//! ignore empty payloads (R12:196). Node JSON control over 4 KiB closes the
//! node with 1009 (R12:162).
//!
//! Limits: 64 sessions and 64 KiB frames from the hello the live relay sends
//! (observed: `experiments/results/mux-two-client.txt`); 64 MiB, 180 s of
//! payload inactivity, and 720 min per session from the relay's policy
//! (R12:164, R12:210-211).
//!
//! ## Disagreements found while specifying
//!
//! The captured
//! `references/talaria0101__dropssh/tree/docs/reverse-relay.md:179-180`
//! step 5 says a
//! received node payload "is bare, with no id to parse". Its own
//! `src/serve.c:842-866` copies the first 32 bytes as the id and queues the
//! rest, and its own `tests/mux-probe.py:370-379` asserts the operator's
//! bare bytes reach the node as `id + payload`. The code, the probe, and
//! the live relay agree against the sentence: the node receives `id +
//! payload` and strips the prefix. This implementation follows the three
//! against the one.
//!
//! ## What this module owns
//!
//! The node leg (register, serve many sessions on one socket, redial) and
//! the operator leg (wait for `ready`, move stdio as bare frames). The relay
//! owns its caps: this side chunks its sends at the announced frame size and
//! answers `open` at once, and never re-enforces the relay's 64 MiB gate.
//! The hello session count is mirrored as prompt admission control
//! (`reject` when full), which the relay itself requires. One gate per action.
//!
//! ⛔ Tokens never enter a log line or an error string. They travel in
//! `X-Relay-Token`, never in a URL, because a URL is written to every access
//! log on the way and a header is not.

use std::collections::{HashMap, VecDeque};
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use crate::error::{Error, Kind};
use crate::transport::{write_stall, Dialer, Stream, WRITE_DEADLINE};
use crate::ws::{Frame, Ws};

/// The relay protocol version this module speaks. Reported in the node's
/// registered line beside the version the relay itself sent, so the two can
/// be compared in one place.
pub const PROTOCOL_VERSION: &str = "reverse-v1";
/// The `hello` version this node accepts. Named, not inlined, so the gate
/// below reads as a version check rather than a magic comparison.
const HELLO_VERSION: u64 = 1;
/// Largest session payload per frame. The relay closes anything over this.
pub const MAX_PAYLOAD: usize = 65536;
/// Length of a session id: 32 ASCII hex characters.
pub const ID_LEN: usize = 32;
/// How long one loop turn waits on the socket before polling sessions.
const SOCKET_POLL: Duration = Duration::from_millis(50);
/// How long one session read waits before the loop moves to the next.
const SESSION_POLL: Duration = Duration::from_millis(10);
/// How long the node waits for `hello` after the upgrade.
const HELLO_WAIT: Duration = Duration::from_secs(15);
/// Cap on stdin bytes queued before the operator sees `ready`. An SSH
/// client speaks first and then waits, so a megabyte of queued bytes with
/// no `ready` means the session is never coming.
const PREREADY_CAP: usize = 1024 * 1024;
/// How long the operator waits for `ready` before failing the session.
const DEFAULT_READY_WAIT: Duration = Duration::from_secs(20);
/// How long the operator lingers for the relay's goodbye after stdin ends.
const GOODBYE_WAIT: Duration = Duration::from_secs(10);

// ------------------------------------------------------------ validation

/// True for a 32-character hex session id. Either case parses: the relay
/// documents lowercase, ids are echoed verbatim, and case never addresses.
pub fn is_session_id(s: &str) -> bool {
    s.len() == ID_LEN && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A relay origin: `wss://host[:port]` anywhere, or `ws://` on loopback for
/// tests. Paths, queries, userinfo, and whitespace are refused: the paths
/// are built here from the name, and anything else in the origin is a caller
/// mistake that would dial somewhere unreviewed.
pub fn parse_origin(origin: &str) -> Result<(bool, String, u16), Error> {
    let usage = |m: String| Error::new(Kind::Usage, "relay origin", m);
    let (tls, rest) = match origin.split_once("://") {
        Some(("wss", r)) => (true, r),
        Some(("ws", r)) => (false, r),
        _ => {
            return Err(usage(format!(
                "{origin:?} is not a relay origin: want wss://host[:port]"
            )))
        }
    };
    if rest.is_empty() {
        return Err(usage("the relay origin has no host".to_string()));
    }
    // ⛔ The origin is scheme plus authority and nothing else. A path or
    // query here would be silently dropped by the split below and the dial
    // would go somewhere the caller did not review.
    if rest.contains(['/', '?', '#', '@'])
        || rest
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
    {
        return Err(usage(format!(
            "{origin:?} carries a path, query, userinfo, or blank: pass the bare origin"
        )));
    }
    let (host, port) = if let Some(inner) = rest.strip_prefix('[') {
        // Bracketed IPv6 literal: `[::1]` or `[::1]:22`.
        let (h, tail) = inner
            .split_once(']')
            .ok_or_else(|| usage(format!("{origin:?} opens [ and never closes it")))?;
        if h.is_empty() {
            return Err(usage("the relay origin has no host".to_string()));
        }
        // ⛔ Brackets are URI spelling, not part of the name: the dial and
        // the TLS server name take the bare literal. IPv6 loopback parses
        // here; no lane drive dials it yet.
        let host = h.to_string();
        match tail.strip_prefix(':') {
            None if tail.is_empty() => (host, None),
            None => {
                return Err(usage(format!(
                    "{origin:?} has {tail:?} after ] where :port or nothing was meant"
                )))
            }
            Some(p) => {
                let port = crate::transport::parse_port(p)
                    .map_err(|e| usage(format!("{origin:?} has a bad port: {}", e.message)))?;
                (host, Some(port))
            }
        }
    } else if rest.matches(':').count() > 1 {
        // A bare IPv6 literal carries no port.
        (rest.to_string(), None)
    } else {
        match rest.split_once(':') {
            None => (rest.to_string(), None),
            Some((h, p)) => {
                if h.is_empty() {
                    return Err(usage("the relay origin has no host".to_string()));
                }
                let port = crate::transport::parse_port(p)
                    .map_err(|e| usage(format!("{origin:?} has a bad port: {}", e.message)))?;
                (h.to_string(), Some(port))
            }
        }
    };
    if host.is_empty() {
        return Err(usage("the relay origin has no host".to_string()));
    }
    if !tls {
        let loopback = host == "127.0.0.1" || host == "::1" || host == "localhost";
        if !loopback {
            return Err(usage(format!(
                "{origin:?}: plain ws:// is loopback-only for tests; use wss://"
            )));
        }
    }
    Ok((tls, host, port.unwrap_or(if tls { 443 } else { 80 })))
}

mod node;
pub use node::*;

mod operator;
pub use operator::*;

#[cfg(test)]
mod tests;
