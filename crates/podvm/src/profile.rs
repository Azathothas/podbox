//! Profiles: strict TOML recipes for `run`. TODO/cli.md T-1654.
//!
//! A profile names a sandbox shape once: `podvm run --profile NAME IMAGE`
//! runs it again. A profile is TOML with the tables `fs`, `limits`, `net`
//! and `env.set`; `extends` merges the child over its parent, and `-s
//! KEY=VALUE` sets one leaf. The effective profile becomes `run` flags,
//! which the one parser of `run` reads, so a profile can do nothing that a
//! flag cannot. An unknown key and a wrong type are errors that name the
//! key.
//!
//! `podvm profile list` names each profile; `podvm profile show NAME [-s
//! KEY=VALUE]...` prints the effective profile and its `run` flags.

pub mod builtin;
pub mod model;
pub mod runtime;
mod values;
pub mod workerd;

#[cfg(test)]
mod tests;

use podvm_image::error::EXIT_FLAG_ERROR;

const USAGE: &str = "\
usage: podvm profile list
       podvm profile show NAME [-s KEY=VALUE]...

A profile is TOML under $XDG_CONFIG_HOME/podvm/profiles/NAME.toml (else
~/.config/podvm/profiles), or built in. `run --profile NAME [-s KEY=VALUE]`
runs a payload with it (TODO/cli.md T-1654).";

/// The effective profile of `name` with `sets` applied, checked.
pub fn effective(
    name: &str,
    sets: &[String],
) -> Result<(toml::Table, model::Checked, model::Source), String> {
    let dir = model::user_dir();
    let (mut table, source) = model::resolve(name, dir.as_deref())?;
    for s in sets {
        model::set(&mut table, s).map_err(|e| format!("profile {name}: {e}"))?;
    }
    let checked = model::check(&table).map_err(|e| format!("profile {name}: {e}"))?;
    Ok((table, checked, source))
}

/// What a profile gives a run: its flags, the prefix of the command, the
/// image where the profile names one, and the programs that it needs.
pub struct Shape {
    pub flags: Vec<String>,
    pub wrap: Vec<String>,
    /// The words of the command that `wrap` already holds (T-1661).
    pub drops: Vec<String>,
    pub image: Option<String>,
    pub needs: Vec<String>,
    /// The command where the run gives none: a runtime whose prefix is a
    /// whole command (workerd) runs with no word of the caller.
    pub command: Vec<String>,
}

/// The shape of `--profile name` with `sets`.
pub fn run_args(verb: &str, name: &str, sets: &[String]) -> Result<Shape, i32> {
    // Each message starts `profile NAME:`, so the line reads `--profile NAME: ...`.
    let fail = |e: String| {
        eprintln!("podvm {verb}: --{e}");
        EXIT_FLAG_ERROR
    };
    let (_, c, _) = effective(name, sets).map_err(fail)?;
    let cwd = std::env::current_dir()
        .map_err(|e| fail(format!("profile {name}: the current directory: {e}")))?;
    let mut flags = model::run_args(&c, &cwd).map_err(|e| fail(format!("profile {name}: {e}")))?;
    // TODO/cli.md T-1661: the pinned workerd and its config, read-only.
    if c.runtime.as_deref() == Some("workerd") {
        let dir = podvm_image::store::Store::default_root(|| None)
            .map_err(|e| e.to_string())
            .and_then(|root| workerd::ensure(&root))
            .map_err(|e| fail(format!("profile {name}: workerd: {e}")))?;
        flags.push("-v".into());
        flags.push(format!("{}:{}:ro", dir.display(), workerd::GUEST_DIR));
    }
    let (wrap, drops) = match &c.runtime {
        Some(r) => runtime::prefix(r, &c.read, &c.write)
            .map_err(|e| fail(format!("profile {name}: {e}")))?,
        None => (c.wrap, Vec::new()),
    };
    Ok(Shape {
        flags,
        wrap,
        drops,
        command: match c.runtime.as_deref() {
            Some("workerd") => vec!["workerd".into()],
            _ => Vec::new(),
        },
        image: c.image,
        needs: c.needs,
    })
}

