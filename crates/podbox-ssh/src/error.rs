//! The one error type, and the one exit code table.
//!
//! ⛔ EVERY FAILURE LEAVES THIS CRATE AS A `podssh::error::Error`. A transport
//! that returns a bare string is a transport whose failure a caller cannot
//! match on, and podssh's caller is an agent that has to decide whether to
//! try the next relay or give up. The `Kind` is what it matches on; the
//! sentence is what a person reads.
//!
//! ⛔ **`Kind::Unreachable` and `Kind::Refused` are different on purpose, and
//! the difference is a measured one.** A relay that answers `CONNECT 403` and
//! a relay that answers `CONNECT 200` and then hangs are both "this relay did
//! not work", and the second one is the dangerous one: it fails at KEX time
//! rather than at connect time, so it looks like a target problem.

use std::fmt;

/// The exit codes podssh returns, and where each one comes from.
///
/// ⛔ `Usage` is 2, ssh's own code for a usage error. `Transport` is 255,
/// OpenSSH's code for "the connection could not be set up", and it is what a
/// `ProxyCommand` that cannot connect must return so the ssh client reports
/// its own standard error rather than a podssh-specific one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// The relay list was exhausted, or nothing was configured that could run.
    /// 1: 0 would be a lie about a session that never started.
    NoRoute = 1,
    /// A flag or an argument named something podssh has no row for.
    Usage = 2,
    /// The transport could not be established.
    Transport = 255,
}

impl Exit {
    pub fn code(self) -> i32 {
        self as i32
    }
}

/// What sort of failure this is. A caller matching on this decides whether to
/// try the next hop, and getting that wrong is the difference between a
/// session that starts and a session that hangs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The configuration itself is wrong: no relay, no credential, a flag
    /// that names nothing. Retrying an identical invocation cannot help.
    Config,
    /// A name did not resolve. Distinct from `Refused` because a sandbox with
    /// dead UDP DNS hits this on almost every target, and the fix is the DoH
    /// resolver rather than another relay.
    Resolve,
    /// Something answered and said no: a `403 not on the egress allowlist`, a
    /// TURN `401`, a TCP `ECONNREFUSED`. The hop is reached; it declined.
    Refused,
    /// Nothing answered inside the bound. The hop may be dead, may be
    /// filtering, or may answer `CONNECT 200` and then go silent.
    Unreachable,
    /// A relay answered but spoke something other than what the hop needs: a
    /// TLS alert, an HTTP status where a stream was wanted, a peer that closed
    /// mid-handshake.
    Protocol,
    /// A credential was rejected.
    Auth,
    /// An underlying IO or TLS error that is none of the above.
    Io,
}

impl Kind {
    /// Whether a caller should try the next hop in the chain after this.
    ///
    /// ⛔ `Config`, `Auth` and `Resolve` return false, and that is the point:
    /// retrying a bad credential against every relay in a list turns one clear
    /// sentence into N identical ones and multiplies the wall clock by the
    /// length of the list. `Resolve` returns false too, because the name will
    /// not resolve on the next relay either.
    pub fn retryable(self) -> bool {
        matches!(self, Kind::Unreachable | Kind::Refused | Kind::Protocol | Kind::Io)
    }

    /// A stable machine-readable token, for `--json` and for an agent that
    /// matches on a string rather than on an exit code.
    pub fn token(self) -> &'static str {
        match self {
            Kind::Config => "config",
            Kind::Resolve => "resolve",
            Kind::Refused => "refused",
            Kind::Unreachable => "unreachable",
            Kind::Protocol => "protocol",
            Kind::Auth => "auth",
            Kind::Io => "io",
        }
    }
}

/// One failure, carrying what kind it is and which hop raised it.
#[derive(Debug, Clone)]
pub struct Error {
    pub kind: Kind,
    /// The hop that failed, in the words a person would use: `egress proxy`,
    /// `relay 1/3 (165.22.103.5:443)`, `target railway.new:22`.
    pub hop: String,
    /// The reason, as a sentence. Never empty.
    pub because: String,
}

impl Error {
    pub fn new(kind: Kind, hop: impl Into<String>, because: impl Into<String>) -> Self {
        Error { kind, hop: hop.into(), because: because.into() }
    }

