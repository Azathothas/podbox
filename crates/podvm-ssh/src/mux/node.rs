use super::*;

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
    let jitter = match podvm_probe::sys::getrandom(&mut b) {
        Ok(_) => Duration::from_millis((b[0] as u64 * 1000) / 256),
        Err(_) => Duration::from_millis(0),
    };
    (base, jitter)
}

// ------------------------------------------------------------ dial

/// Open the relay leg: TCP dial (through the proxy of the environment, if
/// one applies), TLS for `wss`, then the websocket upgrade
/// for `path` with the token in `X-Relay-Token`. Every hop names itself on
/// failure; the token never appears in any of them.
pub(super) fn dial_ws(origin: &str, path: &str, token: &str) -> Result<Ws, Error> {
    validate_token(token)?;
    let (tls, host, port) = parse_origin(origin)?;
    // ⛔ The relay leg takes the proxy of the environment: `HTTPS_PROXY`,
    // `ALL_PROXY` and `NO_PROXY` apply, and a loopback relay goes directly.
    let dialer = Dialer::from_environment(Duration::from_secs(10))
        .map_err(|e| Error::dial(format!("relay proxy {host}:{port}"), e))?;
    let mut base = dialer.dial(&crate::transport::Target::Tcp {
        host: host.clone(),
        port,
    })?;
    // ⛔ The write bound is set here, once, so every frame on this leg
    // carries it: a relay that stops reading ends the op inside
    // `WRITE_DEADLINE` instead of wedging the loop, and the socket ends so
    // the node redials. It is set before TLS, because the handshake runs in
    // `Tls::connect` and its writes take the same bound.
    base.set_write_timeout(Some(WRITE_DEADLINE))
        .map_err(|e| Error::dial(format!("relay write bound {host}:{port}{path}"), e))?;
    if tls {
        let cfg = crate::tls::TlsConfig::load().map_err(|e| Error::dial("relay tls roots", e))?;
        base = Box::new(
            crate::tls::Tls::connect(base, &host, &cfg)
                .map_err(|e| Error::dial(format!("relay tls {host}:{port}"), e))?,
        );
    }
    let host_header = format!("{host}:{port}");
    let headers = vec![("X-Relay-Token".to_string(), token.to_string())];
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
pub(super) fn leg_err(leg: &'static str, e: io::Error) -> Error {
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
                    eprintln!("podvm-ssh: node socket ended; redialing");
                }
            }
            Err(e) => {
                if cfg.once {
                    return Err(e);
                }
                let (base, jitter) = backoff_secs(attempt);
                attempt = attempt.saturating_add(1);
                eprintln!("podvm-ssh: node: {e}; retry in {base}s");
                std::thread::sleep(Duration::from_secs(base) + jitter);
                continue;
            }
        }
        attempt = 0;
    }
}

pub(super) struct Session {
    pub(super) server: crate::server::ServerChild,
    /// Bytes from the relay waiting for the server's stdin.
    pub(super) inbox: VecDeque<Vec<u8>>,
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
        "podvm-ssh: registered as {} via {} (node speaks {PROTOCOL_VERSION}; relay reports {version}; frame cap {payload_cap}, max sessions {max_sessions})",
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
                        eprintln!("podvm-ssh: relay said bye");
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
                            "podvm-ssh: dropped {} bytes for closed session",
                            payload.len()
                        );
                    }
                }
            }
            Some(Frame::Close(code, reason)) => {
                eprintln!(
                    "podvm-ssh: relay closed the node socket: {code} {}",
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
