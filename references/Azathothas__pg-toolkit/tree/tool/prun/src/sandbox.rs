//! Standing in for the sandbox helper a payload runs on itself.
//!
//! ⭐ A bundled browser engine sandboxes its own renderer by re-executing a
//! helper with a long option list, and that list was written for a program
//! installed on the host. Inside the sandbox the bundle's directory is not
//! mounted, so the payload's own libraries are gone and the helper fails in a
//! way that reads like a graphics bug.
//!
//! ⛔ Three of the injections are not conveniences:
//!
//!   - the bundle's own directory, bound at the same path, or nothing inside
//!     the sandbox can open a bundled library;
//!   - `/proc`, because the launcher inside the sandbox resolves itself
//!     through `/proc/self/exe` and has no other way to find its bundle;
//!   - the filter option is REMOVED, because the filter the payload composed
//!     blocks the calls that resolution needs.
//!
//! ⚠ Removing a filter widens what the sandboxed process may do, and that is
//! a real cost stated rather than hidden: it is the payload's own filter, aimed
//! at a helper on the host, and the alternative is a sandbox that cannot start.
//! An option list of ours would compose a filter instead, and that is the
//! collector's entry rather than the launcher's.
//!
//! SPDX-License-Identifier: 0BSD

use std::path::Path;

/// How many arguments each option of the helper takes. ⛔ Getting one of these
/// wrong does not fail loudly: the walk to find where the command begins ends
/// up in the middle of the option list, and the injection lands between an
/// option and its own argument.
fn arity(arg: &str) -> usize {
    match arg {
        "--overlay" => 3,
        "--bind" | "--ro-bind" | "--bind-try" | "--ro-bind-try" | "--dev-bind"
        | "--dev-bind-try" | "--bind-data" | "--ro-bind-data" | "--file" | "--ro-file"
        | "--dev-mknod" | "--symlink" | "--chmod" | "--bind-fd" | "--ro-bind-fd"
        | "--setenv" => 2,
        "--tmpfs" | "--proc" | "--dev" | "--devpts" | "--mqueue" | "--hostname"
        | "--seccomp" | "--block-fd" | "--userns" | "--uid" | "--gid" | "--chdir"
        | "--unsetenv" | "--lock-file" | "--sync-fd" | "--info-fd" | "--json-status-fd"
        | "--add-seccomp-fd" | "--add-feature" | "--args" | "--dir" | "--remount-ro"
        | "--perms" | "--size" | "--argv0" | "--overlay-src" | "--tmp-overlay"
        | "--ro-overlay" | "--exec-label" | "--file-label" | "--userns-block-fd"
        | "--pidns" => 1,
        _ => 0,
    }
}

/// Where the command begins, after the options.
pub fn command_at(args: &[String]) -> usize {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--" {
            return i;
        }
        if args[i].starts_with('-') {
            i += 1 + arity(&args[i]);
            continue;
        }
        return i;
    }
    args.len()
}

/// The options to add before the command.
pub fn injections(bundle: &str, path: &str, tmp_is_needed: bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut opt = |a: &str, b: &str, c: &str| {
        out.push(a.into());
        if !b.is_empty() {
            out.push(b.into());
        }
        if !c.is_empty() {
            out.push(c.into());
        }
    };
    opt("--proc", "/proc", "");
    if !bundle.is_empty() {
        opt("--bind", bundle, bundle);
    }
    if tmp_is_needed {
        // ⚠ Only when something the bundle installed actually lives there.
        // Binding the whole of /tmp into a sandbox is a real widening, and the
        // launcher this replaces does it unconditionally.
        opt("--bind", "/tmp", "/tmp");
    }
    if !bundle.is_empty() {
        opt("--setenv", "PRUN_DIR", bundle);
        opt("--setenv", "SHARUN_DIR", bundle);
        opt("--setenv", "APPDIR", bundle);
    }
    if !path.is_empty() {
        opt("--setenv", "PATH", path);
    }
    out
}

/// Remove an option and its arguments wherever it appears in the option list.
pub fn strip(args: &[String], drop: &str) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        if args[i] == drop {
            // ⛔ The arity of the option BEING DROPPED. Reading it from the
            // previous element indexes past the start on the first argument,
            // and on every later one it drops whatever the option before this
            // one takes - which is a filter left in place and an unrelated
            // argument removed.
            i += 1 + arity(drop);
            continue;
        }
        out.push(args[i].clone());
        i += 1;
    }
    out
}

