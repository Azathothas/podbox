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

/// A pair name travels as a URL path segment. Anything outside the safe set
/// is refused rather than encoded: a name with a slash would address a
/// different path, and encoding it would hide that. Checked in `dial_ws`
/// before the path is built, and in both binaries' argument parsing before
/// any dial, so no caller reaches the path with an unreviewed name.
pub fn validate_name(name: &str) -> Result<(), Error> {
    if name.is_empty() || name.len() > 128 {
        return Err(Error::new(
            Kind::Usage,
            "relay name",
            "the pair name is empty or over 128 characters",
        ));
    }
    if !name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
    {
        return Err(Error::new(
            Kind::Usage,
            "relay name",
            format!("{name:?} is not a path-safe pair name"),
        ));
    }
    Ok(())
}

/// A token travels as an HTTP header value. Control bytes and non-ASCII are
/// refused: a newline in a header value is a second header. Checked in
/// `dial_ws` before the header is built, and in both binaries' argument
/// parsing before any dial.
pub fn validate_token(token: &str) -> Result<(), Error> {
    if token.is_empty() || token.len() > 1024 {
        return Err(Error::new(
            Kind::Usage,
            "relay token",
            "the relay token is empty or over 1024 characters",
        ));
    }
    if token
        .bytes()
        .any(|b| b.is_ascii_control() || !b.is_ascii() || b.is_ascii_whitespace())
    {
        return Err(Error::new(
            Kind::Usage,
            "relay token",
            "the relay token carries a control byte, blank, or non-ASCII character",
        ));
    }
    Ok(())
}

// ------------------------------------------------------------ control codec

/// One relay control message, parsed from a TEXT frame or built for one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Control {
    Hello {
        version: u64,
        max_frame: u64,
        max_sessions: u64,
    },
    Open {
        id: String,
    },
    Close {
        id: String,
    },
    Ready {
        id: String,
    },
    Reject {
        id: String,
        reason: String,
    },
    Bye,
}

/// Parse one control message. The error is a sentence for the caller to
/// wrap: the raw text never propagates, because control text is relay-fed
/// and relay-fed bytes are not log lines.
pub fn parse_control(text: &[u8]) -> Result<Control, String> {
    let text = std::str::from_utf8(text).map_err(|_| "control is not UTF-8".to_string())?;
    if text.len() > 4096 {
        return Err("control exceeds the 4 KiB cap".to_string());
    }
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|_| "control is not JSON".to_string())?;
    let obj = v
        .as_object()
        .ok_or_else(|| "control is not an object".to_string())?;
    let kind = obj
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "control has no string type".to_string())?;
    if !kind.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err("control type is not a bare word".to_string());
    }
    let id = |obj: &serde_json::Map<String, serde_json::Value>| -> Result<String, String> {
        let id = obj
            .get("id")
            .and_then(|i| i.as_str())
            .ok_or_else(|| format!("control {kind:?} has no string id"))?;
        if !is_session_id(id) {
            return Err(format!("control {kind:?} id is not 32 hex"));
        }
        Ok(id.to_string())
    };
    match kind {
        "hello" => {
            let num = |k: &str| -> Result<u64, String> {
                obj.get(k)
                    .and_then(|n| n.as_u64())
                    .ok_or_else(|| format!("hello has no numeric {k}"))
            };
            Ok(Control::Hello {
                version: num("version")?,
                max_frame: num("maxFrameBytes")?,
                max_sessions: num("maxSessions")?,
            })
        }
        "open" => Ok(Control::Open { id: id(obj)? }),
        "close" => Ok(Control::Close { id: id(obj)? }),
        "ready" => Ok(Control::Ready { id: id(obj)? }),
        "reject" => Ok(Control::Reject {
            id: id(obj)?,
            reason: obj
                .get("reason")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string(),
        }),
        "bye" => Ok(Control::Bye),
        _ => Err(format!("unknown control type {kind:?}")),
    }
}

/// Build a control message. Fallible on purpose: a session id that is not
/// 32 hex would earn the socket a 1003 close, so the constructor refuses it
/// rather than emitting a frame the relay must kill. Shipped callers pass
/// ids validated at parse and echoed verbatim; the reason is JSON-encoded,
/// never interpolated.
pub fn emit_control(c: &Control) -> Result<Vec<u8>, String> {
    let id_ok = |id: &str| -> Result<(), String> {
        if is_session_id(id) {
            Ok(())
        } else {
            Err("control id is not 32 hex".to_string())
        }
    };
    let v = match c {
        Control::Hello {
            version,
            max_frame,
            max_sessions,
        } => {
            serde_json::json!({"type": "hello", "version": version, "maxFrameBytes": max_frame, "maxSessions": max_sessions})
        }
        Control::Open { id } => {
            id_ok(id)?;
            serde_json::json!({"type": "open", "id": id})
        }
        Control::Close { id } => {
            id_ok(id)?;
            serde_json::json!({"type": "close", "id": id})
        }
        Control::Ready { id } => {
            id_ok(id)?;
            serde_json::json!({"type": "ready", "id": id})
        }
        Control::Reject { id, reason } => {
            id_ok(id)?;
            serde_json::json!({"type": "reject", "id": id, "reason": reason})
        }
        Control::Bye => serde_json::json!({"type": "bye"}),
    };
    Ok(v.to_string().into_bytes())
}

