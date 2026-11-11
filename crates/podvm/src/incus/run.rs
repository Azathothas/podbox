//! `run --podvm-tier=incus`: one command in an incus system container,
//! TODO/machine.md T-1718, in the form of `docs/incus.md`.
//!
//! 1. The image: podvm's extracted rootfs as an incus unified tarball
//!    ([`super::image`]), imported one time for each manifest digest under
//!    the alias `podvm-HEX16-vN`.
//! 2. The instance: from that alias, with podvm itself as its init
//!    (`lxc.init.cmd = /.podvm/podvm guest-hold`), so an image with no
//!    `sleep` and no shell still starts. podvm pushes itself before the
//!    start.
//! 3. The command: the exec API with `record-output`, so stdout and stderr
//!    come back apart with the exit code. Piped stdin goes in as a file.
//! 4. The end: the instance is deleted, as under `--rm`.
//!
//! The command is not PID 1 here (the hold is); the parity note says so.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::client::{metadata, Client};
use super::image::{alias, unified_tarball};
use podvm_image::error::EXIT_RUNTIME_ERROR;

pub struct Job<'a> {
    pub verb: &'a str,
    pub store_root: &'a Path,
    pub rootfs: &'a Path,
    pub digest: &'a str,
    pub argv: Vec<String>,
    pub env: Vec<String>,
    pub workdir: String,
    pub stdin: Option<Vec<u8>>,
}

/// The alias of the image in incus: imported now where it is not there.
fn ensure_image(c: &Client, job: &Job) -> Result<(String, bool), String> {
    let name = alias(job.digest);
    if c.get(&format!("/1.0/images/aliases/{name}")).is_ok() {
        return Ok((name, true));
    }
    let dir = job.store_root.join("incus");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tarball: PathBuf = dir.join(format!("{name}.tar"));
    unified_tarball(job.rootfs, &tarball)?;
    let bytes = std::fs::read(&tarball).map_err(|e| format!("{}: {e}", tarball.display()))?;
    let r = c.call(
        "POST",
        "/1.0/images",
        Some(("application/octet-stream", &bytes)),
        &[],
    )?;
    let done = c.finish(&r);
    let _ = std::fs::remove_file(&tarball);
    let fp = done?
        .get("fingerprint")
        .and_then(Value::as_str)
        .ok_or("incus: the image import gave no fingerprint")?
        .to_string();
    c.send(
        "POST",
        "/1.0/images/aliases",
        &json!({ "name": name, "target": fp }),
    )?;
    Ok((name, false))
}

/// Push `bytes` to `path` in the instance, owned by 0 with `mode`.
fn push(
    c: &Client,
    inst: &str,
    path: &str,
    bytes: &[u8],
    mode: &str,
    kind: &str,
) -> Result<(), String> {
    let headers = [
        "X-Incus-uid: 0".to_string(),
        "X-Incus-gid: 0".to_string(),
        format!("X-Incus-mode: {mode}"),
        format!("X-Incus-type: {kind}"),
    ];
    let r = c.call(
        "POST",
        &format!("/1.0/instances/{inst}/files?path={path}"),
        Some(("application/octet-stream", bytes)),
        &headers,
    )?;
    metadata(&r).map(|_| ())
}

/// The environment as incus takes it.
pub fn env_map(env: &[String]) -> Value {
    let mut m = serde_json::Map::new();
    for e in env {
        if let Some((k, v)) = e.split_once('=') {
            m.insert(k.to_string(), Value::String(v.to_string()));
        }
    }
    Value::Object(m)
}

/// The command as it runs: the argv, or, with stdin, through `/bin/sh`
/// with the stdin file.
pub fn command(argv: &[String], stdin: bool) -> Vec<String> {
    if !stdin {
        return argv.to_vec();
    }
    let mut v = vec![
        "/bin/sh".to_string(),
        "-c".to_string(),
        "exec \"$@\" </.podvm/stdin".to_string(),
        "sh".to_string(),
    ];
    v.extend(argv.iter().cloned());
    v
}

