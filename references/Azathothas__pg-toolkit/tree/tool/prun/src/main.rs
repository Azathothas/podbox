//! prun - portable run.
//!
//! The launcher inside a bundle. It finds the bundle from its own position,
//! works out which program it was asked to be, applies everything the bundle's
//! layout implies, and starts the payload against the bundle's own loader and
//! library path. ⭐ It needs no namespace of any kind to do it, which is the
//! property the whole rung is built on.
//!
//! ⛔ It is written here. TODO/port.md rule 5: the artefacts it has to be
//! compatible with were studied, and every line below is this project's.
//! `TODO/ledger.json` carries one row per capability of the tool it replaces,
//! and `./check port` refuses a row in none of three states.
//!
//! SPDX-License-Identifier: 0BSD

mod abi;
mod appdir;
mod apprun;
mod dotenv;
mod elf;
mod envx;
mod exec;
mod hostpath;
mod libpath;
mod loaderdir;
mod names;
mod report;
mod sandbox;
mod sys;
mod uexec;

use std::os::unix::ffi::OsStringExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use report::Report;

/// The name a bundle carries this program under, when it was built before the
/// rename. ⚠ One place, and T-117 is the entry that removes it.
const CARRIED_NAME: &str = "sharun";

fn main() {
    let code = start();
    exit(code);
}

fn start() -> i32 {
    let mut args: Vec<String> = std::env::args().collect();
    let arg0 = if args.is_empty() { String::new() } else { args.remove(0) };

    let me = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("prun: cannot find my own path ({e}); a bundle is located from it");
            return 1;
        }
    };
    let mut r = Report::new(names::take("PRUN_REPORT") == "1");

    let root = match bundle_root(&me, &mut r) {
        Some(p) => p,
        None => {
            eprintln!("prun: {} is not inside a bundle: no shared/ above it", me.display());
            return 1;
        }
    };
    names::export_dir(&root.to_string_lossy());

    let bin_dir = root.join("bin");
    let shared_bin = root.join("shared/bin");

    // Which program is this? A hardlink answers with its own name, because the
    // kernel records the path it was started from; a symlink answers with the
    // name it was reached by, because the link resolves to this binary.
    let mut want = program_name(&me, &arg0, &root, &shared_bin);
    let mut payload_arg0 = PathBuf::from(&arg0);

    // ⛔ The test is against the launcher's OWN name, never against the file
    // name it happens to be running under. A hardlink in `bin/` gives
    // `/proc/self/exe` the payload's name, so comparing with that name is true
    // on exactly the path where it must be false - and a bundle then answers
    // every hardlink by listing its own `bin/` directory.
    if want == env!("CARGO_PKG_NAME") || want == CARRIED_NAME {
        match direct(&mut args, &bin_dir, &shared_bin, &root, &mut r) {
            Direct::Done(code) => return code,
            Direct::Program(name, path) => {
                want = name;
                if let Some(p) = path {
                    payload_arg0 = p;
                }
            }
        }
    } else if want == "AppRun" {
        return as_apprun(&root, &bin_dir, &args, &mut r);
    } else if want == "gio-launch-desktop" {
        return as_launch_desktop(&args);
    }

    launch(&root, &want, &payload_arg0, args, &mut r)
}

/// What `prun` was invoked as.
fn self_name(me: &Path) -> String {
    me.file_name().unwrap_or_default().to_string_lossy().into_owned()
}

