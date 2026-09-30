//! Starting the payload, and the three routes that reach it.
//!
//! ⭐ A bundle's payload has to be started against the bundle's OWN loader and
//! the bundle's own library path, and there are exactly three ways to do that.
//! Which one applies is a property of the payload, not a preference:
//!
//! | the payload | the route | what the payload sees as itself |
//! | --- | --- | --- |
//! | has no interpreter | started directly | itself |
//! | names an interpreter this launcher provides | the loader is installed where it says, then the payload is started directly | itself |
//! | anything else | the loader is mapped here and started as the program, with the payload as its argument | this launcher's hardlink |
//!
//! ⭐ **The third row maps the loader in userland, as the launcher this
//! replaces does**, so the payload's `/proc/self/exe` stays the launcher's
//! hardlink, which is named after the program. `uexec` does the mapping. A
//! program that finds its data next to `/proc/self/exe`, and a window class
//! read from it, then see the program's name and not the loader's.
//!
//! ⚠ **When `uexec` refuses, the loader is started with `execve`**, and
//! `/proc/self/exe` is then the loader. The report names the refusal.
//! `PRUN_LOADER_EXEC=execve` asks for that path directly. brioche's packed
//! launcher makes the same choice (docs/research/brioche.md), and
//! scripts/common/uexec-across.sh measures the loader started this way against
//! the kernel on the eleven pinned loaders. `PRUN_REPORT=1` names the route.
//!
//! SPDX-License-Identifier: 0BSD

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::envx;
use crate::loaderdir;
use crate::report::Report;

/// How the payload is started.
#[derive(Debug, PartialEq, Eq)]
pub enum Route {
    /// Nothing to load: the payload is its own program.
    Direct,
    /// The payload names an interpreter the launcher installs for it.
    Installed,
    /// The loader is the program, and the payload is its argument.
    Loader,
}

impl Route {
    pub fn name(&self) -> &'static str {
        match self {
            Route::Direct => "direct: the payload has no interpreter",
            Route::Installed => "installed: the loader was put where the payload names it",
            Route::Loader => "loader: the loader is started with the payload as its argument",
        }
    }
}

/// Everything a start needs.
pub struct Start {
    /// The payload.
    pub bin: PathBuf,
    /// What the payload should see as its own `argv[0]`.
    pub arg0: PathBuf,
    /// The payload's own arguments.
    pub args: Vec<String>,
    /// The bundle's loader.
    pub loader: PathBuf,
    /// The library path the loader is to use.
    pub library_path: String,
    /// Objects to load before everything else.
    pub preload: Vec<String>,
}

/// Decide the route and take it. ⛔ This function returns only on failure: a
/// successful start replaces the process.
pub fn run(s: &Start, route: Route, r: &mut Report) -> std::io::Error {
    r.note(&format!("route {}", route.name()));
    r.say("library path", &s.library_path);
    r.flush("starting the payload");
    match route {
        Route::Direct => Command::new(&s.bin).arg0(&s.arg0).args(&s.args).exec(),
        Route::Installed => {
            // ⛔ The library path becomes an inherited variable on this route
            // and only on this route. There is nowhere else to put it: the
            // kernel starts the loader named in the payload, and a loader
            // started that way takes no arguments.
            envx::set("LD_LIBRARY_PATH", &s.library_path);
            if !s.preload.is_empty() {
                envx::set("LD_PRELOAD", s.preload.join(" "));
            }
            Command::new(&s.bin).arg0(&s.arg0).args(&s.args).exec()
        }
        Route::Loader => {
            let argv = loader_argv(s);
            // ⭐ The loader is mapped here and started as the program, the way
            // the launcher this replaces starts it, so `/proc/self/exe` stays
            // this hardlink - named after the program - and not the loader. A
            // refusal comes back before anything is replaced, so the execve
            // below is still available, and the report names why it was used.
            if envx::get("PRUN_LOADER_EXEC") != "execve" {
                let refusal = userland(&s.loader, &argv);
                r.refused(&format!(
                    "the loader could not be started in userland ({}); starting it with execve",
                    refusal.message()
                ));
            }
            Command::new(&s.loader).args(&argv[1..]).exec()
        }
    }
}

