//! A transport spec is a URL-shaped string. Nothing about it is podssh
//! specific: a scheme names a transport, the authority names where it goes,
//! and for `exec` the rest is a command line.
//!
//! ⛔ The parser is deliberately permissive about what a scheme may be. A
//! transport is looked up by name in [`crate::transport::Dialer`], so a
//! deployment can add one without this file learning about it.

use std::io;

/// One parsed transport spec. `raw` is kept so an error can quote what the
/// operator actually typed rather than a reconstruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub scheme: String,
    pub user: Option<String>,
    pub password: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub path: String,
    pub raw: String,
}

impl Spec {
    /// The default port for a scheme, used when the authority carries none.
    pub fn default_port(scheme: &str) -> Option<u16> {
        match scheme {
            "tcp" => Some(80),
            "tls" => Some(443),
            "ws" | "http" => Some(80),
            "wss" | "https" => Some(443),
            "https-connect" | "http-connect" | "socks5" | "socks5h" => Some(443),
            _ => None,
        }
    }

    pub fn port_or_default(&self) -> Option<u16> {
        self.port.or_else(|| Spec::default_port(&self.scheme))
    }

    /// `host:port`, with the default port filled in.
    pub fn authority(&self) -> io::Result<String> {
        let host = self
            .host
            .as_deref()
            .ok_or_else(|| io::Error::other(format!("{} needs a host", self.raw)))?;
        match self.port_or_default() {
            Some(p) => Ok(format!("{host}:{p}")),
            None => Ok(host.to_string()),
        }
    }
}

/// Parse `scheme://[user[:pass]@]host[:port][/path]`, with `unix` and `exec`
/// taken literally after the scheme.
pub fn parse(raw: &str) -> io::Result<Spec> {
    let (scheme, rest) = raw
        .split_once("://")
        .ok_or_else(|| io::Error::other(format!("{raw:?} has no scheme:// prefix")))?;
    if scheme.is_empty() {
        return Err(io::Error::other(format!("{raw:?} has an empty scheme")));
    }
    let scheme = scheme.to_ascii_lowercase();

    // unix:///a/b, ws+unix:///a/b and exec://cmd are opaque: the rest is a
    // path or a command. `+unix` exists so the websocket and TLS code paths
    // can be exercised in a cage that denies TCP bind, exactly as sandssh's
    // `ws+unix://` did.
    if scheme == "unix" || scheme.ends_with("+unix") {
        return Ok(Spec {
            scheme,
            user: None,
            password: None,
            host: None,
            port: None,
            path: rest.to_string(),
            raw: raw.to_string(),
        });
    }
    if scheme == "exec" {
        return Ok(Spec {
            scheme,
            user: None,
            password: None,
            host: None,
            port: None,
            path: rest.to_string(),
            raw: raw.to_string(),
        });
    }

    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let (userinfo, hostport) = match authority.rsplit_once('@') {
        Some((u, h)) => (Some(u), h),
        None => (None, authority),
    };
    let (user, password) = match userinfo {
        Some(u) => match u.split_once(':') {
            Some((n, p)) => (Some(n.to_string()), Some(p.to_string())),
            None => (Some(u.to_string()), None),
        },
        None => (None, None),
    };

    // IPv6 literals are bracketed: [::1]:8443.
    let (host, port) = if let Some(close) = hostport.strip_prefix('[') {
        let (h, tail) = close
            .split_once(']')
            .ok_or_else(|| io::Error::other(format!("{raw:?} has an unterminated IPv6 literal")))?;
        let port = match tail.strip_prefix(':') {
            Some(p) => Some(parse_port(p, raw)?),
            None if tail.is_empty() => None,
            None => return Err(io::Error::other(format!("{raw:?} has trailing {tail:?}"))),
        };
        (h.to_string(), port)
    } else {
        match hostport.rsplit_once(':') {
            Some((h, p)) if !h.is_empty() => (h.to_string(), Some(parse_port(p, raw)?)),
            Some(_) => return Err(io::Error::other(format!("{raw:?} needs a host"))),
            None => (hostport.to_string(), None),
        }
    };
    if host.is_empty() {
        return Err(io::Error::other(format!("{raw:?} needs a host")));
    }
    Ok(Spec {
        scheme,
        user,
        password,
        host: Some(host),
        port,
        path: path.to_string(),
        raw: raw.to_string(),
    })
}

fn parse_port(p: &str, raw: &str) -> io::Result<u16> {
    p.parse::<u16>()
        .map_err(|_| io::Error::other(format!("{raw:?} has a bad port {p:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_with_port() {
        let s = parse("tls://relay.example.com:8443").unwrap();
        assert_eq!(s.scheme, "tls");
        assert_eq!(s.host.as_deref(), Some("relay.example.com"));
        assert_eq!(s.port, Some(8443));
        assert_eq!(s.path, "");
    }

    #[test]
    fn wss_default_443_and_path() {
        let s = parse("wss://relay.example.com/podssh/v1").unwrap();
        assert_eq!(s.scheme, "wss");
        assert_eq!(s.port_or_default(), Some(443));
        assert_eq!(s.path, "/podssh/v1");
    }

    #[test]
    fn credentials_are_split() {
        let s = parse("http-connect://user:pass@proxy.example:443").unwrap();
        assert_eq!(s.user.as_deref(), Some("user"));
        assert_eq!(s.password.as_deref(), Some("pass"));
        assert_eq!(s.host.as_deref(), Some("proxy.example"));
        assert_eq!(s.port, Some(443));
    }

    #[test]
    fn ipv6_literal() {
        let s = parse("tls://[2606:4700::1111]:443").unwrap();
        assert_eq!(s.host.as_deref(), Some("2606:4700::1111"));
        assert_eq!(s.port, Some(443));
    }

    #[test]
    fn unix_and_exec_are_opaque() {
        let s = parse("unix:///tmp/relay.sock").unwrap();
        assert_eq!(s.path, "/tmp/relay.sock");
        let s = parse("exec:///usr/bin/nc -w5 host 443").unwrap();
        assert_eq!(s.path, "/usr/bin/nc -w5 host 443");
    }

    #[test]
    fn rejects_no_scheme() {
        assert!(parse("relay.example.com:443").is_err());
    }
}