/// Find the bundle.
///
/// ⭐ The environment is asked first and is not believed: a directory named by
/// a variable is accepted only when this binary is actually inside it and it
/// looks like a bundle. A caller who exported the variable for a different
/// bundle would otherwise send this one at the wrong library tree.
fn bundle_root(me: &Path, r: &mut Report) -> Option<PathBuf> {
    let stated = names::peek("PRUN_DIR");
    if !stated.is_empty() {
        let d = PathBuf::from(&stated);
        if let Ok(d) = d.canonicalize() {
            if d.join("shared").is_dir() && me.canonicalize().map(|m| m.starts_with(&d)).unwrap_or(false)
            {
                r.note(&format!("bundle {} (stated)", d.display()));
                return Some(d);
            }
            r.refused(&format!(
                "{} was named as the bundle and this binary is not inside it; finding it from my own path instead",
                d.display()
            ));
        }
    }
    let here = me.parent()?;
    // A launcher in `bin/` is one level below the bundle.
    if here.file_name().map(|n| n == "bin").unwrap_or(false) {
        if let Some(up) = here.parent() {
            if up.join("shared").is_dir() {
                r.note(&format!("bundle {} (from bin/)", up.display()));
                return Some(up.to_path_buf());
            }
        }
    }
    r.note(&format!("bundle {} (beside me)", here.display()));
    Some(here.to_path_buf())
}

/// Which program the launcher was asked to be.
fn program_name(me: &Path, arg0: &str, root: &Path, shared_bin: &Path) -> String {
    let a0 = Path::new(arg0);
    let name = a0.file_name().unwrap_or_default().to_string_lossy().into_owned();
    if let Ok(md) = std::fs::symlink_metadata(a0) {
        if md.file_type().is_symlink() {
            if let Ok(target) = a0.canonicalize() {
                // A link straight at the launcher: the LINK's name is the ask.
                if target.parent() == Some(root) {
                    return name;
                }
                // A link at a program in the bundle: the TARGET's name is.
                let tname = target.file_name().unwrap_or_default().to_string_lossy().into_owned();
                if shared_bin.join(&tname).exists() {
                    return tname;
                }
            }
        }
    }
    // ⭐ A hardlink lands here, and it is the common case: `/proc/self/exe`
    // carries the path the kernel started, so the launcher's own file name IS
    // the program's name.
    self_name(me)
}

/// What handling the direct invocation decided.
enum Direct {
    /// Nothing more to do; this is the exit code.
    Done(i32),
    /// Run this program, optionally under this `argv[0]`.
    Program(String, Option<PathBuf>),
}

