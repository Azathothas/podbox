//! The findings of the probe for `run` and `create`, TODO/probe.md T-1746
//! (ask V-2 of podswarm): from the cache of T-0111 where its key and the
//! build match, and from a probe now where they do not.

use podvm_image::probe_cache::{resolve, Source};

/// The findings, and, where `PODVM_DEBUG` is set, one line that says where
/// they came from.
pub(super) fn findings(verb: &str, store: &podvm_image::Store) -> podvm_probe::Findings {
    let say = |l: &str| {
        if std::env::var_os("PODVM_DEBUG").is_some() {
            eprintln!("podvm {verb}: the probe {l} (TODO/probe.md T-1746)");
        }
    };
    let probe = resolve(store);
    match probe.findings() {
        Some((f, _)) => {
            match &probe.source {
                Source::Cache => say("came from the cache"),
                Source::Measured(why) => say(&format!("ran now: {why}")),
            }
            f
        }
        None => {
            say("ran now: the cache holds no findings to read");
            podvm_probe::run()
        }
    }
}
