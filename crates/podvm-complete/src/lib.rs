//! `TOOL.md` section 6.4: environment completion. Milestone M5,
//! [`TODO/milestones.md`](../../../TODO/milestones.md) T-1106 and
//! [`TODO/complete.md`](../../../TODO/complete.md) T-0401 to T-0415.
//!
//! ⭐ The completion itself is `pod-rootfs` of PodUtils/podcrates: the writes
//! held in the root, the device shims, the resolver, hosts and mtab, passwd,
//! group and nsswitch, the walls of pacman, apt and zypper, the CA bundle and
//! the CA directory, and the `http://` package sources. This crate gives it
//! podvm's values and keeps podvm's own parts:
//!
//! - [`Options`], from podvm's flags;
//! - [`names`]: each name that podvm writes into a root. ⛔ A second run finds
//!   the files of the first by these names, so they keep podvm's values;
//! - the CA bundle of this machine, from `$SSL_CERT_FILE`, `$CURL_CA_BUNDLE`,
//!   `$REQUESTS_CA_BUNDLE` and the paths ([`podvm_image::tls`]);
//! - the probe of a mirror over podvm's transport ([`reach`]);
//! - the record ids of podvm ([`entry`]), the banner and the JSON of
//!   [`Report`], which `--strict`, `inspect` and the container record read.
//!
//! The three rules of the completion hold: each write goes through the
//! containment of the crate, each fixup is a row, and a fixup never changes
//! the exit code of the payload (T-0409 and T-0802).

#![forbid(unsafe_op_in_unsafe_fn)]

pub mod reach;
pub mod shm;

use std::fmt;
use std::path::Path;

pub use pod_rootfs::{Action, Libc};

/// Where the originals of the package sources live while a fixup is
/// applied: beside the rootfs, never inside it (T-0411).
pub const BACKUP_DIR: &str = ".complete-orig";

/// ⚠ One error type, and it carries a sentence rather than a code.
#[derive(Debug)]
pub enum Error {
    Complete(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Complete(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// What the caller asked for. ⛔ Defaults are the conservative ones.
#[derive(Debug, Clone)]
pub struct Options {
    /// The container's name, for `/etc/hosts`. T-0403.
    pub container_name: Option<String>,
    /// `--add-host name:ip`, repeatable. T-0403.
    pub add_hosts: Vec<String>,
    /// T-0411. `--no-source-fixup` clears it, and then any rewrite a previous
    /// run made is **undone**.
    pub source_fixup: bool,
    /// T-0407. `--no-host-cas` clears it.
    pub host_cas: bool,
    /// Where the host's resolver is read from.
    pub host_resolv_conf: String,
    /// T-0412. `--no-steps` clears it.
    pub steps: bool,
    /// T-1762. The file of the store that caches the verdicts of the HTTPS
    /// probe across runs; `None` probes each host.
    pub reach_cache: Option<std::path::PathBuf>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            container_name: None,
            add_hosts: Vec::new(),
            source_fixup: true,
            host_cas: true,
            host_resolv_conf: "/etc/resolv.conf".to_string(),
            steps: true,
            reach_cache: None,
        }
    }
}

/// podvm's names in a root. ⛔ The values of every release before the move
/// to `pod-rootfs`, so a second pass finds the files of the first.
pub fn names() -> pod_rootfs::Names {
    pod_rootfs::Names {
        tool: "podvm".to_string(),
        comment: "# podvm (T-0406): ".to_string(),
        apt_drop_in: "etc/apt/apt.conf.d/99podvm".to_string(),
        host_ca_marker: "# podvm (TODO/complete.md T-0407): this machine's own CA bundle"
            .to_string(),
        host_ca_prefix: "podvm-host-ca-".to_string(),
        scratch_prefix: ".podvm-mknod-".to_string(),
    }
}

/// The entry of podvm's record that owns a fixup id of `pod-rootfs`.
pub fn entry(id: &str) -> &'static str {
    match id {
        "dev-node" | "dev-shim" => "T-0401",
        "resolv.conf" => "T-0402",
        "hosts" => "T-0403",
        "identity" => "T-0404",
        "mtab" => "T-0405",
        "pacman" | "pacman-keyring" => "T-0406",
        "apt" | "apt-sandbox" | "ca-bundle" => "T-0407",
        "zypper" | "zypper-index" => "T-0408",
        "nsswitch" => "T-0410",
        "source-scheme" | "source-reach" => "T-0411",
        "ca-hash-dir" => "T-0412",
        "dev-fd-link" => "T-0413",
        shm::ID => "T-1759",
        _ => "-",
    }
}

