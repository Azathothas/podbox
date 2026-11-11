//! The values of the flags of the hardening line (TODO/cli.md T-1655),
//! `--podvm-cpus` (T-1654) and the file of `--env-file` (T-0801).

use super::*;

/// The seconds of `--timeout`: a whole number over 0. TODO/cli.md T-1655.
pub(super) fn timeout(verb: &str, a: &str) -> Result<u64, i32> {
    match a.parse::<u64>() {
        Ok(n) if n > 0 && n <= u64::MAX / 1000 => Ok(n),
        _ => {
            eprintln!("podvm {verb}: --timeout takes whole seconds over 0, not {a:?}");
            Err(EXIT_FLAG_ERROR)
        }
    }
}

/// One `--ulimit`, checked now so that a bad spec is a flag error before
/// anything is fetched. TODO/cli.md T-1655.
pub(super) fn ulimit(verb: &str, a: &str) -> Result<String, i32> {
    match podvm_enter::ulimit(a) {
        Ok(_) => Ok(a.to_string()),
        Err(why) => {
            eprintln!("podvm {verb}: --ulimit {why}");
            Err(EXIT_FLAG_ERROR)
        }
    }
}

/// One `--security-opt`: `no-new-privileges` (also `:true`, `=true`) sets
/// the bit in the child; each other option is refused by name, because no
/// security backend of podvm takes it. TODO/cli.md T-1655.
pub(super) fn security_opt(verb: &str, a: &str) -> Result<bool, i32> {
    match a {
        "no-new-privileges" | "no-new-privileges:true" | "no-new-privileges=true" => Ok(true),
        "no-new-privileges:false" | "no-new-privileges=false" => Ok(false),
        other => {
            eprintln!(
                "podvm {verb}: --security-opt {other:?} is refused: podvm takes \
                 no-new-privileges only; seccomp, apparmor, label and the rest have \
                 no backend here (TODO/cli.md T-1655)"
            );
            Err(EXIT_FLAG_ERROR)
        }
    }
}

/// One `--cap-drop`: `ALL` sets no-new-privileges in the child, which is
/// what podvm can enforce without a privilege; a named capability is
/// refused by name, because podvm does not change the capability sets.
/// TODO/cli.md T-1655.
pub(super) fn cap_drop(verb: &str, a: &str) -> Result<bool, i32> {
    if a.eq_ignore_ascii_case("all") {
        return Ok(true);
    }
    eprintln!(
        "podvm {verb}: --cap-drop {a:?} is refused: podvm takes ALL only, as \
         no-new-privileges in the payload; it does not change the capability \
         sets (TODO/cli.md T-1655)"
    );
    Err(EXIT_FLAG_ERROR)
}

/// One `--tmpfs PATH[:OPTIONS]`. TODO/cli.md T-1655. PATH is an absolute
/// guest path; `rw`, `ro`, `nosuid`, `nodev`, `exec`, `noexec`, `size=`
/// and `mode=` are read (`ro` makes it read-only; the others have nothing
/// to act on, because the directory is not a mount); each other option is
/// refused by name.
pub(super) fn tmpfs(verb: &str, spec: &str, o: &mut Opts) -> Result<(), i32> {
    let (path, opts) = spec.split_once(':').unwrap_or((spec, ""));
    let refuse = |why: String| {
        eprintln!("podvm {verb}: --tmpfs {spec:?}: {why}");
        EXIT_FLAG_ERROR
    };
    if !path.starts_with('/') || path == "/" {
        return Err(refuse("the path must be absolute and not /".to_string()));
    }
    let path = path.trim_end_matches('/').to_string();
    for opt in opts.split(',').filter(|x| !x.is_empty()) {
        match opt.split_once('=').map_or(opt, |(k, _)| k) {
            "rw" | "nosuid" | "nodev" | "exec" | "noexec" | "size" | "mode" => {}
            "ro" => o.ro_volumes.push(path.clone()),
            other => return Err(refuse(format!("the option {other:?} is not taken"))),
        }
    }
    o.tmpfs.push(path);
    Ok(())
}

/// One `-v`: the pair, and its guest path in the read-only list where the
/// options hold `ro`. TODO/enter.md T-1653, TODO/cli.md T-1655.
pub(super) fn volume(verb: &str, spec: &str, o: &mut Opts) -> std::result::Result<(), i32> {
    let (v, ro) = crate::lifecycle::volume::parse(spec).map_err(|why| {
        eprintln!("podvm {verb}: -v {spec:?}: {why}");
        EXIT_FLAG_ERROR
    })?;
    if ro {
        o.ro_volumes.push(v.guest.trim_end_matches('/').to_string());
    }
    o.volumes.push(v);
    Ok(())
}

/// `--env-file`: docker's file of `KEY=VALUE` lines, loaded at the flag's
/// position so a later `-e` wins over the file and a later file over an
/// earlier flag, the same rule `-e` already documents. TODO/cli.md T-0801.
///
/// The file is bounded (1 MiB) and must be UTF-8: buffering an unbounded
/// body or guessing at bytes is the shape
/// `docs/conventions/forbidden-patterns.md` refuses. A line without `=`
/// is refused naming its number rather than skipped, because a skipped
/// line is a variable the caller thinks is set.
pub(super) fn read_env_file(verb: &str, path: &str) -> std::result::Result<Vec<String>, i32> {
    const MAX_ENV_FILE: u64 = 1_048_576;
    let meta = std::fs::metadata(path).map_err(|e| {
        eprintln!("podvm {verb}: --env-file {path:?}: {e}");
        EXIT_FLAG_ERROR
    })?;
    if !meta.is_file() {
        eprintln!("podvm {verb}: --env-file {path:?} is not a file");
        return Err(EXIT_FLAG_ERROR);
    }
    if meta.len() > MAX_ENV_FILE {
        eprintln!(
            "podvm {verb}: --env-file {path:?} is {} bytes, over the 1048576-byte ceiling",
            meta.len()
        );
        return Err(EXIT_FLAG_ERROR);
    }
    let text = std::fs::read_to_string(path).map_err(|e| {
        eprintln!("podvm {verb}: --env-file {path:?}: {e}");
        EXIT_FLAG_ERROR
    })?;
    let mut out = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            eprintln!(
                "podvm {verb}: --env-file {path:?} line {} has no `=`: {line:?}",
                n + 1
            );
            return Err(EXIT_FLAG_ERROR);
        };
        let key = k.trim();
        if key.is_empty() {
            eprintln!(
                "podvm {verb}: --env-file {path:?} line {} has no name: {line:?}",
                n + 1
            );
            return Err(EXIT_FLAG_ERROR);
        }
        let mut value = v.trim().to_string();
        if value.len() >= 2
            && value.starts_with(['\'', '"'])
            && value.ends_with(value.as_bytes()[0] as char)
        {
            value.remove(0);
            value.pop();
        }
        out.push(format!("{key}={value}"));
    }
    Ok(out)
}

/// The vCPUs of `--podvm-cpus`: a whole number from 1 to 64.
/// TODO/cli.md T-1654.
pub(super) fn cpus(verb: &str, a: &str) -> std::result::Result<u32, i32> {
    match a.parse::<u32>() {
        Ok(n) if (1..=64).contains(&n) => Ok(n),
        _ => {
            eprintln!("podvm {verb}: --podvm-cpus takes a whole number from 1 to 64, not {a:?}");
            Err(EXIT_FLAG_ERROR)
        }
    }
}
