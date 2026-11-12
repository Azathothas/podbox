//! The verbs of an incus instance that stays, TODO/machine.md T-1722.
//!
//! incusd keeps the state of the instance; podvm keeps no record of it.
//! A record of podvm's container table belongs to a launcher, and
//! `reconcile` calls a running record with no launcher dead, so a second
//! copy of the state would lie. An instance of podvm has the label
//! [`LABEL`] and the image of the caller in [`IMAGE_KEY`].
//!
//! * `run -d` and `create` make the instance with podvm as its init
//!   (`guest-hold --run`) and the command in `/.podvm/cmd`
//!   ([`super::Cmd`]); `run -d` starts it. The command's output goes to
//!   `/.podvm/log` and its exit code to `/.podvm/exit`; the instance stops
//!   when the command ends.
//! * `ps`, `start`, `stop`, `rm`, `logs` and `exec` with
//!   `--podvm-tier=incus` call the API on the instance of that name.

use std::io::Write;

use pod_incus::{Client, Exec, FileContent, FileWrite, NewInstance, Payload, Settings, Source};
use podvm_image::error::EXIT_RUNTIME_ERROR;

use super::run::{disk_devices, socket_path, Job};

/// The label of an instance of podvm.
pub const LABEL: &str = "user.podvm";
/// The image as the caller named it.
pub const IMAGE_KEY: &str = "user.podvm.image";
/// The init of an instance that stays.
pub const INIT: &str = "lxc.init.cmd = /.podvm/podvm guest-hold --run";

/// `run -d` (start) or `create` (no start), with the name of `--name`.
#[derive(Debug, Clone)]
pub struct Detach {
    pub name: Option<String>,
    pub start: bool,
    pub image: String,
}

/// The arguments without `--podvm-tier=incus` (or `--podvm-tier incus`),
/// where they hold it.
pub fn wants(args: &[String]) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut found = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--podvm-tier=incus" {
            found = true;
        } else if args[i] == "--podvm-tier" && args.get(i + 1).is_some_and(|v| v == "incus") {
            found = true;
            i += 1;
        } else {
            out.push(args[i].clone());
        }
        i += 1;
    }
    found.then_some(out)
}

/// A valid incus name: a letter first, then letters, digits and `-`, at
/// most 63.
pub fn valid_name(n: &str) -> bool {
    n.len() <= 63
        && n.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// A client of the incusd of [`socket_path`], checked.
pub(super) async fn connect() -> Result<(Client, String), String> {
    let sock = socket_path();
    if !sock.exists() {
        return Err(format!(
            "no incusd answers at {}: set PODVM_INCUS_SOCKET or INCUS_DIR, or start one \
             (docs/incus.md, route A)",
            sock.display()
        ));
    }
    let mut settings = Settings::new(sock.clone());
    settings.user_agent = format!("podvm/{}", env!("CARGO_PKG_VERSION"));
    let c = Client::new(settings);
    let server = c.server().await.map_err(|e| e.to_string())?;
    server.check().map_err(|e| e.to_string())?;
    Ok((c, server.environment.server_version.clone()))
}

fn block_on<F: std::future::Future<Output = i32>>(verb: &str, f: F) -> i32 {
    match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt.block_on(f),
        Err(e) => {
            eprintln!("podvm {verb}: incus tier: the runtime: {e}");
            EXIT_RUNTIME_ERROR
        }
    }
}

/// Make (and for `run -d` start) the instance of `job`; print its name.
pub(super) async fn detached(c: &Client, job: &Job<'_>, image: &str, d: &Detach) -> i32 {
    let fail = |e: String| {
        eprintln!("podvm {}: incus tier: {e}", job.verb);
        EXIT_RUNTIME_ERROR
    };
    let name = match &d.name {
        Some(n) if valid_name(n) => n.clone(),
        Some(n) => {
            eprintln!(
                "podvm {}: --name {n:?}: incus takes a letter first, then letters, digits \
                 and '-', at most 63 (TODO/machine.md T-1722)",
                job.verb
            );
            return podvm_image::error::EXIT_FLAG_ERROR;
        }
        None => {
            let nonce = crate::machine::guest::init::nonce()
                .unwrap_or_else(|_| format!("{}", std::process::id()));
            format!("podvm-{}", &nonce[..nonce.len().min(12)])
        }
    };
    let mut new = NewInstance::container(&name, Source::Alias(image.to_string()));
    new.devices = disk_devices(&job.volumes);
    new.config.insert("raw.lxc".into(), INIT.into());
    new.config.insert(LABEL.into(), "1".into());
    new.config.insert(IMAGE_KEY.into(), d.image.clone());
    if let Err(e) = c.create_instance(&new).await {
        return fail(e.to_string());
    }
    let cmd = super::Cmd {
        argv: job.argv.clone(),
        env: job.env.clone(),
        cwd: job.workdir.clone(),
    };
    let mut w = FileWrite::file(Payload::bytes(cmd.to_json()));
    w.uid = Some(0);
    w.gid = Some(0);
    w.mode = Some(0o600);
    if let Err(e) = c.put_file(&name, super::CMD, w).await {
        let _ = c.delete_instance(&name).await;
        return fail(e.to_string());
    }
    if d.start
        && let Err(e) = c.start(&name).await
    {
        return fail(e.to_string());
    }
    println!("{name}");
    0
}

