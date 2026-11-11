//! The lifecycle verbs: milestone M4.
//!
//! [`TODO/milestones.md`](../../../TODO/milestones.md) T-1105,
//! [`TODO/supervise.md`](../../../TODO/supervise.md) T-0601 to T-0607.
//!
//! ⭐ **`create`, `start`, `ps`, `logs`, `stop`, `kill`, `wait`, `rm`, `cp`,
//! and `exec` and `inspect` against a container.** Every one of them reads the
//! container table rather than the filesystem, and every wait in them is on a
//! condition with an upper bound.
//!
//! ⛔ **Nothing here sleeps and nothing polls.** T-0602: the prior art's own
//! capture of this lifecycle contains a failed run because it decided "running"
//! by sleeping three seconds and looking, and M4 is accepted on twenty
//! consecutive passes precisely because one pass proves nothing about a race.

use std::io::Write;

use podvm_image::error::{
    EXIT_CANNOT_INVOKE, EXIT_CLI_ERROR, EXIT_FLAG_ERROR, EXIT_NOT_FOUND, EXIT_RUNTIME_ERROR,
};
use podvm_image::platform::Platform;
use podvm_image::transport::Policy;
use podvm_supervise::table::{Container, State};

use crate::format;

/// docker's default grace period between `SIGTERM` and `SIGKILL`.
const STOP_GRACE_MS: u64 = 10_000;
/// ⛔ `podvm wait`'s bound. `TODO/RULES.md` section 8: a runtime whose audience
/// is automated may not wait unbounded, so this is long rather than absent and
/// reaching it is its own reported outcome.
const WAIT_BOUND_MS: u64 = 3_600_000;

pub const PS_USAGE: &str = "\
usage: podvm ps [-a|--all] [-q|--quiet] [--format T] [--no-trunc]
       [--filter key=value]

  Fields: .ID .Names .Image .Command .CreatedAt .Status .State .Ports .Pid

  --filter takes name=<substring>, matched against the name (or the id
  prefix), and label=<key>[=<value>], which matches nothing: podvm
  records carry no labels. Repeatable; every filter must match.

  refused: .Ports is always empty and is not an oversight: the payload shares this
    machine's network namespace, so there is nothing to publish.
";

pub const CONTAINER_INSPECT_FIELDS: &[&str] = &[
    "Id",
    "Name",
    "Image",
    "State",
    "Status",
    "Pid",
    "LauncherPid",
    "ExitCode",
    "Created",
    "StartedAt",
    "FinishedAt",
    "RootfsPath",
    "LogPath",
    "Rung",
    "Complete.Fixups",
    "Complete.Degraded",
    "Command",
    "Exec.Mode",
    "Exec.Shares",
    "Contains",
    "Noticed",
    "End.Reason",
    "End.RunMs",
    "End.UserMs",
    "End.SystemMs",
    "End.MaxRssKiB",
    "End.Residual",
    "Interpose.Emulated.mknod",
    "Interpose.Emulated.mount",
    "Interpose.Emulated.unshare",
    "Interpose.Emulated.clone",
    "Interpose.Emulated.procfs",
];

fn store() -> Result<podvm_image::Store, i32> {
    podvm_image::open_store().map_err(|e| {
        eprintln!("podvm: {e}");
        e.exit_code()
    })
}

fn fail(verb: &str, e: podvm_supervise::Error) -> i32 {
    eprintln!("podvm {verb}: {e}");
    EXIT_RUNTIME_ERROR
}

