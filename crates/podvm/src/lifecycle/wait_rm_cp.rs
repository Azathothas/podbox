use super::*;

/// ⚠ By name or by number, and an unknown one is refused rather than defaulted:
/// sending the wrong signal is not something a caller can notice.
pub(super) fn signal_number(v: &str) -> Option<i32> {
    if let Ok(n) = v.parse::<i32>() {
        return (n > 0 && n < 65).then_some(n);
    }
    let name = v.trim_start_matches("SIG").to_ascii_uppercase();
    Some(match name.as_str() {
        "HUP" => 1,
        "INT" => 2,
        "QUIT" => 3,
        "KILL" => 9,
        "USR1" => 10,
        "USR2" => 12,
        "TERM" => 15,
        "CONT" => 18,
        "STOP" => 19,
        _ => return None,
    })
}

/// `podvm wait <container>...`
pub fn wait(args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded = match crate::parity::expand("wait", args) {
        Ok(a) => a,
        Err(member) => {
            return crate::parity::refuse_member(
                "wait",
                &member,
                "usage: podvm wait <container> [container...]",
            )
        }
    };
    let args: &[String] = &expanded;
    if let Some(c) =
        crate::parity::admit_all("wait", args, "usage: podvm wait <container> [container...]")
    {
        return c;
    }
    const WAIT_USAGE: &str = "usage: podvm wait <container> [container...]";
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        return super::usage(args.is_empty(), WAIT_USAGE);
    }
    for want in args {
        if want.starts_with('-') {
            if let Err(c) =
                crate::parity::admit("wait", want, "usage: podvm wait <container> [container...]")
            {
                return c;
            }
            return crate::parity::no_arm("wait", want);
        }
    }
    let s = match store() {
        Ok(s) => s,
        Err(c) => return c,
    };
    let mut code = 0;
    for want in args {
        match podvm_supervise::wait(&s, want, WAIT_BOUND_MS).map(|(c, n)| settle(&s, c, n)) {
            Ok((_, Some(n))) => println!("{n}"),
            Ok((c, None)) => {
                // ⛔ THE BOUND, OR AN UNWATCHED EXIT, AND NEVER A NUMBER. A `0`
                // printed here for a container podvm did not see end is the
                // lie this whole crate is arranged to refuse.
                eprintln!(
                    "podvm wait: {} has no exit code podvm measured: it is {} \
                     (TODO/supervise.md T-0604)",
                    c.name,
                    c.status()
                );
                code = EXIT_RUNTIME_ERROR;
            }
            Err(e) => code = fail("wait", e),
        }
    }
    code
}

/// ⛔ TODO/supervise.md T-1720. A wait that ends with no code while the
/// record still says running is a launcher that went between its answer and
/// its write of the record: read the record again until it has the code or
/// is not running, within [`SETTLE_MS`]. PodUtils/podrunner#49 measured the
/// "it is Up" answer with a payload that ends at once.
pub(super) fn settle(
    s: &podvm_image::Store,
    c: podvm_supervise::table::Container,
    code: Option<i32>,
) -> (podvm_supervise::table::Container, Option<i32>) {
    if code.is_some() || c.state != podvm_supervise::table::State::Running {
        return (c, code);
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(SETTLE_MS);
    let mut last = c;
    while std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let Ok(now) = podvm_supervise::get(s, &last.id) else {
            break;
        };
        if now.exit_code.is_some() || now.state != podvm_supervise::table::State::Running {
            let code = now.exit_code;
            return (now, code);
        }
        last = now;
    }
    (last, None)
}

/// How long `wait` reads the record again after the launcher went.
const SETTLE_MS: u64 = 10_000;

