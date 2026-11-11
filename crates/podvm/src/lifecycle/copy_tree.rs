use super::*;

/// Whether the path itself is a symlink (never followed).
pub(super) fn is_link(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

/// Replicate one symlink as a symlink: the target bytes are copied, never
/// resolved, so an absolute target escapes nothing. The caller gates the
/// link's own path; the target is opaque here by design.
pub(super) fn replicate_link(src: &std::path::Path, dst: &std::path::Path) -> i32 {
    let target = match std::fs::read_link(src) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("podvm cp: {}: {e}", src.display());
            return EXIT_RUNTIME_ERROR;
        }
    };
    if let Some(parent) = dst.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("podvm cp: {}: {e}", parent.display());
        return EXIT_RUNTIME_ERROR;
    }
    match std::os::unix::fs::symlink(&target, dst) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if dst.is_dir() && !is_link(dst) {
                eprintln!("podvm cp: {} exists and is a directory", dst.display());
                return EXIT_RUNTIME_ERROR;
            }
            if let Err(e) = std::fs::remove_file(dst) {
                eprintln!("podvm cp: {}: {e}", dst.display());
                return EXIT_RUNTIME_ERROR;
            }
            if let Err(e) = std::os::unix::fs::symlink(&target, dst) {
                eprintln!("podvm cp: {}: {e}", dst.display());
                return EXIT_RUNTIME_ERROR;
            }
        }
        Err(e) => {
            eprintln!("podvm cp: {}: {e}", dst.display());
            return EXIT_RUNTIME_ERROR;
        }
    }
    eprintln!(
        "podvm cp: symlink {} -> {}",
        dst.display(),
        target.display()
    );
    0
}

/// Every component of `path` below `base`, checked without following: a
/// write through a pre-existing symlink would land where the link points,
/// so the first symlink refuses the copy by name. `base` itself was gated
/// by the caller and is not re-checked.
pub(super) fn clear_of_symlinks(
    base: &std::path::Path,
    path: &std::path::Path,
) -> std::result::Result<(), String> {
    let mut probe = base.to_path_buf();
    let rel = path
        .strip_prefix(base)
        .map_err(|_| format!("{} escapes {}", path.display(), base.display()))?;
    for component in rel.components() {
        probe.push(component);
        if is_link(&probe) {
            return Err(format!(
                "{} is a symlink; writing through one is refused",
                probe.display()
            ));
        }
    }
    Ok(())
}

