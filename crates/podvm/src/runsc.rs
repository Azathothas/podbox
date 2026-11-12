//! `run --podvm-tier=runsc`: gVisor's `runsc` runs the payload where the
//! host grants user namespaces (TODO/enter.md T-1729).
//!
//! gVisor's Sentry answers each system call of the payload in its own
//! kernel, so `uname -r` names gVisor. podvm writes an OCI bundle around
//! the container's own tree (TODO/image.md T-1709) and starts
//! `runsc --rootless run`. A `-v HOST:GUEST` volume is a bind mount of the
//! bundle, so a write lands on the host. The files are under
//! `<store>/runsc/`: one state root for the store (`runsc` keeps its own
//! files there, a mounted `null-netns` among them), and a bundle for each
//! run, which podvm removes at the end with the container's state
//! (`runsc delete --force`), as under `--rm`.
//!
//! The tier is optional and never a fallback: a caller asks for it by name,
//! and where the host has no `runsc` or denies user namespaces, the tier
//! refuses by name with exit 125.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use podvm_image::error::EXIT_RUNTIME_ERROR;

/// The variable that names the `runsc` program; else `runsc` on `PATH`.
pub const VAR: &str = "PODVM_RUNSC";

/// The platform of the Sentry: `systrap` needs no ptrace and no KVM.
pub const PLATFORM: &str = "systrap";

/// One run of the tier.
pub struct Job<'a> {
    pub verb: &'a str,
    pub store_root: &'a Path,
    /// The container's own tree: podvm removes it at the end.
    pub rootfs: &'a Path,
    pub argv: Vec<String>,
    pub env: Vec<String>,
    pub workdir: String,
    pub stdin: Option<Vec<u8>>,
    /// `-v HOST:GUEST`, and the guest paths of the `:ro` ones.
    pub volumes: &'a [podvm_enter::userland::Volume],
    pub ro: &'a [String],
    /// `--read-only`: the root of the bundle is read-only.
    pub read_only: bool,
    /// `--user UID[:GID]`, numeric only.
    pub user: (u32, u32),
}

/// The `runsc` program: [`VAR`], else `runsc` on `PATH`.
pub fn binary() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os(VAR) {
        let p = PathBuf::from(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!("{VAR}={} names no file", p.display()))
        };
    }
    crate::tier::which_qemu("runsc").ok_or_else(|| {
        format!("no runsc on PATH: install gVisor's runsc, or set {VAR} to its path")
    })
}

/// Ok where this process can make a user namespace: rootless `runsc`
/// needs one. The check runs `unshare(CLONE_NEWUSER)` in a child.
pub fn user_namespaces() -> Result<(), String> {
    let r = podvm_probe::inside::checks::escalation()
        .into_iter()
        .find(|r| r.target == "unshare-user");
    match r {
        Some(r) if r.verdict == podvm_probe::inside::Verdict::Allow => Ok(()),
        Some(r) => Err(format!(
            "this host denies user namespaces (unshare(CLONE_NEWUSER): {}), and rootless \
             runsc needs one",
            r.detail
        )),
        None => Err("the check of user namespaces gave no answer".into()),
    }
}

/// `--user` as the tier takes it: `UID` or `UID:GID`, numeric.
pub fn numeric_user(user: Option<&str>) -> Result<(u32, u32), String> {
    let Some(u) = user else {
        return Ok((0, 0));
    };
    let (a, b) = u.split_once(':').unwrap_or((u, u));
    match (a.parse::<u32>(), b.parse::<u32>()) {
        (Ok(uid), Ok(gid)) => Ok((uid, gid)),
        _ => Err(format!(
            "--user {u:?}: the runsc tier takes a numeric UID or UID:GID"
        )),
    }
}