/// TODO/cli.md T-1659: each program that the profile needs, on the PATH of
/// the image in `rootfs`, or a refusal that names the missing ones. A
/// pipeline such as `curl ... | sh` exits 0 where curl is absent, so the
/// absence is detected, never inferred from an exit code.
pub fn check_needs(verb: &str, rootfs: &str, env: &[String], needs: &[String]) -> Result<(), i32> {
    if needs.is_empty() {
        return Ok(());
    }
    let path = env
        .iter()
        .find_map(|e| e.strip_prefix("PATH="))
        .unwrap_or("/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
    let root = std::path::Path::new(rootfs);
    let missing: Vec<&str> = needs
        .iter()
        .map(String::as_str)
        .filter(|n| {
            !path.split(':').any(|d| {
                let p = root.join(d.trim_start_matches('/')).join(n);
                std::fs::symlink_metadata(&p).is_ok()
            })
        })
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    eprintln!(
        "podvm {verb}: --profile: the image has no {} on its PATH ({path}), and the \
         profile needs it; a pipeline that pipes it would exit 0 and do nothing \
         (TODO/cli.md T-1659)",
        missing.join(", ")
    );
    Err(podvm_image::error::EXIT_RUNTIME_ERROR)
}

/// `podvm profile ...`.
pub fn main(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("list") if args.len() == 1 => list(),
        Some("show") => show(&args[1..]),
        Some("-h" | "--help") => {
            println!("{USAGE}");
            0
        }
        _ => {
            eprintln!("{USAGE}");
            EXIT_FLAG_ERROR
        }
    }
}

fn list() -> i32 {
    let dir = model::user_dir();
    let mut names: Vec<String> = builtin::NAMES.iter().map(|s| s.to_string()).collect();
    if let Some(d) = &dir
        && let Ok(rd) = std::fs::read_dir(d)
    {
        let mut extra: Vec<String> = rd
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                n.strip_suffix(".toml").map(str::to_string)
            })
            .filter(|n| !names.contains(n))
            .collect();
        extra.sort();
        names.extend(extra);
    }
    println!("{:<12} {:<10} DESCRIPTION", "NAME", "SOURCE");
    let mut code = 0;
    for n in &names {
        match effective(n, &[]) {
            Ok((_, c, source)) => {
                let src = match source {
                    model::Source::BuiltIn => "built-in".to_string(),
                    model::Source::File(_) => "user".to_string(),
                };
                println!("{n:<12} {src:<10} {}", c.description);
            }
            Err(e) => {
                println!("{n:<12} {:<10} ERROR: {e}", "user");
                code = EXIT_FLAG_ERROR;
            }
        }
    }
    code
}

fn show(args: &[String]) -> i32 {
    let mut name = None;
    let mut sets = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-s" | "--set" => match it.next() {
                Some(v) => sets.push(v.clone()),
                None => {
                    eprintln!("podvm profile show: -s needs KEY=VALUE");
                    return EXIT_FLAG_ERROR;
                }
            },
            other if other.starts_with("--set=") => sets.push(other[6..].to_string()),
            other if other.starts_with('-') => {
                eprintln!("podvm profile show: unknown option {other:?}\n{USAGE}");
                return EXIT_FLAG_ERROR;
            }
            other if name.is_none() => name = Some(other.to_string()),
            other => {
                eprintln!("podvm profile show: one NAME only, and {other:?} is a second");
                return EXIT_FLAG_ERROR;
            }
        }
    }
    let Some(name) = name else {
        eprintln!("{USAGE}");
        return EXIT_FLAG_ERROR;
    };
    let (table, c, source) = match effective(&name, &sets) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("podvm profile show: {e}");
            return EXIT_FLAG_ERROR;
        }
    };
    println!("# profile {name} ({})", source.word());
    print!("{}", model::render(&table));
    let cwd = std::env::current_dir().unwrap_or_default();
    match model::run_args(&c, &cwd) {
        Ok(a) => {
            println!("\n# run flags: {}", a.join(" "));
            if !c.wrap.is_empty() {
                println!("# the command runs under: {}", c.wrap.join(" "));
            }
            // TODO/cli.md T-1661: the runtime's flags beside podvm's map.
            if let Some(r) = &c.runtime
                && let Ok((p, _)) = runtime::prefix(r, &c.read, &c.write)
            {
                println!("# the runtime enforces: {}", p.join(" "));
                println!(
                    "# podvm enforces: the path map of the run flags (-v, --read-only, --tmpfs)"
                );
            }
            0
        }
        Err(e) => {
            eprintln!("podvm profile show: profile {name}: {e}");
            EXIT_FLAG_ERROR
        }
    }
}
