use super::*;

/// The usage text for whichever verb this parser is serving.
///
/// ⛔ `create` shares `run`'s parser, so before this existed `podvm create
/// --no-such-flag` answered `podvm run: unknown option` and printed
/// `usage: podvm run`, naming a verb the caller had not typed. A diagnostic
/// that names the wrong operation is the class TODO/cli.md T-0805 exists to
/// close, and T-0804 forbids outright.
pub fn usage(verb: &str) -> String {
    let mut s = format!("usage: podvm {verb} [options] <image> [command] [arg...]\n\n");
    if verb != "run" {
        // ⭐ Said rather than implied. The option set below is `run`'s, because
        // one parser serves both, and `podvm system info` says the same thing
        // about the same rows instead of holding a second copy of them.
        s.push_str(&format!(
            "  note: podvm {verb} takes podvm run's option set, listed below, because one\n    \
             parser serves both. `podvm system info` lists those rows under `run`.\n\n"
        ));
    }
    s.push_str(RUN_OPTIONS);
    s
}

/// The options of `run` and `create`, with `--profile` applied: the
/// profile's flags go first, so that a flag of the command line wins over
/// the profile where a flag takes one value. TODO/cli.md T-1654.
pub(super) fn parse(verb: &str, args: &[String]) -> std::result::Result<Opts, i32> {
    // A profile can name the image, so this first read needs none.
    let o = parse_flags_with(verb, args, false)?;
    let Some(name) = o.profile.clone() else {
        if o.image.is_none() {
            eprint!("{}", usage(verb));
            return Err(EXIT_CLI_ERROR);
        }
        if !o.sets.is_empty() {
            eprintln!("podvm {verb}: -s needs --profile NAME: it sets one key of a profile");
            return Err(EXIT_FLAG_ERROR);
        }
        return Ok(o);
    };
    let shape = crate::profile::run_args(verb, &name, &o.sets)?;
    let mut flags = shape.flags;
    match &shape.image {
        // TODO/cli.md T-1659: the profile names the image, so each word of
        // the command line after the flags is the command.
        Some(image) => {
            let positional = usize::from(o.image.is_some()) + o.command.len();
            let cut = args.len().saturating_sub(positional);
            flags.extend(args[..cut].iter().filter(|a| *a != "--").cloned());
            flags.push("--".into());
            flags.push(image.clone());
            flags.extend(args[cut..].iter().cloned());
        }
        None => flags.extend(args.iter().cloned()),
    }
    let mut o = parse_flags(verb, &flags)?;
    if o.command.is_empty() {
        o.command = shape.command;
    }
    o.wrap = shape.wrap;
    o.drops = shape.drops;
    o.needs = shape.needs;
    Ok(o)
}

/// ⛔ Parsing stops at the image name: everything after it is the payload's.
/// `podvm run alpine ls -l` must pass `-l` to `ls` and not read it as podvm's,
/// which is docker's rule and the one thing a caller cannot work around.
///
/// ⚠ `verb` is the one the CALLER typed, not the one whose rows are consulted.
/// `create` is served here too, and every message this function writes has to
/// name what the caller ran.
fn parse_flags(verb: &str, args: &[String]) -> std::result::Result<Opts, i32> {
    parse_flags_with(verb, args, true)
}