/// The loader's own command line: the library path, the name the payload
/// sees as argv[0], what to load first, then the payload and its arguments.
/// `ld.so` reads the first three as options (`--argv0` needs glibc 2.33).
pub fn loader_argv(s: &Start) -> Vec<std::ffi::OsString> {
    let mut a: Vec<std::ffi::OsString> = vec![s.loader.clone().into_os_string()];
    a.push("--library-path".into());
    a.push(s.library_path.clone().into());
    a.push("--argv0".into());
    a.push(s.arg0.clone().into_os_string());
    if !s.preload.is_empty() {
        a.push("--preload".into());
        a.push(s.preload.join(" ").into());
    }
    a.push(s.bin.clone().into_os_string());
    a.extend(s.args.iter().map(std::ffi::OsString::from));
    a
}

/// Start the loader as the program without `execve`: `uexec` maps it, builds
/// the stack and jumps. It returns only a refusal; on success it never returns.
fn userland(loader: &Path, argv: &[std::ffi::OsString]) -> crate::uexec::Refusal {
    use std::os::unix::ffi::OsStrExt;
    let cstr = |b: &[u8]| std::ffi::CString::new(b.to_vec());
    let Ok(argv) = argv.iter().map(|a| cstr(a.as_bytes())).collect::<Result<Vec<_>, _>>() else {
        return crate::uexec::Refusal::Unreadable("an argument contains a NUL".to_string());
    };
    let Ok(envp) = std::env::vars_os()
        .map(|(k, v)| {
            let mut b = k.as_bytes().to_vec();
            b.push(b'=');
            b.extend_from_slice(v.as_bytes());
            cstr(&b)
        })
        .collect::<Result<Vec<_>, _>>()
    else {
        return crate::uexec::Refusal::Unreadable("an environment entry contains a NUL".to_string());
    };
    crate::uexec::exec(&crate::uexec::Request {
        program: loader.to_path_buf(),
        argv,
        envp,
        interpreter: None,
        no_interpreter: true,
    })
}

/// Which route a payload needs, and what had to be done to make it possible.
///
/// ⭐ The installed route is offered rather than assumed: when the directory a
/// payload names cannot be made private, the loader route reaches the same
/// program. Declining costs the payload its own `/proc/self/exe` and costs
/// nothing else, which is why it is a fallback rather than a refusal.
pub fn choose(payload: &crate::elf::Payload, loader: &Path, r: &mut Report) -> Route {
    if payload.statik {
        return Route::Direct;
    }
    let Some(interp) = payload.interp.as_deref() else {
        return Route::Loader;
    };
    let target = Path::new(interp);
    if target.exists() {
        // ⚠ Something is already at that path. For an ordinary payload it is
        // the HOST's loader, and the bundle's is the one that has to run; the
        // only case where the file there is usable is one this launcher put
        // there itself, in a directory it still owns.
        if loaderdir::already_installed(loader, target) && loaderdir::parent_is_ours(target).is_ok()
        {
            return Route::Installed;
        }
        return Route::Loader;
    }
    // ⛔ The interpreter this payload names is not on this host, so the payload
    // cannot start on its own. Putting the bundle's loader there is what the
    // build that patched it intended - and it is done only where the directory
    // can be made this user's and nobody else's.
    match loaderdir::install(loader, target) {
        Ok(()) => Route::Installed,
        Err(e) => {
            r.refused(&format!(
                "{} is where this payload says its loader is, and it cannot be made private ({e}); starting it the other way",
                target.display()
            ));
            Route::Loader
        }
    }
}

