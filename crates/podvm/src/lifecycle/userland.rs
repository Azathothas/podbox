use super::*;

/// The interposer's opt-in beside [`podvm_enter::ladder::GUEST_EXE_VAR`].
///
/// TODO/enter.md T-1409. Duplicate by value of the interposer's
/// `procfs::GUEST_EXE_FORCE_VAR`: this crate takes no dependency on that
/// object, and the spelling plus the exact value `"1"` is the contract
/// between the two. Set only on loader-family userland entries alongside
/// `PODVM_GUEST_EXE`, so the payload reads its own path from
/// `/proc/self/exe` rather than the loader's.
pub const GUEST_EXE_FORCE_VAR: &str = "PODVM_GUEST_EXE_FORCE";
/// The exact value the interposer answers to. Anything else leaves the
/// host answer in place.
pub const GUEST_EXE_FORCE_ON: &str = "1";

/// TODO/enter.md T-1407 and T-1648. The host-root line of the
/// `--unsafe-host-paths` opt-in, which prints under `-q` too.
pub const HOST_PATHS_LINE: &str = "podvm: userland: running with \
     --unsafe-host-paths: absolute guest paths resolve on the host with no \
     PODVM_MAPS anchoring, and a write lands on the host tree \
     (TODO/enter.md T-1407, T-1648)";

/// What [`userland_env`] decided: the environment the payload starts with,
/// one banner account, and the decided opt-out `--strict` classifies on.
/// The anchoring and the exe-force are banner facts only: the banner names
/// them, and no second value carries them beside it.
#[derive(Debug)]
pub struct UserlandEnv {
    pub env: Vec<String>,
    pub banner: String,
    /// The decided `--unsafe-host-paths` opt-out, which `--strict` refuses
    /// as safety-relevant (TODO/cli.md T-1415). The call sites feed this
    /// decided value to the gate rather than re-reading their own flag.
    pub unsafe_paths: bool,
}

/// The environment a userland entry starts with, in one place.
///
/// TODO/enter.md T-1407 (the root-anchoring `PODVM_MAPS` table) and T-1409
/// (the `PODVM_GUEST_EXE_FORCE` opt-in). One path for `run` and both `exec`
/// entries, so the table, the scrub, and the banner cannot drift between
/// them (`docs/conventions/code.md`).
///
/// A caller-supplied `PODVM_MAPS` is scrubbed first, the way
/// `PODVM_GUEST_EXE` is: without that a nested `podvm run` inherits the
/// outer payload's table. Unless `--unsafe-host-paths` opts into the
/// pass-through, the built table anchors guest `/` inside `rootfs`, and a
/// rootfs the table cannot spell refuses at 125 rather than rewriting
/// nothing (an empty table on this rung is a silent host write).
/// `PODVM_GUEST_EXE_FORCE=1` rides only the loader family beside an
/// already-set `PODVM_GUEST_EXE`; the memfd family never sees it.
///
/// ⛔ TODO/enter.md T-1648. The opt-in's host-root line goes to `err` at
/// once, never into the banner: the quiet default of T-1416 and `-q`
/// hide the banner, and a host pass-through is never silent. The rest
/// of the banner stays under `--verbose`.
pub fn userland_env(
    verb: &str,
    rootfs: &str,
    env: Vec<String>,
    family: podvm_enter::userland::Family,
    unsafe_paths: bool,
    err: &mut dyn Write,
) -> Result<UserlandEnv, i32> {
    use podvm_enter::userland::MAPS_VAR;
    let mut out: Vec<String> = env
        .into_iter()
        .filter(|e| e.split('=').next().unwrap_or("") != MAPS_VAR)
        .collect();
    let mut banner = String::new();
    if unsafe_paths {
        let _ = writeln!(err, "{HOST_PATHS_LINE}");
    } else {
        match podvm_enter::userland::userland_maps_table(rootfs) {
            Ok(table) => {
                out.push(format!("{MAPS_VAR}={table}"));
                banner.push_str(&format!(
                    "podvm: userland: absolute guest paths anchor inside {rootfs} \
                     through {MAPS_VAR}={table} (TODO/enter.md T-1407)\n"
                ));
            }
            Err(why) => {
                eprintln!("podvm {verb}: userland: {why}");
                return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
            }
        }
    }
    if family == podvm_enter::userland::Family::Loader
        && out
            .iter()
            .any(|e| e.split('=').next().unwrap_or("") == podvm_enter::ladder::GUEST_EXE_VAR)
    {
        out.retain(|e| e.split('=').next().unwrap_or("") != GUEST_EXE_FORCE_VAR);
        out.push(format!("{GUEST_EXE_FORCE_VAR}={GUEST_EXE_FORCE_ON}"));
        banner.push_str(
            "podvm: userland: /proc/self/exe answers the payload's own path \
             from PODVM_GUEST_EXE rather than the loader's \
             (TODO/enter.md T-1409)\n",
        );
    }
    Ok(UserlandEnv {
        env: out,
        banner,
        unsafe_paths,
    })
}
/// What a userland drive did: the exit code, and whether the payload
/// ran. A decision-time refusal (unreadable bytes, unstaged memfd)
/// never entered, so the caller skips the post-run cleanup its own
/// `--rm` would do; a payload that ran, however it ended, takes the
/// normal path out.
pub struct UserlandOutcome {
    pub code: i32,
    pub entered: bool,
}