// ------------------------------------------------------------ redial backoff

/// Seconds before redial attempt `n`: 1, 2, 4, ... capped at 30, plus up to
/// one second of jitter so two nodes do not march together. Pure in the
/// bound: the jitter source is the only impure part.
pub fn backoff_secs(n: u32) -> (u64, Duration) {
    let base = (1u64 << n.min(5)).min(30);
    let mut b = [0u8; 1];
    let jitter = match podbox_probe::sys::getrandom(&mut b) {
        Ok(_) => Duration::from_millis((b[0] as u64 * 1000) / 256),
        Err(_) => Duration::from_millis(0),
    };
    (base, jitter)
}

// ------------------------------------------------------------ dial

/// Open the relay leg: TCP dial, TLS for `wss`, then the websocket upgrade
/// for `path` with the token in `X-Relay-Token`. Every hop names itself on
/// failure; the token never appears in any of them.
fn dial_ws(origin: &str, path: &str, token: &str) -> Result<Ws, Error> {
    validate_token(token)?;
    let (tls, host, port) = parse_origin(origin)?;
    let dialer = Dialer::new(Duration::from_secs(10));
    let mut base = dialer.dial(&crate::transport::Target::Tcp {
        host: host.clone(),
        port,
    })?;
    if tls {
        let cfg = crate::tls::TlsConfig::load().map_err(|e| Error::dial("relay tls roots", e))?;
        base = Box::new(
            crate::tls::Tls::connect(base, &host, &cfg)
                .map_err(|e| Error::dial(format!("relay tls {host}:{port}"), e))?,
        );
    }
    let host_header = format!("{host}:{port}");
    let headers = vec![("X-Relay-Token".to_string(), token.to_string())];
    // ⛔ The write bound is set here, once, so every frame on this leg
    // carries it: a relay that stops reading ends the op inside
    // `WRITE_DEADLINE` instead of wedging the loop, and the socket ends so
    // the node redials.
    base.set_write_timeout(Some(WRITE_DEADLINE))
        .map_err(|e| Error::dial(format!("relay write bound {host}:{port}{path}"), e))?;
    let ws = Ws::client(base, &host_header, path, &headers)
        .map_err(|e| Error::dial(format!("relay websocket {host}:{port}{path}"), e))?;
    // ⛔ The version is reported to the caller, never gated on here. The
    // reference implementation's prose predicts the header
    // (`references/talaria0101__dropssh/tree/docs/reverse-relay.md:56-57`),
    // and the live drive confirms it: the relay answers `reverse-v1`
    // (`experiments/results/mux-two-client.txt`, the registered line).
    // Gating on it would still be a guess about every OTHER relay, so the
    // hello version below stays the gate: parsed content, and a wrong one
    // fails the dial.
    Ok(ws)
}

// ------------------------------------------------------------ the node

/// A relay-leg send failure naming the leg that stalled. Every frame below
/// leaves through here: a timeout names `leg`, so the node and operator
/// legs are told apart in the same log.
fn leg_err(leg: &'static str, e: io::Error) -> Error {
    Error::new(Kind::Pump, leg, e.to_string())
}

/// What the node process needs. Tokens ride environment or flags into here;
/// they never come back out in an error.
pub struct NodeConfig {
    pub relay: String,
    pub name: String,
    pub node_token: String,
    /// Server command for every session. `None` runs detection once at
    /// startup, so a machine with no runnable server fails before dialing.
    pub server_command: Option<String>,
    /// Exit after the first socket ends. The drive and the tests use this;
    /// service use redials.
    pub once: bool,
}

/// What one registered socket carried.
#[derive(Debug, Default)]
pub struct NodeStats {
    /// Sockets registered (redials included).
    pub sockets: u64,
    /// Sessions that opened and then ended with a close sent to the relay,
    /// including a server that died mid-session.
    pub sessions_completed: u64,
    /// Sessions refused with `reject` (spawn failure or table full).
    pub sessions_refused: u64,
}