/// Copy a directory tree without following anything. Every rootfs-side
/// path passes `contain::within`: destinations on the way in, sources on
/// the way out: except a symlink source, which is gated on its own path
/// (its parent) and replicated verbatim, never resolved, per T-0305's
/// rule. Two passes: the first validates the whole tree, so a special
/// file or a destination through a pre-existing symlink refuses before a
/// byte lands and no half-made tree is left behind (TODO/cli.md T-1323).
pub(super) fn copy_tree(
    src: &std::path::Path,
    dst: &std::path::Path,
    root: &std::path::Path,
    out_of: bool,
) -> i32 {
    let gate = |p: &std::path::Path| podvm_image::contain::within(root, p);
    // The destination side's base for the write-through check: the rootfs
    // on the way in, the fresh-or-merging top on the way out. A write
    // through a pre-existing link would land where the link points, so
    // the first such destination refuses the copy by name.
    let dst_base: &std::path::Path = if out_of { dst } else { root };
    // Pass one: walk without following, gating and collecting. A special
    // file anywhere, or a destination through a pre-existing symlink,
    // refuses the whole copy by name.
    let mut files: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    let mut links: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    let mut stack = vec![src.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                // ⭐ T-0414: the listing is required (a tree copy walks
                // it), so the refusal names the operation and the path
                // with the kernel's errno, never a missing file.
                eprintln!("podvm cp: cannot list {}: {e}", dir.display());
                return EXIT_RUNTIME_ERROR;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("podvm cp: {e}");
                    return EXIT_RUNTIME_ERROR;
                }
            };
            let src_path = entry.path();
            let rel = match src_path.strip_prefix(src) {
                Ok(r) => r,
                Err(_) => {
                    eprintln!("podvm cp: {} escapes {}", src_path.display(), src.display());
                    return EXIT_RUNTIME_ERROR;
                }
            };
            let dst_path = dst.join(rel);
            // ⛔ Never follow: metadata, not metadata-through.
            let kind = match std::fs::symlink_metadata(&src_path) {
                Ok(m) => m.file_type(),
                Err(e) => {
                    eprintln!("podvm cp: {}: {e}", src_path.display());
                    return EXIT_RUNTIME_ERROR;
                }
            };
            if kind.is_symlink() {
                // ⚠ Gated on its own path, never resolved: the target bytes
                // are opaque, and an absolute target escapes nothing.
                let own = if out_of {
                    match src_path.parent() {
                        Some(parent) => parent,
                        None => {
                            eprintln!("podvm cp: {} has no parent to gate", src_path.display());
                            return EXIT_RUNTIME_ERROR;
                        }
                    }
                } else {
                    &dst_path
                };
                if let Err(e) = gate(own) {
                    eprintln!("podvm cp: {e}");
                    return EXIT_RUNTIME_ERROR;
                }
                if let Err(msg) = clear_of_symlinks(dst_base, &dst_path) {
                    eprintln!("podvm cp: {msg}");
                    return EXIT_RUNTIME_ERROR;
                }
                links.push((src_path, dst_path));
                continue;
            }
            let gated = if out_of { &src_path } else { &dst_path };
            if let Err(e) = gate(gated) {
                eprintln!("podvm cp: {e}");
                return EXIT_RUNTIME_ERROR;
            }
            if let Err(msg) = clear_of_symlinks(dst_base, &dst_path) {
                eprintln!("podvm cp: {msg}");
                return EXIT_RUNTIME_ERROR;
            }
            if kind.is_dir() {
                dirs.push(dst_path);
                stack.push(src_path);
                continue;
            }
            if !kind.is_file() {
                eprintln!(
                    "podvm cp: {} is not a file or directory; special files are refused",
                    src_path.display()
                );
                return EXIT_RUNTIME_ERROR;
            }
            files.push((src_path, dst_path));
        }
    }
    // Pass two: the tree is validated, so only I/O can still fail, and
    // each such error names its file. A refusal in pass one leaves no
    // half-made tree behind.
    if is_link(dst) {
        eprintln!(
            "podvm cp: {} is a symlink; writing through one is refused",
            dst.display()
        );
        return EXIT_RUNTIME_ERROR;
    }
    if dst.is_file() {
        eprintln!("podvm cp: {} exists and is not a directory", dst.display());
        return EXIT_RUNTIME_ERROR;
    }
    if let Err(e) = std::fs::create_dir_all(dst) {
        eprintln!("podvm cp: {}: {e}", dst.display());
        return EXIT_RUNTIME_ERROR;
    }
    for dir in &dirs {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("podvm cp: {}: {e}", dir.display());
            return EXIT_RUNTIME_ERROR;
        }
    }
    let mut bytes = 0u64;
    for (from, to) in &files {
        if let Some(parent) = to.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            eprintln!("podvm cp: {}: {e}", parent.display());
            return EXIT_RUNTIME_ERROR;
        }
        match std::fs::copy(from, to) {
            Ok(n) => bytes += n,
            Err(e) => {
                eprintln!("podvm cp: {}: {e}", from.display());
                return EXIT_RUNTIME_ERROR;
            }
        }
    }
    for (from, to) in &links {
        if replicate_link(from, to) != 0 {
            return EXIT_RUNTIME_ERROR;
        }
    }
    eprintln!(
        "podvm cp: {} files ({bytes} bytes), {} symlinks {} -> {}",
        files.len(),
        links.len(),
        src.display(),
        dst.display()
    );
    0
}

/// `podvm inspect` against a container rather than an image.
///
/// ⚠ Tried after the image lookup fails, so a name that is both is an image, as
/// docker resolves it.
pub fn inspect_container(want: &str, template: Option<&str>) -> Option<i32> {
    let s = podvm_image::open_store().ok()?;
    let c = podvm_supervise::get(&s, want).ok()?;
    let fields = container_fields(&s, &c);
    match template {
        Some(t) => match format::render(t, &fields) {
            Ok(line) => {
                println!("{line}");
                Some(0)
            }
            Err(bad) => {
                eprintln!("podvm inspect: {bad}");
                Some(EXIT_CLI_ERROR)
            }
        },
        None => {
            println!("{}", container_json(&s, &c));
            Some(0)
        }
    }
}

/// One field of the end of the last run, or a dash where no run ended.
fn end_field(c: &Container, f: impl Fn(&podvm_supervise::End) -> String) -> String {
    c.end.as_ref().map(f).unwrap_or_else(|| "-".into())
}

/// The word of an end reason, as the record writes it.
pub(super) fn reason_word(r: podvm_supervise::Reason) -> &'static str {
    match r {
        podvm_supervise::Reason::Exit => "exit",
        podvm_supervise::Reason::Signal => "signal",
        podvm_supervise::Reason::Wall => "wall",
    }
}

