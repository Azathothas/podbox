use super::*;

pub(super) fn container_json(s: &podvm_image::Store, c: &Container) -> String {
    let fields = container_fields(s, c);
    let mut o = serde_json::Map::new();
    for (k, v) in &fields {
        // ⚠ The nested pair is nested here too, so a caller reading the
        // document and one reading `--format` do not find two shapes.
        if let Some(rest) = k.strip_prefix("Exec.") {
            let e = o
                .entry("Exec".to_string())
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
}

/// Resolve a container reference to the rootfs `exec` re-enters.
///
/// ⚠ A container name first, then an image reference, because `podvm exec` on a
/// running container is the common case and an image of the same name is the
/// unusual one.
pub fn rootfs_of(s: &podvm_image::Store, want: &str) -> Option<(String, State)> {
    let c = podvm_supervise::get(s, want).ok()?;
    Some((c.of.rootfs, c.state))
}

/// TODO/milestones.md T-1112. Refuse a guest that is not Linux by name,
/// with the missing leg, on every entry path.
///
/// podvm runs Linux guests only: the chroot tier shares the host kernel
/// and the machine tier boots Linux images, so a request naming another OS
/// is refused here, before anything is fetched or entered, rather than
/// pulled and failed inside.
pub fn ensure_linux_guest(verb: &str, os: &str, arch: &str) -> Result<(), i32> {
    if os == podvm_image::platform::OS {
        return Ok(());
    }
    // ⭐ TODO/milestones.md T-1112. A guest that is not Linux is not an OCI
    // rootfs: it is a disk image an emulator boots, so it has a verb of its
    // own and this path cannot run it. The refusal names that verb rather
    // than claiming no support exists, because since T-1112 one does.
    if let Some(verb_name) = guest_verb(os) {
        eprintln!(
            "podvm {verb}: {os}/{arch} is not an OCI rootfs, so nothing on \
             this path pulls or enters it. A disposable {os} guest is its own \
             verb: `{verb_name} --image <disk> -- <command>` \
             (TODO/milestones.md T-1112)"
        );
        return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    eprintln!(
        "podvm {verb}: {os}/{arch} is not a Linux guest. podvm runs Linux \
         guests only: the chroot tier shares the host kernel and the machine \
         tier boots Linux images. No {os} guest support exists \
         (TODO/milestones.md T-1112), so there is nothing to pull or enter \
         for this platform"
    );
    Err(podvm_image::error::EXIT_RUNTIME_ERROR)
}

/// Where a non-Linux guest lives, if podvm has one: the command a caller
/// who asked for it here should have used. `None` is a refusal.
///
/// ⛔ Separate from [`ensure_linux_guest`] so the routing is a value the
/// tests pin, rather than a sentence buried in an `eprintln!`.
pub fn guest_verb(os: &str) -> Option<&'static str> {
    match os {
        "windows" => Some("podvm windows run"),
        _ => None,
    }
}

// TODO/enter.md T-1411. T-1317's strict chroot gate stood here: `create`
// kept it because a record promised a launcher entry and the launcher
// chrooted. The launcher now drives the userland loader family
// detached, so no gate keeps it: `create` and `run -d` take the
// two-tier `ensure_entry_possible` gate and the record carries the
// loader drive, and `start` checks the record instead of the probe.

/// The pure half of [`ensure_entry_possible`]: anything could run where
/// chroot holds, the static family needs the kernel's memfd, and the
/// loader family needs an interposer object in this binary. Tests pin
/// the table; the wrapper below reads the machine.
pub(super) fn entry_possible(chroot_ok: bool, memfd_ok: bool, objects: bool) -> bool {
    chroot_ok || memfd_ok || objects
}

/// TODO/enter.md T-1317. The pre-fetch tier of the entry gate: refuse only
/// where nothing could run.
///
/// The exact rung needs the payload, which is post-extract, so this gate
/// admits where chroot holds, where a memfd executes (the static no-chroot
/// family may run; T-1688), or where this binary embeds an interposer
/// object (the loader no-chroot family may run). Refusal names all three
/// misses at 125, fetching nothing.
///
/// ⭐ TODO/image.md T-1418. Past that tier, the store itself must execute
/// what it creates: a default store on a `noexec` filesystem pulls and
/// extracts fully, then dies at `execve` with raw `EACCES`. The probe runs
/// here, before any byte is fetched, and refuses with the path and the
/// `$PODVM_STORE` remedy. A probe that cannot run names its path and
/// never reads as "no".
pub fn ensure_entry_possible(
    verb: &str,
    findings: &podvm_probe::Findings,
    store_root: &std::path::Path,
) -> Result<(), i32> {
    // T-1688: the memfd leg is measured only where chroot is denied; where
    // chroot holds the answer cannot change the result.
    let chroot_ok = podvm_probe::probes::chroot_usable(findings);
    let memfd = (!chroot_ok).then(podvm_enter::memfd::memfd_exec);
    if entry_possible(
        chroot_ok,
        memfd.is_some_and(|m| m.runs()),
        crate::interpose::object(crate::interpose::Libc::Gnu).is_some()
            || crate::interpose::object(crate::interpose::Libc::Musl).is_some(),
    ) {
        return ensure_store_exec(verb, store_root);
    }
    let memfd_text = memfd
        .map(|m| podvm_enter::memfd::probe::answer(m).sentence())
        .unwrap_or_default();
    eprintln!(
        "podvm {verb}: chroot(2) is denied on this machine, and no no-chroot \
         family can run either: a memfd does not execute ({memfd_text}), and \
         this podvm binary embeds no interposer object for the loader family \
         (see `podvm probe` and `podvm doctor`, and \
         ./scripts/build-interpose.sh builds them) (TODO/enter.md T-1317, \
         T-1688)"
    );
    Err(podvm_image::error::EXIT_RUNTIME_ERROR)
}

/// TODO/image.md T-1418. Probe whether files created under the store
/// execute, before any byte is fetched.
///
/// `ok` runs the entry on; `ok == false` refuses at 125 with the probe's
/// own detail, which names the directory and the `$PODVM_STORE` remedy;
/// a probe error refuses naming the path, never as "no".
///
/// ⚠ Unix only: the probe executes a script, and the chroot entry it
/// guards does not exist off unix either, so the gate below refuses there
/// instead with its own sentence.
pub fn ensure_store_exec(verb: &str, store_root: &std::path::Path) -> Result<(), i32> {
    #[cfg(not(unix))]
    {
        let _ = (verb, store_root);
        return Ok(());
    }
    #[cfg(unix)]
    {
        match podvm_image::exec_probe::probe_store_exec(store_root) {
            Ok(probe) if probe.ok => Ok(()),
            Ok(probe) => {
                eprintln!("podvm {verb}: {}", probe.detail);
                Err(podvm_image::error::EXIT_RUNTIME_ERROR)
            }
            Err(e) => {
                eprintln!("podvm {verb}: the store exec probe could not run: {e}");
                Err(podvm_image::error::EXIT_RUNTIME_ERROR)
            }
        }
    }
}

/// TODO/cli.md T-1414. The no-chroot refusal code follows the payload
/// status on every rung, exactly as the chroot path emits from the child:
/// `Absent` is 127, found-but-not-invocable is 126 (the errno split
/// T-0802 pins: ENOENT against EPERM, ENOEXEC, EACCES, EISDIR and
/// ETXTBSY), and a genuine podvm failure stays 125.
///
/// ⚠ Matched on `ladder::resolve_payload`'s own sentences rather than
/// re-derived: the resolution kinds are rendered there into strings, so
/// re-walking the symlinks here would be the second copy
/// `docs/conventions/code.md` refuses. The three shapes are pinned by
/// `resolve_codes_follow_the_payload_status` below, so a rewording there
/// fails here instead of silently returning 125.
pub(super) fn resolve_code(msg: &str) -> i32 {
    if msg.contains("names no file in the image") {
        EXIT_NOT_FOUND
    } else if msg.contains("escapes the image") || msg.contains("too many levels of symlinks") {
        EXIT_CANNOT_INVOKE
    } else {
        EXIT_RUNTIME_ERROR
    }
}

/// TODO/enter.md T-1648. Why a static payload refuses where chroot is
/// denied and `--unsafe-host-paths` is absent, and the routes past the
/// refusal. A constant, so the tests pin the sentence a caller reads.
pub const STATIC_HOST_PATHS: &str = "the payload is statically linked, so \
     it makes raw syscalls past the interposer and podvm rewrites no path: \
     its absolute paths would resolve on the host, and a write would land \
     on the host tree. podvm refuses before the exec. Use a host where \
     chroot(2) is permitted, or `--unsafe-host-paths` to accept this host \
     pass-through on the memfd family. The planned rungs for static \
     payloads are the emulate tier (TODO/enter.md T-1651) and the \
     native-static rung (TODO/enter.md T-1652) (TODO/enter.md T-1648)";

/// A no-chroot entry this run takes, decided post-extract and pre-fixup.
#[derive(Debug, Clone)]
pub struct Userland {
    /// Which family runs the payload.
    pub family: podvm_enter::userland::Family,
    /// The exact argv the child execs (the loader family: loader, payload
    /// by host path, then the payload's own arguments).
    pub loader_argv: Vec<String>,
    /// The image's library directories by host path (the loader family).
    pub lib_dirs: Vec<String>,
    /// The rung account lines for the banner.
    pub banner: String,
}

/// TODO/enter.md T-1317. The exact rung, post-extract and pre-fixup: the
/// rootfs exists and nothing has been written into it yet.
///
/// `None` is the chroot sequence, as before. Where chroot is denied the
/// loader family runs a dynamic payload the tier reaches (loader present,
/// libraries present, native image) and the memfd family runs a static
/// one under the opt-in; where neither runs the refusal names every tried rung with its
/// missing leg: the last resort, never the whole answer. The refusal code
/// follows the payload status on every rung (T-1414): 127 where the
/// payload resolved to nothing, 126 where it was found but not invocable,
/// 125 for a genuine podvm-side failure.
///
/// Read-only: classification, resolution and ELF reads only. Placement
/// (`place`) and fixups run after this decides, so a refusal writes
/// nothing into the image.
///
/// ⛔ TODO/enter.md T-1648. A static payload enters the memfd family only
/// under `unsafe_host_paths`, the decided `--unsafe-host-paths` opt-in:
/// its absolute paths resolve on the host, so without the opt-in it
/// refuses at 125 before the exec ([`STATIC_HOST_PATHS`]).
#[allow(clippy::too_many_arguments)]
pub fn decide_entry(
    verb: &str,
    rootfs: &str,
    argv: &[String],
    env: &[String],
    support_native: bool,
    findings: &podvm_probe::Findings,
    unsafe_host_paths: bool,
) -> Result<Option<Userland>, i32> {
    if podvm_probe::probes::chroot_usable(findings) {
        return Ok(None);
    }
    let argv0 = argv.first().map(String::as_str).unwrap_or("");
    let path_dirs = podvm_enter::Plan::path_from(env);
    let no_chroot = |code: i32, family_note: String| {
        eprintln!(
            "podvm {verb}: chroot(2) is denied on this machine, and no \
             no-chroot family runs this payload: {family_note}. The probe's \
             chroot leg reports the denial (see `podvm probe`) \
             (TODO/enter.md T-1317)"
        );
        Err(code)
    };
    // ⭐ T-1413 and T-1414. Resolved once, up front: the path rides the
    // script refusal below, the resolution sentences decide the refusal
    // code (127 absent, 126 found-but-not-invocable), and the loader arm
    // reuses the same path rather than resolving twice.
    let resolved = match podvm_enter::ladder::resolve_payload(rootfs, argv0, &path_dirs) {
        Ok(p) => p,
        Err(e) => {
            let code = resolve_code(&e.to_string());
            return no_chroot(code, format!("{argv0} resolves to nothing runnable: {e}"));
        }
    };
    // The payload bytes decide the family. Read-only: classification and
    // ELF reads only, so a refusal writes nothing into the image.
    let bytes = match podvm_enter::ladder::payload_bytes(rootfs, argv0, &path_dirs) {
        Ok(b) => b,
        Err(e) => {
            let msg = e.to_string();
            // ⚠ T-0802's errno split: a directory is found-but-not-
            // invocable (EISDIR is a 126), while an unreadable file or an
            // over-ceiling one is a genuine podvm-side failure (125).
            let code = if msg.contains("Is a directory") {
                EXIT_CANNOT_INVOKE
            } else {
                EXIT_RUNTIME_ERROR
            };
            return no_chroot(
                code,
                format!("{argv0} resolved but its bytes could not be read: {e}"),
            );
        }
    };
    let interp = match podvm_enter::memfd::eligible(&bytes) {
        Ok(()) => None,
        Err(podvm_enter::memfd::MemfdRefusal::RoutePastScript) => {
            // ⭐ T-1413. The refusal names the exact payload path and its
            // `#!` line, resolved and read above: an anonymous refusal is
            // a defect in the message. Longer term the interpreter routes
            // inside the rootfs where the T-1407 map table covers it.
            let refusal = podvm_enter::memfd::script_refusal(&resolved, &bytes);
            return no_chroot(EXIT_CANNOT_INVOKE, refusal.message());
        }
        Err(podvm_enter::memfd::MemfdRefusal::NotElf(why)) => {
            return no_chroot(
                EXIT_CANNOT_INVOKE,
                format!("the payload is not an ELF file: {why}"),
            );
        }
        Err(podvm_enter::memfd::MemfdRefusal::HasInterp(interp)) => Some(interp),
    };
    if interp.is_none() {
        // Static: the memfd family, where a memfd executes.
        if !support_native {
            return no_chroot(
                EXIT_RUNTIME_ERROR,
                "the payload is statically linked but the image is foreign: \
                 binfmt resolution without a chroot is refused (T-0506)"
                    .to_string(),
            );
        }
        // ⛔ T-1648: refusal beats a silent host write. The opt-in is
        // decided before the memfd probe, so a refusal executes nothing.
        if !unsafe_host_paths {
            return no_chroot(EXIT_RUNTIME_ERROR, STATIC_HOST_PATHS.to_string());
        }
        // ⛔ T-1688: the probe executes a memfd, so a sysctl that seals it
        // refuses here, before staging, and not at the launch's step 5.
        let memfd = podvm_enter::memfd::memfd_exec();
        if !memfd.runs() {
            return no_chroot(
                EXIT_RUNTIME_ERROR,
                format!(
                    "the payload is statically linked but a memfd does not \
                     execute on this machine: {} (TODO/enter.md T-1688)",
                    podvm_enter::memfd::probe::answer(memfd).sentence()
                ),
            );
        }
        return Ok(Some(Userland {
            family: podvm_enter::userland::Family::Memfd,
            loader_argv: Vec::new(),
            lib_dirs: Vec::new(),
            banner: "entering without chroot on the memfd family under \
             --unsafe-host-paths: the payload runs from a staged memfd with the host's root and its working \
             directory inside the image; absolute paths resolve on the host \
             because the static payload makes raw syscalls past the interposer, \
             so no path is rewritten, and it reads /proc/self/exe as the staged \
             memfd path, so a self-locating runtime cannot resolve itself here \
             (TODO/enter.md T-1317, T-1409, T-1648)"
                .to_string(),
        }));
    }
    // Dynamic: the loader family, where the tier reaches the payload.
    let interp = interp.unwrap_or_default();
    if !support_native {
        return no_chroot(
            EXIT_RUNTIME_ERROR,
            "the payload is dynamically linked but the image is foreign: binfmt \
             resolution without a chroot is refused (T-0506)"
                .to_string(),
        );
    }
    // ⭐ Resolved once above and reused here: the invocation keeps the
    // invoked name where it names something openable. Measured on the lane
    // (probe-3): alpine's `/bin/sh` points at the absolute `/bin/busybox`,
    // which dangles on the host side, so the resolved file is what opens
    // and `loader_argv_for` below carries the invoked name as the applet.
    // A relative link (debian's `/bin/sh`) opens as invoked.
    let payload_host = podvm_enter::userland::invocation_path(rootfs, argv0, &path_dirs, &resolved);
    let plan = match podvm_enter::userland::loader_plan(rootfs, &interp, &payload_host) {
        Ok(p) => p,
        Err(why) => {
            // ⚠ Found, but the loader cannot run it: 126, never 125.
            return no_chroot(EXIT_CANNOT_INVOKE, why);
        }
    };
    match crate::interpose::classify(rootfs, argv0, &path_dirs) {
        crate::interpose::Reach::Declined(why) => {
            // ⚠ Found, but the tier cannot virtualize it (Go markers, an
            // unknown libc, an unreadable ELF): without the interposer no
            // path is rewritten, so 126, never a silent host run.
            return no_chroot(
                EXIT_CANNOT_INVOKE,
                format!("the loader family declines the payload: {why}"),
            );
        }
        crate::interpose::Reach::Preload { .. } => {}
    }
    // The loader consumes argv[0] (its own path) and argv[1] (the program):
    // the payload sees its own arguments with argv[0] as the host path.
    // That path is what was executed, and the banner says so. Where the
    // payload resolved to busybox through another name, the invoked name
    // rides beside the opened file so the applet selection survives.
    let (loader_argv, applet) = podvm_enter::userland::loader_argv_for(
        &plan.loader_host,
        &plan.payload_host,
        argv0,
        argv.get(1..).unwrap_or(&[]),
    );
    Ok(Some(Userland {
        family: podvm_enter::userland::Family::Loader,
        loader_argv,
        lib_dirs: plan.lib_dirs,
        banner: format!(
            "entering without chroot on the loader family: {} runs {} with the \
             image's libraries; absolute guest paths anchor inside the rootfs \
             through PODVM_MAPS, and host pass-through happens only under \
             --unsafe-host-paths, and argv[0] keeps the invoked name{}; \
             a child that execs a dynamic program or a #! script enters \
             through the image's loader inside the rootfs, with the image's \
             libraries from PODVM_LIB_PATH (TODO/enter.md T-1317, T-1407, \
             T-1411; TODO/interpose.md T-1646)",
            plan.loader_host,
            plan.payload_host,
            if applet {
                format!(
                    "; the invoked name {argv0:?} rides as the applet the resolved \
                     busybox file selects"
                )
            } else {
                String::new()
            },
        ),
    }))
}