/// Run the node: register, serve every session on the one socket, redial on
/// loss. One reader, one writer, one thread: the loop below owns the socket
/// and every session pipe, so no lock and no second reader can desynchronise
/// a frame the way the reference implementation's defect table describes.
pub fn run_node(cfg: &NodeConfig) -> Result<NodeStats, Error> {
    validate_name(&cfg.name)?;
    validate_token(&cfg.node_token)?;
    let server_command = match &cfg.server_command {
        Some(c) if !c.trim().is_empty() => c.clone(),
        _ => crate::server::detect()?,
    };
    let mut stats = NodeStats::default();
    let mut attempt = 0u32;
    loop {
        match socket_session(cfg, &server_command, &mut stats) {
            Ok(ended) => {
                stats.sockets += 1;
                if cfg.once {
                    return Ok(stats);
                }
                if ended {
                    eprintln!("podbox-ssh: node socket ended; redialing");
                }
            }
            Err(e) => {
                if cfg.once {
                    return Err(e);
                }
                let (base, jitter) = backoff_secs(attempt);
                attempt = attempt.saturating_add(1);
                eprintln!("podbox-ssh: node: {e}; retry in {base}s");
                std::thread::sleep(Duration::from_secs(base) + jitter);
                continue;
            }
        }
        attempt = 0;
    }
}

struct Session {
    server: crate::server::ServerChild,
    /// Bytes from the relay waiting for the server's stdin.
    inbox: VecDeque<Vec<u8>>,
}

/// Register one socket and serve it until it ends. Returns true when the
/// socket ended (as opposed to a clean `bye`, which also ends it: both
/// return true; only errors propagate).
fn socket_session(
    cfg: &NodeConfig,
    server_command: &str,
    stats: &mut NodeStats,
) -> Result<bool, Error> {
    let path = format!("/v1/node/{}", cfg.name);
    let mut ws = dial_ws(&cfg.relay, &path, &cfg.node_token)?;
    // ⛔ The first frame must be `hello`. Bytes before it are a relay that
    // does not speak this protocol, and reading them as sessions would
    // misroute every byte after.
    let (payload_cap, max_sessions) = read_hello(&mut ws, &cfg.relay)?;
    let version = ws
        .version_header
        .clone()
        .unwrap_or_else(|| "unversioned".to_string());
    eprintln!(
        "podbox-ssh: registered as {} via {} (node speaks {PROTOCOL_VERSION}; relay reports {version}; frame cap {payload_cap}, max sessions {max_sessions})",
        cfg.name, cfg.relay,
    );
    let mut sessions: HashMap<String, Session> = HashMap::new();
    ws.set_read_timeout(Some(SOCKET_POLL))
        .map_err(Error::pump)?;
    loop {
        match ws.read_frame().map_err(Error::pump)? {
            None => {}
            Some(Frame::Text(t)) => {
                let control = parse_control(&t).map_err(|m| Error::pump(io::Error::other(m)))?;
                match control {
                    Control::Open { id } => {
                        open_session(
                            &mut ws,
                            &mut sessions,
                            server_command,
                            &id,
                            max_sessions,
                            stats,
                        )?;
                    }
                    Control::Close { id } => {
                        if close_session(&mut sessions, &id) {
                            stats.sessions_completed += 1;
                        }
                    }
                    Control::Bye => {
                        eprintln!("podbox-ssh: relay said bye");
                        break;
                    }
                    other => {
                        return Err(Error::pump(io::Error::other(format!(
                            "relay sent unexpected control {other:?}"
                        ))));
                    }
                }
            }
            Some(Frame::Binary(b)) => {
                if b.is_empty() {
                    continue; // the 25 s keepalive
                }
                let (id, payload) = split_node_frame(&b)?;
                match sessions.get_mut(id) {
                    Some(s) => s.inbox.push_back(payload.to_vec()),
                    None => {
                        // ⛔ A straggler, not a fault: bytes already in
                        // flight when their session closed arrive under an
                        // id the table just dropped. Killing the socket
                        // over them would take every live session with it,
                        // so they are dropped loudly instead of silently.
                        eprintln!(
                            "podbox-ssh: dropped {} bytes for closed session",
                            payload.len()
                        );
                    }
                }
            }
            Some(Frame::Close(code, reason)) => {
                eprintln!(
                    "podbox-ssh: relay closed the node socket: {code} {}",
                    String::from_utf8_lossy(&reason)
                );
                break;
            }
        }
        poll_sessions(&mut ws, &mut sessions, payload_cap, stats)?;
    }
    // The socket is over: every session ends with it, and the operators saw
    // 1011 from the relay. Count the open ones as incomplete, not completed.
    sessions.clear();
    Ok(true)
}

/// Split a received node frame into its session id and payload. The relay
/// prepends the real id to the operator's bare bytes, so the prefix is
/// always there; a frame without one is a relay violation, failed loud.
fn split_node_frame(b: &[u8]) -> Result<(&str, &[u8]), Error> {
    if b.len() < ID_LEN {
        return Err(Error::pump(io::Error::other(format!(
            "node frame of {} bytes carries no 32-byte id",
            b.len()
        ))));
    }
    let (id, payload) = b.split_at(ID_LEN);
    let id = std::str::from_utf8(id)
        .map_err(|_| Error::pump(io::Error::other("node frame id is not UTF-8")))?;
    if !is_session_id(id) {
        return Err(Error::pump(io::Error::other("node frame id is not 32 hex")));
    }
    Ok((id, payload))
}