/// [`parse_flags`], where `need_image` false lets the image be absent.
fn parse_flags_with(
    verb: &str,
    args: &[String],
    need_image: bool,
) -> std::result::Result<Opts, i32> {
    // ⭐ TODO/cli.md T-1330. Bundled shorts expand here, once, for `run`
    // and for `create`, which this parser serves: admission below then sees
    // one flag per argument, exactly as a caller spelling them out.
    let expanded;
    let args = match crate::parity::expand(verb, args) {
        Ok(a) => {
            expanded = a;
            &expanded
        }
        Err(member) => return Err(crate::parity::refuse_member(verb, &member, &usage(verb))),
    };
    let mut o = Opts::new();
    let mut expecting: Option<&'static str> = None;
    let mut ended = false;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(flag) = expecting.take() {
            match flag {
                "-e" => o.env.extend(from_host(a)),
                "--env-file" => {
                    let es = read_env_file(verb, a)?;
                    o.env.extend(es);
                }
                "--log-driver" => match a.as_str() {
                    // ⚠ Stub by the table, by value: `json-file` is the one
                    // driver this runtime's single sink already is, so it is
                    // accepted and changes nothing. Anything else names a
                    // driver podvm does not have and is refused naming it.
                    // TODO/supervise.md T-0605.
                    "json-file" => {}
                    other => {
                        eprintln!(
                            "podvm {verb}: --log-driver takes json-file, not {other:?}: \
                             podvm captures into one interleaved file per container"
                        );
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                // TODO/cli.md T-1695: the record keeps each label.
                "--label" => o.hardening.labels.push(label(verb, a)?),
                "-w" => o.workdir = Some(a.clone()),
                "--entrypoint" => o.entrypoint = Some(a.clone()),
                "--platform" => o.platform = Some(a.clone()),
                "--pull" => o.pull = a.clone(),
                "--name" => o.name = Some(a.clone()),
                "--user" => o.user = Some(a.clone()),
                "--add-host" => crate::complete::add_host(&mut o.ask, verb, a)?,
                "--podvm-tier" => match crate::tier::want(a) {
                    Some(w) => o.tier = Some(w),
                    None => {
                        eprintln!("{}", crate::tier::refusal(verb, a));
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                "--podvm-qemu-arg" => o.qemu_args.push(a.clone()),
                "--podvm-net" if a == "none" => o.net_none = true,
                "--podvm-net" => {
                    eprintln!(
                        "podvm {verb}: --podvm-net takes none, not {a:?} (TODO/machine.md T-1751)"
                    );
                    return Err(EXIT_FLAG_ERROR);
                }
                "--podvm-channel" => {
                    o.channel = Some(crate::machine::guest::channel::parse(verb, a)?)
                }
                "--podvm-mem" => match crate::tier::parse_mem(a) {
                    Some(m) => o.mem = Some(m),
                    None => {
                        eprintln!(
                            "podvm {verb}: --podvm-mem takes a byte count with an optional K/M/G/T suffix, not {a:?}"
                        );
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                "--podvm-cpus" => o.cpus = Some(cpus(verb, a)?),
                // TODO/cli.md T-1695: `run` reads the path before the parse.
                "--report" => report(verb)?,
                "--podvm-confine" => {
                    let c = confine(verb, a)?;
                    o.hardening.confine = Some(podvm_enter::confine::render(&c));
                    o.confine = Some(c);
                }
                "--profile" => o.profile = Some(a.clone()),
                "--network" => network(verb, a, &mut o)?,
                "--ports" => o.ports.extend(ports(verb, a)?),
                "--allow" => o
                    .allow
                    .extend(a.split(',').filter(|h| !h.is_empty()).map(String::from)),
                "-s" => o.sets.push(a.clone()),
                "-v" => volume(verb, a, &mut o)?,
                "--timeout" => o.timeout = Some(timeout(verb, a)?),
                "--tmpfs" => tmpfs(verb, a, &mut o)?,
                "--ulimit" => o.hardening.ulimits.push(ulimit(verb, a)?),
                "--security-opt" => o.hardening.no_new_privs |= security_opt(verb, a)?,
                "--cap-drop" => o.hardening.no_new_privs |= cap_drop(verb, a)?,
                "--device" => match podvm_enter::device::parse(a) {
                    Ok(_) => o.devices.push(a.clone()),
                    Err(why) => {
                        eprintln!("podvm {verb}: --device {why}");
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                _ => o.insecure.push(a.clone()),
            }
            i += 1;
            continue;
        }
        if o.image.is_some() {
            // ⛔ Past the image name. Everything from here is the payload's,
            // dashes and all.
            o.command.push(a.clone());
            i += 1;
            continue;
        }
        // `--` ends podvm's flags: the next word is the image (docker's
        // rule), even where it starts with a dash. TODO/cli.md T-1659.
        if a == "--" && !ended {
            ended = true;
            i += 1;
            continue;
        }
        if ended {
            o.image = Some(a.clone());
            i += 1;
            continue;
        }
        // ⛔ TODO/cli.md T-0801. THE TABLE DECIDES, and the match arms below
        // only implement what it already admitted. A flag with no row cannot
        // reach an arm, so an arm for an unlisted flag is unreachable rather
        // than a second, quieter surface; and a `None` row is refused here with
        // its own reason instead of reading as an unknown option.
        if a.starts_with('-') {
            crate::parity::admit(verb, a, &usage(verb))?;
            // ⭐ T-0804 reads this. Recorded HERE, at the one place every flag
            // passes through, so a flag added to a match arm and forgotten in a
            // second list cannot exist.
            o.ask
                .seen_flags
                .push(a.split('=').next().unwrap_or(a).to_string());
        }
        match a.as_str() {
            "-h" | "--help" => {
                print!("{}", usage(verb));
                return Err(0);
            }
            "--rm" => o.rm = true,
            "-d" | "--detach" => o.detach = true,
            "--name" => expecting = Some("--name"),
            other if other.starts_with("--name=") => o.name = Some(other[7..].to_string()),
            "-t" | "--tty" => o.tty = true,
            "-q" | "--quiet" => o.ask.quiet = true,
            "--verbose" => o.ask.verbose = true,
            // podvm does not detach stdin, so a foreground run is already
            // interactive; `run -d -i` of the incus tier pushes the piped
            // stdin into the instance (TODO/machine.md T-1777).
            "-i" | "--interactive" => o.interactive = true,
            "-e" | "--env" => expecting = Some("-e"),
            "--env-file" => expecting = Some("--env-file"),
            "--label" => expecting = Some("--label"),
            "--log-driver" => expecting = Some("--log-driver"),
            "--attach" => {
                // ⚠ Stub by the table: podvm always captures stdout and
                // stderr together, so stream selection changes nothing.
            }
            "--expose" => {
                // ⚠ Stub by the table: docker's --expose only documents
                // ports and podvm publishes none, so the run is unchanged.
            }
            "-w" | "--workdir" => expecting = Some("-w"),
            "--device" => expecting = Some("--device"),
            "-v" | "--volume" => expecting = Some("-v"),
            other if other.starts_with("--volume=") => volume(verb, &other[9..], &mut o)?,
            "-u" | "--user" => expecting = Some("--user"),
            "--entrypoint" => expecting = Some("--entrypoint"),
            "--platform" => expecting = Some("--platform"),
            "--pull" => expecting = Some("--pull"),
            "--insecure-registry" => expecting = Some("--insecure-registry"),
            "--tls-verify" => o.tls_verify = Some(true),
            "--podvm-sealed" => o.sealed = true,
            "--add-host" => expecting = Some("--add-host"),
            "--podvm-tier" => expecting = Some("--podvm-tier"),
            "--podvm-qemu-arg" => expecting = Some("--podvm-qemu-arg"),
            "--podvm-mem" => expecting = Some("--podvm-mem"),
            "--podvm-net" => expecting = Some("--podvm-net"),
            "--podvm-channel" => expecting = Some("--podvm-channel"),
            "--podvm-cpus" => expecting = Some("--podvm-cpus"),
            "--podvm-confine" => expecting = Some("--podvm-confine"),
            "--report" => expecting = Some("--report"),
            "--profile" => expecting = Some("--profile"),
            "--network" | "--net" => expecting = Some("--network"),
            "--ports" => expecting = Some("--ports"),
            "--allow" => expecting = Some("--allow"),
            "-s" | "--set" => expecting = Some("-s"),
            "--timeout" => expecting = Some("--timeout"),
            "--read-only" => o.read_only = true,
            "--tmpfs" => expecting = Some("--tmpfs"),
            other if other.starts_with("--tmpfs=") => {
                tmpfs(verb, &other["--tmpfs=".len()..], &mut o)?
            }
            "--ulimit" => expecting = Some("--ulimit"),
            "--security-opt" => expecting = Some("--security-opt"),
            "--cap-drop" => expecting = Some("--cap-drop"),
            other if other.starts_with("--cap-drop=") => {
                o.hardening.no_new_privs |= cap_drop(verb, &other["--cap-drop=".len()..])?
            }
            other if other.starts_with("--ulimit=") => o
                .hardening
                .ulimits
                .push(ulimit(verb, &other["--ulimit=".len()..])?),
            other if other.starts_with("--security-opt=") => {
                o.hardening.no_new_privs |= security_opt(verb, &other["--security-opt=".len()..])?
            }
            other if other.starts_with("--timeout=") => {
                o.timeout = Some(timeout(verb, &other["--timeout=".len()..])?)
            }
            other if other.starts_with("--add-host=") => {
                crate::complete::add_host(&mut o.ask, verb, &other[11..])?
            }
            "--no-source-fixup" => o.ask.no_source_fixup = true,
            "--no-host-cas" => o.ask.no_host_cas = true,
            "--no-steps" => o.ask.no_steps = true,
            "--strict" => o.ask.strict = true,
            "--unsafe-host-paths" => o.unsafe_host_paths = true,
            other if other.starts_with("--strict=") => match &other[9..] {
                // ⭐ T-1415: `--strict=all` keeps the T-0804 behavior,
                // refusing every Degraded and Stub difference.
                "all" => {
                    o.ask.strict = true;
                    o.ask.strict_all = true;
                }
                v => {
                    eprintln!("podvm {verb}: --strict takes all or nothing, not {v:?}");
                    return Err(EXIT_FLAG_ERROR);
                }
            },
            other if other.starts_with("--env=") => o.env.extend(from_host(&other[6..])),
            other if other.starts_with("--log-driver=") => match &other[13..] {
                "json-file" => {}
                v => {
                    eprintln!(
                        "podvm {verb}: --log-driver takes json-file, not {v:?}: \
                         podvm captures into one interleaved file per container"
                    );
                    return Err(EXIT_FLAG_ERROR);
                }
            },
            other if other.starts_with("--env-file=") => {
                let es = read_env_file(verb, &other[11..])?;
                o.env.extend(es);
            }
            other if other.starts_with("--label=") => o
                .hardening
                .labels
                .push(label(verb, &other["--label=".len()..])?),
            other if other.starts_with("--user=") => o.user = Some(other[7..].to_string()),
            other if other.starts_with("--workdir=") => o.workdir = Some(other[10..].to_string()),
            other if other.starts_with("--device=") => {
                match podvm_enter::device::parse(&other[9..]) {
                    Ok(_) => o.devices.push(other[9..].to_string()),
                    Err(why) => {
                        eprintln!("podvm {verb}: --device {why}");
                        return Err(EXIT_FLAG_ERROR);
                    }
                }
            }
            other if other.starts_with("--entrypoint=") => {
                o.entrypoint = Some(other[13..].to_string())
            }
            other if other.starts_with("--platform=") => o.platform = Some(other[11..].to_string()),
            other if other.starts_with("--pull=") => o.pull = other[7..].to_string(),
            other if other.starts_with("--podvm-tier=") => {
                match crate::tier::want(&other["--podvm-tier=".len()..]) {
                    Some(w) => o.tier = Some(w),
                    None => {
                        eprintln!(
                            "{}",
                            crate::tier::refusal(verb, &other["--podvm-tier=".len()..])
                        );
                        return Err(EXIT_FLAG_ERROR);
                    }
                }
            }
            other if other.starts_with("--podvm-qemu-arg=") => o
                .qemu_args
                .push(other["--podvm-qemu-arg=".len()..].to_string()),
            other if other.starts_with("--network=") || other.starts_with("--net=") => {
                network(verb, other.split_once('=').map_or("", |(_, v)| v), &mut o)?
            }
            other if other.starts_with("--ports=") => o.ports.extend(ports(verb, &other[8..])?),
            other if other.starts_with("--allow=") => o.allow.extend(
                other[8..]
                    .split(',')
                    .filter(|h| !h.is_empty())
                    .map(String::from),
            ),
            other if other.starts_with("--profile=") => {
                o.profile = Some(other["--profile=".len()..].to_string())
            }
            other if other.starts_with("--set=") => {
                o.sets.push(other["--set=".len()..].to_string())
            }
            other if other.starts_with("--report=") => report(verb)?,
            other if other.starts_with("--podvm-confine=") => {
                let c = confine(verb, &other["--podvm-confine=".len()..])?;
                o.hardening.confine = Some(podvm_enter::confine::render(&c));
                o.confine = Some(c);
            }
            other if other.starts_with("--podvm-cpus=") => {
                o.cpus = Some(cpus(verb, &other["--podvm-cpus=".len()..])?)
            }
            other if other.starts_with("--podvm-mem=") => {
                match crate::tier::parse_mem(&other["--podvm-mem=".len()..]) {
                    Some(m) => o.mem = Some(m),
                    None => {
                        eprintln!(
                            "podvm {verb}: --podvm-mem takes a byte count with an optional K/M/G/T suffix, not {:?}",
                            &other["--podvm-mem=".len()..]
                        );
                        return Err(EXIT_FLAG_ERROR);
                    }
                }
            }
            other if other.starts_with("--insecure-registry=") => {
                o.insecure.push(other[20..].to_string())
            }
            other if other.starts_with("--tls-verify=") => match &other[13..] {
                "true" | "1" => o.tls_verify = Some(true),
                "false" | "0" => o.tls_verify = Some(false),
                v => {
                    eprintln!("podvm {verb}: --tls-verify takes true or false, not {v:?}");
                    return Err(EXIT_FLAG_ERROR);
                }
            },
            other if other.starts_with('-') => {
                // ⛔ Unreachable through the table above, and it is here as an
                // assertion rather than as a fallback: the table admitted this
                // flag and this parser has no arm for it, which is a defect in
                // podvm and is reported as one.
                return Err(crate::parity::no_arm(verb, other));
            }
            other => o.image = Some(other.to_string()),
        }
        i += 1;
    }
    if let Some(flag) = expecting {
        eprintln!("podvm {verb}: {flag} needs a value");
        return Err(EXIT_FLAG_ERROR);
    }
    if need_image && o.image.is_none() {
        eprint!("{}", usage(verb));
        return Err(EXIT_CLI_ERROR);
    }
    // ⭐ TODO/enter.md T-1653: the table checks of the pairs, up front. The
    // rootfs is not known yet, so `/` stands for it: the root pair is
    // checked when the run builds its table.
    if let Err(why) = podvm_enter::userland::Maps::new(&podvm_enter::NAMES, "/", &o.volumes)
        .and_then(|m| podvm_enter::userland::userland_volume_table(&podvm_enter::NAMES, &m))
    {
        eprintln!("podvm {verb}: -v: {why}");
        return Err(EXIT_FLAG_ERROR);
    }
    if !matches!(o.pull.as_str(), "never" | "missing" | "always") {
        eprintln!(
            "podvm {verb}: --pull takes never, missing or always, not {:?}",
            o.pull
        );
        return Err(EXIT_FLAG_ERROR);
    }
    Ok(o)
}

mod values;
pub(crate) use values::from_host;
use values::*;

mod opts;
pub(super) use opts::Opts;
