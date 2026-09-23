//! Build provenance: the inputs that produced this binary.
//!
//! [`TODO/packaging.md`](../../../TODO/packaging.md) T-1004. A single-file
//! artefact whose inputs are not recorded cannot be traced back to what
//! produced it. `crates/podbox-cli/build.rs` records each input as a
//! `cargo::rustc-env` value; this module reads them back and renders the
//! `version --verbose` document. A missing input renders as `unknown`:
//! a blank gets checked, a plausible value gets used.
//!
//! ⛔ Nothing here shells out and nothing here probes at run time. The
//! record is fixed at build time, which is what makes two binaries
//! comparable.

/// One field of the record. Every field is a build-time string; `bool` is
/// not used because `env!` only yields strings and a second spelling of
/// one value is how two reports of it disagree.
pub struct BuildInfo {
    /// `CARGO_PKG_VERSION`: the artefact's own version.
    pub version: &'static str,
    /// The git commit built, with `-dirty` where the tree was modified,
    /// or `unknown` where no commit was readable.
    pub commit: &'static str,
    /// `rustc --version` of the building toolchain, or `unknown`.
    pub rustc: &'static str,
    /// The `TARGET` triple built for.
    pub target: &'static str,
    /// Hex sha256 of the embedded glibc interposer object, or `absent`
    /// where `build.rs` embedded the placeholder.
    pub interpose_gnu: &'static str,
    /// Hex sha256 of the embedded musl interposer object, or `absent`.
    pub interpose_musl: &'static str,
    /// `yes` where the binary links `crt-static`, else `no`.
    pub crt_static: &'static str,
}

/// Read the record `build.rs` wrote. Every value is fixed at compile time,
/// so this never fails: the worst case is `unknown`, never an error.
pub fn info() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        commit: env!("PODBOX_BUILD_COMMIT"),
        rustc: env!("PODBOX_BUILD_RUSTC"),
        target: env!("PODBOX_BUILD_TARGET"),
        interpose_gnu: env!("PODBOX_INTERPOSE_GNU"),
        interpose_musl: env!("PODBOX_INTERPOSE_MUSL"),
        crt_static: env!("PODBOX_BUILD_CRT_STATIC"),
    }
}

impl BuildInfo {
    /// The `version --verbose` document, one `name: value` line per input,
    /// in the order [`BuildInfo`] declares them. A parser reads it by
    /// name, never by position.
    pub fn render_verbose(&self) -> String {
        format!(
            "version: {}\ncommit: {}\nrustc: {}\ntarget: {}\ninterpose-gnu: {}\ninterpose-musl: {}\ncrt-static: {}\n",
            self.version,
            self.commit,
            self.rustc,
            self.target,
            self.interpose_gnu,
            self.interpose_musl,
            self.crt_static,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unknown() -> BuildInfo {
        BuildInfo {
            version: "0.1.0",
            commit: "unknown",
            rustc: "unknown",
            target: "unknown",
            interpose_gnu: "absent",
            interpose_musl: "absent",
            crt_static: "unknown",
        }
    }

    /// T-1004 names six inputs; the document carries one line per input so
    /// a client can report the binary it ran.
    #[test]
    fn verbose_names_every_recorded_input() {
        let doc = unknown().render_verbose();
        for field in [
            "version",
            "commit",
            "rustc",
            "target",
            "interpose-gnu",
            "interpose-musl",
            "crt-static",
        ] {
            assert!(
                doc.lines().any(|l| l.starts_with(field)),
                "no {field:?} line in:\n{doc}"
            );
        }
    }

    /// ⛔ No fabricated number: what was not recorded renders as a stated
    /// unknown, and no line renders an empty value.
    #[test]
    fn unknown_is_stated_never_blank() {
        let doc = unknown().render_verbose();
        for line in doc.lines() {
            let (_, value) = line
                .split_once(':')
                .unwrap_or_else(|| panic!("no name in {line:?}"));
            assert!(!value.trim().is_empty(), "blank value in {line:?}");
        }
        assert!(doc.contains("unknown"), "no stated unknown in:\n{doc}");
    }

    /// The version line answers what `podbox version` answers: one source
    /// for the number, not two that can drift.
    #[test]
    fn info_matches_the_binary() {
        assert_eq!(info().version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn version_documents_are_plain_ascii() {
        // TODO/cli.md T-1336: `version` and `version --verbose` print
        // bytes 0x00-0x7F only.
        let text = format!("podbox {}\n{}", info().version, unknown().render_verbose());
        let bad = text.bytes().filter(|b| *b > 0x7F).count();
        assert_eq!(bad, 0, "version output carries {bad} non-ASCII bytes");
    }
}