/// `ps`, `start`, `stop`, `rm` and `logs` with `--podvm-tier=incus`.
pub fn dispatch(verb: &str, args: &[String]) -> i32 {
    let flags: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| a.starts_with('-'))
        .collect();
    let names: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();
    let known: &[&str] = match verb {
        "ps" => &["-a", "--all"],
        "rm" => &["-f", "--force"],
        _ => &[],
    };
    if let Some(f) = flags.iter().find(|f| !known.contains(f)) {
        eprintln!(
            "podvm {verb}: {f} is not served with --podvm-tier=incus (TODO/machine.md T-1722)"
        );
        return podvm_image::error::EXIT_FLAG_ERROR;
    }
    if verb != "ps" && names.is_empty() {
        eprintln!("podvm {verb}: name an instance");
        return podvm_image::error::EXIT_FLAG_ERROR;
    }
    let all = flags.iter().any(|f| *f == "-a" || *f == "--all");
    let force = flags.iter().any(|f| *f == "-f" || *f == "--force");
    block_on(verb, async move {
        let c = match connect().await {
            Ok((c, _)) => c,
            Err(e) => {
                eprintln!("podvm {verb}: incus tier: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        };
        if verb == "ps" {
            return ps(&c, all).await;
        }
        let mut code = 0;
        for n in &names {
            if let Err(e) = one(&c, verb, n, force).await {
                eprintln!("podvm {verb}: {n}: {e}");
                code = 1;
            }
        }
        code
    })
}

/// Whether `inst` is an instance of podvm.
fn ours(i: &pod_incus::Instance) -> bool {
    i.config.get(LABEL).is_some_and(|v| v == "1")
}

async fn ps(c: &Client, all: bool) -> i32 {
    let list = match c.instances().await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("podvm ps: incus tier: {e}");
            return EXIT_RUNTIME_ERROR;
        }
    };
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{:<24} {:<10} IMAGE", "NAMES", "STATUS");
    for i in list.iter().filter(|i| ours(i)) {
        if all || i.status == "Running" {
            let image = i.config.get(IMAGE_KEY).map(String::as_str).unwrap_or("-");
            let _ = writeln!(out, "{:<24} {:<10} {image}", i.name, i.status);
        }
    }
    0
}

async fn one(c: &Client, verb: &str, name: &str, force: bool) -> Result<(), String> {
    let i = c.instance(name).await.map_err(|e| e.to_string())?;
    if !ours(&i) {
        return Err("not an instance of podvm (no user.podvm label)".into());
    }
    let running = i.status == "Running";
    match verb {
        "start" if !running => {
            c.start(name).await.map_err(|e| e.to_string())?;
        }
        "stop" if running => {
            c.stop(name, false, Some(std::time::Duration::from_secs(10)))
                .await
                .map_err(|e| e.to_string())?;
        }
        "rm" => {
            if running && !force {
                return Err("the instance runs: stop it, or rm -f".into());
            }
            if running {
                c.stop(name, true, Some(std::time::Duration::from_secs(10)))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            c.delete_instance(name).await.map_err(|e| e.to_string())?;
        }
        "logs" => {
            let f = c
                .get_file(name, super::LOG)
                .await
                .map_err(|e| e.to_string())?;
            if let FileContent::Bytes(b) = f.content {
                let _ = std::io::stdout().write_all(&b);
            }
            return Ok(());
        }
        "start" | "stop" => {}
        _ => return Err(format!("{verb} is not a verb of the incus tier")),
    }
    if verb != "logs" {
        println!("{name}");
    }
    Ok(())
}

/// `exec --podvm-tier=incus NAME CMD...`: the command in the instance, its
/// output and its exit code.
pub fn exec(name: &str, argv: &[String], env: &[String], cwd: Option<&str>) -> i32 {
    block_on("exec", async move {
        let c = match connect().await {
            Ok((c, _)) => c,
            Err(e) => {
                eprintln!("podvm exec: incus tier: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        };
        match c.instance(name).await {
            Ok(i) if ours(&i) && i.status == "Running" => {}
            Ok(i) if ours(&i) => {
                eprintln!("podvm exec: {name}: the instance is {}", i.status);
                return EXIT_RUNTIME_ERROR;
            }
            Ok(_) => {
                eprintln!("podvm exec: {name}: not an instance of podvm");
                return EXIT_RUNTIME_ERROR;
            }
            Err(e) => {
                eprintln!("podvm exec: {name}: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        }
        let mut x = Exec::new(argv.to_vec());
        x.environment = super::run::env_map(env);
        x.cwd = cwd.unwrap_or("").to_string();
        match c.exec_recorded(name, &x).await {
            Ok(o) => {
                let _ = std::io::stdout().write_all(&o.stdout);
                let _ = std::io::stdout().flush();
                let _ = std::io::stderr().write_all(&o.stderr);
                o.exit_code
            }
            Err(e) => {
                eprintln!("podvm exec: {name}: {e}");
                EXIT_RUNTIME_ERROR
            }
        }
    })
}