/// `podvm create [options] <image> [command...]`, and the shared half of
/// `podvm run -d`.
pub fn create(args: &[String]) -> i32 {
    let s = match store() {
        Ok(s) => s,
        Err(c) => return c,
    };
    match crate::run::prepare(args, &s, "create") {
        Ok(p) => {
            // ⭐ TODO/enter.md T-1411. The detached drive rides beside the
            // record: the loader argv where `prepare` decided the loader
            // family, empty everywhere else (chroot records, and the
            // driverless memfd family, which `prepare` refused already).
            // The libraries, the `PODVM_MAPS` table and
            // `PODVM_GUEST_EXE` ride in `p.env`, decided beside it.
            match podvm_supervise::create(
                &s,
                p.name.as_deref(),
                &p.image,
                &p.record.manifest_digest,
                &p.rootfs,
                p.argv,
                p.env,
                p.working_dir,
                &p.rung,
                p.completion.clone(),
                p.completion_degraded,
                detached_drive(p.userland.as_ref()),
            ) {
                Ok(c) => {
                    let kept = podvm_supervise::set_wall(&s, &c.id, p.wall_ms).and_then(|_| {
                        podvm_supervise::set_hardening(&s, &c.id, p.hardening.clone())
                    });
                    if let Err(e) = kept {
                        let _ = podvm_supervise::remove(&s, &c.id, true);
                        let _ = std::fs::remove_file(&p.memo_host_path);
                        return fail("create", e);
                    }
                    // ⭐ T-0710: the ephemeral memo becomes the container's,
                    // beside its record. The stored environment already carries
                    // the constant descriptor number.
                    let dest = podvm_supervise::table::memo_path(&s, &c.id);
                    if let Err(e) = std::fs::rename(&p.memo_host_path, &dest) {
                        eprintln!("podvm create: the ownership memo could not be stored: {e}");
                        let _ = podvm_supervise::remove(&s, &c.id, true);
                        return podvm_image::error::EXIT_RUNTIME_ERROR;
                    }
                    println!("{}", c.id);
                    0
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&p.memo_host_path);
                    fail("create", e)
                }
            }
        }
        Err(code) => code,
    }
}

/// `podvm start <container>...`
pub fn start(args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded = match crate::parity::expand("start", args) {
        Ok(a) => a,
        Err(member) => {
            return crate::parity::refuse_member(
                "start",
                &member,
                "usage: podvm start <container> [container...]",
            )
        }
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(
        "start",
        args,
        "usage: podvm start <container> [container...]",
    ) {
        return c;
    }
    let s = match store() {
        Ok(s) => s,
        Err(c) => return c,
    };
    let mut code = 0;
    let mut any = false;
    for want in args {
        if want == "-h" || want == "--help" {
            println!("usage: podvm start <container> [container...]");
            return 0;
        }
        if want.starts_with('-') {
            if let Err(c) = crate::parity::admit(
                "start",
                want,
                "usage: podvm start <container> [container...]",
            ) {
                return c;
            }
            return crate::parity::no_arm("start", want);
        }
        any = true;
        if let Err(c) = start_one(&s, "start", want) {
            code = c;
        }
    }
    if !any {
        println!("usage: podvm start <container> [container...]");
        return EXIT_CLI_ERROR;
    }
    code
}

/// Start one container: the per-name half of `start`, shared with
/// `restart` (TODO/cli.md T-1331). `verb` names the caller in every
/// message, because a restart's start-half failure must not read as a
/// `start` failure.
fn start_one(s: &podvm_image::Store, verb: &str, want: &str) -> Result<String, i32> {
    let c = match podvm_supervise::get(s, want) {
        Ok(c) => c,
        Err(e) => return Err(fail(verb, e)),
    };
    // ⚠ The image record is needed so the LAUNCHER can hold the image lock
    // for the container's whole life. A lock this process took would go
    // with this process, which exits as soon as the container is up.
    let record = match s.find_one(&c.image) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("podvm {verb}: {e}");
            return Err(EXIT_RUNTIME_ERROR);
        }
    };
    // ⭐ TODO/milestones.md T-1112. A container created before the OS gate
    // (or by hand in the store) still names its image's OS here rather
    // than inside the guest.
    ensure_linux_guest(verb, &record.os, &record.architecture)?;
    // ⭐ TODO/enter.md T-1411. This reverses T-1317's last-resort decision
    // at this gate: the launcher now drives the userland loader family
    // detached (`spawn_userland` in podvm-supervise, with pidfd
    // supervision as today), so a record carrying that drive starts
    // instead of refusing here. A record without one genuinely blocks
    // detached operation, and the refusal names the call with the
    // affected verbs.
    let findings = podvm_probe::run();
    if !podvm_probe::probes::chroot_usable(&findings) && c.userland_exec.is_empty() {
        eprintln!(
            "podvm {verb}: chroot(2) is denied on this machine and container {} carries \
             no userland loader drive in its record (rung {:?}): `run -d`, `create` and \
             `start` drive the loader family detached through \
             podvm_enter::spawn_userland and refuse anything else at this gate; recreate \
             it where the foreground enters userland, or run foreground \
             (TODO/enter.md T-1411)",
            c.name, c.rung
        );
        return Err(EXIT_RUNTIME_ERROR);
    }
    match podvm_supervise::start(s, want, &record) {
        Ok(c) => {
            println!("{}", c.name);
            Ok(c.name)
        }
        Err(e) => Err(fail(verb, e)),
    }
}

