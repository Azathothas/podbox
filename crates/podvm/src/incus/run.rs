//! `run --podvm-tier=incus`: one command in an incus system container,
//! TODO/machine.md T-1718, in the form of `docs/incus.md`. The calls of the
//! incus REST API are `pod-incus` of PodUtils/podcrates (T-1727).
//!
//! 1. The image: podvm's extracted rootfs as an incus unified tarball
//!    ([`super::image`]) with podvm itself at `/.podvm/podvm`, imported
//!    one time for each manifest digest and podvm build under the alias
//!    `podvm-HEX16-PODVM16-vN` (TODO/machine.md T-1722).
//! 2. The instance: from that alias, with podvm itself as its init
//!    (`lxc.init.cmd = /.podvm/podvm guest-hold`), so an image with no
//!    `sleep` and no shell still starts.
//! 3. The command: the exec API with `record-output`, so stdout and stderr
//!    come back apart with the exit code. Piped stdin goes in as a file.
//! 4. The end: the instance is deleted, as under `--rm`.
//!
//! A `-v HOST:GUEST[:ro]` is a `disk` device of the instance
//! ([`disk_devices`], TODO/machine.md T-1722): incusd maps the host
//! directory into the instance, so a write lands on the host.
//!
//! The command is not PID 1 here (the hold is); the parity note says so.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use pod_incus::{
    Client, Exec, FileWrite, ImageData, ImageImport, NewInstance, Payload, Settings, Source,
};
use podvm_image::error::EXIT_RUNTIME_ERROR;

use super::image::{alias, unified_tarball};

pub struct Job<'a> {
    pub verb: &'a str,
    pub store_root: &'a Path,
    pub rootfs: &'a Path,
    pub digest: &'a str,
    pub argv: Vec<String>,
    pub env: Vec<String>,
    pub workdir: String,
    pub stdin: Option<Vec<u8>>,
    /// `-v HOST:GUEST[:ro]`; `true` is read-only.
    pub volumes: Vec<(String, String, bool)>,
    /// `run -d` and `create`: an instance that stays (T-1722).
    pub detach: Option<super::verbs::Detach>,
    /// `--podvm-net none` and `--podvm-channel` ([`super::net`], T-1779).
    pub net_none: bool,
    pub channel: Option<(PathBuf, u16)>,
}

/// The devices of an instance of `job`: the volumes, the mask of the NIC
/// and the channel.
pub fn job_devices(job: &Job<'_>) -> pod_incus::Devices {
    let mut d = disk_devices(&job.volumes);
    d.extend(super::net::devices(
        job.net_none,
        &["eth0".to_string()],
        job.channel.as_ref(),
    ));
    d
}

/// One `disk` device for each volume, named `podvm-volN`.
pub fn disk_devices(volumes: &[(String, String, bool)]) -> pod_incus::Devices {
    volumes
        .iter()
        .enumerate()
        .map(|(i, (host, guest, ro))| {
            let mut d = BTreeMap::new();
            d.insert("type".to_string(), "disk".to_string());
            d.insert("source".to_string(), host.clone());
            d.insert("path".to_string(), guest.clone());
            if *ro {
                d.insert("readonly".to_string(), "true".to_string());
            }
            (format!("podvm-vol{i}"), d)
        })
        .collect()
}

/// The socket of an incusd: `PODVM_INCUS_SOCKET`, else `INCUS_DIR`'s, else
/// the system daemon's.
pub fn socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os("PODVM_INCUS_SOCKET") {
        return PathBuf::from(p);
    }
    if let Some(d) = std::env::var_os("INCUS_DIR") {
        return PathBuf::from(d).join("unix.socket");
    }
    PathBuf::from("/var/lib/incus/unix.socket")
}

/// The alias of the image in incus: imported now where it is not there.
async fn ensure_image(c: &Client, job: &Job<'_>) -> Result<(String, bool), String> {
    let me = std::fs::read("/proc/self/exe").map_err(|e| format!("/proc/self/exe: {e}"))?;
    let name = alias(job.digest, &crate::machine::guest::kernel::sha256_hex(&me));
    if c.alias(&name).await.map_err(|e| e.to_string())?.is_some() {
        return Ok((name, true));
    }
    let dir = job.store_root.join("incus");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tarball: PathBuf = dir.join(format!("{name}.tar"));
    unified_tarball(job.rootfs, &tarball, &me)?;
    let file = tokio::fs::File::open(&tarball)
        .await
        .map_err(|e| format!("{}: {e}", tarball.display()))?;
    let length = file.metadata().await.map(|m| m.len()).ok();
    let mut import = ImageImport::new(ImageData::Unified(Payload::new(file, length)));
    import.aliases = vec![name.clone()];
    let done = c.import_image(import).await;
    let _ = std::fs::remove_file(&tarball);
    done.map_err(|e| e.to_string())?;
    Ok((name, false))
}