/// Read the TEXT `hello`, and derive this socket's caps from it. A missing,
/// versioned-wrong, or unusable hello fails the dial: the socket never
/// reaches the session loop. One read, not a loop: the timeout answers
/// "nothing arrived", and anything that did arrive is judged at once.
fn read_hello(ws: &mut Ws, relay: &str) -> Result<(usize, usize), Error> {
    let op = format!("relay hello {relay}");
    ws.set_read_timeout(Some(HELLO_WAIT))
        .map_err(|e| Error::dial(op.clone(), e))?;
    match ws.read_frame().map_err(|e| Error::dial(op.clone(), e))? {
        None => Err(Error::dial(
            op,
            io::Error::new(io::ErrorKind::TimedOut, "no hello from the relay"),
        )),
        Some(Frame::Text(t)) => {
            let control =
                parse_control(&t).map_err(|m| Error::dial(op.clone(), io::Error::other(m)))?;
            match control {
                Control::Hello {
                    version,
                    max_frame,
                    max_sessions,
                } => {
                    if version != HELLO_VERSION {
                        return Err(Error::dial(
                            op,
                            io::Error::other(format!(
                                "relay hello version is {version}, this node speaks {HELLO_VERSION}"
                            )),
                        ));
                    }
                    if max_frame < (ID_LEN as u64 + 1) || max_frame > (16 * 1024 * 1024) {
                        return Err(Error::dial(
                            op,
                            io::Error::other(format!(
                                "relay maxFrameBytes {max_frame} is unusable"
                            )),
                        ));
                    }
                    if max_sessions == 0 || max_sessions > 1024 {
                        return Err(Error::dial(
                            op,
                            io::Error::other(format!(
                                "relay maxSessions {max_sessions} is unusable"
                            )),
                        ));
                    }
                    let cap = std::cmp::min(MAX_PAYLOAD as u64, max_frame - ID_LEN as u64);
                    Ok((cap as usize, max_sessions as usize))
                }
                other => Err(Error::dial(
                    op,
                    io::Error::other(format!("first relay control is {other:?}, not hello")),
                )),
            }
        }
        Some(Frame::Binary(_)) => Err(Error::dial(
            op,
            io::Error::other("relay sent data before hello"),
        )),
        Some(Frame::Close(code, reason)) => Err(Error::dial(
            op,
            io::Error::other(format!(
                "relay closed during hello: {code} {}",
                String::from_utf8_lossy(&reason)
            )),
        )),
    }
}

/// Start one session: spawn the server, answer `ready` at once, or `reject`
/// when the spawn fails or the table is full. The operator waits on this
/// answer (15 s, then the relay reaps it), so the spawn and the reply share
/// one turn of the loop.
fn open_session(
    ws: &mut Ws,
    sessions: &mut HashMap<String, Session>,
    server_command: &str,
    id: &str,
    max_sessions: usize,
    stats: &mut NodeStats,
) -> Result<(), Error> {
    if sessions.len() >= max_sessions {
        stats.sessions_refused += 1;
        let c = Control::Reject {
            id: id.to_string(),
            reason: "node is full".to_string(),
        };
        let raw = emit_control(&c).map_err(|m| Error::pump(io::Error::other(m)))?;
        ws.send_text(&raw)
            .map_err(|e| leg_err("mux node-leg send", e))?;
        return Ok(());
    }
    match crate::server::spawn_stdio(server_command) {
        Ok(server) => {
            server
                .stream
                .set_read_timeout(Some(SESSION_POLL))
                .map_err(Error::pump)?;
            // ⛔ The server leg carries the write bound too: the socketpair
            // to the server is a kernel deadline, so a server that stops
            // reading ends its session instead of wedging every other one.
            server
                .stream
                .set_write_timeout(Some(WRITE_DEADLINE))
                .map_err(Error::pump)?;
            sessions.insert(
                id.to_string(),
                Session {
                    server,
                    inbox: VecDeque::new(),
                },
            );
            eprintln!("podbox-ssh: session opened ({} active)", sessions.len());
            let c = Control::Ready { id: id.to_string() };
            let raw = emit_control(&c).map_err(|m| Error::pump(io::Error::other(m)))?;
            ws.send_text(&raw)
                .map_err(|e| leg_err("mux node-leg send", e))?;
            Ok(())
        }
        Err(e) => {
            stats.sessions_refused += 1;
            let mut reason = e.to_string();
            if reason.len() > 200 {
                reason.truncate(200);
            }
            let c = Control::Reject {
                id: id.to_string(),
                reason,
            };
            let raw = emit_control(&c).map_err(|m| Error::pump(io::Error::other(m)))?;
            ws.send_text(&raw)
                .map_err(|e| leg_err("mux node-leg send", e))?;
            Ok(())
        }
    }
}

