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

#[derive(Debug)]
pub(super) struct Opts {
    pub(super) rm: bool,
    pub(super) detach: bool,
    pub(super) name: Option<String>,
    pub(super) env: Vec<String>,
    pub(super) workdir: Option<String>,
    pub(super) entrypoint: Option<String>,
    pub(super) platform: Option<String>,
    pub(super) pull: String,
    pub(super) insecure: Vec<String>,
    pub(super) tls_verify: Option<bool>,
    pub(super) tty: bool,
    pub(super) image: Option<String>,
    pub(super) command: Vec<String>,
    /// `--user`. Carried raw here and resolved against the image once its
    /// rootfs exists, because a name means the image's own passwd entry.
    pub(super) user: Option<String>,
    /// `--podvm-tier`. Carried raw here and resolved in `prepare`.
    /// TODO/machine.md T-1302.
    pub(super) tier: Option<crate::tier::Want>,
    /// `--podvm-qemu-arg`. One emulator token per occurrence, carried for
    /// the machine tier's driver (T-1303); refused on every other tier
    /// rather than silently dropped.
    pub(super) qemu_args: Vec<String>,
    /// `--podvm-mem`. The validated guest memory in bytes, carried for the
    /// machine tier's ceiling check in `prepare`; refused on every other
    /// tier rather than silently dropped. TODO/machine.md T-1305.
    pub(super) mem: Option<u64>,
    /// `--device`. Carried raw here and parsed now: a malformed shape is a
    /// flag error, before anything is fetched. TODO/enter.md T-0501.
    pub(super) devices: Vec<String>,
    /// `--unsafe-host-paths`. Opts into the userland pass-through with no
    /// `PODVM_MAPS` anchoring. TODO/enter.md T-1407.
    pub(super) unsafe_host_paths: bool,
    /// `-v`. Parsed now, so a shape podvm cannot serve is a flag error
    /// before anything is fetched. TODO/enter.md T-1653.
    pub(super) volumes: Vec<podvm_enter::userland::Volume>,
    /// M5 and T-0804. ⚠ Carried in one struct so `run`, `create` and the
    /// launcher cannot each grow their own copy of the same three answers.
    pub(super) ask: crate::complete::Ask,
}

impl Opts {
    /// TODO/cli.md T-1649. The first word of the caller where `argv[0]`
    /// is the Entrypoint of the image and the caller gave a command: the
    /// word that a refusal of that script names after `--entrypoint`.
    /// `None` under `--entrypoint`, with no command, or with no
    /// Entrypoint (the command is then `argv` itself).
    pub(super) fn retry_word(&self, image_entrypoint: Option<&[String]>) -> Option<&str> {
        if self.entrypoint.is_some() || image_entrypoint.is_none_or(<[String]>::is_empty) {
            return None;
        }
        self.command.first().map(String::as_str)
    }
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
fn read_env_file(verb: &str, path: &str) -> std::result::Result<Vec<String>, i32> {
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

/// One `-v` value, TODO/enter.md T-1653: a flag error names the value.
fn volume(verb: &str, spec: &str) -> std::result::Result<podvm_enter::userland::Volume, i32> {
    crate::lifecycle::volume::parse(spec).map_err(|why| {
        eprintln!("podvm {verb}: -v {spec:?}: {why}");
        EXIT_FLAG_ERROR
    })
}

/// ⛔ Parsing stops at the image name: everything after it is the payload's.
/// `podvm run alpine ls -l` must pass `-l` to `ls` and not read it as podvm's,
/// which is docker's rule and the one thing a caller cannot work around.
///
/// ⚠ `verb` is the one the CALLER typed, not the one whose rows are consulted.
/// `create` is served here too, and every message this function writes has to
/// name what the caller ran.
pub(super) fn parse(verb: &str, args: &[String]) -> std::result::Result<Opts, i32> {
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
    let mut o = Opts {
        rm: false,
        detach: false,
        name: None,
        env: Vec::new(),
        workdir: None,
        entrypoint: None,
        platform: None,
        pull: "missing".into(),
        insecure: Vec::new(),
        tls_verify: None,
        tty: false,
        image: None,
        command: Vec::new(),
        user: None,
        tier: None,
        qemu_args: Vec::new(),
        mem: None,
        devices: Vec::new(),
        unsafe_host_paths: false,
        volumes: Vec::new(),
        ask: crate::complete::Ask::default(),
    };
    let mut expecting: Option<&'static str> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(flag) = expecting.take() {
            match flag {
                "-e" => o.env.push(a.clone()),
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
                "--label" => {
                    // ⚠ Stub by the table: accepted and dropped. Podvm
                    // records carry no labels, so there is nowhere to keep
                    // it, and `ps --filter label=` says so out loud.
                }
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
                        eprintln!("podvm {verb}: --podvm-tier takes machine or chroot, not {a:?}");
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                "--podvm-qemu-arg" => o.qemu_args.push(a.clone()),
                "--podvm-mem" => match crate::tier::parse_mem(a) {
                    Some(m) => o.mem = Some(m),
                    None => {
                        eprintln!(
                            "podvm {verb}: --podvm-mem takes a byte count with an optional K/M/G/T suffix, not {a:?}"
                        );
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                "-v" => o.volumes.push(volume(verb, a)?),
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
            "-i" | "--interactive" => {
                // ⚠ Accepted and a no-op, deliberately: podvm does not detach
                // stdin, so it is already interactive when the caller's is.
                // TODO/cli.md T-0801 calls this Stub and the banner lists it.
            }
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
            other if other.starts_with("--volume=") => o.volumes.push(volume(verb, &other[9..])?),
            "-u" | "--user" => expecting = Some("--user"),
            "--entrypoint" => expecting = Some("--entrypoint"),
            "--platform" => expecting = Some("--platform"),
            "--pull" => expecting = Some("--pull"),
            "--insecure-registry" => expecting = Some("--insecure-registry"),
            "--tls-verify" => o.tls_verify = Some(true),
            "--add-host" => expecting = Some("--add-host"),
            "--podvm-tier" => expecting = Some("--podvm-tier"),
            "--podvm-qemu-arg" => expecting = Some("--podvm-qemu-arg"),
            "--podvm-mem" => expecting = Some("--podvm-mem"),
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
            other if other.starts_with("--env=") => o.env.push(other[6..].to_string()),
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
            other if other.starts_with("--label=") => {
                // ⚠ As above: Stub, accepted and dropped.
            }
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
                            "podvm {verb}: --podvm-tier takes machine or chroot, not {:?}",
                            &other["--podvm-tier=".len()..]
                        );
                        return Err(EXIT_FLAG_ERROR);
                    }
                }
            }
            other if other.starts_with("--podvm-qemu-arg=") => o
                .qemu_args
                .push(other["--podvm-qemu-arg=".len()..].to_string()),
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
    if o.image.is_none() {
        eprint!("{}", usage(verb));
        return Err(EXIT_CLI_ERROR);
    }
    // ⭐ TODO/enter.md T-1653: the table checks of the pairs, up front.
    if let Err(why) = podvm_enter::userland::userland_maps_table(None, &o.volumes) {
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