/// One line of the completion record.
#[derive(Debug, Clone)]
pub struct Fixup {
    /// The entry that owns it, so a reader can find the reasoning.
    pub entry: &'static str,
    /// A stable, greppable id.
    pub id: &'static str,
    /// Rootfs-relative, with no leading slash, or `-`.
    pub path: String,
    pub action: Action,
    pub detail: String,
    /// ⭐ T-0804: `--strict` refuses the run when any of these is set.
    pub degraded: bool,
}

/// A command podvm runs **inside** the rootfs before the payload (T-0412).
/// ⛔ An argv, never a shell line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub entry: &'static str,
    pub id: &'static str,
    pub argv: Vec<String>,
    pub why: String,
}

/// Everything the completion layer did to one rootfs.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub fixups: Vec<Fixup>,
    pub steps: Vec<Step>,
    pub libc: Libc,
}

impl From<pod_rootfs::Report> for Report {
    fn from(r: pod_rootfs::Report) -> Report {
        Report {
            fixups: r
                .fixups
                .into_iter()
                .map(|f| Fixup {
                    entry: entry(f.id),
                    id: f.id,
                    path: f.path,
                    action: f.action,
                    detail: f.detail,
                    degraded: f.degraded,
                })
                .collect(),
            steps: r
                .steps
                .into_iter()
                .map(|s| Step {
                    entry: entry(s.id),
                    id: s.id,
                    argv: s.argv,
                    why: s.why,
                })
                .collect(),
            libc: r.libc,
        }
    }
}

impl Report {
    /// The banner half. ⛔ One line per fixup that CHANGED something or
    /// failed, and one summary line, so the absence of edits is stated.
    pub fn banner(&self) -> String {
        let mut s = String::new();
        for f in &self.fixups {
            if !f.action.changed() && f.action != Action::Failed {
                continue;
            }
            s.push_str(&format!(
                "podvm: complete: {} {} ({}, {}){}\n",
                f.action.word(),
                if f.path.is_empty() { "-" } else { &f.path },
                f.id,
                f.entry,
                if f.detail.is_empty() {
                    String::new()
                } else {
                    format!(": {}", f.detail)
                }
            ));
        }
        // ⭐ T-0412 rule 3: each step is NAMED HERE, before the caller runs it.
        for (i, st) in self.steps.iter().enumerate() {
            s.push_str(&format!(
                "podvm: complete: step {} of {}: `{}` ({}, {}): {}\n",
                i + 1,
                self.steps.len(),
                st.argv.join(" "),
                st.id,
                st.entry,
                st.why
            ));
        }
        let changed = self.fixups.iter().filter(|f| f.action.changed()).count();
        let failed = self
            .fixups
            .iter()
            .filter(|f| f.action == Action::Failed)
            .count();
        s.push_str(&format!(
            "podvm: complete: {} fixup(s) applied to this image's rootfs, {} \
             degraded, {} failed, {} step(s) to run inside it; libc={}. refused: These \
             are edits podvm made inside somebody else's image and the payload \
             can see them\n",
            changed,
            self.degradations().len(),
            failed,
            self.steps.len(),
            self.libc.word()
        ));
        s
    }