/// A file or a directory in the instance, owned by 0 with `mode`.
fn owned(mut w: FileWrite, mode: u32) -> FileWrite {
    w.uid = Some(0);
    w.gid = Some(0);
    w.mode = Some(mode);
    w
}

/// The environment as incus takes it.
pub fn env_map(env: &[String]) -> BTreeMap<String, String> {
    env.iter()
        .filter_map(|e| e.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
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
    let fail = |e: String| {
        eprintln!("podvm {}: incus tier: {e}", job.verb);
        EXIT_RUNTIME_ERROR
    };
    let sock = socket_path();
    if !sock.exists() {
        return fail(format!(
            "no incusd answers at {}: set PODVM_INCUS_SOCKET or INCUS_DIR, or start one \
             (docs/incus.md, route A)",
            sock.display()
        ));
    }
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return fail(format!("the runtime: {e}")),
    };
    rt.block_on(run_async(job, sock))
}

async fn run_async(job: &Job<'_>, sock: PathBuf) -> i32 {
    let fail = |e: String| {
        eprintln!("podvm {}: incus tier: {e}", job.verb);
        EXIT_RUNTIME_ERROR
    };
    let mut settings = Settings::new(sock.clone());
    settings.user_agent = format!("podvm/{}", env!("CARGO_PKG_VERSION"));
    let c = Client::new(settings);
    let server = match c.server().await {
        Ok(s) => s,
        Err(e) => return fail(e.to_string()),
    };
    if let Err(e) = server.check() {
        return fail(e.to_string());
    }
    let (image, cached) = match ensure_image(&c, job).await {
        Ok(a) => a,
        Err(e) => return fail(e),
    };
    eprintln!(
        "podvm {}: incus tier: incusd {} at {}, image {image} {} (TODO/machine.md T-1718)",
        job.verb,
        server.environment.server_version,
        sock.display(),
        if cached { "from incus" } else { "imported now" }
    );
    if let Some(d) = &job.detach {
        return super::verbs::detached(&c, job, &image, d).await;
    }
    let nonce =
        crate::machine::guest::init::nonce().unwrap_or_else(|_| format!("{}", std::process::id()));
    let inst = format!("podvm-{}", &nonce[..nonce.len().min(12)]);
    let code = run_in(&c, job, &image, &inst).await;
    // As under --rm: the instance goes, whatever the command gave.
    let _ = c
        .stop(&inst, true, Some(std::time::Duration::from_secs(10)))
        .await;
    let _ = c.delete_instance(&inst).await;
    code
}

async fn run_in(c: &Client, job: &Job<'_>, image: &str, inst: &str) -> i32 {
    let fail = |e: String| {
        eprintln!("podvm {}: incus tier: {e}", job.verb);
        EXIT_RUNTIME_ERROR
    };
    let mut new = NewInstance::container(inst, Source::Alias(image.to_string()));
    new.devices = job_devices(job);
    new.config.insert(
        "raw.lxc".into(),
        "lxc.init.cmd = /.podvm/podvm guest-hold".into(),
    );
    if let Err(e) = super::net::create(c, &new, job.net_none, job.channel.as_ref()).await {
        return fail(e);
    }
    // podvm is in the image (T-1722): a run pushes only its stdin.
    let mut pushes = Vec::new();
    if let Some(b) = &job.stdin {
        pushes.push((
            super::STDIN,
            owned(FileWrite::file(Payload::bytes(b.clone())), 0o600),
        ));
    }
    for (path, w) in pushes {
        if let Err(e) = c.put_file(inst, path, w).await {
            return fail(e.to_string());
        }
    }
    if let Err(e) = super::net::push_port(c, inst, job.channel.as_ref()).await {
        return fail(e);
    }
    if let Err(e) = c.start(inst).await {
        return fail(e.to_string());
    }
    if let Err(e) = super::net::wait_ready(c, inst, job.channel.as_ref()).await {
        return fail(e);
    }
    let mut exec = Exec::new(command(&job.argv, job.stdin.is_some()));
    exec.environment = env_map(&job.env);
    exec.cwd = job.workdir.clone();
    let out = match c.exec_recorded(inst, &exec).await {
        Ok(o) => o,
        Err(e) => return fail(e.to_string()),
    };
    let _ = std::io::stdout().write_all(&out.stdout);
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().write_all(&out.stderr);
    out.exit_code
}