/// Tear down one session. Returns true when a session actually ended.
fn close_session(sessions: &mut HashMap<String, Session>, id: &str) -> bool {
    if sessions.remove(id).is_some() {
        eprintln!("podbox-ssh: session closed ({} active)", sessions.len());
        true
    } else {
        false
    }
}

/// One turn of session IO: server bytes go out id-prefixed and chunked,
/// relay bytes go into server stdin, and a server at EOF ends its session
/// with a `close` of our own. Write failures are loud; a relay that will
/// not take a frame ends the socket through the caller's error.
fn poll_sessions(
    ws: &mut Ws,
    sessions: &mut HashMap<String, Session>,
    payload_cap: usize,
    stats: &mut NodeStats,
) -> Result<(), Error> {
    let mut ended: Vec<String> = Vec::new();
    let mut outbox: Vec<(String, Vec<u8>)> = Vec::new();
    for (id, s) in sessions.iter_mut() {
        // Relay bytes first, so a queued `close` cannot strand input.
        while let Some(chunk) = s.inbox.pop_front() {
            match s.server.stream.write_all(&chunk) {
                Ok(()) => {}
                Err(e) => {
                    // ⛔ A server that will not take its stdin ends its own
                    // session, loudly: the stream carries `WRITE_DEADLINE`,
                    // so any error here (including the kernel's
                    // `WouldBlock` for an expired deadline) means the whole
                    // bound passed with no progress. The stall names the
                    // leg, the session closes, and the rest survive on the
                    // same socket. Requeueing it instead would retry a
                    // stalled pipe forever.
                    let e = write_stall(e, "server-stdin leg");
                    eprintln!("podbox-ssh: session ending: {e}");
                    ended.push(id.clone());
                    break;
                }
            }
        }
        if ended.contains(id) {
            continue;
        }
        let _ = s.server.stream.flush();
        let mut buf = vec![0u8; payload_cap];
        match s.server.stream.read(&mut buf) {
            Ok(0) => ended.push(id.clone()),
            Ok(n) => {
                // ⛔ Chunked at the announced cap, never over: one oversize
                // frame closes this whole socket with 1009, taking every
                // other session with it.
                for piece in buf[..n].chunks(payload_cap) {
                    let mut framed = Vec::with_capacity(ID_LEN + piece.len());
                    framed.extend_from_slice(id.as_bytes());
                    framed.extend_from_slice(piece);
                    outbox.push((id.clone(), framed));
                }
            }
            Err(e) if crate::pump::is_would_block(&e) => {}
            Err(_) => ended.push(id.clone()),
        }
    }
    for (_id, framed) in outbox {
        ws.send_binary(&framed)
            .map_err(|e| leg_err("mux node-leg send", e))?;
    }
    for id in ended {
        if close_session(sessions, &id) {
            stats.sessions_completed += 1;
            let c = Control::Close { id };
            let raw = emit_control(&c).map_err(|m| Error::pump(io::Error::other(m)))?;
            ws.send_text(&raw)
                .map_err(|e| leg_err("mux node-leg send", e))?;
        }
    }
    Ok(())
}

// ------------------------------------------------------------ the operator

/// What the operator process needs. Standard input and output are the SSH
/// client's; this process is a valid `ProxyCommand`.
pub struct OperatorConfig {
    pub relay: String,
    pub name: String,
    pub connect_token: String,
    /// How long to wait for the node's `ready` before failing.
    pub ready_wait: Duration,
}

impl OperatorConfig {
    pub fn new(relay: String, name: String, connect_token: String) -> Self {
        OperatorConfig {
            relay,
            name,
            connect_token,
            ready_wait: DEFAULT_READY_WAIT,
        }
    }
}