    /// The exit code this failure maps to.
    pub fn exit(&self) -> Exit {
        match self.kind {
            Kind::Config => Exit::NoRoute,
            _ => Exit::Transport,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "podssh: {}: {}", self.hop, self.because)?;
        if self.kind != Kind::Io {
            write!(f, " [{}]", self.kind.token())?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    /// An `io::Error` reaches here without a hop, because a bare conversion
    /// would have to invent one and an invented hop name is a lie. The callers
    /// that wrap IO set `Error::hop` themselves.
    fn from(e: std::io::Error) -> Self {
        let kind = match e.kind() {
            std::io::ErrorKind::ConnectionRefused => Kind::Refused,
            std::io::ErrorKind::TimedOut => Kind::Unreachable,
            _ => Kind::Io,
        };
        Error::new(kind, "podssh", e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Take the error out of a `Result` whose `Ok` side has no `Debug`.
///
/// ⛔ `Box<dyn Stream>` is not `Debug`, so `.unwrap_err()` does not compile on
/// a `Result<Box<dyn Stream>>`. Tests need the error, so this takes it without
/// formatting the success value, and PANICS if the result was `Ok` rather than
/// inventing a placeholder error.
#[cfg(test)]
pub fn err_of<T>(r: Result<T>) -> Error {
    match r {
        Err(e) => e,
        Ok(_) => panic!("expected an error, got Ok"),
    }
}

/// The same, for a `Result` whose `Ok` side is a boxed trait object. The
/// `dyn Stream` has no `Debug`, so the test cannot use `unwrap_err`.
#[cfg(test)]
pub fn err_of_boxed(r: std::result::Result<Box<dyn crate::transport::Stream>, Error>) -> Error {
    match r {
        Err(e) => e,
        Ok(_) => panic!("expected an error, got Ok"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_and_a_timeout_are_different_kinds() {
        // Measured, 2026-09-26: the sandbox egress answers
        // `403 not on the egress allowlist` in under a millisecond for a port
        // it does not carry, and a dead public relay times out at 8s. One
        // code for both would make a policy refusal look like a slow relay.
        let refused = Error::new(Kind::Refused, "egress", "403 not on the egress allowlist");
        let silent = Error::new(Kind::Unreachable, "relay 1/3", "no response in 8s");
        assert_ne!(refused.kind, silent.kind);
        assert_eq!(refused.kind.token(), "refused");
        assert_eq!(silent.kind.token(), "unreachable");
    }

    #[test]
    fn only_a_bad_configuration_and_a_bad_credential_stop_the_chain() {
        // ⛔ The regression this holds: a TURN 401 retried against every relay
        // in a list turns one sentence into N and multiplies the wall clock by
        // the list length, for an outcome that cannot change.
        for kind in [Kind::Config, Kind::Auth, Kind::Resolve] {
            assert!(!kind.retryable(), "{kind:?} must not be retried against the next hop");
        }
        for kind in [Kind::Unreachable, Kind::Refused, Kind::Protocol, Kind::Io] {
            assert!(kind.retryable(), "{kind:?} must be retried against the next hop");
        }
    }

    #[test]
    fn a_config_failure_is_not_a_transport_failure() {
        assert_eq!(Error::new(Kind::Config, "config", "no relay").exit(), Exit::NoRoute);
        assert_eq!(Error::new(Kind::Auth, "turn", "401").exit(), Exit::Transport);
        assert_eq!(Error::new(Kind::Unreachable, "relay", "timeout").exit(), Exit::Transport);
        assert_eq!(Exit::Usage.code(), 2);
        assert_eq!(Exit::Transport.code(), 255);
    }

    #[test]
    fn an_error_always_says_which_hop_and_why() {
        let e = Error::new(Kind::Protocol, "relay 2/3", "answered HTTP 200 and closed");
        let s = e.to_string();
        assert!(s.contains("relay 2/3"), "{s}");
        assert!(s.contains("answered HTTP 200 and closed"), "{s}");
        assert!(s.contains("protocol"), "{s}");
    }

    #[test]
    fn a_refused_tcp_error_keeps_its_kind_across_the_conversion() {
        let io = std::io::Error::from(std::io::ErrorKind::ConnectionRefused);
        assert_eq!(Error::from(io).kind, Kind::Refused);
        let io = std::io::Error::from(std::io::ErrorKind::TimedOut);
        assert_eq!(Error::from(io).kind, Kind::Unreachable);
    }
}
