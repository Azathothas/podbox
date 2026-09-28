//! The one error type, and the re-exported runtime exit code.
//!
//! ⛔ Every failure leaves this crate as an [`Error`]. A transport that
//! returns a bare string is a transport whose failure a caller cannot match
//! on. The [`Kind`] is what it matches on; the sentence is what a person
//! reads.

use std::fmt;
use std::io;

// ⛔ Exit codes live in one place: `podbox-probe` holds docker's codes, and a
// second declaration is how two verbs came to disagree about one of them
// (TODO/cli.md T-0802). This crate re-exports rather than re-declares.
pub use podbox_probe::exit::EXIT_RUNTIME_ERROR;

/// What sort of failure this is. A caller matching on this decides what to
/// report, and getting that wrong turns one clear sentence into noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The invocation itself is wrong: a bad flag, a missing argument, a
    /// port that is not a port. Retrying an identical invocation cannot help.
    Usage,
    /// The target could not be reached: refused, timed out, or missing.
    Dial,
    /// The SSH server could not be started, probed, or configured.
    Server,
    /// A spliced link broke while bytes were moving.
    Pump,
}

impl Kind {
    /// A stable machine-readable token for logs and for a caller that
    /// matches on a string rather than on the variant.
    pub fn token(self) -> &'static str {
        match self {
            Kind::Usage => "usage",
            Kind::Dial => "dial",
            Kind::Server => "server",
            Kind::Pump => "pump",
        }
    }
}

/// One failure, carrying what kind it is and what it was doing.
#[derive(Debug)]
pub struct Error {
    kind: Kind,
    /// What was under way: `dial tcp 127.0.0.1:22`, `probe sshd`, `pump`.
    pub op: String,
    /// The reason, as a sentence. Never empty.
    pub message: String,
}

impl Error {
    /// Build an error with no underlying IO failure.
    pub fn new(kind: Kind, op: impl Into<String>, message: impl Into<String>) -> Self {
        Error {
            kind,
            op: op.into(),
            message: message.into(),
        }
    }

    /// Build a dial error from the IO failure that caused it. The target
    /// stays on the error so the caller never has to re-derive which hop
    /// failed from a bare IO sentence.
    pub fn dial(op: impl Into<String>, source: io::Error) -> Self {
        Error {
            kind: Kind::Dial,
            op: op.into(),
            message: source.to_string(),
        }
    }

    /// Build a pump error from the write failure that stopped it. A pump
    /// that cannot write has nowhere to put the bytes, so this is always
    /// loud and never a half-close.
    pub fn pump(source: io::Error) -> Self {
        Error {
            kind: Kind::Pump,
            op: "pump".to_string(),
            message: source.to_string(),
        }
    }

    /// Build a server error from the IO failure that caused it.
    pub fn server(op: impl Into<String>, source: io::Error) -> Self {
        Error {
            kind: Kind::Server,
            op: op.into(),
            message: source.to_string(),
        }
    }

    /// What kind of failure this is.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The process exit code this failure maps to.
    pub fn exit_code(&self) -> i32 {
        EXIT_RUNTIME_ERROR
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "podbox-ssh: {}: {} [{}]",
            self.op,
            self.message,
            self.kind.token()
        )
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_operation_and_the_kind() {
        let e = Error::new(Kind::Dial, "dial tcp 127.0.0.1:22", "connection refused");
        let s = e.to_string();
        assert!(s.contains("dial tcp 127.0.0.1:22"), "{s}");
        assert!(s.contains("connection refused"), "{s}");
        assert!(s.contains("dial"), "{s}");
    }

    #[test]
    fn every_kind_maps_to_the_runtime_exit_code() {
        for kind in [Kind::Usage, Kind::Dial, Kind::Server, Kind::Pump] {
            let e = Error::new(kind, "test", "test failure");
            assert_eq!(e.exit_code(), EXIT_RUNTIME_ERROR, "{kind:?}");
            assert_eq!(e.kind(), kind);
        }
        assert_eq!(EXIT_RUNTIME_ERROR, 125);
    }

    #[test]
    fn kind_tokens_are_stable() {
        assert_eq!(Kind::Usage.token(), "usage");
        assert_eq!(Kind::Dial.token(), "dial");
        assert_eq!(Kind::Server.token(), "server");
        assert_eq!(Kind::Pump.token(), "pump");
    }

    #[test]
    fn dial_keeps_the_io_sentence_beside_the_target() {
        let source = io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused");
        let e = Error::dial("dial tcp 127.0.0.1:22", source);
        assert_eq!(e.kind(), Kind::Dial);
        assert!(e.to_string().contains("127.0.0.1:22"), "{e}");
    }

    #[test]
    fn pump_and_server_conversions_keep_their_kind() {
        let e = Error::pump(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"));
        assert_eq!(e.kind(), Kind::Pump);
        let e = Error::server(
            "probe sshd",
            io::Error::new(io::ErrorKind::NotFound, "no sshd"),
        );
        assert_eq!(e.kind(), Kind::Server);
        assert!(e.to_string().contains("probe sshd"), "{e}");
    }
}