/// `prun` invoked as itself: options, or a program name.
fn direct(
    args: &mut Vec<String>,
    bin_dir: &Path,
    shared_bin: &Path,
    root: &Path,
    r: &mut Report,
) -> Direct {
    if args.is_empty() {
        eprintln!("prun: name one of the programs in {}:", bin_dir.display());
        if let Ok(entries) = bin_dir.read_dir() {
            let mut names: Vec<String> = entries
                .flatten()
                .filter(|e| sys::executable(&e.path()))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            for n in names {
                println!("{n}");
            }
        }
        return Direct::Done(1);
    }
    match args[0].as_str() {
        "-v" | "--version" => {
            println!("prun {}", env!("CARGO_PKG_VERSION"));
            return Direct::Done(0);
        }
        // ⭐ The userland-exec route, exposed so it can be RUN. A loader that
        // maps an image and jumps to it cannot be tested by reading it, and
        // this is the seam a criterion drives it through.
        //
        // ⚠ It starts the program with the environment this process has,
        // because that is what a caller comparing it against `execve` needs to
        // be able to hold constant. The library-path and argv0 arithmetic the
        // launcher does belongs on its own path rather than in a probe.
        "--uexec" => {
            if args.len() < 2 {
                eprintln!("prun --uexec needs a program");
                return Direct::Done(2);
            }
            let program = PathBuf::from(&args[1]);
            let argv: Vec<std::ffi::CString> = args[1..]
                .iter()
                .filter_map(|a| std::ffi::CString::new(a.as_bytes()).ok())
                .collect();
            let envp: Vec<std::ffi::CString> = std::env::vars_os()
                .filter_map(|(k, v)| {
                    let mut pair = k.into_vec();
                    pair.push(b'=');
                    pair.extend(v.into_vec());
                    std::ffi::CString::new(pair).ok()
                })
                .collect();
            let interpreter = std::env::var_os("PRUN_UEXEC_INTERP").map(PathBuf::from);
            let req = uexec::Request {
                program,
                argv,
                envp,
                interpreter,
                no_interpreter: std::env::var_os("PRUN_UEXEC_NO_INTERP").is_some(),
            };
            // ⛔ Only reached when the start did NOT happen: on success the
            // process is the payload and never comes back here.
            eprintln!("prun: {}", uexec::exec(&req).message());
            return Direct::Done(1);
        }
        "-h" | "--help" => {
            usage();
            return Direct::Done(0);
        }
        "-g" | "--gen-lib-path" => {
            let mut wrote = 0;
            for tree in ["lib", "lib32"] {
                let t = root.join(tree);
                if !t.is_dir() {
                    continue;
                }
                let body = libpath::render(&libpath::scan(&t));
                let at = t.join("lib.path");
                match std::fs::write(&at, body) {
                    Ok(()) => {
                        wrote += 1;
                        eprintln!("prun: wrote {}", at.display());
                    }
                    Err(e) => eprintln!("prun: cannot write {}: {e}", at.display()),
                }
            }
            if wrote == 0 {
                eprintln!("prun: {} holds no lib/ or lib32/ to scan", root.display());
                return Direct::Done(1);
            }
            return Direct::Done(0);
        }
        _ => {}
    }

    let mut name = args.remove(0);
    let in_bin = bin_dir.join(&name);
    if let Ok(target) = in_bin.canonicalize() {
        let tname = target.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let is_link = std::fs::symlink_metadata(&in_bin)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if is_link && shared_bin.join(&tname).exists() {
            name = tname;
        }
        if sys::executable(&target) {
            envx::prepend("PATH", bin_dir);
            match elf::is_script(&in_bin) {
                Ok(true) => {
                    let e = exec::script(&in_bin, args);
                    eprintln!("prun: cannot run the script {}: {e}", in_bin.display());
                    return Direct::Done(1);
                }
                Ok(false) => {
                    // ⚠ A hardlink to this launcher is a name for a payload,
                    // not a program: running it would recurse. Everything else
                    // in `bin/` is a program in its own right and is run.
                    if same_file(&target, &std::env::current_exe().unwrap_or_default()) {
                        r.note(&format!("{name} is a name for a payload"));
                    } else {
                        let e = Command::new(&target).args(args.clone()).exec();
                        eprintln!("prun: cannot run {}: {e}", target.display());
                        return Direct::Done(1);
                    }
                }
                Err(e) => {
                    eprintln!("prun: cannot read {}: {e}", in_bin.display());
                    return Direct::Done(1);
                }
            }
        }
    }
    Direct::Program(name, None)
}

fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
        _ => false,
    }
}

fn as_apprun(root: &Path, bin_dir: &Path, args: &[String], r: &mut Report) -> i32 {
    match apprun::prepare(root, bin_dir, r) {
        Some(apprun::Program::Script(s)) => {
            let Some(shell) = shell() else {
                eprintln!("prun: {} needs a shell and this host has none", s.display());
                return 1;
            };
            r.flush("starting the bundle's own script");
            let e = Command::new(shell).arg(&s).args(args).exec();
            eprintln!("prun: cannot run {}: {e}", s.display());
            1
        }
        Some(apprun::Program::Named(p)) => {
            r.flush("starting the bundle's program");
            let e = Command::new(&p).args(args).exec();
            eprintln!("prun: cannot run {}: {e}", p.display());
            1
        }
        None => 1,
    }
}

/// A shell, for a bundle that carries its own opening script.
///
/// ⛔ `/bin/sh` is not assumed to exist. It does not on every host this family
/// targets, and a launcher that assumed it would fail on the distributions the
/// bundle exists to reach.
fn shell() -> Option<PathBuf> {
    for c in ["/bin/sh", "/bin/bash", "/usr/bin/sh", "/usr/bin/bash"] {
        let p = Path::new(c);
        if sys::executable(p) {
            return Some(p.to_path_buf());
        }
    }
    exec::which("sh").or_else(|| exec::which("bash"))
}