/// The objects a bundle asks the loader to load first.
pub fn preload(root: &Path) -> Vec<String> {
    let p = root.join(".preload");
    let Ok(body) = std::fs::read_to_string(&p) else {
        return Vec::new();
    };
    body.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

/// Start a script through the interpreter its first line names.
///
/// ⛔ A script is never handed to the loader. It is not an ELF, and the loader
/// would refuse it with a message about a bad magic number that says nothing
/// about the real problem.
pub fn script(path: &Path, args: &[String]) -> std::io::Error {
    let first = match std::fs::read_to_string(path) {
        Ok(body) => body.lines().next().unwrap_or_default().to_string(),
        Err(e) => return e,
    };
    let Some(line) = first.strip_prefix("#!") else {
        return std::io::Error::new(std::io::ErrorKind::InvalidData, "no interpreter line");
    };
    let parts: Vec<&str> = line.trim().split_whitespace().collect();
    if parts.is_empty() {
        return std::io::Error::new(std::io::ErrorKind::InvalidData, "empty interpreter line");
    }
    // ⭐ The interpreter is looked up on PATH by its BASE NAME first, so a
    // script written for `/usr/bin/python3` runs against the bundle's own
    // `bin/python3` - which is on PATH by now, and which is the whole point of
    // bundling an interpreter.
    let named = Path::new(parts[0]);
    let base = named.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let (prog, rest) = if base == "env" && parts.len() > 1 {
        (which(parts[1]).unwrap_or_else(|| PathBuf::from(parts[1])), &parts[2..])
    } else {
        (which(&base).unwrap_or_else(|| named.to_path_buf()), &parts[1..])
    };
    Command::new(prog).args(rest).arg(path).args(args).exec()
}

/// Find a program on PATH.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = envx::get("PATH");
    for dir in path.split(':') {
        if !envx::usable(dir) {
            continue;
        }
        let p = Path::new(dir).join(name);
        if crate::sys::executable(&p) {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf::Payload;

    #[test]
    fn a_static_payload_is_started_directly() {
        let mut r = Report::new(false);
        let p = Payload { elf32: false, statik: true, interp: None };
        assert_eq!(choose(&p, Path::new("/nowhere/ld"), &mut r), Route::Direct);
    }

    #[test]
    fn an_ordinary_payload_goes_through_the_loader() {
        let mut r = Report::new(false);
        let p = Payload {
            elf32: false,
            statik: false,
            interp: Some("/lib64/ld-linux-x86-64.so.2".into()),
        };
        assert_eq!(choose(&p, Path::new("/nowhere/ld"), &mut r), Route::Loader);
    }

    // ⛔ The refusal, which is the third improvement's own case: a payload
    // naming a directory the launcher cannot make private still starts.
    #[test]
    fn a_payload_naming_a_directory_we_cannot_own_still_starts() {
        let mut r = Report::new(false);
        let p = Payload {
            elf32: false,
            statik: false,
            interp: Some("/proc/prun-cannot-be-made/ld".into()),
        };
        assert_eq!(choose(&p, Path::new("/nowhere/ld"), &mut r), Route::Loader);
    }

    #[test]
    fn the_loader_argv_carries_the_path_the_name_and_the_payload() {
        let s = Start {
            bin: PathBuf::from("/b/shared/bin/app"),
            arg0: PathBuf::from("/b/app"),
            args: vec!["one".into(), "two words".into()],
            loader: PathBuf::from("/b/shared/lib/ld-linux-x86-64.so.2"),
            library_path: "/b/shared/lib".into(),
            preload: vec!["/b/a.so".into(), "/b/b.so".into()],
        };
        let a: Vec<String> = loader_argv(&s).iter().map(|x| x.to_string_lossy().into_owned()).collect();
        assert_eq!(
            a,
            vec![
                "/b/shared/lib/ld-linux-x86-64.so.2", "--library-path", "/b/shared/lib", "--argv0",
                "/b/app", "--preload", "/b/a.so /b/b.so", "/b/shared/bin/app", "one", "two words",
            ]
        );
    }

    #[test]
    fn preload_skips_blank_lines_and_comments() {
        let d = std::env::temp_dir().join(format!("prun-preload-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(".preload"), "a.so\n\n# note\n  b.so  \n").unwrap();
        assert_eq!(preload(&d), vec!["a.so".to_string(), "b.so".to_string()]);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_bundle_with_no_preload_file_asks_for_nothing() {
        assert!(preload(Path::new("/no/such/bundle")).is_empty());
    }
}
