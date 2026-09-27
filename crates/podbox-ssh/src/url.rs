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

    /// Extra request headers, written `?header=Name:Value` in the spec.
    ///
    /// ⛔ THIS EXISTS FOR AN AUTHENTICATED RELAY, AND THE HEADER FORM IS
    /// PREFERRED OVER THE QUERY FORM FOR A REASON THAT IS ABOUT LOGS. The
    /// ajam relay at `tcp.ssh.relay.ajam.dev` takes its forward token as
    /// `X-Relay-Token`, or as `?token=`, and says "when possible" about the
    /// header. A query string is written to every proxy access log along the
    /// way; a header is not. So a token that travels in a URL is a token that
    /// ends up in a log somebody else keeps.
    pub headers: Vec<(String, String)>,
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
        // An opaque spec makes no HTTP request, so it has no headers.
        headers: Vec::new(),
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
        // An opaque spec makes no HTTP request, so it has no headers.
        headers: Vec::new(),
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
    // `?header=Name:Value` (repeatable) becomes a request header. Both the
    // name and the value are validated, because a header assembled from a
    // configuration string is a place where a newline would smuggle a second
    // header into the request.
    let mut headers = Vec::new();
    if let Some((_, q)) = path.split_once('?') {
        for pair in q.split('&').filter(|s| !s.is_empty()) {
            let (k, v) = pair.split_once('=').ok_or_else(|| {
                io::Error::other(format!("{raw:?} has a query parameter with no ="))
            })?;
            if k != "header" {
                continue;
            }
            let (name, value) = v
                .split_once(':')
                .ok_or_else(|| io::Error::other(format!("{raw:?}: header= wants Name:Value")))?;
            let name = percent_decode(name);
            let value = percent_decode(value);
            if name.is_empty() || !name.bytes().all(is_token_char) {
                return Err(io::Error::other(format!(
                    "{raw:?}: {name:?} is not a valid header name"
                )));
            }
            if value.bytes().any(|b| b == b'\r' || b == b'\n' || b == 0) {
                return Err(io::Error::other(format!(
                    "{raw:?}: a header value may not carry a newline"
                )));
            }
            headers.push((name, value));
        }
    }
    Ok(Spec {
        scheme,
        user,
        password,
        host: Some(host),
        port,
        path: path.to_string(),
        raw: raw.to_string(),
        headers,
    })
}

fn parse_port(p: &str, raw: &str) -> io::Result<u16> {
    p.parse::<u16>()
        .map_err(|_| io::Error::other(format!("{raw:?} has a bad port {p:?}")))
}

/// An RFC 7230 token character: the only bytes a header name may be made of.
fn is_token_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

/// Decode `%XX` and `+`, which is enough for a header value in a spec string.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                Ok(v) => {
                    out.push(v);
                    i += 3;
                }
                Err(_) => {
                    out.push(b[i]);
                    i += 1;
                }
            },
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
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

    #[test]
    fn a_relay_token_travels_in_a_header_and_not_only_a_query() {
        // ⛔ The shape the ajam relay documents: a forward token as
        // `X-Relay-Token`, reachable as `?header=X-Relay-Token:<token>`.
        let s = parse(
            "wss://tcp.ssh.relay.ajam.dev/connect/railway.new/22?header=X-Relay-Token:abc123",
        )
        .unwrap();
        assert_eq!(s.host.as_deref(), Some("tcp.ssh.relay.ajam.dev"));
        // The spec carried no port, so the scheme's default applies. That the
        // default is applied HERE and not stored in `port` is deliberate: a
        // stored default would make a spec that read `wss://h` and one that
        // read `wss://h:443` compare equal to different things.
        assert_eq!(s.port, None);
        assert_eq!(s.port_or_default(), Some(443));
        assert_eq!(s.path, "/connect/railway.new/22?header=X-Relay-Token:abc123");
        assert_eq!(s.headers, vec![("X-Relay-Token".to_string(), "abc123".to_string())]);
    }

    #[test]
    fn a_percent_encoded_token_survives_and_a_newline_does_not() {
        // ⛔ A token is operator-supplied text that lands in a request line, so
        // the one thing that must be impossible is injecting a second header
        // with it. This is request smuggling through a configuration string.
        let s = parse("wss://h/connect/a/22?header=X-Relay-Token:a%20b").unwrap();
        assert_eq!(s.headers[0].1, "a b");
        assert!(parse("wss://h/p?header=X-Relay-Token:a%0d%0aX-Evil:1").is_err());
        assert!(parse("wss://h/p?header=Bad%20Name:1").is_err());
        assert!(parse("wss://h/p?header=NoColon").is_err());
    }

    #[test]
    fn a_spec_with_no_query_carries_no_headers_and_the_query_survives() {
        let s = parse("wss://h/connect/a/22").unwrap();
        assert!(s.headers.is_empty());
        // The query stays in the path, so `?token=` keeps working for a relay
        // that only reads the query form.
        let q = parse("wss://h/connect/a/22?token=xyz").unwrap();
        assert_eq!(q.path, "/connect/a/22?token=xyz");
    }
}