/// Drive a decided userland entry, on either verb.
///
/// TODO/enter.md T-1410. One path for `run` and both `exec` entries, so
/// the two verbs cannot diverge on the rung they share: the loader argv
/// execs directly, the memfd family stages post-fixup bytes first (and
/// refuses where the fixups changed what the decision read, rather than
/// entering something unjudged). Messages name the calling verb.
pub fn enter_userland(
    verb: &str,
    root: &podvm_enter::RootDir,
    plan: &podvm_enter::Plan,
    userland: &Userland,
    rootfs: &str,
    err: &mut dyn Write,
) -> UserlandOutcome {
    let failed = |code: i32| UserlandOutcome {
        code,
        entered: false,
    };
    let active = podvm_probe::select::Rung::Userland.word();
    let (exec_argv, fd) = match userland.family {
        podvm_enter::userland::Family::Loader => (userland.loader_argv.clone(), None),
        podvm_enter::userland::Family::Memfd => {
            // T-1649: the guest argv, so a `#!` payload stages its
            // interpreter, the program that the decision read.
            let argv0 = userland.guest_argv.first().cloned().unwrap_or_default();
            let bytes = match podvm_enter::ladder::payload_bytes(rootfs, &argv0, &plan.path_dirs) {
                Ok(b) => b,
                Err(e) => {
                    let _ = writeln!(err, "podvm {verb}: {e}");
                    return failed(e.exit_code());
                }
            };
            if podvm_enter::memfd::eligible(&bytes).is_err() {
                // Fixups changed what the decision read: loud, never
                // silent, and the fixups stay (they are the image's now,
                // not this run's).
                let _ = writeln!(
                    err,
                    "podvm {verb}: the payload the memfd family was decided \
                     on no longer stages: refusing rather than entering \
                     something unjudged (TODO/enter.md T-1317)"
                );
                return failed(podvm_image::error::EXIT_RUNTIME_ERROR);
            }
            match podvm_enter::memfd::stage(&bytes) {
                Ok(f) => (userland.guest_argv.clone(), Some(f)),
                Err(e) => {
                    let _ = writeln!(err, "podvm {verb}: {e}");
                    return failed(e.exit_code());
                }
            }
        }
    };
    match podvm_enter::run_userland(root, plan, exec_argv, active, fd, err) {
        Ok(c) => UserlandOutcome {
            code: c,
            entered: true,
        },
        Err(e) => {
            let _ = writeln!(err, "podvm {verb}: {e}");
            UserlandOutcome {
                code: e.exit_code(),
                entered: true,
            }
        }
    }
}

/// TODO/enter.md T-1411. The detached drive a record carries: the loader
/// argv where the loader family was decided, nothing everywhere else.
///
/// One function for `create` and `run -d`, so the two verbs cannot
/// disagree about which family has a detached driver
/// (`docs/conventions/code.md`).
pub fn detached_drive(userland: Option<&Userland>) -> Vec<String> {
    userland
        .filter(|u| u.family == podvm_enter::userland::Family::Loader)
        .map(|u| u.loader_argv.clone())
        .unwrap_or_default()
}

/// TODO/enter.md T-1411. The detached refusal, naming the blocking call.
///
/// T-1317's last resort survives only for entries with no detached
/// driver: the memfd family and anything the foreground cannot run. The
/// loader family drives detached through `spawn_userland` since T-1411
/// reversed that decision in writing, so this names the loader only in
/// the sentence that says it runs. `create` records the loader drive
/// beside the refusal for the rest, because a record promises a launcher
/// entry and only the loader family has one without a chroot.
pub fn detached_userland_refusal(verb: &str, family: &str) -> String {
    format!(
        "podvm {verb}: chroot(2) is denied and the {family} family has no detached \
         driver: the launcher enters detached payloads through \
         podvm_enter::spawn_selected (the chroot sequence and its namespace variant) \
         and, where the record carries the loader drive, \
         podvm_enter::spawn_userland. `run -d` and `start` refuse this {family} entry \
         here and `create` records no drive for it; run foreground \
         (TODO/enter.md T-1317, T-1411)"
    )
}