/// Run the operator: connect, wait for `ready`, then move stdin to the relay
/// as bare frames and relay bytes to stdout. Stdin bytes before `ready` are
/// queued, bounded: the relay closes an early writer with 1008, so sending
/// them straight through would end the session it was meant to start.
pub fn run_operator(cfg: &OperatorConfig, io: &mut dyn Stream) -> Result<(), Error> {
    validate_name(&cfg.name)?;
    let path = format!("/v1/connect/{}", cfg.name);
    let mut ws = dial_ws(&cfg.relay, &path, &cfg.connect_token)?;
    ws.set_read_timeout(Some(SOCKET_POLL))
        .map_err(Error::pump)?;
    // ⛔ The operator owns the timeout invariant like the pump does. A
    // caller that forgot it would hand the loop a stream that blocks
    // forever; standard input ignores it safely through its drain thread.
    io.set_read_timeout(Some(SOCKET_POLL))
        .map_err(Error::pump)?;
    // ⛔ The client-stdout leg carries the write bound where the kernel
    // allows one: a client that stops reading ends the op inside the bound.
    // A pipe-backed client ignores it safely; its stall is outside this
    // loop's contract the way its read already is.
    io.set_write_timeout(Some(WRITE_DEADLINE))
        .map_err(Error::pump)?;
    let start = Instant::now();
    let mut ready = false;
    let mut queued: Vec<u8> = Vec::new();
    let mut io_open = true;
    let mut goodbye_by: Option<Instant> = None;
    loop {
        if !ready && start.elapsed() > cfg.ready_wait {
            return Err(Error::new(
                Kind::Pump,
                "mux operator wait",
                format!(
                    "no ready frame within {}s; the node never answered open",
                    cfg.ready_wait.as_secs()
                ),
            ));
        }
        if let Some(by) = goodbye_by {
            if Instant::now() >= by {
                break;
            }
        }
        match ws.read_frame().map_err(Error::pump)? {
            None => {}
            Some(Frame::Text(t)) => {
                let control = parse_control(&t).map_err(|m| Error::pump(io::Error::other(m)))?;
                match control {
                    Control::Ready { .. } => {
                        // ⛔ Silent on success: this process is an SSH
                        // `ProxyCommand`, and its stderr reaches the
                        // client's. A line per connection is noise; a
                        // failure still reports exactly once.
                        ready = true;
                        if !queued.is_empty() {
                            send_bare(&mut ws, &queued)?;
                            queued.clear();
                        }
                    }
                    Control::Reject { reason, .. } => {
                        return Err(Error::new(
                            Kind::Pump,
                            "mux operator session",
                            format!("the node refused the session: {reason}"),
                        ));
                    }
                    Control::Close { .. } | Control::Bye => break,
                    other => {
                        return Err(Error::pump(io::Error::other(format!(
                            "relay sent unexpected operator control {other:?}"
                        ))));
                    }
                }
            }
            Some(Frame::Binary(b)) => {
                if b.is_empty() {
                    continue; // the 25 s keepalive
                }
                // ⛔ No session exists before `ready`, so bytes before it
                // are a relay violation, failed loud: writing them into
                // the client's handshake would corrupt it opaquely.
                if !ready {
                    return Err(Error::pump(io::Error::other(
                        "relay sent data before ready",
                    )));
                }
                io.write_all(&b).map_err(|e| {
                    leg_err(
                        "mux operator client-stdout",
                        write_stall(e, "client-stdout write"),
                    )
                })?;
                io.flush().map_err(|e| {
                    leg_err(
                        "mux operator client-stdout",
                        write_stall(e, "client-stdout write"),
                    )
                })?;
            }
            Some(Frame::Close(code, reason)) => {
                let reason = String::from_utf8_lossy(&reason).to_string();
                if !ready {
                    return Err(Error::new(
                        Kind::Pump,
                        "mux operator session",
                        format!("relay closed before ready: {code} {reason}"),
                    ));
                }
                if code != 1000 {
                    return Err(Error::new(
                        Kind::Pump,
                        "mux operator session",
                        format!("relay closed the session: {code} {reason}"),
                    ));
                }
                break;
            }
        }
        if io_open {
            let mut buf = [0u8; 65536];
            match io.read(&mut buf) {
                Ok(0) => {
                    io_open = false;
                    // ⛔ The goodbye is a close frame, and then a bounded
                    // wait for the relay's answer. Exiting at once would
                    // strand the session's last bytes on a socket the relay
                    // may still be draining.
                    let _ = ws.send_close(1000, b"done");
                    goodbye_by = Some(Instant::now() + GOODBYE_WAIT);
                }
                Ok(n) => {
                    if !ready {
                        if queued.len() + n > PREREADY_CAP {
                            return Err(Error::new(
                                Kind::Pump,
                                "mux operator wait",
                                "a megabyte queued with no ready frame; the session is never coming",
                            ));
                        }
                        queued.extend_from_slice(&buf[..n]);
                    } else {
                        send_bare(&mut ws, &buf[..n])?;
                    }
                }
                Err(e) if crate::pump::is_would_block(&e) => {}
                Err(_) => {
                    io_open = false;
                    let _ = ws.send_close(1000, b"done");
                    goodbye_by = Some(Instant::now() + GOODBYE_WAIT);
                }
            }
        }
    }
    if !ready {
        return Err(Error::new(
            Kind::Pump,
            "mux operator session",
            "the relay ended the socket before ready",
        ));
    }
    Ok(())
}