/// The desktop-launch shim: record which desktop entry started this, then get
/// out of the way.
fn as_launch_desktop(args: &[String]) -> i32 {
    envx::set("GIO_LAUNCHED_DESKTOP_FILE_PID", std::process::id().to_string());
    let Some((prog, rest)) = args.split_first() else {
        eprintln!("prun: nothing to launch");
        return 1;
    };
    let e = Command::new(prog).args(rest).exec();
    eprintln!("prun: cannot launch {prog}: {e}");
    1
}

/// The main path: everything a payload needs, then the start.
fn launch(root: &Path, want: &str, arg0: &Path, mut args: Vec<String>, r: &mut Report) -> i32 {
    let shared_bin = root.join("shared/bin");
    let mut bin = if Path::new(want).is_absolute() {
        PathBuf::from(want)
    } else {
        shared_bin.join(want)
    };
    if !bin.exists() && !Path::new(want).is_absolute() {
        if let Ok(true) = elf::is_script(Path::new(want)) {
            let e = exec::script(Path::new(want), &args);
            eprintln!("prun: cannot run the script {want}: {e}");
            return 1;
        }
        match exec::which(want) {
            Some(p) => bin = p,
            None => {
                eprintln!("prun: no {want} in {} and none on PATH", shared_bin.display());
                return 1;
            }
        }
    }

    let payload = match elf::read(&bin) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("prun: cannot read {}: {e}", bin.display());
            return 1;
        }
    };
    let lib = root.join(if payload.elf32 { "lib32" } else { "lib" });

    // The bundle's own file speaks first, so a later decision can see what it
    // set. ⚠ Its `unset` lines are applied at the very end, after everything
    // this launcher does, which is what lets a bundle keep a variable for its
    // own expansions and out of the payload's environment.
    let (env_file, complaints) = match std::fs::read_to_string(root.join(".env")) {
        Ok(body) => dotenv::parse(&body, &dotenv::Process),
        Err(_) => (dotenv::DotEnv::default(), Vec::new()),
    };
    for c in &complaints {
        r.refused(&format!(".env {c}"));
    }
    for (k, v) in &env_file.set {
        envx::set(k, v);
        r.say(k, v);
    }

    // ⛔ Two variables of the caller's are cleared unless the bundle says
    // otherwise: both name objects to load into the payload, and both were
    // written for the host's libraries rather than the bundle's.
    if !names::on("PRUN_KEEP_LD_PRELOAD") {
        envx::unset("LD_PRELOAD");
    }
    if !names::on("PRUN_KEEP_QT_PLUGIN_PATH") {
        envx::unset("QT_PLUGIN_PATH");
    }

    let working = names::take("PRUN_WORKING_DIR");
    if !working.is_empty() {
        if let Err(e) = std::env::set_current_dir(&working) {
            eprintln!("prun: cannot change to {working}: {e}");
            return 1;
        }
    }

    let gfx = graphics(r);
    let bundle = appdir::Bundle::new(root, &lib);
    let (dirs, refused) = lib_dirs(&lib, r);
    for line in refused {
        r.refused(&format!("lib.path names {line}, which is outside the bundle; ignored"));
    }
    let classes = libpath::classes(&lib, &dirs);
    appdir::apply(&bundle, &classes, gfx.as_ref(), r);

    let library_path = search_path(&lib, &dirs, &gfx, payload.elf32, r);

    for name in &env_file.unset {
        envx::unset(name);
    }

    if names::on("PRUN_PRINTENV") {
        for (k, v) in std::env::vars() {
            eprintln!("{k}={v}");
        }
    }

    // The sandbox helper, when the bundle is standing in for one.
    let mut _held: Option<std::fs::File> = None;
    if want == "bwrap" {
        let path = envx::get("PATH");
        let bundle_s = root.to_string_lossy().into_owned();
        let needs_tmp = root.join("shared").join("tmp").exists() || Path::new("/tmp").join(".prun-tmp-marker").exists();
        let scratch = loaderdir::private_dir(root).ok();
        match sandbox::args_fd(&args) {
            Some((at, fd)) => {
                match sandbox::rewrite_fd(&args, at, fd, &bundle_s, &path, needs_tmp, scratch.as_deref()) {
                    Some((new, held)) => {
                        args = new;
                        _held = Some(held);
                    }
                    None => r.refused("the sandbox option list could not be rewritten; passing it through"),
                }
            }
            None => args = sandbox::rewrite(&args, &bundle_s, &path, needs_tmp),
        }
        if !shared_bin.join("bwrap").exists() {
            return system_sandbox(&args, r);
        }
    }

    let loader = match find_loader(&lib) {
        Some(p) => p,
        None => {
            if payload.statik {
                PathBuf::new()
            } else {
                eprintln!("prun: {} carries no loader, and {} needs one", lib.display(), bin.display());
                return 1;
            }
        }
    };

    let route = exec::choose(&payload, &loader, r);
    let start = exec::Start {
        bin: bin.clone(),
        // ⚠ A 32-bit payload is told its own path rather than the caller's
        // `argv[0]`, which is what the artefact this replaces does and what
        // every committed measurement was taken against.
        arg0: if payload.elf32 { bin.clone() } else { arg0.to_path_buf() },
        args,
        loader,
        library_path,
        preload: exec::preload(root),
    };
    let e = exec::run(&start, route, r);
    eprintln!("prun: cannot start {}: {e}", bin.display());
    1
}