    /// ⭐ T-0804's input. A `Skipped` row did nothing, so it is not one.
    pub fn degradations(&self) -> Vec<&Fixup> {
        self.fixups
            .iter()
            .filter(|f| f.degraded && f.action != Action::Skipped)
            .collect()
    }

    /// The record as JSON, for `inspect`. ⚠ The field order is the contract.
    pub fn json(&self) -> String {
        let rows: Vec<serde_json::Value> = self
            .fixups
            .iter()
            .map(|f| {
                serde_json::json!({
                    "entry": f.entry,
                    "id": f.id,
                    "path": f.path,
                    "action": f.action.word(),
                    "detail": f.detail,
                    "degraded": f.degraded,
                })
            })
            .collect();
        let steps: Vec<serde_json::Value> = self
            .steps
            .iter()
            .map(|s| {
                serde_json::json!({
                    "entry": s.entry,
                    "id": s.id,
                    "argv": s.argv,
                    "why": s.why,
                })
            })
            .collect();
        serde_json::json!({ "libc": self.libc.word(), "fixups": rows, "steps": steps }).to_string()
    }
}

/// The settings of `pod-rootfs` for one run of podvm.
pub fn settings(rootfs: &str, opts: &Options) -> pod_rootfs::Settings {
    pod_rootfs::Settings {
        names: names(),
        host: podvm_probe::settings(),
        container_name: opts.container_name.clone(),
        add_hosts: opts.add_hosts.clone(),
        source_fixup: opts.source_fixup,
        host_cas: opts.host_cas,
        steps: opts.steps,
        // ⭐ T-0407: `announced_by` names the variable, and only an announced
        // bundle is appended to the image's own trust store.
        host_bundle: podvm_image::tls::host_bundle_bytes().map(|h| pod_rootfs::HostBundle {
            path: h.path,
            announced_by: h.announced_by,
            bytes: h.bytes,
        }),
        host_resolv_conf: opts.host_resolv_conf.clone().into(),
        mounts: "/proc/self/mounts".into(),
        originals: Some(
            Path::new(rootfs)
                .parent()
                .unwrap_or_else(|| Path::new("/"))
                .join(BACKUP_DIR),
        ),
    }
}

/// Prepare `rootfs` for a payload, and say what was done.
///
/// ⛔ **Never fails the run on a fixup that could not be applied.** The
/// failure is a `Failed` row, the banner names it, and `--strict` is how a
/// caller turns it into a refusal.
pub fn complete(rootfs: &str, opts: &Options) -> Result<Report> {
    let mut probe = reach::cached(opts.reach_cache.clone());
    let mut r = pod_rootfs::complete(Path::new(rootfs), &settings(rootfs, opts), &mut probe)
        .map(Report::from)
        .map_err(|e| Error::Complete(e.to_string()))?;
    r.fixups.push(shm::dev_shm(Path::new(rootfs)));
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_that_changed_nothing_still_says_so() {
        let r = Report::default();
        let b = r.banner();
        assert!(b.contains("0 fixup(s) applied"), "{b}");
    }

    /// ⛔ Each id of the crate has an entry of podvm's record: a banner row
    /// with `-` in place of the entry is a row nobody can trace.
    #[test]
    fn each_id_of_the_crate_has_its_entry() {
        for id in pod_rootfs::IDS {
            assert_ne!(entry(id), "-", "{id}");
        }
    }

    /// ⛔ podvm's names pass the crate's own check, and a second pass finds
    /// the files of the first by them. The originals stay beside the rootfs.
    #[test]
    fn podvm_names_are_the_names_of_earlier_releases() {
        let n = names();
        assert_eq!(n.apt_drop_in, "etc/apt/apt.conf.d/99podvm");
        assert_eq!(n.host_ca_prefix, "podvm-host-ca-");
        let s = settings("/store/rootfs/x/rootfs", &Options::default());
        assert_eq!(
            s.originals.as_deref(),
            Some(Path::new("/store/rootfs/x/.complete-orig"))
        );
    }
}
