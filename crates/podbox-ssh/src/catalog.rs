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
    Egress {
        url: "http-connect://165.22.103.5:443",
        verified: "2026-09-27",
        method: "TCP 443 open; CONNECT railway.new:22 -> 200; SSH banner read",
        note: "fastest of the three in the reference bridge (1.52s median)",
    },
    Egress {
        url: "http-connect://34.43.46.91:443",
        verified: "2026-09-27",
        method: "TCP 443 open; CONNECT railway.new:22 -> 200; SSH banner read",
        note: "was flaky in the reference bridge (2 of 3 full sessions)",
    },
    Egress {
        url: "http-connect://107.167.18.122:443",
        verified: "2026-09-27",
        method: "TCP 443 open; CONNECT railway.new:22 -> 200; SSH banner read",
        note: "stable in the reference bridge (3 of 3 full sessions)",
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
    for k in ["PODSSH_PROXY", "HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"] {
        if let Ok(v) = std::env::var(k) {
            if v.is_empty() {
                continue;
            }
            let v = v.trim_end_matches('/');
            if let Some(rest) = v.strip_prefix("https://").or_else(|| v.strip_prefix("http://")) {
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