/// `podvm rm [-f] <container>...`
pub fn rm(args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded = match crate::parity::expand("rm", args) {
        Ok(a) => a,
        Err(member) => {
            return crate::parity::refuse_member(
                "rm",
                &member,
                "usage: podvm rm [-f|--force] <container> [container...]",
            )
        }
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(
        "rm",
        args,
        "usage: podvm rm [-f|--force] <container> [container...]",
    ) {
        return c;
    }
    let mut force = false;
    let mut names = Vec::new();
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                println!("usage: podvm rm [-f|--force] <container> [container...]");
                return 0;
            }
            "-f" | "--force" => force = true,
            "-v" | "--volumes" => {}
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(
                    "rm",
                    other,
                    "usage: podvm rm [-f|--force] <container> [container...]",
                ) {
                    return c;
                }
                return crate::parity::no_arm("rm", other);
            }
            other => names.push(other.to_string()),
        }
    }
    if names.is_empty() {
        eprintln!("usage: podvm rm [-f|--force] <container> [container...]");
        return EXIT_CLI_ERROR;
    }
    let s = match store() {
        Ok(s) => s,
        Err(c) => return c,
    };
    let mut code = 0;
    for want in &names {
        code = match remove_tree(&s, want, force) {
            Ok(None) => code,
            Ok(Some(c)) => c,
            Err(e) => fail("rm", e),
        };
    }
    code
}

/// `rm` of one container. ⛔ TODO/supervise.md T-1708: `rm -f` of a running
/// container stops it with SIGKILL through its launcher. Where processes of
/// the tree stayed after the kill, the record stays and the verb exits 125
/// naming the count; a later `rm` removes it and names the count again.
pub(super) fn remove_tree(
    s: &podvm_image::Store,
    want: &str,
    force: bool,
) -> Result<Option<i32>, podvm_supervise::Error> {
    let c = podvm_supervise::get(s, want)?;
    let killed_now = force && c.state == podvm_supervise::table::State::Running;
    let c = if killed_now {
        podvm_supervise::stop(s, &c.id, 0)?;
        podvm_supervise::get(s, &c.id)?
    } else {
        c
    };
    let stayed = match c.end.as_ref().map(|e| e.residual_processes) {
        Some(podvm_supervise::Residual::Count(n)) if n > 0 => Some(n),
        _ => None,
    };
    if let Some(n) = stayed {
        if killed_now {
            eprintln!(
                "podvm rm: {}: {n} process(es) of its tree stayed after SIGKILL, so \
                 podvm keeps the record (`podvm inspect --format '{{{{.End.Residual}}}}' \
                 {}`); a later rm removes it (TODO/supervise.md T-1708)",
                c.name, c.name
            );
            return Ok(Some(EXIT_RUNTIME_ERROR));
        }
        eprintln!(
            "podvm rm: {}: {n} process(es) of its tree stayed after its end, and podvm \
             no longer tracks them (TODO/supervise.md T-1708)",
            c.name
        );
    }
    let c = podvm_supervise::remove(s, &c.id, force)?;
    println!("{}", c.name);
    Ok(None)
}

/// `podvm cp <src> <dest>`, where one side is `<name>:<path>` and the
/// name is a container or, failing that, an image.
pub(super) const CP_USAGE: &str = "\
usage: podvm cp [-r|--recursive] <container|image>:<path> <dest>
       podvm cp [-r|--recursive] <src> <container|image>:<path>

  -r, --recursive  copy a directory tree. A file copies with or without
                   it; a directory without it is refused naming the flag.

  The name before the colon is a container first (as before), then an
  image, whose rootfs is extracted if needed. A directory copies its
  contents into DEST: created where missing, and it must be a directory
  where present. Symlinks are replicated as symlinks, never followed,
  and every rootfs-side path passes the same containment gate as a
  single file; special files are refused by name rather than recreated
  (TODO/cli.md T-1323).
";

/// What `cp` was asked for.
pub(super) struct CpArgs {
    pub(super) recursive: bool,
    pub(super) a: String,
    pub(super) b: String,
}

