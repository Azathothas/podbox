//! The two interposer objects as bytes, and which one a payload may load.
//!
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-0702. `build.rs` puts
//! them in `OUT_DIR`, or puts two empty files there when
//! `scripts/build-interpose.sh` has not run.
//!
//! ⛔ **ONE OBJECT PER LIBC, AND THE CHOICE IS NOT A PREFERENCE.** A preloaded
//! object is loaded by the payload's own dynamic loader and resolves its
//! imports against the payload's libc. `experiments/results/interposer-abi.txt`
//! measured both directions: a musl-linked object into a glibc payload dies at
//! `/lib/x86_64-linux-gnu/libc.so: invalid ELF header`, before a symbol is
//! read, because musl's libc declares no SONAME and the glibc path of that name
//! is a linker script. The reverse dies on a missing `__snprintf_chk`.
//!
//! ⚠ **THIS MODULE DOES NOT PLACE THE OBJECT OR SET `LD_PRELOAD`.** That is the
//! other half of T-0702 and it is not done: the object has to be written INSIDE
//! the rootfs before the chroot, because an absolute path from outside it does
//! not resolve inside it. The entry carries what is left.

/// Which libc an object is linked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Libc {
    Gnu,
    Musl,
}

/// The glibc-linked object, empty where it was not built.
pub const GNU: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/interpose-gnu.so"));

/// The musl-linked object, empty where it was not built.
pub const MUSL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/interpose-musl.so"));

/// The object for `libc`, or `None` where this binary carries none.
///
/// ⛔ `None` is a fact and not an error. A podbox built on a machine with no
/// zig carries no musl object, and the caller's job is then to decline with a
/// named reason rather than to preload something that cannot load.
pub fn object(libc: Libc) -> Option<&'static [u8]> {
    let bytes = match libc {
        Libc::Gnu => GNU,
        Libc::Musl => MUSL,
    };
    (!bytes.is_empty()).then_some(bytes)
}

/// One line naming what this binary carries, for `podbox system info`.
pub fn carried() -> String {
    match (object(Libc::Gnu).is_some(), object(Libc::Musl).is_some()) {
        (true, true) => "interposer: glibc and musl objects embedded".into(),
        (true, false) => "interposer: the glibc object only. A musl payload is declined".into(),
        (false, true) => "interposer: the musl object only. A glibc payload is declined".into(),
        (false, false) => "interposer: none embedded, so every payload is declined. \
             ./scripts/build-interpose.sh builds them"
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐ **The assertion that would have caught two glibc objects**, which is
    /// the failure `scripts/build-interpose.sh` records as its own reason for
    /// existing: before it asserted `DT_NEEDED`, the build exited 0 having
    /// produced the wrong object twice.
    ///
    /// ⚠ It reads the bytes rather than running `readelf`, so it holds inside
    /// `cargo test` with no external tool. The dynamic string table carries the
    /// `DT_NEEDED` names verbatim, and the two spellings are the discriminator:
    /// musl's libc declares no SONAME, so its object names `libc.so`, and a
    /// glibc one names `libc.so.6`.
    ///
    /// ⛔ **It SKIPS rather than fails where nothing is embedded.** A machine
    /// with no zig builds podbox and must still pass its tests; T-0706's
    /// channel is what reports the absence to a caller at run time.
    #[test]
    fn each_embedded_object_names_its_own_libc() {
        fn has(bytes: &[u8], needle: &str) -> bool {
            bytes.windows(needle.len()).any(|w| w == needle.as_bytes())
        }
        if let Some(gnu) = object(Libc::Gnu) {
            assert!(
                has(gnu, "libc.so.6"),
                "the glibc object does not name libc.so.6, so it is not glibc-linked"
            );
        }
        if let Some(musl) = object(Libc::Musl) {
            assert!(has(musl, "libc.so"), "the musl object names no libc at all");
            // ⚠ An exact spelling, because `libc.so.6` CONTAINS `libc.so`. The
            // same trap `scripts/build-interpose.sh` names, one layer up.
            assert!(
                !has(musl, "libc.so.6"),
                "the musl object names libc.so.6, so it is a SECOND GLIBC \
                 OBJECT and the whole one-object-per-libc requirement is \
                 inverted (TODO/interpose.md T-0702)"
            );
        }
    }

    /// ⚠ The two objects are different artefacts. Embedding the same bytes
    /// twice would pass the check above for the glibc half and read as success.
    #[test]
    fn the_two_embedded_objects_are_not_the_same_bytes() {
        if let (Some(gnu), Some(musl)) = (object(Libc::Gnu), object(Libc::Musl)) {
            assert_ne!(gnu, musl, "one object is embedded under both names");
        }
    }

    /// ⛔ The line a caller reads has to say which state this binary is in,
    /// including the state where it carries nothing.
    #[test]
    fn the_carried_line_names_the_state_it_is_in() {
        let line = carried();
        assert!(line.starts_with("interposer: "), "{line}");
        match (object(Libc::Gnu).is_some(), object(Libc::Musl).is_some()) {
            (true, true) => assert!(line.contains("glibc and musl"), "{line}"),
            (false, false) => assert!(line.contains("declined"), "{line}"),
            _ => assert!(line.contains("declined"), "{line}"),
        }
    }
}
