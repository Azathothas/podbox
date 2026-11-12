//! The origin of a relay, as the binaries take it: `wss://host[:port]`
//! anywhere, or `ws://` on the loopback for tests.

use crate::error::{Error, Kind};

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
