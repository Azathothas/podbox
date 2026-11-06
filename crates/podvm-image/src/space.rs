//! `TODO/image.md` T-0203: check blocks **and inodes**, before any download and
//! again before any extraction, and name the destination in the message.
//!
//! `pod_hostprobe::space` makes the checks. This module gives it podvm's
//! settings, so each message names `$PODVM_STORE`, and turns its failures
//! into this crate's errors: no space or an unknown free space is
//! [`Error::NoSpace`], a path that cannot reach the kernel is
//! [`Error::Store`].

use pod_core::{Class, Failure};
use pod_hostprobe::space as host;

pub use pod_hostprobe::space::{choose, dir_bytes, file_ceiling, mib, Candidate, Free, Have};

use crate::error::{Error, Result};

/// What one filesystem has left.
pub fn read(path: &str) -> Result<Have> {
    host::read(path).map_err(host_error)
}

/// ⛔ The precheck. Both halves, and the message names the destination, the
/// free amount, the required amount and the unit.
///
/// `headroom_bytes` is what is asked for beyond the payload, so a store is not
/// filled to its last block by a pull that then leaves nothing for the
/// extraction M2 performs.
pub fn require(
    destination: &str,
    what: &str,
    need_bytes: u64,
    need_inodes: u64,
    headroom_bytes: u64,
) -> Result<Have> {
    let settings = podvm_probe::settings();
    host::require(
        &settings,
        destination,
        what,
        need_bytes,
        need_inodes,
        headroom_bytes,
    )
    .map_err(host_error)
}

/// A failure of `pod_hostprobe` (its stage is `Host`) as this crate's error.
/// An I/O failure (the exec probe) keeps its kind and its path; of the
/// others, no space or an unknown free space is `NoSpace`, and the rest
/// (a path with a NUL) is `Store`.
pub(crate) fn host_error(f: Failure) -> Error {
    let io = std::error::Error::source(&f).and_then(|e| e.downcast_ref::<std::io::Error>());
    match (io, f.class) {
        (Some(e), _) => {
            let cause = format!(": {e}");
            let path = f.text().strip_suffix(&cause).unwrap_or(f.text());
            Error::io(path, std::io::Error::new(e.kind(), e.to_string()))
        }
        (None, Class::Unavailable) => Error::NoSpace(f.chain()),
        (None, _) => Error::Store(f.chain()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_requirement_over_the_free_space_is_no_space_naming_the_store_setting() {
        let e = require("/tmp", "alpine:latest", u64::MAX / 4, 1, 0).unwrap_err();
        assert!(matches!(e, Error::NoSpace(_)), "{e:?}");
        let text = format!("{e}");
        assert!(
            text.contains("/tmp") && text.contains("$PODVM_STORE"),
            "{text}"
        );
    }

    #[test]
    fn a_path_with_a_nul_is_a_store_error() {
        let e = read("/tmp/a\0b").unwrap_err();
        assert!(matches!(e, Error::Store(_)), "{e:?}");
    }

    #[test]
    fn an_unknown_free_space_is_no_space_naming_the_errno() {
        let e = read("/proc/self/no-such-directory").unwrap_err();
        assert!(matches!(e, Error::NoSpace(_)), "{e:?}");
        assert!(format!("{e}").contains("ENOENT"), "{e}");
    }
}
