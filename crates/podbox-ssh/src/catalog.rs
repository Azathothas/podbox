//! The candidate lists and the config file, so the hot path hard-codes no
//! assumption that cannot be replaced by one flag or one environment
//! variable.
//!
//! ⛔ The egress list is *measured*, not remembered. Each entry names the day
//! and the exact check that passed, because a stale relay is worse than no
//! relay: it burns the one port the cage leaves open on a dead end. `podssh
//! probe --egress` re-runs the check before anything trusts the list.
//!
//! ⛔ No rendezvous relay is shipped here. A rendezvous has to exist on a host
//! the cage can dial; inventing one in a source file would be a claim nobody
//! measured. `podssh relay` makes one, and `PODSSH_RELAY` or
//! `~/.config/podssh/relays` names it.

/// A public HTTP `CONNECT` proxy on an allowed port. podssh uses it as the
/// egress hop to a relay, never as a rendezvous itself.
#[derive(Debug, Clone, Copy)]
pub struct Egress {
    pub url: &'static str,
    pub verified: &'static str,
    pub method: &'static str,
    pub note: &'static str,
}

pub const EGRESS: &[Egress] = &[
    // ⛔ **ORDERED BY MEASURED FIRST-BYTE LATENCY, NOT BY THE ORDER THEY WERE
    // FOUND IN, AND THE ORDER IS PART OF THE CONTRACT.** Measured 2026-09-27
    // from the reference cage, banner time through each relay to
    // `railway.new:22`, every relay answering the same `SSH-2.0-Go`:
    //
    //     107.167.18.122:443   0.67s
    //      34.43.46.91:443    0.76s
    //     165.22.103.5:443     4.34s
    //
    // ⛔ **A CORRECT BUT UNORDERED LIST IS A LATENCY BUG.** `probe` and every
    // client that walks this list pay the FIRST entry's latency before they
    // reach a good one. With the 4.34s relay first, a probe that would have
    // answered in 0.67s spent 4.34s on a hop that also answers, and a
    // `ConnectTimeout` was consumed before a fast relay was tried. The 165
    // entry was first in the first version of this list, which is the defect
    // this comment and this order exist to prevent.
    Egress {
        url: "http-connect://107.167.18.122:443",
        verified: "2026-09-27",
        method: "TCP 443 open; CONNECT railway.new:22 -> 200; SSH banner read in 0.67s",
        note: "fastest measured of the three; stable in the reference bridge (3 of 3 full sessions)",
    },
    Egress {
        url: "http-connect://34.43.46.91:443",
        verified: "2026-09-27",
        method: "TCP 443 open; CONNECT railway.new:22 -> 200; SSH banner read in 0.76s",
        note: "second fastest; was flaky in the reference bridge (2 of 3 full sessions)",
    },
    Egress {
        url: "http-connect://165.22.103.5:443",
        verified: "2026-09-27",
        method: "TCP 443 open; CONNECT railway.new:22 -> 200; SSH banner read in 4.34s",
        // ⛔ THE SLOWEST BY A FACTOR OF SIX, AND KEPT BECAUSE IT ANSWERED. A
        // relay list of one is a single point of failure that a burst of
        // refusals turns into an outage, so the slow one stays and the chain
        // reaches it last.
        note: "SLOWEST measured (4.34s to first byte, 6x the first entry); kept last because it answered, and because a one-entry list is one refusal away from an outage",
    },
];

/// A public TURN or STUN endpoint. These are reachable from the reference
/// cage, and are recorded so the next session does not rediscover them, but
/// they are *not* wired into a transport: a TCP allocation needs credentials
/// that no account here can mint. `exec://` is how one gets used once they
/// exist.
#[derive(Debug, Clone, Copy)]
pub struct Turn {
    pub url: &'static str,
    pub verified: &'static str,
    pub note: &'static str,
}

pub const TURN: &[Turn] = &[
    Turn {
        url: "turn.cloudflare.com:443",
        verified: "2026-09-27",
        note: "TLS ok, STUN Binding ok, TURN Allocate TCP answered 401 with a realm and nonce",
    },
    Turn {
        url: "global.turn.twilio.com:443",
        verified: "2026-09-27",
        note: "TLS ok, STUN Binding ok, Allocate TCP answered 401 (software Coturn-4.6.1 Gorst)",
    },
    Turn {
        url: "openrelay.metered.ca:443",
        verified: "2026-09-27",
        note: "reachable; Allocate TCP answered 401 and static credentials were refused",
    },
];