/// Shared by `run` and `create`: everything a container needs before it exists.
#[allow(clippy::too_many_arguments)]
pub fn platform_and_policy(
    verb: &str,
    platform: Option<&str>,
    insecure: &[String],
    tls_verify: Option<bool>,
) -> Result<(Platform, Policy), i32> {
    let p = Platform::wanted(platform).map_err(|e| {
        eprintln!("podvm {verb}: {e}");
        e.exit_code()
    })?;
    let pol = Policy::resolve(insecure, tls_verify).map_err(|e| {
        eprintln!("podvm {verb}: {e}");
        e.exit_code()
    })?;
    Ok((p, pol))
}

/// Resolve `--user` into the environment, for T-0711.
///
/// ⭐ One call for `run`, `create` and both `exec` paths: the value is the
/// resolved `UID:GID`, the caller's own spelling under the same name loses,
/// and a name means the image's own files. `None` changes nothing.
pub fn apply_user(
    verb: &str,
    rootfs: &str,
    user: Option<&String>,
    env: &mut Vec<String>,
) -> Result<(), i32> {
    let Some(spec) = user else {
        return Ok(());
    };
    let (uid, gid) = resolve_user(verb, rootfs, spec)?;
    env.retain(|e| e.split('=').next().unwrap_or("") != crate::interpose::IDENTITY_VAR);
    env.push(format!("{}={uid}:{gid}", crate::interpose::IDENTITY_VAR));
    Ok(())
}

/// Resolve `--user UID[:GID]` against the image, for T-0711.
///
/// Either side is a decimal id or a name from the image's own files: the
/// missing group means the user's own, as docker resolves it. Anything else
/// is the caller's mistake and refuses as a flag error, naming what was
/// asked for rather than running as somebody unintended.
pub fn resolve_user(verb: &str, rootfs: &str, spec: &str) -> Result<(u32, u32), i32> {
    fn num(s: &str) -> Option<u32> {
        if s.is_empty() || s.len() > 10 || !s.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        s.parse().ok()
    }
    /// The `UID` and primary `GID` for `want` in the image's own passwd
    /// file: fields 2 and 3 past the name. A name that is not there is
    /// `None`, not zero.
    fn passwd(rootfs: &str, want: &str) -> Option<(u32, u32)> {
        let text = std::fs::read_to_string(format!("{rootfs}/etc/passwd")).ok()?;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split(':');
            if fields.next() != Some(want) {
                continue;
            }
            fields.next()?;
            let uid: u32 = fields.next()?.trim().parse().ok()?;
            let gid: u32 = fields.next()?.trim().parse().ok()?;
            return Some((uid, gid));
        }
        None
    }
    /// The id for `want` in the image's own group file: field 2 past the
    /// name. A name that is not there is `None`, not zero.
    fn lookup(rootfs: &str, want: &str) -> Option<u32> {
        let text = std::fs::read_to_string(format!("{rootfs}/etc/group")).ok()?;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split(':');
            if fields.next() != Some(want) {
                continue;
            }
            fields.next()?;
            return fields.next()?.trim().parse().ok();
        }
        None
    }
    let (user, group) = match spec.split_once(':') {
        Some((u, g)) => (u, Some(g)),
        None => (spec, None),
    };
    // ⭐ A bare name means that user's own primary group from the image's
    // passwd file, not the uid repeated: the two agree on most images and
    // differ exactly where inventing one would run as somebody unintended.
    let (uid, primary) = match num(user) {
        Some(u) => (Some(u), None),
        None => match passwd(rootfs, user) {
            Some((u, g)) => (Some(u), Some(g)),
            None => (None, None),
        },
    };
    let gid = match group {
        Some(g) => num(g).or_else(|| lookup(rootfs, g)),
        None => primary.or(uid),
    };
    match (uid, gid, group) {
        (Some(u), _, None) => Ok((u, gid.unwrap_or(u))),
        (Some(u), Some(g), _) => Ok((u, g)),
        _ => {
            eprintln!(
                "podvm {verb}: --user takes uid[:gid] with decimal ids or names \
                 from the image's own passwd and group files, not {spec:?}"
            );
            Err(podvm_image::error::EXIT_FLAG_ERROR)
        }
    }
}