/// `podvm ps`
pub fn ps(args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission, so
    // `ps -aq` reads as `-a -q` everywhere below.
    let expanded = match crate::parity::expand("ps", args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member("ps", &member, PS_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all("ps", args, PS_USAGE) {
        return c;
    }
    let mut all = false;
    let mut quiet = false;
    let mut no_trunc = false;
    let mut template: Option<String> = None;
    let mut filters: Vec<crate::parity::Filter> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{PS_USAGE}");
                return 0;
            }
            "-a" | "--all" => all = true,
            "-q" | "--quiet" => quiet = true,
            "--no-trunc" => no_trunc = true,
            "--format" => match it.next() {
                Some(t) => template = Some(t.clone()),
                None => {
                    eprintln!("podvm ps: --format needs a template");
                    return EXIT_FLAG_ERROR;
                }
            },
            other if other.starts_with("--format=") => {
                template = Some(other["--format=".len()..].to_string())
            }
            "-f" | "--filter" => match it.next() {
                Some(f) => match crate::parity::parse_filter(f) {
                    Ok(p) => filters.push(p),
                    Err(bad) => {
                        eprintln!("podvm ps: {bad}");
                        return EXIT_FLAG_ERROR;
                    }
                },
                None => {
                    eprintln!("podvm ps: --filter needs a key=value");
                    return EXIT_FLAG_ERROR;
                }
            },
            other if other.starts_with("--filter=") => {
                match crate::parity::parse_filter(&other["--filter=".len()..]) {
                    Ok(p) => filters.push(p),
                    Err(bad) => {
                        eprintln!("podvm ps: {bad}");
                        return EXIT_FLAG_ERROR;
                    }
                }
            }
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit("ps", other, PS_USAGE) {
                    return c;
                }
                return crate::parity::no_arm("ps", other);
            }
            other => {
                eprintln!("podvm ps: unknown option {other:?}");
                eprint!("{PS_USAGE}");
                return EXIT_FLAG_ERROR;
            }
        }
    }
    let fields = ps_field_names();
    // ⛔ Checked before anything is read, exactly as `images --format` is: a
    // template validated only inside the loop over results is never checked at
    // all when there are none, and a caller's typo then reads as "no containers".
    if let Some(t) = &template
        && let Err(bad) = format::check(t, &fields)
    {
        eprintln!("podvm ps: {bad}");
        return EXIT_CLI_ERROR;
    }
    let s = match store() {
        Ok(s) => s,
        Err(c) => return c,
    };
    let list = match podvm_supervise::list(&s, all) {
        Ok(l) => l,
        Err(e) => return fail("ps", e),
    };
    // ⭐ TODO/cli.md T-1331. Caller-side predicate over the listed
    // records: every `--filter` must match (docker ANDs them).
    let list: Vec<_> = list
        .into_iter()
        .filter(|c| {
            filters.iter().all(|f| match f.key.as_str() {
                "name" => c.name.contains(&f.value) || c.id.starts_with(&f.value),
                _ => false,
            })
        })
        .collect();
    if quiet {
        for c in &list {
            println!("{}", if no_trunc { c.id.clone() } else { c.short_id() });
        }
        return 0;
    }
    if let Some(t) = &template {
        for c in &list {
            match format::render(t, &ps_fields(c, no_trunc)) {
                Ok(line) => println!("{line}"),
                Err(bad) => {
                    eprintln!("podvm ps: {bad}");
                    return EXIT_CLI_ERROR;
                }
            }
        }
        return 0;
    }
    let mut rows = vec![vec![
        "CONTAINER ID".to_string(),
        "IMAGE".into(),
        "COMMAND".into(),
        "CREATED".into(),
        "STATUS".into(),
        "NAMES".into(),
    ]];
    for c in &list {
        rows.push(vec![
            if no_trunc { c.id.clone() } else { c.short_id() },
            c.image.clone(),
            command_of(c),
            c.created_at.clone(),
            c.status(),
            c.name.clone(),
        ]);
    }
    print!("{}", crate::images::table(&rows));
    0
}

mod logs_stop;
pub use logs_stop::*;

mod wait_rm_cp;
pub use wait_rm_cp::*;

mod copy_tree;
pub use copy_tree::*;

mod entry;
pub use entry::*;

mod userland;
pub use userland::*;

mod script;

mod native;

pub mod volume;

#[cfg(test)]
mod tests;
