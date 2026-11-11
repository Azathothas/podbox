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

pub use super::native::STATIC_HOST_PATHS;

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
    /// TODO/cli.md T-1649. The guest argv that runs: the caller's, or for
    /// a `#!` payload the argv of the kernel (interpreter, its argument,
    /// the script path, then the arguments). The interposer classifies,
    /// and `/proc/self/exe` names, its first word.
    pub guest_argv: Vec<String>,
    /// TODO/cli.md T-1650. The capability gaps of the family for this
    /// payload, which `--strict` reads.
    pub gaps: Vec<crate::complete::Gap>,
    /// TODO/enter.md T-1653. The table of the run: the rootfs and the
    /// volumes. The entry resolves the payload and the working directory
    /// through it, so either can be in a volume.
    pub maps: podvm_enter::userland::Maps,
    /// TODO/enter.md T-1652. The guest path of the program on the native-static rung.
    pub dispatch_exe: Option<String>,
    /// TODO/enter.md T-1651. The emulator of the emulate tier (`MACHINE:HOSTPATH`).
    pub emulator: Option<String>,
    /// TODO/enter.md T-1657. The syscall tier on the native-static rung.
    pub syscall: bool,
}

/// TODO/cli.md T-1650. The capability gaps that `--strict` reads: those of
/// the decided family, none on the chroot rung.
pub fn gaps_of(userland: Option<&Userland>) -> Vec<crate::complete::Gap> {
    userland.map(|u| u.gaps.clone()).unwrap_or_default()
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
/// Read-only: placement and fixups run after this decides, so a refusal
/// writes nothing into the image.
///
/// ⛔ TODO/enter.md T-1652 and T-1648. A static payload takes the
/// native-static rung where it arms; else the memfd family only under
/// `--unsafe-host-paths`, and a refusal at 125 ([`STATIC_HOST_PATHS`]).
///
/// ⭐ TODO/enter.md T-1653. `maps` is the table of the run (the rootfs and
/// the volumes): each guest path resolves through it, so a first program
/// in a volume is the file of the volume.
///
/// ⭐ TODO/enter.md T-1707. A relative first program resolves against
/// `working_dir`, as docker's does.
///
/// ⭐ TODO/cli.md T-1649. A `#!` payload runs its interpreter, resolved
/// inside the rootfs, with the script path as its argument
/// ([`script::follow`]). `retry` is the first word of the caller where
/// `argv[0]` is the Entrypoint of the image and the caller gave a
/// command: a refusal after a followed script then names
/// `--entrypoint` with that word.
#[allow(clippy::too_many_arguments)]
pub fn decide_entry(
    verb: &str,
    maps: &podvm_enter::userland::Maps,
    argv: &[String],
    working_dir: &str,
    env: &[String],
    support_native: bool,
    findings: &podvm_probe::Findings,
    unsafe_host_paths: bool,
    retry: Option<&str>,
) -> Result<Option<Userland>, i32> {
    if podvm_probe::probes::chroot_usable(findings) {
        return Ok(None);
    }
    let path_dirs = podvm_enter::Plan::path_from(env);
    let no_chroot = |code: i32, family_note: String, routed: bool| {
        eprintln!(
            "{}",
            script::no_chroot_message(verb, &family_note, routed, retry)
        );
        Err(code)
    };
    let f = match script::follow(maps, argv, &path_dirs, working_dir) {
        Ok(f) => f,
        Err((code, note, routed)) => return no_chroot(code, note, routed),
    };
    let routed = !f.scripts.is_empty();
    let script_note = script::banner_note(&f);
    let argv0 = f.argv.first().map(String::as_str).unwrap_or("");
    let interp = match podvm_enter::memfd::eligible(&f.bytes) {
        Ok(()) => None,
        // `follow` returns no script, so the first arm reads as not ELF.
        Err(
            why @ (podvm_enter::memfd::MemfdRefusal::RoutePastScript
            | podvm_enter::memfd::MemfdRefusal::NotElf(_)),
        ) => {
            return no_chroot(
                EXIT_CANNOT_INVOKE,
                format!("the payload is not an ELF file: {why}"),
                routed,
            );
        }
        Err(podvm_enter::memfd::MemfdRefusal::HasInterp(interp)) => Some(interp),
    };
    let Some(interp) = interp else {
        // Static: the memfd family, where a memfd executes.
        if !support_native {
            return no_chroot(
                EXIT_RUNTIME_ERROR,
                "the payload is statically linked but the image is foreign: \
                 binfmt resolution without a chroot is refused (T-0506)"
                    .to_string(),
                routed,
            );
        }
        // ⭐ T-1652 where the caller asked for no host paths; else T-1648.
        if !unsafe_host_paths {
            let why = match super::native::decide(&f, maps, &script_note) {
                Ok(u) => return Ok(Some(u)),
                Err(why) => why,
            };
            return no_chroot(
                EXIT_RUNTIME_ERROR,
                format!("{STATIC_HOST_PATHS}; the native-static rung: {why}"),
                routed,
            );
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
                routed,
            );
        }
        return Ok(Some(Userland {
            family: podvm_enter::userland::Family::Memfd,
            loader_argv: Vec::new(),
            lib_dirs: Vec::new(),
            banner: format!(
                "entering without chroot on the memfd family under \
                 --unsafe-host-paths: the payload runs from a staged memfd with the host's root and its working \
                 directory inside the image; absolute paths resolve on the host \
                 because the static payload makes raw syscalls past the interposer, \
                 so no path is rewritten, and it reads /proc/self/exe as the staged \
                 memfd path, so a self-locating runtime cannot resolve itself here \
                 (TODO/enter.md T-1317, T-1409, T-1648){script_note}"
            ),
            guest_argv: f.argv.clone(),
            gaps: Vec::new(),
            maps: maps.clone(),
            dispatch_exe: None,
            emulator: None,
            syscall: false,
        }));
    };
    // Dynamic: the loader family, where the tier reaches the payload.
    if !support_native {
        return no_chroot(
            EXIT_RUNTIME_ERROR,
            "the payload is dynamically linked but the image is foreign: binfmt \
             resolution without a chroot is refused (T-0506)"
                .to_string(),
            routed,
        );
    }
    // ⭐ Resolved once above and reused here: the invocation keeps the
    // invoked name where it names something openable. Measured on the lane
    // (probe-3): alpine's `/bin/sh` points at the absolute `/bin/busybox`,
    // which dangles on the host side, so the resolved file is what opens
    // and `loader_argv_for` below carries the invoked name as the applet.
    // A relative link (debian's `/bin/sh`) opens as invoked.
    // T-1707: the lookups take the name joined to the workdir.
    let name = podvm_enter::program_in(argv0, working_dir);
    let payload_host = podvm_enter::userland::invocation_path(maps, &name, &path_dirs, &f.resolved);
    let plan = match podvm_enter::userland::loader_plan(maps, &interp, &payload_host) {
        Ok(p) => p,
        // ⚠ Found, but the loader cannot run it: 126, never 125.
        Err(why) => return no_chroot(EXIT_CANNOT_INVOKE, why, routed),
    };
    if let crate::interpose::Reach::Declined(why) =
        crate::interpose::classify(maps, &name, &path_dirs)
    {
        // ⚠ Found, but the tier cannot virtualize it (Go markers, an
        // unknown libc, an unreadable ELF): without the interposer no
        // path is rewritten, so 126, never a silent host run.
        return no_chroot(
            EXIT_CANNOT_INVOKE,
            format!("the loader family declines the payload: {why}"),
            routed,
        );
    }
    // The loader consumes argv[0] (its own path) and argv[1] (the program):
    // the payload sees its own arguments with argv[0] as the host path.
    // That path is what was executed, and the banner says so. Where the
    // payload resolved to busybox through another name, the invoked name
    // rides beside the opened file so the applet selection survives.
    let (mut loader_argv, applet) = podvm_enter::userland::loader_argv_for(
        &plan.loader_host,
        &plan.payload_host,
        argv0,
        f.argv.get(1..).unwrap_or(&[]),
    );
    // T-1719 (no ld.so.cache of the host) and T-1660 (argv[0] is the guest name).
    let (host, cache) = (&plan.loader_host, plan.inhibit_cache);
    super::argv0::options(&mut loader_argv, host, cache, argv0, applet);
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
             libraries from PODVM_LIB_PATH; a raw call through libc's \
             syscall() takes the same table, and a raw call with a path that \
             the table cannot map, or io_uring_setup, answers ENOSYS, so \
             libuv uses its thread pool (TODO/enter.md T-1317, T-1407, \
             T-1411; TODO/interpose.md T-1646, T-1697){script_note}",
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
        gaps: crate::complete::loader_gaps(maps, argv, routed, &path_dirs),
        guest_argv: f.argv,
        maps: maps.clone(),
        dispatch_exe: None,
        emulator: None,
        syscall: false,
    }))
}
