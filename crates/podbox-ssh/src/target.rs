//! What a hop is asked to reach: a `host:port`, parsed once, used everywhere.
//!
//! ⛔ **`Target` is the only place a destination is parsed, and `ProxyCommand`
//! hands us the same string `ssh` was given.** OpenSSH's `ProxyCommand` passes
//! exactly two arguments: `%h` and `%p`, substituted. So the string that
//! reaches `podssh proxy` is the host the operator typed, and podssh must not
//! re-parse it differently than ssh did, or a session goes to one host and
//! podssh thinks it went to another. One parser, used by the ProxyCommand path
//! and the `connect` path alike.

use crate::error::{Error, Kind, Result};

/// A destination: a host (name or literal address) and a port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
}

impl Target {
    /// Parse `host:port`, `[v6]:port`, or a bare `host` with `default_port`.
    ///
    /// ⛔ **A bare host gets `default_port` and never a guessed port.** The
    /// temptation is to assume 22 because the whole tool is about ssh; a caller
    /// that passes a bare host through a TURN allocation to a database port
    /// would silently get the wrong service. 22 is `default_port` at the ssh
    /// call sites, not a rule here.
    pub fn parse(spec: &str, default_port: u16) -> Result<Target> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err(Error::new(Kind::Config, "target", "the destination is empty"));
        }
        // Bracketed IPv6, with or without a port: [::1] or [::1]:22.
        if let Some(rest) = spec.strip_prefix('[') {
            let (host, tail) = rest.split_once(']').ok_or_else(|| {
                Error::new(Kind::Config, "target", format!("{spec:?} opens [ and never closes it"))
            })?;
            if host.is_empty() {
                return Err(Error::new(Kind::Config, "target", format!("{spec:?} has an empty host")));
            }
            // The tail after `]` is `""` or `":port"`, so the leading colon
            // belongs to the separator, not to the port.
            let port = match tail.strip_prefix(':') {
                None if tail.is_empty() => default_port,
                None => {
                    return Err(Error::new(
                        Kind::Config,
                        "target",
                        format!("{spec:?} has {tail:?} after ] where :port or nothing was meant"),
                    ))
                }
                Some(p) => parse_port(p, spec)?,
            };
            return Ok(Target { host: host.to_string(), port });
        }
        // More than one colon and no brackets is a bare IPv6 literal, which
        // carries no port. `::1` and `fe80::1` are addresses, not `host:port`.
        if spec.matches(':').count() > 1 {
            return Ok(Target { host: spec.to_string(), port: default_port });
        }
        match spec.split_once(':') {
            Some((host, p)) => {
                if host.is_empty() {
                    return Err(Error::new(Kind::Config, "target", format!("{spec:?} has no host")));
                }
                Ok(Target { host: host.to_string(), port: parse_port(p, spec)? })
            }
            None => Ok(Target { host: spec.to_string(), port: default_port }),
        }
    }

    /// The `host:port` spelling a CONNECT request carries, with an IPv6
    /// literal bracketed because `CONNECT` follows the URI authority rule and
    /// an unbracketed `::1:22` is ambiguous to every parser down the chain.
    pub fn authority(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// A `host:port` spelling for a person, unbracketed, which is how a relay
    /// list is written and how an error names the thing that failed.
    pub fn plain(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Whether the host is a literal address rather than a name to resolve.
    pub fn host_is_literal(&self) -> bool {
        self.host.parse::<std::net::IpAddr>().is_ok()
    }
}

fn parse_port(p: &str, whole: &str) -> Result<u16> {
    // ⛔ A non-numeric port is a CONFIG failure and not a parse panic: `ssh -p
    // http` must not abort the process. And 0 is refused: a destination on
    // port 0 is never what a caller meant, and several hops would treat it as
    // "any port" and answer with something surprising.
    let n: u32 = p
        .parse()
        .map_err(|_| Error::new(Kind::Config, "target", format!("{whole:?}: {p:?} is not a port")))?;
    if n == 0 {
        return Err(Error::new(Kind::Config, "target", format!("{whole:?}: port 0 names no service")));
    }
    if n > u16::MAX as u32 {
        return Err(Error::new(Kind::Config, "target", format!("{whole:?}: port {n} is over 65535")));
    }
    Ok(n as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_spellings_a_destination_arrives_in() {
        // ssh's ProxyCommand passes `%h %p` as TWO arguments, so the bare
        // host and the bare port are both real inputs and not edge cases.
        assert_eq!(Target::parse("example.com", 22).unwrap(),
                   Target { host: "example.com".into(), port: 22 });
        assert_eq!(Target::parse("example.com:2222", 22).unwrap(),
                   Target { host: "example.com".into(), port: 2222 });
        assert_eq!(Target::parse("[2001:db8::1]:22", 22).unwrap(),
                   Target { host: "2001:db8::1".into(), port: 22 });
        assert_eq!(Target::parse("[2001:db8::1]", 22).unwrap(),
                   Target { host: "2001:db8::1".into(), port: 22 });
    }

    #[test]
    fn a_bare_ipv6_literal_is_an_address_and_carries_the_default_port() {
        // ⛔ Measured against the failure this prevents: `::1` splits into
        // three colon-separated fields, and a naive `split_once(':')` reads
        // the host as empty and refuses. More importantly `fe80::1%eth0` is an
        // address, not a host:port pair.
        assert_eq!(Target::parse("::1", 22).unwrap(), Target { host: "::1".into(), port: 22 });
        assert_eq!(Target::parse("fe80::1", 22).unwrap(), Target { host: "fe80::1".into(), port: 22 });
    }

    #[test]
    fn a_bad_port_is_a_config_failure_and_not_a_panic() {
        // `ssh -p http` reaches here. It must refuse, and it must refuse with
        // a sentence rather than aborting a ProxyCommand mid-handshake.
        for spec in ["example.com:http", "example.com:0", "example.com:99999", "example.com:-1", ":22"] {
            let e = Target::parse(spec, 22).unwrap_err();
            assert_eq!(e.kind, Kind::Config, "{spec} should be a config failure");
        }
        assert_eq!(Target::parse("", 22).unwrap_err().kind, Kind::Config);
        assert_eq!(Target::parse("[::1", 22).unwrap_err().kind, Kind::Config);
        assert_eq!(Target::parse("[]:22", 22).unwrap_err().kind, Kind::Config);
    }

    #[test]
    fn an_ipv6_authority_is_bracketed_and_a_name_is_not() {
        // ⛔ `CONNECT ::1:22 HTTP/1.1` is ambiguous and the relay parses the
        // wrong thing. The authority rule is RFC 9110 section 4.2.3.
        assert_eq!(Target::parse("[::1]:22", 22).unwrap().authority(), "[::1]:22");
        assert_eq!(Target::parse("example.com:22", 22).unwrap().authority(), "example.com:22");
    }

    #[test]
    fn literal_detection_survives_a_port() {
        assert!(Target::parse("192.0.2.10:22", 22).unwrap().host_is_literal());
        assert!(!Target::parse("example.com:22", 22).unwrap().host_is_literal());
        assert!(Target::parse("2001:db8::1", 22).unwrap().host_is_literal());
    }
}