pub fn run(job: &Job) -> i32 {
    let sock = super::client::socket_path();
    let fail = |e: String| {
        eprintln!("podvm {}: incus tier: {e}", job.verb);
        EXIT_RUNTIME_ERROR
    };
    if !sock.exists() {
        return fail(format!(
            "no incusd answers at {}: set PODVM_INCUS_SOCKET or INCUS_DIR, or start one \
             (docs/incus.md, route A)",
            sock.display()
        ));
    }
    let c = Client::new(&sock);
    let server = match c.get("/1.0") {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let version = server
        .pointer("/environment/server_version")
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_string();
    let (image, cached) = match ensure_image(&c, job) {
        Ok(a) => a,
        Err(e) => return fail(e),
    };
    eprintln!(
        "podvm {}: incus tier: incusd {version} at {}, image {image} {} (TODO/machine.md T-1718)",
        job.verb,
        sock.display(),
        if cached { "from incus" } else { "imported now" }
    );
    let nonce =
        crate::machine::guest::init::nonce().unwrap_or_else(|_| format!("{}", std::process::id()));
    let inst = format!("podvm-{}", &nonce[..nonce.len().min(12)]);
    let code = run_in(&c, job, &image, &inst);
    // As under --rm: the instance goes, whatever the command gave.
    let _ = c.send(
        "PUT",
        &format!("/1.0/instances/{inst}/state"),
        &json!({ "action": "stop", "force": true, "timeout": 10 }),
    );
    let _ = c.finish(
        &match c.call("DELETE", &format!("/1.0/instances/{inst}"), None, &[]) {
            Ok(r) => r,
            Err(_) => return code,
        },
    );
    code
}

fn run_in(c: &Client, job: &Job, image: &str, inst: &str) -> i32 {
    let fail = |e: String| {
        eprintln!("podvm {}: incus tier: {e}", job.verb);
        EXIT_RUNTIME_ERROR
    };
    let create = json!({
        "name": inst,
        "source": { "type": "image", "alias": image },
        "config": { "raw.lxc": "lxc.init.cmd = /.podvm/podvm guest-hold" },
    });
    if let Err(e) = c.send("POST", "/1.0/instances", &create) {
        return fail(e);
    }
    let me = match std::fs::read("/proc/self/exe") {
        Ok(b) => b,
        Err(e) => return fail(format!("/proc/self/exe: {e}")),
    };
    let pushed = push(c, inst, "/.podvm", &[], "0755", "directory")
        .and_then(|()| push(c, inst, "/.podvm/podvm", &me, "0755", "file"))
        .and_then(|()| match &job.stdin {
            Some(b) => push(c, inst, "/.podvm/stdin", b, "0600", "file"),
            None => Ok(()),
        });
    if let Err(e) = pushed {
        return fail(e);
    }
    let start = json!({ "action": "start", "timeout": 30 });
    if let Err(e) = c.send("PUT", &format!("/1.0/instances/{inst}/state"), &start) {
        return fail(e);
    }
    let exec = json!({
        "command": command(&job.argv, job.stdin.is_some()),
        "environment": env_map(&job.env),
        "cwd": job.workdir,
        "wait-for-websocket": false,
        "record-output": true,
        "interactive": false,
    });
    let m = match c.send("POST", &format!("/1.0/instances/{inst}/exec"), &exec) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    for (fd, mut out) in [
        ("1", Box::new(std::io::stdout()) as Box<dyn Write>),
        ("2", Box::new(std::io::stderr()) as Box<dyn Write>),
    ] {
        if let Some(path) = m.pointer(&format!("/output/{fd}")).and_then(Value::as_str)
            && let Ok(r) = c.call("GET", path, None, &[])
        {
            let _ = out.write_all(&r.body);
            let _ = out.flush();
        }
    }
    m.get("return")
        .and_then(Value::as_i64)
        .map(|c| c as i32)
        .unwrap_or(EXIT_RUNTIME_ERROR)
}