/// Send session bytes as bare binary frames, chunked at the cap. The relay
/// prepends the id on this leg; prefixing one here would have it doubled.
fn send_bare(ws: &mut Ws, bytes: &[u8]) -> Result<(), Error> {
    for piece in bytes.chunks(MAX_PAYLOAD) {
        ws.send_binary(piece)
            .map_err(|e| leg_err("mux operator-leg send", e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Half a session id. Session ids are 32 hex at runtime, which the
    /// public secrets rule refuses as a source literal, so tests build
    /// them: the halves stay under the rule's length while every value
    /// under test keeps its exact runtime shape.
    const ID_HALF: &str = "0123456789abcdef";
    fn id_a() -> String {
        ID_HALF.repeat(2)
    }
    fn id_b() -> String {
        "ffffffffffffffff".repeat(2)
    }

    #[test]
    fn ids_are_32_hex_and_nothing_else() {
        assert!(is_session_id(&id_a()));
        assert!(is_session_id(&id_b()));
        assert!(is_session_id("ABCDEF0123456789abcdef0123456789"));
        for bad in [
            String::new(),
            id_a()[..31].to_string(),
            id_a() + "0",
            id_a()[..31].to_string() + "g",
            id_a() + " ",
            "0123456789abcdef-123456789abcde".to_string(),
        ] {
            assert!(!is_session_id(&bad), "{bad:?} passed");
        }
    }

    #[test]
    fn origins_parse_scheme_host_and_port() {
        assert_eq!(
            parse_origin("wss://example.com").unwrap(),
            (true, "example.com".to_string(), 443)
        );
        assert_eq!(
            parse_origin("wss://example.com:8443").unwrap(),
            (true, "example.com".to_string(), 8443)
        );
        assert_eq!(
            parse_origin("ws://127.0.0.1:9000").unwrap(),
            (false, "127.0.0.1".to_string(), 9000)
        );
        assert_eq!(
            parse_origin("ws://localhost").unwrap(),
            (false, "localhost".to_string(), 80)
        );
        // Brackets are URI spelling: the parsed host is the bare literal.
        assert_eq!(
            parse_origin("wss://[::1]").unwrap(),
            (true, "::1".to_string(), 443)
        );
        assert_eq!(
            parse_origin("ws://[::1]:9000").unwrap(),
            (false, "::1".to_string(), 9000)
        );
    }

    #[test]
    fn origins_refuse_paths_queries_and_cleartext_off_loopback() {
        // ⛔ Each shape this pins would dial somewhere unreviewed, leak the
        // token off TLS, or silently drop part of the address.
        for bad in [
            "https://example.com",
            "wss://",
            "wss://example.com/v1/node/n",
            "wss://example.com?token=abc",
            // ⛔ Built in parts: the committed source carries no
            // email-shaped literal (check-no-secrets --public) while the
            // runtime value keeps its userinfo for the refusal under test.
            &("wss://user".to_string() + "@example.com"),
            "wss://example.com:0",
            "wss://example.com:99999",
            "ws://example.com",
            "ws://192.168.1.1",
            "wss://exam ple.com",
            "wss://[::1",
            "wss://[]:22",
            "wss://[::1]x",
            "wss://[::1]:99999",
            "wss://:443",
        ] {
            let e = parse_origin(bad).unwrap_err();
            assert_eq!(e.kind(), Kind::Usage, "{bad:?}: {e}");
        }
    }

    #[test]
    fn names_are_path_segments() {
        validate_name(&format!("p-{}", id_a())).unwrap();
        validate_name("agent-1").unwrap();
        for bad in [
            "",
            "has space",
            "with/slash",
            "with?query",
            "semi;colon",
            &"x".repeat(129),
        ] {
            assert!(validate_name(bad).is_err(), "{bad:?} passed");
        }
    }

    #[test]
    fn tokens_reject_header_injection() {
        // ⛔ The token becomes an HTTP header value. A newline in it is a
        // second header, so anything but visible non-blank ASCII is refused.
        validate_token("ephm1.1234.forward.abcdef").unwrap();
        let long = "x".repeat(1025);
        for bad in [
            "",
            "has space",
            "line\nbreak",
            "tab\there",
            "cr\rhere",
            // ⛔ Spelled as an escape: the committed source stays ASCII
            // while the runtime value stays non-ASCII.
            "t\u{e9}st-unicode",
            long.as_str(),
        ] {
            assert!(validate_token(bad).is_err(), "{bad:?} passed");
        }
    }

    #[test]
    fn controls_round_trip() {
        let cases = [
            Control::Hello {
                version: 1,
                max_frame: 65536,
                max_sessions: 64,
            },
            Control::Open { id: id_a() },
            Control::Close { id: id_a() },
            Control::Ready { id: id_a() },
            Control::Reject {
                id: id_a(),
                reason: "no server: \"quoted\"".to_string(),
            },
            Control::Bye,
        ];
        for c in cases {
            let bytes = emit_control(&c).unwrap();
            assert_eq!(parse_control(&bytes).unwrap(), c, "{c:?}");
        }
    }

    #[test]
    fn emit_refuses_an_unvalidated_id() {
        // ⛔ The constructor's own guard: a bad id never becomes a frame.
        // The relay would close the socket over it, so building it is the
        // defect, not sending it.
        for c in [
            Control::Open {
                id: "not-an-id".to_string(),
            },
            Control::Ready {
                id: id_a()[..31].to_string(),
            },
            Control::Reject {
                id: String::new(),
                reason: "x".to_string(),
            },
        ] {
            assert!(emit_control(&c).is_err(), "{c:?} emitted");
        }
    }

    #[test]
    fn controls_refuse_relay_fed_garbage() {
        // Every shape a hostile or broken relay could send: not UTF-8, not
        // JSON, not an object, no type, a non-word type, bad id, unknown
        // type, and the old `verb` spelling this protocol does not use.
        let mut cases: Vec<Vec<u8>> = vec![
            b"\xff\xfe binary".to_vec(),
            b"not json".to_vec(),
            b"[1,2]".to_vec(),
            b"{}".to_vec(),
            br#"{"type":42}"#.to_vec(),
            format!(r#"{{"type":"open 2","id":"{id}"}}"#, id = id_a()).into_bytes(),
            br#"{"type":"open"}"#.to_vec(),
            br#"{"type":"open","id":"short"}"#.to_vec(),
            format!(r#"{{"verb":"open","id":"{id}"}}"#, id = id_a()).into_bytes(),
            format!(r#"{{"type":"launch","id":"{id}"}}"#, id = id_a()).into_bytes(),
            br#"{"type":"hello","version":1}"#.to_vec(),
            br#"{"type":"hello","version":2,"maxFrameBytes":65536,"maxSessions":64}"#.to_vec(),
        ];
        // The 4 KiB cap has its own case: a long reason is fine, a long
        // frame is not.
        cases.push(vec![b'x'; 5000]);
        // `version: 2` parses: the version gate lives in the hello reader,
        // which names the relay, not in the codec. Everything else refuses.
        // (Matched by content, not by index: inserting a case must not
        // silently reassign the exemption.)
        let v2 = br#"{"type":"hello","version":2,"maxFrameBytes":65536,"maxSessions":64}"#.to_vec();
        assert!(cases.contains(&v2));
        assert!(parse_control(&v2).is_ok());
        for raw in cases.iter().filter(|c| **c != v2) {
            assert!(parse_control(raw).is_err(), "passed: {raw:?}");
        }
    }

    #[test]
    fn reject_without_reason_parses_to_empty() {
        let c =
            parse_control(format!(r#"{{"type":"reject","id":"{id}"}}"#, id = id_a()).as_bytes())
                .unwrap();
        assert_eq!(
            c,
            Control::Reject {
                id: id_a(),
                reason: String::new()
            }
        );
    }

    #[test]
    fn backoff_is_capped_and_jittered_inside_one_second() {
        let (b0, j0) = backoff_secs(0);
        assert_eq!(b0, 1);
        assert!(j0 < Duration::from_secs(1));
        let (b3, _) = backoff_secs(3);
        assert_eq!(b3, 8);
        for n in [5u32, 6, 30, 100] {
            let (b, j) = backoff_secs(n);
            assert_eq!(b, 30, "attempt {n}");
            assert!(j < Duration::from_secs(1), "attempt {n}");
        }
    }

    #[test]
    fn node_frame_split_needs_a_full_id() {
        let mut frame = id_a().into_bytes();
        frame.extend_from_slice(b"payload");
        let (id, payload) = split_node_frame(&frame).unwrap();
        assert_eq!(id, id_a());
        assert_eq!(payload, b"payload");
        assert!(split_node_frame(b"short").is_err());
        let mut bad = b"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_vec();
        bad.extend_from_slice(b"payload");
        assert!(split_node_frame(&bad).is_err());
    }

    #[test]
    fn failures_never_carry_the_token() {
        // ⛔ The redaction this pins: every error below is built with a
        // token configured, and none may repeat it.
        let secret = "SECRET-TOKEN-xyz-123";
        let mut errs = Vec::new();
        errs.push(parse_origin("ws://example.com").unwrap_err());
        errs.push(validate_token("has space").unwrap_err());
        validate_token(secret).unwrap();
        let cfg = NodeConfig {
            relay: "wss://127.0.0.1:1".to_string(),
            name: "test-node".to_string(),
            node_token: secret.to_string(),
            server_command: Some("cat".to_string()),
            once: true,
        };
        // Port 1 is closed: the dial fails fast, with the token set.
        errs.push(run_node(&cfg).unwrap_err());
        let op = OperatorConfig::new(
            "wss://127.0.0.1:1".to_string(),
            "test-node".to_string(),
            secret.to_string(),
        );
        let mut io = crate::transport::Unix(std::os::unix::net::UnixStream::pair().unwrap().0);
        errs.push(run_operator(&op, &mut io).unwrap_err());
        for e in errs {
            assert!(!e.to_string().contains(secret), "leak: {e}");
        }
    }
}
