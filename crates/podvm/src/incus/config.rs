//! The incus tier by configuration, TODO/machine.md T-1722: `tier = incus`
//! in `$store/config` makes `run` and `create` take the incus tier with no
//! `--podvm-tier`, where an incusd socket is there; the lifecycle verbs
//! take it for a name that is an instance of podvm in incus.
//!
//! An explicit `--podvm-tier` always wins. Where the configuration names
//! incus and no socket is there, `run` and `create` say so and the ladder
//! selects the rung. A `tier` value that the configuration does not serve
//! is refused by name: a setting that podvm ignores is a lie.

use super::run::socket_path;

/// What the configuration asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Configured {
    /// No `tier` key.
    Nothing,
    /// `tier = incus`, and the socket is there or not.
    Incus { socket: bool },
}

/// Read the `tier` key of the store's configuration.
pub fn configured() -> Result<Configured, String> {
    // The path of the store, never the store itself: a verb that refuses
    // must write nothing, and `open_store` makes the directory.
    let Ok(root) = podvm_image::Store::default_root(|| None) else {
        return Ok(Configured::Nothing);
    };
    match crate::complete::config_value_at(&root, crate::complete::TIER_KEY).as_deref() {
        None | Some("") => Ok(Configured::Nothing),
        Some("incus") => Ok(Configured::Incus {
            socket: socket_path().exists(),
        }),
        Some(other) => Err(format!(
            "{}/{}: tier = {other} is not served by the configuration; tier = incus is \
             (TODO/machine.md T-1722)",
            root.display(),
            crate::complete::CONFIG_FILE
        )),
    }
}

/// Whether `args` name a tier themselves.
pub fn explicit(args: &[String]) -> bool {
    args.iter()
        .take_while(|a| a.as_str() != "--")
        .any(|a| a == "--podvm-tier" || a.starts_with("--podvm-tier="))
}

/// The first positional argument of a lifecycle verb: its name.
fn first_name(verb: &str, args: &[String]) -> Option<String> {
    // The flags of each verb that take a value: `-f` is `--format` for
    // `inspect` and a switch (`--follow`, `--force`) elsewhere.
    let valued: &[&str] = match verb {
        "exec" => &[
            "-e",
            "--env",
            "-w",
            "--workdir",
            "-u",
            "--user",
            "--env-file",
        ],
        "inspect" => &["-f", "--format"],
        "logs" => &["--tail"],
        _ => &[],
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if valued.contains(&a.as_str()) {
            it.next();
            continue;
        }
        if !a.starts_with('-') {
            return Some(a.clone());
        }
    }
    None
}

/// The arguments of `verb` with `--podvm-tier=incus` first where the
/// configuration selects the tier; `Ok(None)` where it does not;
/// `Err(code)` where the configuration is refused.
pub fn route(verb: &str, args: &[String]) -> Result<Option<Vec<String>>, i32> {
    let make = matches!(verb, "run" | "create");
    let lifecycle = matches!(verb, "start" | "stop" | "rm" | "logs" | "inspect" | "exec");
    if !(make || lifecycle) || explicit(args) {
        return Ok(None);
    }
    let socket = match configured() {
        Ok(Configured::Nothing) => return Ok(None),
        Ok(Configured::Incus { socket }) => socket,
        Err(e) if make => {
            eprintln!("podvm {verb}: {e}");
            return Err(podvm_image::error::EXIT_FLAG_ERROR);
        }
        Err(_) => return Ok(None),
    };
    let with = |args: &[String]| {
        let mut v = vec!["--podvm-tier=incus".to_string()];
        v.extend(args.iter().cloned());
        Some(v)
    };
    if make {
        if !socket {
            eprintln!(
                "podvm {verb}: the configuration names tier = incus, and no incusd answers at \
                 {}: the ladder selects the rung (TODO/machine.md T-1722)",
                socket_path().display()
            );
            return Ok(None);
        }
        return Ok(with(args));
    }
    if lifecycle
        && socket
        && let Some(name) = first_name(verb, args)
        && super::verbs::is_instance(&name)
    {
        return Ok(with(args));
    }
    Ok(None)
}

/// The arguments of `verb` after [`route`]: the caller's own where the
/// configuration selects nothing.
pub fn routed(verb: Option<&str>, args: &[String]) -> Result<Vec<String>, i32> {
    match verb {
        Some(v) => Ok(route(v, args)?.unwrap_or_else(|| args.to_vec())),
        None => Ok(args.to_vec()),
    }
}

/// The rung that `system info` names as `.EnteredRung` where the
/// configuration selects incus and its socket is there.
pub fn entered_word() -> Option<&'static str> {
    matches!(configured(), Ok(Configured::Incus { socket: true })).then_some("incus")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn an_explicit_tier_wins_over_the_configuration() {
        assert!(explicit(&v(&["--podvm-tier=machine", "img"])));
        assert!(explicit(&v(&["--podvm-tier", "incus", "img"])));
        assert!(!explicit(&v(&["img", "true"])));
        // After `--` the words are the command's.
        assert!(!explicit(&v(&["img", "--", "--podvm-tier=x"])));
    }

    #[test]
    fn the_name_of_a_lifecycle_verb_skips_the_values_of_exec() {
        assert_eq!(
            first_name("exec", &v(&["-e", "A=1", "-w", "/x", "web", "sh"])).as_deref(),
            Some("web")
        );
        assert_eq!(first_name("rm", &v(&["-f", "web"])).as_deref(), Some("web"));
        assert_eq!(first_name("logs", &v(&["-f"])), None);
        assert_eq!(
            first_name("logs", &v(&["-f", "web"])).as_deref(),
            Some("web")
        );
        assert_eq!(
            first_name("inspect", &v(&["--format", "{{.Name}}", "web"])).as_deref(),
            Some("web")
        );
        assert_eq!(
            first_name("inspect", &v(&["-f", "{{.Id}}", "web"])).as_deref(),
            Some("web")
        );
    }
}