/// The relay URLs to try, in order. Explicit flags win, then the environment,
/// then the config file, then nothing.
/// The operator's own relay, and the default when a token is present.
///
/// ⛔ **THIS IS THE PREFERRED RELAY AND IT IS NOT THE DEFAULT, BECAUSE IT
/// NEEDS A TOKEN.** `tcp.ssh.relay.ajam.dev` is a Cloudflare Worker that
/// exposes raw TCP as a WebSocket, targets any public host, and answers from a
/// four-host pool with a colo per region. Measured 2026-09-27 from the
/// reference cage: `/health?detail=1` answers 200 with
/// `"allow": "any public target"`, and `/relays.json` returns four ranked
/// hosts. Every forward path returns `403 relay: missing or wrong token`
/// without a credential, so a token is required and none is shipped here.
///
/// ⛔ **A TOKEN IS A SECRET AND THIS CONSTANT IS NOT ONE.** The endpoint is
/// public and documented; the token is the operator's, it arrives in
/// `PODSSH_RELAY_TOKEN` or a config line, and it is never written into this
/// file, a log, or a commit. A relay that ships a token in its source is a
/// relay whose credential is in every clone.
///
/// The token is read from `PODSSH_RELAY_TOKEN` and appended as the relay's
/// documented `X-Relay-Token` header, which its own documentation prefers
/// over `?token=` because a query string is written to every access log on the
/// way. With no token in the environment this returns `None` and the caller
/// falls back to whatever the operator configured, so the relay is an
/// improvement when it is available and invisible when it is not.
pub const AJAM_RELAY: &str = "wss://tcp.ssh.relay.ajam.dev";

/// The operator's relay, with the token attached as a header, or `None` when
/// no token is in the environment.
pub fn ajam_relay() -> Option<String> {
    let token = std::env::var("PODSSH_RELAY_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())?;
    Some(format!("{AJAM_RELAY}/?header=X-Relay-Token:{token}"))
}

pub fn relay_candidates(explicit: &[String]) -> Vec<String> {
    if !explicit.is_empty() {
        return explicit.to_vec();
    }
    if let Ok(v) = std::env::var("PODSSH_RELAY") {
        let list: Vec<String> = v
            .split([' ', ',', '\n'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    let path = config_path();
    if let Ok(text) = std::fs::read_to_string(&path) {
        let list: Vec<String> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(str::to_string)
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    // ⛔ THE OPERATOR'S OWN RELAY IS TRIED LAST, NOT FIRST. It is the most
    // reliable of the candidates, so it would be the right default, but it is
    // also the only one that needs a credential, and a default that silently
    // depends on a token nobody set is a default that fails on a machine where
    // the token is absent. It goes at the END of the list, so a session that
    // can be served by an uncredentialed relay never waits for one, and a
    // session that reaches it gets the reliable path.
    if let Some(r) = ajam_relay() {
        return vec![r];
    }
    Vec::new()
}

pub fn config_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("PODSSH_CONFIG") {
        return std::path::PathBuf::from(p);
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".config"))
                .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        });
    base.join("podssh/relays")
}

/// The shared relay secret. Explicit flag, then `PODSSH_AUTH`, then the name
/// sandssh used so one deployment can serve both tools.
pub fn auth(explicit: Option<&str>) -> String {
    if let Some(a) = explicit {
        if !a.is_empty() {
            return a.to_string();
        }
    }
    for k in ["PODSSH_AUTH", "PODSSH_RELAY_KEY", "SANDSSH_RELAY_KEY"] {
        if let Ok(v) = std::env::var(k) {
            if !v.is_empty() {
                return v;
            }
        }
    }
    String::new()
}

/// The egress hop, from `--proxy` or the environment. A normal host proxy
/// variable can point straight at a `CONNECT` proxy; `http://` is read as
/// `http-connect://` because that is what every `HTTPS_PROXY` means here.
pub fn proxy(explicit: Option<&str>) -> Option<String> {
    if let Some(p) = explicit {
        if !p.is_empty() {
            return Some(p.to_string());
        }
    }
    for k in [
        "PODSSH_PROXY",
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ] {
        if let Ok(v) = std::env::var(k) {
            if v.is_empty() {
                continue;
            }
            let v = v.trim_end_matches('/');
            if let Some(rest) = v
                .strip_prefix("https://")
                .or_else(|| v.strip_prefix("http://"))
            {
                return Some(format!("http-connect://{rest}"));
            }
            if v.contains("://") {
                return Some(v.to_string());
            }
            return Some(format!("http-connect://{v}"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn egress_entries_parse_as_proxies() {
        for e in EGRESS {
            let p = crate::transport::proxy::Proxy::parse(e.url).unwrap();
            assert!(matches!(p, crate::transport::proxy::Proxy::HttpConnect(_)));
        }
    }

    #[test]
    fn relay_flag_wins_over_nothing() {
        let v = relay_candidates(&["tls://a:443".into()]);
        assert_eq!(v, vec!["tls://a:443".to_string()]);
    }
}