pub(super) fn parse_cp(args: &[String]) -> std::result::Result<CpArgs, i32> {
    let mut recursive = false;
    let mut positionals: Vec<String> = Vec::new();
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{CP_USAGE}");
                return Err(0);
            }
            "-r" | "--recursive" => recursive = true,
            other if other.starts_with('-') => {
                crate::parity::admit("cp", other, CP_USAGE)?;
                return Err(crate::parity::no_arm("cp", other));
            }
            other => positionals.push(other.to_string()),
        }
    }
    if positionals.len() != 2 {
        eprintln!("podvm cp: takes exactly two paths, one of them <name>:<path>");
        return Err(EXIT_CLI_ERROR);
    }
    let mut it = positionals.into_iter();
    Ok(CpArgs {
        recursive,
        a: it.next().expect("checked"),
        b: it.next().expect("checked"),
    })
}

/// `podvm cp <src> <dest>`, where one side is `<container>:<path>`.
pub fn cp(args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded = match crate::parity::expand("cp", args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member("cp", &member, CP_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all("cp", args, CP_USAGE) {
        return c;
    }
    let o = match parse_cp(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let s = match store() {
        Ok(s) => s,
        Err(c) => return c,
    };
    let (a, b) = (o.a.as_str(), o.b.as_str());
    let (from_named, to_named) = (named(a), named(b));
    match (from_named, to_named) {
        (true, true) => {
            eprintln!("podvm cp: copying between two named filesystems is not implemented");
            EXIT_RUNTIME_ERROR
        }
        (false, false) => {
            eprintln!("podvm cp: one of the two paths has to be <name>:<path>");
            EXIT_CLI_ERROR
        }
        (true, false) => copy_named(&s, a, std::path::Path::new(b), true, o.recursive),
        (false, true) => copy_named(&s, b, std::path::Path::new(a), false, o.recursive),
    }
}

/// A side addresses a named filesystem: a container (`split`, no slashes)
/// or an image reference (slashes, tags and all).
fn named(arg: &str) -> bool {
    split(arg).is_some() || split_image(arg).is_some()
}

/// Copy through a name that is a container first and an image second.
/// Container-first keeps every existing invocation resolving exactly as
/// before; the image half splits on the LAST colon, because a tag carries
/// one (`alpine:3.20:/etc/hosts`).
fn copy_named(
    s: &podvm_image::Store,
    arg: &str,
    outside: &std::path::Path,
    out_of: bool,
    recursive: bool,
) -> i32 {
    if let Some((name, inside)) = split(arg)
        && let Ok(c) = podvm_supervise::get(s, &name)
    {
        // T-1775: `cp` joins the mounts of a running payload of the
        // namespace rung, as `exec` does (T-1765); `setns` moves the cwd.
        let outside = std::path::absolute(outside).unwrap_or_else(|_| outside.to_path_buf());
        if c.state == podvm_supervise::table::State::Running && c.rung == "namespace" {
            crate::exec::enter::join_mounts(c.pid);
        }
        return copy_one(
            std::path::Path::new(&c.rootfs),
            &inside,
            &outside,
            out_of,
            recursive,
        );
    }
    let (image, inside) = match arg.rsplit_once(':') {
        Some((image, inside)) if !image.is_empty() && !inside.is_empty() => (image, inside),
        _ => ("", ""),
    };
    let record = match s.find_one(image) {
        Ok(r) => r,
        Err(image_err) => {
            // TODO/supervise.md T-1805: one refusal, with the code of the
            // image error (1 for a name that nothing has), not two and 125.
            let Some((name, _)) = split(arg) else {
                eprintln!("podvm cp: one of the two paths has to be <name>:<path>");
                return EXIT_CLI_ERROR;
            };
            eprintln!("podvm cp: no container is named {name}, and {image_err}");
            return image_err.exit_code();
        }
    };
    // ⚠ The image lock for the whole copy: the rootfs below must not be
    // deleted between the extraction check and the last byte, which is
    // T-0204's hold and T-1322's query answering together.
    let _held = match s.hold(&record) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("podvm cp: {e}");
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    if !podvm_extract::is_extracted(s, &record.manifest_digest)
        && let Err(c) = crate::run::extract_now("cp", s, &record)
    {
        return c;
    }
    let (rootfs, _) = podvm_extract::paths(s, &record.manifest_digest);
    copy_one(&rootfs, inside, outside, out_of, recursive)
}

/// `name:/path` where the name is not a Windows drive letter or a bare path.
pub(super) fn split(arg: &str) -> Option<(String, String)> {
    let (name, path) = arg.split_once(':')?;
    if name.is_empty() || path.is_empty() || name.contains('/') {
        return None;
    }
    Some((name.to_string(), path.to_string()))
}

/// `reference:path` where the reference may carry a registry, a tag, or
/// both. Slashes allowed, unlike a container name: this is the image half
/// of addressing, and `copy_named` tries the container half first.
fn split_image(arg: &str) -> Option<(String, String)> {
    let (image, path) = arg.rsplit_once(':')?;
    if image.is_empty() || path.is_empty() {
        return None;
    }
    Some((image.to_string(), path.to_string()))
}

pub(super) fn copy_one(
    root: &std::path::Path,
    inside: &str,
    outside: &std::path::Path,
    out_of: bool,
    recursive: bool,
) -> i32 {
    let joined = root.join(inside.trim_start_matches('/'));
    // ⚠ Stat BEFORE gating: a symlink source is gated on its parent, never
    // resolved; `within` resolves, and `/etc/mtab -> /proc/self/mounts` of
    // a distro escapes nothing (TODO/extract.md T-0305).
    let src_side: &std::path::Path = if out_of { &joined } else { outside };
    let src_is_link = is_link(src_side);
    // ⛔ THE SAME CONTAINMENT GATE as each other path: `cp` is the verb most
    // likely to be handed a `../../etc/shadow`.
    let target = if src_is_link && out_of {
        match joined.parent() {
            Some(parent) => match podvm_image::contain::within(root, parent) {
                Ok(_) => joined.clone(),
                Err(e) => {
                    eprintln!("podvm cp: {e}");
                    return EXIT_RUNTIME_ERROR;
                }
            },
            None => {
                eprintln!("podvm cp: {} has no parent to gate", joined.display());
                return EXIT_RUNTIME_ERROR;
            }
        }
    } else {
        match podvm_image::contain::within(root, &joined) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("podvm cp: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        }
    };
    let (src, dst) = if out_of {
        (target, outside.to_path_buf())
    } else {
        (outside.to_path_buf(), target)
    };
    // ⛔ Never write through a pre-existing symlink: the path is gated but
    // the write would land where the link points.
    if is_link(&dst) {
        eprintln!(
            "podvm cp: {} is a symlink; writing through one is refused",
            dst.display()
        );
        return EXIT_RUNTIME_ERROR;
    }
    if !out_of && let Err(msg) = clear_of_symlinks(root, &dst) {
        eprintln!("podvm cp: {msg}");
        return EXIT_RUNTIME_ERROR;
    }
    // ⚠ Stat the SOURCE side: a missing destination directory on the way
    // in must still read as a directory copy. (`src_is_link` above already
    // decided the gate; this match decides the copy shape.)
    match std::fs::symlink_metadata(&src).map(|m| m.file_type()) {
        Ok(t) if t.is_symlink() => {
            return replicate_link(&src, &dst);
        }
        Ok(t) if t.is_dir() => {
            if !recursive {
                eprintln!(
                    "podvm cp: {} is a directory; pass -r to copy it",
                    src.display()
                );
                return EXIT_FLAG_ERROR;
            }
            return copy_tree(&src, &dst, root, out_of);
        }
        Ok(t) if !t.is_file() => {
            eprintln!(
                "podvm cp: {} is not a file or directory; special files are refused",
                src.display()
            );
            return EXIT_RUNTIME_ERROR;
        }
        _ => {}
    }
    if let Some(parent) = dst.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::copy(&src, &dst) {
        Ok(n) => {
            eprintln!(
                "podvm cp: {} bytes {} -> {}",
                n,
                src.display(),
                dst.display()
            );
            0
        }
        Err(e) => {
            eprintln!("podvm cp: {}: {e}", src.display());
            EXIT_RUNTIME_ERROR
        }
    }
}