/// The bundle's library directories, from its own file or from a walk.
fn lib_dirs(lib: &Path, r: &mut Report) -> (Vec<PathBuf>, Vec<String>) {
    let file = lib.join("lib.path");
    if let Ok(body) = std::fs::read_to_string(&file) {
        return libpath::resolve(lib, &body);
    }
    if !lib.is_dir() {
        return (Vec::new(), Vec::new());
    }
    let scanned = libpath::scan(lib);
    // ⭐ Written when the bundle can be written to, so the next start is a
    // read of one file rather than a walk of a tree. A read-only bundle - the
    // mounted-image case, and the common one - is walked every time and says
    // nothing about it, because there is nothing the user could do.
    if sys::writable(lib) {
        if std::fs::write(&file, libpath::render(&scanned)).is_ok() {
            r.note(&format!("wrote {}", file.display()));
        }
    }
    let mut dirs = vec![lib.to_path_buf()];
    dirs.extend(scanned.iter().map(|d| lib.join(d)));
    (dirs, Vec::new())
}

/// An external graphics installation, when the caller named one.
fn graphics(r: &mut Report) -> Option<appdir::Graphics> {
    let at = names::take("PRUN_MESA_DIR");
    if at.is_empty() {
        return None;
    }
    let p = PathBuf::from(&at);
    if !p.is_dir() {
        r.refused(&format!("{at} was named as a graphics installation and is not a directory"));
        return None;
    }
    // ⛔ A whole system prefix is refused. The variable names a directory that
    // holds a graphics stack and nothing else; pointed at `/usr` it would put
    // every host library ahead of the bundle's.
    if p == Path::new("/usr") || p == Path::new("/") {
        r.refused(&format!("{at} is a system prefix, not a graphics installation; ignored"));
        return None;
    }
    Some(appdir::Graphics { share: p.join("share"), lib: p.join("lib") })
}