/// The OCI configuration of the bundle.
pub fn spec(job: &Job) -> serde_json::Value {
    use serde_json::json;
    let mut mounts = vec![
        json!({"destination": "/proc", "type": "proc", "source": "proc"}),
        json!({"destination": "/dev", "type": "tmpfs", "source": "tmpfs"}),
        json!({"destination": "/sys", "type": "sysfs", "source": "sysfs",
               "options": ["nosuid", "noexec", "nodev", "ro"]}),
    ];
    for v in job.volumes {
        let mode = if job.ro.iter().any(|g| g == &v.guest) {
            "ro"
        } else {
            "rw"
        };
        mounts.push(
            json!({"destination": v.guest, "type": "bind", "source": v.host,
                           "options": ["rbind", mode]}),
        );
    }
    json!({
        "ociVersion": "1.0.0",
        "process": {
            "terminal": false,
            "user": {"uid": job.user.0, "gid": job.user.1},
            "args": job.argv,
            "env": job.env,
            "cwd": job.workdir,
            "rlimits": [{"type": "RLIMIT_NOFILE", "hard": 1024, "soft": 1024}],
        },
        "root": {"path": job.rootfs, "readonly": job.read_only},
        "hostname": "podvm",
        "mounts": mounts,
        "linux": {"namespaces": [{"type": "pid"}, {"type": "network"}, {"type": "ipc"},
                                 {"type": "uts"}, {"type": "mount"}]},
    })
}

/// The arguments of `runsc` for one run. The network is the host's, as on
/// the other rungs of podvm; the root writes go to the container's own
/// tree (no overlay), and the tree goes at the end.
pub fn args(state: &Path, bundle: &Path, id: &str) -> Vec<String> {
    vec![
        "--rootless".into(),
        "--network=host".into(),
        "--ignore-cgroups".into(),
        format!("--platform={PLATFORM}"),
        "--overlay2=none".into(),
        format!("--root={}", state.display()),
        "run".into(),
        format!("--bundle={}", bundle.display()),
        id.into(),
    ]
}

/// The first line of `runsc --version`.
fn version(runsc: &Path) -> String {
    Command::new(runsc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .map(str::to_string)
        })
        .unwrap_or_else(|| "-".into())
}

/// Run the job; the exit code of the payload, or 125.
pub fn run(job: &Job) -> i32 {
    let fail = |e: String| {
        eprintln!(
            "podvm {}: --podvm-tier=runsc: {e} (TODO/enter.md T-1729)",
            job.verb
        );
        EXIT_RUNTIME_ERROR
    };
    let runsc = match binary() {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    if let Err(e) = user_namespaces() {
        return fail(e);
    }
    let id = format!("podvm-{}", std::process::id());
    let state = job.store_root.join("runsc").join("state");
    let bundle = job.store_root.join("runsc").join(&id);
    let code = (|| {
        std::fs::create_dir_all(&state).map_err(|e| format!("{}: {e}", state.display()))?;
        std::fs::create_dir_all(&bundle).map_err(|e| format!("{}: {e}", bundle.display()))?;
        let cfg = bundle.join("config.json");
        let text = serde_json::to_vec_pretty(&spec(job)).map_err(|e| e.to_string())?;
        std::fs::write(&cfg, text).map_err(|e| format!("{}: {e}", cfg.display()))?;
        eprintln!(
            "podvm {}: entering on the runsc tier: {} ({}) answers each system call of the \
             payload in its own kernel, rootless with platform {PLATFORM}; the network is \
             the host's, and each -v is a bind mount (TODO/enter.md T-1729)",
            job.verb,
            version(&runsc),
            runsc.display()
        );
        let mut cmd = Command::new(&runsc);
        cmd.args(args(&state, &bundle, &id));
        if job.stdin.is_some() {
            cmd.stdin(Stdio::piped());
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("{}: {e}", runsc.display()))?;
        if let (Some(b), Some(mut w)) = (&job.stdin, child.stdin.take()) {
            let _ = w.write_all(b);
        }
        let st = child.wait().map_err(|e| e.to_string())?;
        Ok::<i32, String>(st.code().unwrap_or(EXIT_RUNTIME_ERROR))
    })();
    // As under --rm: the state of the container and the bundle go.
    let _ = Command::new(&runsc)
        .args(["--rootless", &format!("--root={}", state.display())])
        .args(["delete", "--force", &id])
        .stderr(Stdio::null())
        .status();
    let _ = std::fs::remove_dir_all(&bundle);
    match code {
        Ok(c) => c,
        Err(e) => fail(e),
    }
}

#[cfg(test)]
#[path = "runsc_tests.rs"]
mod tests;
