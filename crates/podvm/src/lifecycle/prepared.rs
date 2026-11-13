//! The prepared image of a run ([`Prepared`]) and the JSON of `inspect` of a container.

use super::*;

pub(super) fn container_json(s: &podvm_image::Store, c: &Container) -> String {
    let fields = container_fields(s, c);
    let mut o = serde_json::Map::new();
    for (k, v) in &fields {
        // ⚠ The nested pair is nested here too, so a caller reading the
        // document and one reading `--format` do not find two shapes.
        if let Some((group, rest)) = k
            .strip_prefix("Exec.")
            .map(|r| ("Exec", r))
            .or_else(|| k.strip_prefix("End.").map(|r| ("End", r)))
        {
            let e = o
                .entry(group.to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let Some(m) = e.as_object_mut() {
                m.insert(rest.to_string(), serde_json::Value::String(v.clone()));
            }
            continue;
        }
        o.insert(k.to_string(), serde_json::Value::String(v.clone()));
    }
    serde_json::Value::Array(vec![serde_json::Value::Object(o)]).to_string()
}

/// The image `run -d` and `create` need, and everything derived from it.
pub struct Prepared {
    pub record: podvm_image::Record,
    pub image: String,
    pub rootfs: String,
    pub argv: Vec<String>,
    pub env: Vec<String>,
    pub working_dir: String,
    pub name: Option<String>,
    pub rung: String,
    pub detach: bool,
    pub rm: bool,
    /// `--timeout` in milliseconds: the wall bound of the run, or `None`.
    /// TODO/cli.md T-1655.
    pub wall_ms: Option<u64>,
    /// `--ulimit` and `--security-opt no-new-privileges`, for the child.
    pub hardening: podvm_supervise::table::Hardening,
    /// ⭐ T-0710: the host memo file this entry hands the payload, beside the
    /// container record once the container exists. For `prepare` itself this is
    /// an ephemeral file under staging; `create` and `run -d` move it to
    /// `containers/<id>/ownership.memo`, foreground `run` deletes it on exit.
    /// The environment already carries `PODVM_MEMO_FD` for it.
    pub memo_host_path: std::path::PathBuf,
    /// ⭐ TODO/packaging.md T-1003. The forced launch rung `prepare` admitted,
    /// driven by foreground `run` and by nothing else. None is the default
    /// chroot-by-path entry.
    pub ladder: Option<podvm_enter::ladder::Mode>,
    /// The probe rows the entry was decided under. `run` feeds them to the
    /// ladder; `create` and `run -d` carry them unused, because neither
    /// drives a rung.
    pub findings: podvm_probe::Findings,
    /// ⭐ T-0804 rule 3: `inspect` reports the TRUE mode per container, and a
    /// shimmed `/dev/null` is part of that mode. One line per fixup that
    /// changed a byte or failed, carried into the container record.
    pub completion: Vec<String>,
    pub completion_degraded: usize,
    /// ⭐ TODO/enter.md T-1317. The no-chroot entry foreground `run` takes
    /// where chroot is denied. None is the chroot sequence, as before.
    pub userland: Option<Userland>,
    /// `--podvm-confine`, guest paths (TODO/enter.md T-1658).
    pub confine: Option<podvm_enter::confine::Spec>,
}