/// The loader inside the bundle.
fn find_loader(lib: &Path) -> Option<PathBuf> {
    let named = names::take("PRUN_LOADER_NAME");
    if !named.is_empty() {
        let p = lib.join(&named);
        return if p.exists() { Some(p) } else { None };
    }
    for n in LOADER_NAMES {
        let p = lib.join(n);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// The loader file names, by architecture. ⚠ Both libc spellings, because a
/// bundle may carry either and the launcher cannot tell from the payload.
const LOADER_NAMES: &[&str] = &[
    #[cfg(target_arch = "x86_64")]
    "ld-linux-x86-64.so.2",
    #[cfg(target_arch = "x86_64")]
    "ld-musl-x86_64.so.1",
    #[cfg(target_arch = "x86_64")]
    "ld-linux.so.2",
    #[cfg(target_arch = "aarch64")]
    "ld-linux-aarch64.so.1",
    #[cfg(target_arch = "aarch64")]
    "ld-musl-aarch64.so.1",
    #[cfg(target_arch = "riscv64")]
    "ld-linux-riscv64-lp64d.so.1",
    #[cfg(target_arch = "riscv64")]
    "ld-musl-riscv64.so.1",
    #[cfg(target_arch = "loongarch64")]
    "ld-linux-loongarch-lp64d.so.1",
    #[cfg(target_arch = "loongarch64")]
    "ld-musl-loongarch64.so.1",
    #[cfg(target_arch = "powerpc64")]
    "ld64.so.1",
    #[cfg(target_arch = "powerpc64")]
    "ld64.so.2",
    #[cfg(target_arch = "powerpc64")]
    "ld-musl-powerpc64.so.1",
    #[cfg(target_arch = "powerpc64")]
    "ld-musl-powerpc64le.so.1",
];

/// The library search path, lowest priority last.
fn search_path(
    lib: &Path,
    dirs: &[PathBuf],
    gfx: &Option<appdir::Graphics>,
    elf32: bool,
    r: &mut Report,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    let extra = names::take("PRUN_EXTRA_LIBRARY_PATH");
    if !extra.is_empty() {
        parts.push(extra);
    }
    if let Some(g) = gfx {
        if g.lib.is_dir() {
            parts.push(g.lib.to_string_lossy().into_owned());
        }
    }
    if dirs.is_empty() {
        parts.push(lib.to_string_lossy().into_owned());
    }
    parts.extend(dirs.iter().map(|d| d.to_string_lossy().into_owned()));
    let caller = envx::get("LD_LIBRARY_PATH");
    if !caller.is_empty() {
        parts.push(caller);
    }

    let policy = hostpath::Policy::read();
    if policy == hostpath::Policy::Default {
        r.note("host directories: the built-in list");
    } else {
        r.note("host directories: stated by the bundle");
    }
    parts.extend(policy.dirs(elf32, Path::new("/etc/ld.so.cache")));

    let fallback = names::take("PRUN_FALLBACK_LIBRARY_PATH");
    if !fallback.is_empty() {
        parts.push(fallback);
    }
    envx::join(parts)
}

/// A sandbox helper the bundle does not carry: use the host's.
fn system_sandbox(args: &[String], r: &mut Report) -> i32 {
    let hostpath = envx::get("HOSTPATH");
    let search = if hostpath.is_empty() { envx::get("PATH") } else { hostpath };
    let me = std::env::current_exe().unwrap_or_default();
    for dir in search.split(':') {
        if !envx::usable(dir) {
            continue;
        }
        let c = Path::new(dir).join("bwrap");
        if sys::executable(&c) && !same_file(&c, &me) {
            r.note(&format!("using the host's sandbox helper at {}", c.display()));
            r.flush("starting the sandbox");
            let e = Command::new(&c).args(args).exec();
            eprintln!("prun: cannot run {}: {e}", c.display());
            return 1;
        }
    }
    eprintln!("prun: this bundle asks for a sandbox helper and neither it nor the host has one");
    1
}

fn usage() {
    println!("prun {} - {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_DESCRIPTION"));
    println!();
    println!("  prun [OPTION]");
    println!("  prun PROGRAM [ARG]...        run a program from the bundle's bin/");
    println!("  <a name linked to prun>      run that program");
    println!();
    println!("Options:");
    println!("  -g, --gen-lib-path           write lib.path for the bundle's library trees");
    println!("  -v, --version                print the version");
    println!("  -h, --help                   print this");
    println!();
    println!("Environment:");
    for v in names::VARS {
        if v.carried.is_empty() {
            println!("  {:<28} {}", v.ours, v.what);
        } else {
            println!("  {:<28} {}", v.ours, v.what);
            println!("  {:<28}   also read as {}", "", v.carried);
        }
    }
    println!();
    println!("It sets these when the bundle's own directories call for them:");
    let mut line = String::from("  ");
    for name in appdir::SURFACE {
        if line.len() + name.len() > 76 {
            println!("{line}");
            line = String::from("  ");
        }
        line.push_str(name);
        line.push(' ');
    }
    if line.trim().len() > 0 {
        println!("{line}");
    }
}