/// Point a command at the bundle's own copy of the program, when it has one.
///
/// ⛔ Only the executable bit decides, and only these three directories are
/// looked in. A rewrite that searched the whole bundle could send a helper at
/// a data file with the right name.
pub fn remap(path: &str, bundle: &str) -> Option<String> {
    if !path.starts_with('/') || bundle.is_empty() {
        return None;
    }
    let base = Path::new(path).file_name()?.to_string_lossy().into_owned();
    if base.is_empty() {
        return None;
    }
    for dir in ["bin", "lib", "libexec"] {
        let candidate = Path::new(bundle).join(dir).join(&base);
        if crate::sys::executable(&candidate) {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

/// The options a caller passed on a file descriptor instead of on the command
/// line, rewritten and handed back on a descriptor of ours.
///
/// ⛔ This path is not optional. A bundled browser engine passes its whole
/// option list this way, so a launcher that only rewrote `argv` would inject
/// nothing at all for the payload that needs it most - and would look correct,
/// because the injection would be there in the other case.
///
/// ⚠ The command moves OUT of the descriptor and onto the command line. It has
/// to: the remap has to see it, and a descriptor that carried it would need
/// rewriting twice.
pub fn rewrite_fd(
    args: &[String],
    at: usize,
    fd: std::os::unix::io::RawFd,
    bundle: &str,
    path: &str,
    tmp_is_needed: bool,
    scratch: Option<&Path>,
) -> Option<(Vec<String>, std::fs::File)> {
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::unix::io::{AsRawFd, FromRawFd};

    let mut src = unsafe { std::fs::File::from_raw_fd(fd) };
    let mut buf = Vec::new();
    src.read_to_end(&mut buf).ok()?;
    // ⚠ The caller's descriptor is theirs; hand it back rather than closing it
    // when this File goes out of scope.
    std::mem::forget(src);

    let from_fd: Vec<String> = buf
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    let cmd = command_at(&from_fd);
    let mut opts = strip(&from_fd[..cmd], "--seccomp");
    opts.extend(injections(bundle, path, tmp_is_needed));

    // ⚠ An anonymous file first, and a private one second. A kernel without
    // the first is old rather than broken, and a launcher that gave up there
    // would drop the whole descriptor route on it.
    let mut anon = match crate::sys::anon_file("prun-sandbox-args") {
        Some(f) => f,
        None => crate::sys::scratch_file(scratch?, "sandbox-args")?,
    };
    for o in &opts {
        anon.write_all(o.as_bytes()).ok()?;
        anon.write_all(&[0]).ok()?;
    }
    anon.seek(SeekFrom::Start(0)).ok()?;

    let mut out: Vec<String> = Vec::with_capacity(args.len() + from_fd.len());
    for (i, a) in args.iter().enumerate() {
        if i == at {
            out.push("--args".into());
            out.push(anon.as_raw_fd().to_string());
        } else if i != at + 1 {
            out.push(a.clone());
        }
    }
    out.extend_from_slice(&from_fd[cmd..]);
    let target = command_at(&out);
    let target = if out.get(target).map(|s| s == "--").unwrap_or(false) { target + 1 } else { target };
    if let Some(c) = out.get(target).cloned() {
        if let Some(rm) = remap(&c, bundle) {
            out[target] = rm;
        }
    }
    Some((out, anon))
}

/// Where a `--args` option sits, if the caller used one.
pub fn args_fd(args: &[String]) -> Option<(usize, std::os::unix::io::RawFd)> {
    for (i, a) in args.iter().enumerate() {
        if a == "--args" && i + 1 < args.len() {
            if let Ok(fd) = args[i + 1].parse::<std::os::unix::io::RawFd>() {
                return Some((i, fd));
            }
        }
    }
    None
}

/// Rewrite the whole argument list: strip the filter, inject, and remap the
/// command.
pub fn rewrite(args: &[String], bundle: &str, path: &str, tmp_is_needed: bool) -> Vec<String> {
    let stripped = strip(args, "--seccomp");
    let at = command_at(&stripped);
    let inj = injections(bundle, path, tmp_is_needed);
    let mut out = Vec::with_capacity(stripped.len() + inj.len());
    out.extend_from_slice(&stripped[..at]);
    out.extend(inj);
    out.extend_from_slice(&stripped[at..]);
    // The command is now at `at + inj.len()`, or after a `--`.
    let cmd = command_at(&out);
    let target = if out.get(cmd).map(|s| s == "--").unwrap_or(false) { cmd + 1 } else { cmd };
    if let Some(c) = out.get(target).cloned() {
        if let Some(r) = remap(&c, bundle) {
            out[target] = r;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_command_is_found_past_the_options() {
        assert_eq!(command_at(&v(&["--bind", "/a", "/b", "/usr/bin/prog", "-x"])), 3);
        assert_eq!(command_at(&v(&["--overlay", "a", "b", "c", "prog"])), 4);
        assert_eq!(command_at(&v(&["--dev", "/dev", "--", "prog"])), 2);
        assert_eq!(command_at(&v(&["--unshare-all", "prog"])), 1);
        assert_eq!(command_at(&v(&["--bind", "/a", "/b"])), 3);
    }

    // ⛔ The arity table is what makes the walk correct, and this is the case
    // that goes wrong silently when an entry is missing: a two-argument option
    // read as taking none puts the injection between it and its argument.
    #[test]
    fn an_option_argument_is_never_mistaken_for_the_command() {
        let args = v(&["--setenv", "HOME", "/root", "/usr/bin/prog"]);
        assert_eq!(command_at(&args), 3);
    }

    #[test]
    fn the_filter_and_its_argument_go_together() {
        assert_eq!(strip(&v(&["--seccomp", "9", "prog"]), "--seccomp"), v(&["prog"]));
        assert_eq!(
            strip(&v(&["--bind", "/a", "/b", "--seccomp", "9", "prog"]), "--seccomp"),
            v(&["--bind", "/a", "/b", "prog"])
        );
    }

    #[test]
    fn the_bundle_is_bound_and_the_process_table_is_mounted() {
        let inj = injections("/mnt/app", "/mnt/app/bin:/usr/bin", false);
        assert!(inj.windows(3).any(|w| w == ["--bind", "/mnt/app", "/mnt/app"]));
        assert!(inj.windows(2).any(|w| w == ["--proc", "/proc"]));
        assert!(!inj.iter().any(|a| a == "/tmp"), "tmp is bound only when it is needed");
        let inj = injections("/mnt/app", "", true);
        assert!(inj.windows(3).any(|w| w == ["--bind", "/tmp", "/tmp"]));
    }

    #[test]
    fn a_rewrite_puts_the_injection_before_the_command() {
        let got = rewrite(&v(&["--unshare-all", "--seccomp", "7", "/usr/bin/prog", "-v"]), "/app", "", false);
        let at = command_at(&got);
        assert_eq!(got[at], "/usr/bin/prog");
        assert!(!got.iter().any(|a| a == "--seccomp"));
        assert_eq!(got[0], "--unshare-all");
    }
}