/// The count of the processes that stayed after the kill of the tree, or
/// `unknown`, never a zero for an unknown count.
pub(super) fn residual_word(r: podvm_supervise::Residual) -> String {
    match r {
        podvm_supervise::Residual::Count(n) => n.to_string(),
        podvm_supervise::Residual::Unknown => "unknown".to_string(),
    }
}

pub(super) fn container_fields(
    s: &podvm_image::Store,
    c: &Container,
) -> Vec<(&'static str, String)> {
    // T-0708: the emulation tally beside the container record. A dash where
    // the tier never ran (no memo file); counts where it did, zero included.
    let memo = podvm_supervise::table::memo_path(s, &c.id);
    let em = memo
        .exists()
        .then(|| podvm_supervise::table::emulated_counts(&memo));
    let count = |f: fn(&podvm_supervise::table::EmulatedCounts) -> u64| {
        em.as_ref()
            .map(f)
            .map(|x| x.to_string())
            .unwrap_or_else(|| "-".into())
    };
    let mknod = count(|e| e.mknod);
    let mount = count(|e| e.mount);
    let unshare = count(|e| e.unshare);
    let clone = count(|e| e.clone);
    // T-0413: the served /proc answers beside the four older stories.
    let procfs = count(|e| e.procfs);
    vec![
        ("Id", c.id.clone()),
        ("Name", c.name.clone()),
        ("Image", c.image.clone()),
        ("State", c.state.word().to_string()),
        ("Status", c.status()),
        (
            "Pid",
            c.pid.map(|p| p.to_string()).unwrap_or_else(|| "0".into()),
        ),
        (
            "LauncherPid",
            c.launcher_pid
                .map(|p| p.to_string())
                .unwrap_or_else(|| "0".into()),
        ),
        // ⛔ A dash where there is no code, never a zero. AGENTS.md
        // absolute 3, and `wait` refuses for the same reason.
        (
            "ExitCode",
            c.exit_code
                .map(|p| p.to_string())
                .unwrap_or_else(|| "-".into()),
        ),
        ("Created", c.created_at.clone()),
        (
            "StartedAt",
            c.started_at.clone().unwrap_or_else(|| "-".into()),
        ),
        (
            "FinishedAt",
            c.finished_at.clone().unwrap_or_else(|| "-".into()),
        ),
        ("RootfsPath", c.rootfs.clone()),
        (
            "LogPath",
            podvm_supervise::table::log_path(s, &c.id)
                .display()
                .to_string(),
        ),
        ("Rung", c.rung.clone()),
        // ⭐ T-0804 rule 3. The mode a caller reads includes the shims, or the
        // report says devices exist when they do not.
        ("Complete.Fixups", c.completion_note()),
        ("Complete.Degraded", c.completion_degraded.to_string()),
        ("Command", c.argv.join(" ")),
        ("Exec.Mode", crate::images::EXEC_MODE.to_string()),
        ("Exec.Shares", crate::images::EXEC_SHARES.to_string()),
        // ⛔ SAID, NOT IMPLIED. TODO/supervise.md T-1708: the launcher is the
        // child subreaper of the payload's tree and kills each descendant at
        // the end. podvm has no PID namespace, so a process that reparents
        // above the launcher stays out of reach, and a caller reads this
        // rather than assuming docker's containment.
        (
            "Contains",
            "the payload's tree: the launcher is its child subreaper and kills each \
             descendant at the end; a process that reparents above the launcher is \
             outside its reach"
                .to_string(),
        ),
        ("Noticed", c.noticed.clone().unwrap_or_else(|| "-".into())),
        // TODO/supervise.md T-1708: the end of the last run, as the launcher
        // measured it. A dash where no run ended, never a zero.
        (
            "End.Reason",
            end_field(c, |e| reason_word(e.reason).to_string()),
        ),
        ("End.RunMs", end_field(c, |e| e.run_ms.to_string())),
        ("End.UserMs", end_field(c, |e| e.user_ms.to_string())),
        ("End.SystemMs", end_field(c, |e| e.system_ms.to_string())),
        ("End.MaxRssKiB", end_field(c, |e| e.max_rss_kib.to_string())),
        (
            "End.Residual",
            end_field(c, |e| residual_word(e.residual_processes)),
        ),
        // T-0708: how many times the preloaded tier emulated an operation
        // rather than running the kernel's answer. A dash where the tier
        // never ran; `inspect --format` reads these names, `ps` does not.
        // T-0413 adds the served /proc answers behind the same memo.
        ("Interpose.Emulated.mknod", mknod),
        ("Interpose.Emulated.mount", mount),
        ("Interpose.Emulated.unshare", unshare),
        ("Interpose.Emulated.clone", clone),
        ("Interpose.Emulated.procfs", procfs),
    ]
}
