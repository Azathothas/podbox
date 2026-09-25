//! `podbox run`: milestone M3.
//!
//! [`TODO/milestones.md`](../../../TODO/milestones.md) T-1104,
//! [`TODO/enter.md`](../../../TODO/enter.md) T-0501 to T-0506,
//! [`TODO/cli.md`](../../../TODO/cli.md) T-0802.
//!
//! ⭐ **This verb is the product requirement the whole specification exists
//! for**: an agent that knows `docker` needs zero new knowledge. So its
//! argument surface, its output channels and its exit codes are docker's, and
//! where podbox cannot honour something it says so in one line rather than
//! pretending.

use std::io::Write;

use podbox_enter::{binfmt, Fds, Plan, RootDir};
use podbox_image::error::{EXIT_CLI_ERROR, EXIT_FLAG_ERROR};
use podbox_image::platform::Platform;
use podbox_image::transport::Policy;

/// ⭐ The option block, with no first line, because `create` is served by this
/// same parser and its usage must not say `run`. [`usage`] builds that line from
/// the verb the caller actually typed. One block, so a flag documented here
/// cannot be missing from the other verb's help. TODO/cli.md T-0801.
pub const RUN_OPTIONS: &str = "\
  -d, --detach     start the container and print its id, and do not wait
  --name NAME      a name for the container. note: Refused if one already has it
  --rm             remove the extracted rootfs when the payload exits,
                   unless a container record references it: the rootfs is
                   shared, and a failed run must not delete what `keeper`
                   needs (TODO/image.md T-1322)
  -e, --env K=V    set an environment variable. Repeatable; a later one wins
  --env-file F     read KEY=VALUE lines from F into the environment.
                   Repeatable; entries load at the flag's position, so a
                   later -e wins over the file (TODO/cli.md T-0801)
  -w, --workdir D  working directory inside the container
  -u, --user U:G   run as this identity: numeric uid and gid, or names from
                   the image's own passwd and group files. The requested id
                   is answered through the interposer identity memo for a
                   reachable payload, which the banner names and --strict
                   refuses (TODO/interpose.md T-0711)
  --entrypoint P   replace the image's entrypoint. note: As docker: this also
                   drops the image's Cmd, because those were that
                   entrypoint's default arguments
  --add-host N:IP  add a name to the container's /etc/hosts. Repeatable
  --no-source-fixup
                   leave the image's package sources exactly as extracted.
                   note: podbox rewrites http:// to https:// for a mirror that
                   answers over HTTPS, because tcp/80 HANGS on the runtimes
                   podbox targets. This turns that off, and UNDOES a rewrite
                   an earlier container made in the same shared rootfs
  --no-host-cas    do not append this machine's announced CA bundle
                   ($SSL_CERT_FILE, $CURL_CA_BUNDLE, $REQUESTS_CA_BUNDLE) to
                   the image's own trust store. note: Where this machine
                   intercepts TLS, an https package source then fails to
                   verify inside the container, exactly as it does under
                   docker
  --no-steps       refused: do not run any COMMAND inside the rootfs before the
                   payload. Two fixups cannot be made from outside the
                   chroot -- `pacman-key --init` for an empty keyring, and
                   `openssl rehash` for a hash-indexed CA directory, which is
                   the only trust store libzypp reads -- and podbox names
                   each on the banner before it runs it. This refuses them
                   all, and the fixup log then says what the caller gave up
  --strict         refused: refuse to run at all where anything about this
                   invocation is Degraded or Stub: a flag, the selected rung,
                   or a fixup the completion layer had to make
  --platform P     which platform of a multi-platform image to run
  --pull WHEN      never | missing (default) | always
  --insecure-registry HOST, --tls-verify=B
                   as `podbox pull`; used only when something must be fetched
  -t, --tty        refused: REFUSED BY NAME where /dev/ptmx is unusable, rather
                   than silently degraded (TODO/enter.md T-0503)
  --podbox-tier T  podbox's own: machine selects the machine tier, chroot the
                   chroot tier. `podvm` defaults to machine; an explicit flag
                   wins over argv[0], and podbox states the tier where the two
                   disagree (TODO/podvm.md T-1302)
  --podbox-qemu-arg A
                   podbox's own, machine tier only and refused elsewhere: one
                   token for the emulator per occurrence. Repeatable, and never
                   split on whitespace (TODO/podvm.md T-1302)
  --podbox-mem S   podbox's own, machine tier only and refused elsewhere: the
                   guest memory in bytes, with an optional K/M/G/T suffix. A
                   guest over the RLIMIT_FSIZE ceiling is refused before it
                   starts, naming both numbers (TODO/podvm.md T-1305)

  refused: podbox run enters a CHROOT, not a container. It shares this machine's
    process table, network, IPC and mount namespaces with the payload. The
    banner on stderr says so on every run and names what the selected rung
    must never claim.

  note: The payload owns stdout. Every word podbox prints goes to stderr, so
    `podbox run <image> cmd | consumer` gives the consumer the payload's
    bytes and nothing else.

  note: The exit code is the payload's own, and a signalled payload is 128+signal.
    125 is podbox failing to run the command, 126 the command found and not
    invocable, 127 not found. Those are docker's.
";

/// The usage text for whichever verb this parser is serving.
///
/// ⛔ `create` shares `run`'s parser, so before this existed `podbox create
/// --no-such-flag` answered `podbox run: unknown option` and printed
/// `usage: podbox run`, naming a verb the caller had not typed. A diagnostic
/// that names the wrong operation is the class TODO/cli.md T-0805 exists to
/// close, and T-0804 forbids outright.
pub fn usage(verb: &str) -> String {
    let mut s = format!("usage: podbox {verb} [options] <image> [command] [arg...]\n\n");
    if verb != "run" {
        // ⭐ Said rather than implied. The option set below is `run`'s, because
        // one parser serves both, and `podbox system info` says the same thing
        // about the same rows instead of holding a second copy of them.
        s.push_str(&format!(
            "  note: podbox {verb} takes podbox run's option set, listed below, because one\n    \
             parser serves both. `podbox system info` lists those rows under `run`.\n\n"
        ));
    }
    s.push_str(RUN_OPTIONS);
    s
}

#[derive(Debug)]
struct Opts {
    rm: bool,
    detach: bool,
    name: Option<String>,
    env: Vec<String>,
    workdir: Option<String>,
    entrypoint: Option<String>,
    platform: Option<String>,
    pull: String,
    insecure: Vec<String>,
    tls_verify: Option<bool>,
    tty: bool,
    image: Option<String>,
    command: Vec<String>,
    /// `--user`. Carried raw here and resolved against the image once its
    /// rootfs exists, because a name means the image's own passwd entry.
    user: Option<String>,
    /// `--podbox-tier`. Carried raw here and resolved against argv[0] in
    /// `prepare`, because the default depends on the name podbox was
    /// invoked under. TODO/podvm.md T-1302.
    tier: Option<crate::tier::Want>,
    /// `--podbox-qemu-arg`. One emulator token per occurrence, carried for
    /// the machine tier's driver (T-1303); refused on every other tier
    /// rather than silently dropped.
    qemu_args: Vec<String>,
    /// `--podbox-mem`. The validated guest memory in bytes, carried for the
    /// machine tier's ceiling check in `prepare`; refused on every other
    /// tier rather than silently dropped. TODO/podvm.md T-1305.
    mem: Option<u64>,
    /// M5 and T-0804. ⚠ Carried in one struct so `run`, `create` and the
    /// launcher cannot each grow their own copy of the same three answers.
    ask: crate::complete::Ask,
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
        eprintln!("podbox {verb}: --env-file {path:?}: {e}");
        EXIT_FLAG_ERROR
    })?;
    if !meta.is_file() {
        eprintln!("podbox {verb}: --env-file {path:?} is not a file");
        return Err(EXIT_FLAG_ERROR);
    }
    if meta.len() > MAX_ENV_FILE {
        eprintln!(
            "podbox {verb}: --env-file {path:?} is {} bytes, over the 1048576-byte ceiling",
            meta.len()
        );
        return Err(EXIT_FLAG_ERROR);
    }
    let text = std::fs::read_to_string(path).map_err(|e| {
        eprintln!("podbox {verb}: --env-file {path:?}: {e}");
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
                "podbox {verb}: --env-file {path:?} line {} has no `=`: {line:?}",
                n + 1
            );
            return Err(EXIT_FLAG_ERROR);
        };
        let key = k.trim();
        if key.is_empty() {
            eprintln!(
                "podbox {verb}: --env-file {path:?} line {} has no name: {line:?}",
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

/// ⛔ Parsing stops at the image name: everything after it is the payload's.
/// `podbox run alpine ls -l` must pass `-l` to `ls` and not read it as podbox's,
/// which is docker's rule and the one thing a caller cannot work around.
///
/// ⚠ `verb` is the one the CALLER typed, not the one whose rows are consulted.
/// `create` is served here too, and every message this function writes has to
/// name what the caller ran.
fn parse(verb: &str, args: &[String]) -> std::result::Result<Opts, i32> {
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
                    // driver podbox does not have and is refused naming it.
                    // TODO/supervise.md T-0605.
                    "json-file" => {}
                    other => {
                        eprintln!(
                            "podbox {verb}: --log-driver takes json-file, not {other:?}: \
                             podbox captures into one interleaved file per container"
                        );
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                "--label" => {
                    // ⚠ Stub by the table: accepted and dropped. Podbox
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
                "--podbox-tier" => match crate::tier::want(a) {
                    Some(w) => o.tier = Some(w),
                    None => {
                        eprintln!(
                            "podbox {verb}: --podbox-tier takes machine or chroot, not {a:?}"
                        );
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                "--podbox-qemu-arg" => o.qemu_args.push(a.clone()),
                "--podbox-mem" => match crate::tier::parse_mem(a) {
                    Some(m) => o.mem = Some(m),
                    None => {
                        eprintln!(
                            "podbox {verb}: --podbox-mem takes a byte count with an optional K/M/G/T suffix, not {a:?}"
                        );
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
            "-i" | "--interactive" => {
                // ⚠ Accepted and a no-op, deliberately: podbox does not detach
                // stdin, so it is already interactive when the caller's is.
                // TODO/cli.md T-0801 calls this Stub and the banner lists it.
            }
            "-e" | "--env" => expecting = Some("-e"),
            "--env-file" => expecting = Some("--env-file"),
            "--label" => expecting = Some("--label"),
            "--log-driver" => expecting = Some("--log-driver"),
            "--attach" => {
                // ⚠ Stub by the table: podbox always captures stdout and
                // stderr together, so stream selection changes nothing.
            }
            "--expose" => {
                // ⚠ Stub by the table: docker's --expose only documents
                // ports and podbox publishes none, so the run is unchanged.
            }
            "-w" | "--workdir" => expecting = Some("-w"),
            "-u" | "--user" => expecting = Some("--user"),
            "--entrypoint" => expecting = Some("--entrypoint"),
            "--platform" => expecting = Some("--platform"),
            "--pull" => expecting = Some("--pull"),
            "--insecure-registry" => expecting = Some("--insecure-registry"),
            "--tls-verify" => o.tls_verify = Some(true),
            "--add-host" => expecting = Some("--add-host"),
            "--podbox-tier" => expecting = Some("--podbox-tier"),
            "--podbox-qemu-arg" => expecting = Some("--podbox-qemu-arg"),
            "--podbox-mem" => expecting = Some("--podbox-mem"),
            other if other.starts_with("--add-host=") => {
                crate::complete::add_host(&mut o.ask, verb, &other[11..])?
            }
            "--no-source-fixup" => o.ask.no_source_fixup = true,
            "--no-host-cas" => o.ask.no_host_cas = true,
            "--no-steps" => o.ask.no_steps = true,
            "--strict" => o.ask.strict = true,
            other if other.starts_with("--env=") => o.env.push(other[6..].to_string()),
            other if other.starts_with("--log-driver=") => match &other[13..] {
                "json-file" => {}
                v => {
                    eprintln!(
                        "podbox {verb}: --log-driver takes json-file, not {v:?}: \
                         podbox captures into one interleaved file per container"
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
            other if other.starts_with("--entrypoint=") => {
                o.entrypoint = Some(other[13..].to_string())
            }
            other if other.starts_with("--platform=") => o.platform = Some(other[11..].to_string()),
            other if other.starts_with("--pull=") => o.pull = other[7..].to_string(),
            other if other.starts_with("--podbox-tier=") => match crate::tier::want(&other[14..]) {
                Some(w) => o.tier = Some(w),
                None => {
                    eprintln!(
                        "podbox {verb}: --podbox-tier takes machine or chroot, not {:?}",
                        &other[14..]
                    );
                    return Err(EXIT_FLAG_ERROR);
                }
            },
            other if other.starts_with("--podbox-qemu-arg=") => {
                o.qemu_args.push(other[18..].to_string())
            }
            other if other.starts_with("--podbox-mem=") => {
                match crate::tier::parse_mem(&other[13..]) {
                    Some(m) => o.mem = Some(m),
                    None => {
                        eprintln!(
                            "podbox {verb}: --podbox-mem takes a byte count with an optional K/M/G/T suffix, not {:?}",
                            &other[13..]
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
                    eprintln!("podbox {verb}: --tls-verify takes true or false, not {v:?}");
                    return Err(EXIT_FLAG_ERROR);
                }
            },
            other if other.starts_with('-') => {
                // ⛔ Unreachable through the table above, and it is here as an
                // assertion rather than as a fallback: the table admitted this
                // flag and this parser has no arm for it, which is a defect in
                // podbox and is reported as one.
                return Err(crate::parity::no_arm(verb, other));
            }
            other => o.image = Some(other.to_string()),
        }
        i += 1;
    }
    if let Some(flag) = expecting {
        eprintln!("podbox {verb}: {flag} needs a value");
        return Err(EXIT_FLAG_ERROR);
    }
    if o.image.is_none() {
        eprint!("{}", usage(verb));
        return Err(EXIT_CLI_ERROR);
    }
    if !matches!(o.pull.as_str(), "never" | "missing" | "always") {
        eprintln!(
            "podbox {verb}: --pull takes never, missing or always, not {:?}",
            o.pull
        );
        return Err(EXIT_FLAG_ERROR);
    }
    Ok(o)
}

pub fn run(args: &[String]) -> i32 {
    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("podbox run: {e}");
            return e.exit_code();
        }
    };
    let p = match prepare(args, &store, "run") {
        Ok(p) => p,
        Err(code) => return code,
    };
    let mut err = std::io::stderr().lock();

    // ⭐ M4. `-d` hands the container to a detached launcher and returns as soon
    // as its payload is running, which is a condition rather than a duration:
    // TODO/supervise.md T-0602.
    if p.detach {
        // ⚠ The banner was printed by `prepare`, which had to print it before
        // running T-0412's steps inside the rootfs.
        drop(err);
        let c = match podbox_supervise::create(
            &store,
            p.name.as_deref(),
            &p.image,
            &p.record.manifest_digest,
            &p.rootfs,
            p.argv.clone(),
            p.env.clone(),
            p.working_dir.clone(),
            &p.rung,
            p.completion.clone(),
            p.completion_degraded,
        ) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("podbox run: {e}");
                let _ = std::fs::remove_file(&p.memo_host_path);
                return podbox_image::error::EXIT_RUNTIME_ERROR;
            }
        };
        // ⭐ T-0710: the ephemeral memo becomes the container's, beside its
        // record. The environment already carries the constant descriptor
        // number, so nothing in it moves with the file.
        let dest = podbox_supervise::table::memo_path(&store, &c.id);
        if let Err(e) = std::fs::rename(&p.memo_host_path, &dest) {
            eprintln!("podbox run: the ownership memo could not be stored: {e}");
            let _ = podbox_supervise::remove(&store, &c.id, true);
            return podbox_image::error::EXIT_RUNTIME_ERROR;
        }
        return match podbox_supervise::start(&store, &c.id, &p.record) {
            Ok(c) => {
                println!("{}", c.id);
                0
            }
            Err(e) => {
                eprintln!("podbox run: {e}");
                // ⚠ The record is removed again: a container that could not be
                // started is not one `ps` should list as created, and leaving
                // it would also hold its name against a retry.
                let _ = podbox_supervise::remove(&store, &c.id, true);
                podbox_image::error::EXIT_RUNTIME_ERROR
            }
        };
    }

    // ---------------------------------------------------- the foreground entry
    let held = match store.hold(&p.record) {
        Ok(h) => h,
        Err(e) => {
            let _ = writeln!(err, "podbox run: {e}");
            let _ = std::fs::remove_file(&p.memo_host_path);
            return podbox_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    let root = match RootDir::open(&p.rootfs) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(err, "podbox run: {e}");
            let _ = std::fs::remove_file(&p.memo_host_path);
            return e.exit_code();
        }
    };
    // ⭐ T-0710: the host memo, handed as a descriptor that survives the
    // `chroot` and the `execve`. The file lives beside no container record
    // here, but still on the host under staging, where the payload cannot
    // reach it by path.
    let memo = match crate::interpose::open_memo(&p.memo_host_path) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(
                err,
                "podbox run: the ownership memo could not be opened: {e}"
            );
            let _ = std::fs::remove_file(&p.memo_host_path);
            return podbox_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    use std::os::fd::AsRawFd;
    let memo_host = memo.as_raw_fd() as i64;
    let plan = Plan {
        argv: p.argv.clone(),
        env: p.env.clone(),
        working_dir: p.working_dir.clone(),
        fds: Fds {
            pass: vec![(crate::interpose::MEMO_CHILD_FD, memo_host)],
        },
        // ⚠ Empty, and that is the same reason the launcher's is: `prepare`
        // printed the banner, because T-0412's steps run inside the rootfs
        // after it and every one of them has to be named before it runs.
        banner: String::new(),
        path_dirs: Plan::path_from(&p.env),
    };
    // ⭐ T-0204 and T-0211. The lock is handed to the payload immediately
    // before the fork that leads to its exec, and to nothing else.
    if let Err(e) = held.hand_to_payload() {
        let _ = writeln!(err, "podbox run: {e}");
        drop(memo);
        let _ = std::fs::remove_file(&p.memo_host_path);
        return podbox_image::error::EXIT_RUNTIME_ERROR;
    }
    // ⭐ TODO/packaging.md T-1003. A forced launch rung drives through the
    // ladder; unset means the chroot by path below, untouched. The banner
    // names the entered rung, so a forced run says where it went.
    let code = match p.ladder {
        Some(mode) => {
            let _ = writeln!(
                err,
                "podbox run: entering on the {} rung at PODBOX_MODE's request \
                 (TODO/packaging.md T-1003)",
                mode.name()
            );
            let argv0 = plan.argv.first().cloned().unwrap_or_default();
            match crate::ladder::enter_forced(
                &root,
                &plan,
                mode,
                &p.rootfs,
                &argv0,
                &plan.path_dirs,
                &p.findings,
                store.root(),
                &p.record.manifest_digest,
                &mut err,
            ) {
                Ok(c) => c,
                Err(e) => {
                    let _ = writeln!(err, "podbox run: {e}");
                    e.exit_code()
                }
            }
        }
        None => {
            // ⭐ TODO/enter.md T-1317. The no-chroot families enter here: the
            // loader argv was decided pre-fixup, and the static family's
            // memfd stages now, against post-fixup bytes, mirroring the
            // ladder's forced drive.
            if let Some(u) = &p.userland {
                let active = podbox_probe::select::Rung::Userland.word();
                let (exec_argv, fd) = match u.family {
                    podbox_enter::userland::Family::Loader => (u.loader_argv.clone(), None),
                    podbox_enter::userland::Family::Memfd => {
                        let argv0 = plan.argv.first().cloned().unwrap_or_default();
                        let bytes = match podbox_enter::ladder::payload_bytes(
                            &p.rootfs,
                            &argv0,
                            &plan.path_dirs,
                        ) {
                            Ok(b) => b,
                            Err(e) => {
                                let _ = writeln!(err, "podbox run: {e}");
                                drop(memo);
                                let _ = std::fs::remove_file(&p.memo_host_path);
                                return e.exit_code();
                            }
                        };
                        if podbox_enter::memfd::eligible(&bytes).is_err() {
                            // Fixups changed what the decision read: loud,
                            // never silent, and the fixups stay (they are the
                            // image's now, not this run's).
                            let _ = writeln!(
                                err,
                                "podbox run: the payload the memfd family was decided \
                                 on no longer stages: refusing rather than entering \
                                 something unjudged (TODO/enter.md T-1317)"
                            );
                            drop(memo);
                            let _ = std::fs::remove_file(&p.memo_host_path);
                            return podbox_image::error::EXIT_RUNTIME_ERROR;
                        }
                        match podbox_enter::memfd::stage(&bytes) {
                            Ok(f) => (plan.argv.clone(), Some(f)),
                            Err(e) => {
                                let _ = writeln!(err, "podbox run: {e}");
                                drop(memo);
                                let _ = std::fs::remove_file(&p.memo_host_path);
                                return e.exit_code();
                            }
                        }
                    }
                };
                match podbox_enter::run_userland(&root, &plan, exec_argv, active, fd, &mut err) {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = writeln!(err, "podbox run: {e}");
                        e.exit_code()
                    }
                }
            } else {
                match podbox_enter::run(&root, &plan, &mut err) {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = writeln!(err, "podbox run: {e}");
                        e.exit_code()
                    }
                }
            }
        }
    };
    drop(err);
    // ⭐ T-0710: the ephemeral memo goes with the run. The child holds its own
    // dup past the `execve`; this handle and the staging file are the parent's.
    drop(memo);
    let _ = std::fs::remove_file(&p.memo_host_path);

    if p.rm {
        // ⚠ The lock is dropped first: `remove_extracted` deletes the tree the
        // lock exists to protect, and holding it while deleting would be podbox
        // refusing its own request.
        drop(held);
        // ⭐ TODO/image.md T-1322. The rootfs is shared, so `--rm` removes it
        // only where no container record references it: a failed run must
        // not delete the rootfs `keeper` needs. A run of its own writes no
        // record, so an ephemeral run never counts itself. Where the query
        // itself errors, the rootfs stays: deleting on an unanswered
        // question is how the defect above happens.
        match podbox_supervise::referencing(&store, &p.record.manifest_digest) {
            Ok(referrers) if !referrers.is_empty() => {
                let mut names: Vec<&str> = referrers.iter().map(|c| c.name.as_str()).collect();
                names.sort_unstable();
                eprintln!(
                    "podbox run: --rm keeps the rootfs: it is referenced by container {}",
                    names.join(", ")
                );
            }
            Ok(_) => {
                if let Err(e) = podbox_extract::remove_extracted(&store, &p.record.manifest_digest)
                {
                    eprintln!("podbox run: --rm could not remove the rootfs: {e}");
                }
            }
            Err(e) => {
                eprintln!("podbox run: --rm could not check references: {e}");
            }
        }
    }
    code
}

/// Everything `run` and `create` both need: the image in the store, extracted,
/// its plan resolved and the rung selected, with nothing entered yet.
///
/// ⛔ ONE PATH for both verbs. `create` that resolved its own image would be a
/// second implementation of the hardest half of `run`, and the one nobody
/// exercises is the one that diverges.
pub(crate) fn prepare(
    args: &[String],
    store: &podbox_image::Store,
    verb: &str,
) -> std::result::Result<crate::lifecycle::Prepared, i32> {
    let o = parse(verb, args)?;
    // ⭐ TODO/podvm.md T-1302. The tier is decided before anything is
    // fetched: a machine-tier refusal names the legs, and there is nothing
    // to pull for a tier that cannot run.
    let tier = crate::tier::resolve(&crate::names::invoked_as(), o.tier);
    if let Some(note) = &tier.note {
        eprintln!("podbox {verb}: {note}");
    }
    // ⭐ TODO/packaging.md T-1003. A forced launch rung is refused here, before
    // anything is fetched, where the verb cannot honor it: only foreground
    // `run` drives the ladder, and the machine tier never reaches it. It sits
    // ahead of the machine branch so a force cannot fall through it silently.
    if let Err(text) = crate::ladder::refuse_where_undriven(
        verb,
        o.detach,
        tier.tier == crate::tier::Tier::Machine,
    ) {
        eprintln!("podbox {verb}: {text}");
        return Err(podbox_image::error::EXIT_RUNTIME_ERROR);
    }
    let ladder = match crate::ladder::forced_mode() {
        Ok(forced) => forced,
        Err(e) => {
            eprintln!("podbox {verb}: {e}");
            return Err(e.exit_code());
        }
    };
    if tier.tier == crate::tier::Tier::Machine {
        return Err(crate::tier::enter_machine(verb, o.mem));
    }
    if !o.qemu_args.is_empty() {
        // ⛔ Refused rather than silently dropped. An emulator argument the
        // chroot tier accepts and ignores is a limit the caller believes is
        // set and podbox never passed anywhere.
        eprintln!("podbox {verb}: --podbox-qemu-arg needs --podbox-tier=machine");
        return Err(EXIT_FLAG_ERROR);
    }
    if o.mem.is_some() {
        // ⛔ The same rule for the memory the machine tier judges against
        // the file-size ceiling: accepted-and-ignored elsewhere would be a
        // limit the caller believes is enforced. TODO/podvm.md T-1305.
        eprintln!("podbox {verb}: --podbox-mem needs --podbox-tier=machine");
        return Err(EXIT_FLAG_ERROR);
    }
    let image = o.image.clone().expect("checked in parse");

    let (platform, policy) = crate::lifecycle::platform_and_policy(
        verb,
        o.platform.as_deref(),
        &o.insecure,
        o.tls_verify,
    )?;
    // ⭐ TODO/milestones.md T-1112. The OS gate sits before the fetch: there
    // is nothing to pull for a platform no tier can enter.
    crate::lifecycle::ensure_linux_guest(verb, &platform.os, &platform.arch)?;
    // ⭐ TODO/enter.md T-1317. The probe runs HERE, before the fetch, the
    // lock, the extraction and every fixup: the entered rung always
    // chroots, so a denied chroot refuses now, naming chroot(2), with the
    // rootfs untouched. The machine tier returned above and never chroots.
    // `findings` is reused for the banner below, so the probe runs once.
    let findings = podbox_probe::run();
    // ⭐ TODO/enter.md T-0503. The flag-specific refusal fires where the
    // flag alone decides, ahead of every gate: `-t` with an unusable ptmx
    // refuses naming ptmx whatever else holds, because no rung here
    // allocates a pty without it. Where chroot is denied too the chroot
    // sentence prints beside it rather than being masked by it. Exit
    // stays 125, one line per reason.
    if o.tty && !podbox_probe::probes::ptmx_usable(&findings) {
        eprintln!(
            "podbox {verb}: -t was asked for and /dev/ptmx is not usable on this \
             machine, so podbox cannot allocate a pty. It refuses rather than \
             running without one and letting the payload discover it \
             (TODO/enter.md T-0503)"
        );
        if !podbox_probe::probes::chroot_usable(&findings) {
            eprintln!(
                "podbox {verb}: chroot(2) is denied on this machine as well \
                 (see `podbox probe`) (TODO/enter.md T-1317)"
            );
        }
        return Err(podbox_enter::EXIT_RUNTIME_ERROR);
    }
    // ⭐ TODO/enter.md T-1317. Records promise a launcher entry, which
    // chroots, so `create` keeps the strict gate. Foreground `run` takes
    // the two-tier gate: the exact rung needs the payload, which is
    // post-extract, so this refuses only where nothing could run.
    if verb == "create" {
        crate::lifecycle::ensure_chroot_usable(verb, &findings)?;
    } else {
        crate::lifecycle::ensure_entry_possible(verb, &findings)?;
    }

    // ------------------------------------------------------------- the image
    let record = acquire(verb, store, &image, &platform, &policy, &o.pull)?;
    // ⭐ TODO/milestones.md T-1112. The stored record is what gets entered,
    // so its OS is gated too: a record for another OS answers here rather
    // than inside the guest.
    crate::lifecycle::ensure_linux_guest(verb, &record.os, &record.architecture)?;

    // ⛔ The lock BEFORE the extraction check, so a concurrent `rmi` cannot
    // delete the rootfs between podbox deciding it is there and entering it.
    // TODO/image.md T-0204. ⚠ Dropped at the end of this function: the caller
    // takes its own, for the process that actually enters.
    let held = store.hold(&record).map_err(|e| {
        eprintln!("podbox {verb}: {e}");
        podbox_image::error::EXIT_RUNTIME_ERROR
    })?;

    if !podbox_extract::is_extracted(store, &record.manifest_digest) {
        extract_now(verb, store, &record)?;
    }
    let (rootfs, _) = podbox_extract::paths(store, &record.manifest_digest);
    let rootfs = rootfs.to_string_lossy().to_string();

    // ---------------------------------------------------- the foreign platform
    let host = Platform::host();
    let support = binfmt::support_for(&record.architecture, &host.arch);
    let mut err = std::io::stderr().lock();
    match &support {
        binfmt::Support::Native => {}
        binfmt::Support::Interpreter(r) => {
            // ⭐ The F flag: the kernel holds the interpreter open, so it works
            // inside the chroot with nothing copied in.
            let _ = writeln!(
                err,
                "podbox: this image is {}/{} and this machine is {}. Running it \
                 through the registered interpreter {} (binfmt flag F, so it is \
                 reachable inside the chroot)",
                record.os, record.architecture, host, r.interpreter
            );
        }
        binfmt::Support::InterpreterNeedsCopyIn(r) => {
            // ⛔ podbox is about to WRITE A FILE INTO SOMEBODY ELSE'S IMAGE.
            // T-0506: the honesty rules have no exception for a helpful edit,
            // and the payload can see this file.
            match binfmt::copy_interpreter_in(&rootfs, r) {
                Ok(dest) => {
                    // ⛔ podbox has written a file into somebody else's image.
                    // T-0506: the honesty rules have no exception for a helpful
                    // edit, and the payload can see that file.
                    let _ = writeln!(
                        err,
                        "podbox: this image is {}/{} and this machine is {}. Its \
                         interpreter {} is registered WITHOUT the binfmt F flag, so \
                         the kernel opens it by path at exec time and that path is \
                         inside the container. refused: podbox has COPIED it to {} inside \
                         the image's own rootfs; the payload can see that file.",
                        record.os,
                        record.architecture,
                        host,
                        r.interpreter,
                        dest.display()
                    );
                }
                Err(e) => {
                    let _ = writeln!(
                        err,
                        "podbox {verb}: this image is {}/{} and this machine is {}. Its \
                         interpreter {} lacks the binfmt F flag and could not be \
                         copied into the rootfs: {e}",
                        record.os, record.architecture, host, r.interpreter
                    );
                    return Err(podbox_enter::EXIT_RUNTIME_ERROR);
                }
            }
        }
        binfmt::Support::None { why } => {
            // ⛔ Refused with everything a caller needs, never an `Exec format
            // error` from the kernel with nothing attached to it.
            let _ = writeln!(
                err,
                "podbox {verb}: {image} is {}/{} and this machine is {}. podbox \
                 cannot execute it: {why}",
                record.os, record.architecture, host
            );
            return Err(podbox_enter::EXIT_RUNTIME_ERROR);
        }
    }
    let support_native = matches!(support, binfmt::Support::Native);

    // --------------------------------------------------------------- the plan
    let cfg = config_of(verb, store, &record)?;
    let argv = Plan::argv_for(
        cfg.config.entrypoint.as_deref(),
        cfg.config.cmd.as_deref(),
        &o.command,
        o.entrypoint.as_deref(),
    );
    if argv.is_empty() {
        eprintln!(
            "podbox {verb}: {image} declares neither an Entrypoint nor a Cmd, and no \
             command was given. There is nothing to run"
        );
        return Err(EXIT_FLAG_ERROR);
    }
    let mut env = Plan::env_for(&cfg.config.env, &o.env);
    // ⭐ T-0711: the requested identity travels in the environment, resolved
    // against the image, before anything classifies the payload: the object
    // reads it on first use in every process, which is what survives fork
    // and exec where a file descriptor would need re-handing.
    crate::lifecycle::apply_user(verb, &rootfs, o.user.as_ref(), &mut env)?;
    // ⭐ T-0710: the host memo, beside the container record once it exists.
    // Ephemeral here under staging; `create` and `run -d` move it to
    // `containers/<id>/`, foreground `run` deletes it on exit. The descriptor
    // number is constant, so the environment set here is final for every later
    // entry into the same container.
    let memo_host_path = crate::interpose::ephemeral_memo_path(store);
    if let Err(e) = crate::interpose::ensure_memo_file(&memo_host_path) {
        eprintln!("podbox {verb}: the ownership memo could not be created: {e}");
        return Err(podbox_image::error::EXIT_RUNTIME_ERROR);
    }
    env.retain(|e| e.split('=').next().unwrap_or("") != crate::interpose::MEMO_FD_VAR);
    env.push(crate::interpose::memo_fd_env());
    // ⭐ TODO/enter.md T-1317. The exact rung, decided here: the rootfs
    // exists and nothing has been written into it yet (`place` and the
    // fixups run below). `create` never decides past its strict gate
    // above, so only foreground `run` takes a no-chroot entry here.
    // A detached run has no userland launcher yet and refuses naming
    // the foreground fallback rather than starting unstartable.
    let userland =
        crate::lifecycle::decide_entry(verb, &rootfs, &argv, &env, support_native, &findings)?;
    if userland.is_some() && o.detach {
        eprintln!(
            "podbox {verb}: chroot(2) is denied and only the no-chroot families \
             run here, which `run -d` does not drive: run foreground \
             (TODO/enter.md T-1317)"
        );
        return Err(podbox_image::error::EXIT_RUNTIME_ERROR);
    }
    // ⭐ T-0702 and T-0706: classify the payload and place the object BEFORE
    // the banner is built, so the banner names the write before anything of
    // the payload's runs. The note joins the banner below.
    let mut interpose_note = String::new();
    if let Err(e) = crate::interpose::apply(verb, &rootfs, &argv, &mut env, &mut interpose_note) {
        eprintln!("{e}");
        return Err(podbox_image::error::EXIT_RUNTIME_ERROR);
    }
    // ⭐ TODO/enter.md T-1317. Without a chroot the loader resolves guest
    // paths on the host root: the preload becomes the host path and the
    // libraries come from the image. A caller preload naming guest paths
    // is dropped and named, because it cannot resolve.
    let mut dropped_preload: Option<String> = None;
    if let Some(u) = &userland {
        if u.family == podbox_enter::userland::Family::Loader {
            let (hosted, dropped) = podbox_enter::userland::host_env(
                &rootfs,
                crate::interpose::GUEST_PATH,
                &env,
                &u.lib_dirs,
            );
            env = hosted;
            dropped_preload = dropped;
        }
    }
    let path_dirs = Plan::path_from(&env);
    let working_dir = o
        .workdir
        .clone()
        .unwrap_or_else(|| cfg.config.working_dir.clone());

    // ------------------------------------------------------------- the banner
    // ⭐ TODO/probe.md T-0107 and T-0108's remaining halves: the rung is
    // selected here, for a real entry, and the banner names it and what it must
    // never claim. The probe ran up front (T-1317's gate); the banner reuses
    // its findings.
    let selection = podbox_probe::select::Selection::choose(&findings);
    // ⭐ T-0804 rule 4. The banner is built from the rung podbox ENTERS with,
    // not the one the machine would permit: the chroot sequence or, where
    // chroot is denied and a no-chroot family runs, the userland one. One
    // value, so the banner, the container record and `--strict` cannot
    // disagree.
    let entered = userland
        .as_ref()
        .map(|_| podbox_probe::select::Rung::Userland)
        .unwrap_or(podbox_enter::ENTERED_RUNG);
    let mut banner = podbox_probe::report::entry_banner(&findings, &selection, entered);
    // ⭐ TODO/cli.md T-0803. Where podbox was reached under somebody else's
    // name, the banner says which name was used and that this is podbox.
    // Taking the name is the product requirement; taking it silently is what
    // TOOL.md section 4.1 forbids.
    if let Some(note) = crate::names::alias_note() {
        banner.push_str(&note);
    }
    banner.push_str(&interpose_note);
    // ⭐ TODO/enter.md T-1317. The family account joins the banner beside
    // the mode: what runs the payload and what it does not isolate.
    if let Some(u) = &userland {
        banner.push_str(&u.banner);
        banner.push('\n');
        if let Some(d) = &dropped_preload {
            banner.push_str(&format!(
                "podbox: userland: caller LD_PRELOAD {d:?} dropped: it names guest \
                 paths, which resolve on the host without a chroot\n"
            ));
        }
    }
    // ⭐ M5. The completion layer runs HERE: after the rootfs exists and the
    // image lock is held, and before anything is entered. Its report is part of
    // the banner, because every one of these is an edit podbox made inside
    // somebody else's image and the payload can see it.
    let mut ask = o.ask.clone();
    ask.container_name = o.name.clone();
    let mut completion = crate::complete::prepare(verb, &rootfs, &ask, &mut banner)?;
    if !cfg.config.user.is_empty() {
        // ⚠ Read and REPORTED, never applied. podbox cannot setuid to an id
        // this machine does not map, which is the wall the project is about.
        banner.push_str(&format!(
            "podbox: the image asks to run as user {:?}; podbox runs as uid 0 \
             and does not change to it\n",
            cfg.config.user
        ));
    }
    // ⭐ T-0804 rule 1, applied HERE and nowhere else, so `run`, `run -d`,
    // `create` and the launcher cannot disagree about whether this machine
    // prints it. ⛔ It moved ahead of every refusal below on purpose: a refused
    // caller used to get the reasons with no banner under `--strict`, and
    // `create` printed neither, while T-0412's steps mean podbox may now run a
    // command inside the image, which rule 3 says is named before it runs.
    let quiet = crate::complete::banner_quiet(store);
    if !quiet {
        let _ = write!(err, "{banner}");
    }
    // ⛔ T-0503, decided up front beside the entry gate above: `-t` with
    // an unusable ptmx refuses there naming ptmx, so no `-t` check
    // remains here. `exec` keeps its own: it is ungated by chroot, so
    // nothing there masks it.
    // ⭐ T-0804, and it is the LAST thing before the run is committed to, so a
    // refused caller still gets the whole account of why on stderr. ⛔ The
    // refusal prints even where the banner is suppressed: the config switch
    // silences a notice, never a refusal.
    crate::complete::strict_refusal(verb, &ask, entered.word(), &completion, &mut err)?;
    // ⭐ T-0412 and T-0710. The steps share this entry's host memo: a fixup's
    // `chown` must land in the same record the payload reads, or the two
    // disagree about who owns the file. Opened here for the steps alone; the
    // payload's own handle is opened by the caller that spawns it.
    {
        use std::os::fd::AsRawFd;
        let memo_for_steps = crate::interpose::open_memo(&memo_host_path).ok();
        let memo_host = memo_for_steps.as_ref().map(|f| f.as_raw_fd() as i64);
        crate::complete::run_steps_with_memo(
            verb,
            &rootfs,
            &env,
            &mut completion,
            quiet,
            &mut err,
            memo_host,
        );
    }
    let _ = path_dirs;
    // ⚠ The lock this function took goes here. It existed to keep the rootfs
    // from being deleted between the extraction check and now; the process that
    // ENTERS takes its own, and for `-d` that is the launcher rather than this.
    drop(held);
    Ok(crate::lifecycle::Prepared {
        record,
        image,
        rootfs,
        argv,
        env,
        working_dir,
        name: o.name.clone(),
        rung: entered.word().to_string(),
        detach: o.detach,
        rm: o.rm,
        memo_host_path,
        ladder,
        findings,
        completion: completion
            .fixups
            .iter()
            .filter(|f| f.action.changed() || f.action == podbox_complete::Action::Failed)
            .map(|f| {
                format!(
                    "{} {} ({}, {})",
                    f.action.word(),
                    if f.path.is_empty() { "-" } else { &f.path },
                    f.id,
                    f.entry
                )
            })
            .collect(),
        completion_degraded: completion.degradations().len(),
        userland,
    })
}

/// Make sure the store holds the image, honouring `--pull`.
fn acquire(
    verb: &str,
    store: &podbox_image::Store,
    image: &str,
    platform: &Platform,
    policy: &Policy,
    pull: &str,
) -> std::result::Result<podbox_image::Record, i32> {
    let found = store.find_for(image, Some(platform)).unwrap_or_default();
    let others = found.other_platforms.clone();
    let have = found.one();
    match (pull, have) {
        ("never", Some(r)) => Ok(r),
        ("never", None) => {
            // ⛔ Two different sentences for two different situations. "held,
            // for another platform" sends the caller to --platform; "not held"
            // sends them to a pull. Collapsing them into "no such image" is the
            // message a caller cannot act on.
            if others.is_empty() {
                eprintln!(
                    "podbox {verb}: {image} is not in the store and --pull never was \
                     given, so podbox will not fetch it"
                );
            } else {
                eprintln!(
                    "podbox {verb}: the store holds {image} for {}, and {platform} was \
                     asked for. --pull never was given, so podbox will not fetch \
                     the one that was asked for",
                    others.join(", ")
                );
            }
            Err(podbox_image::error::EXIT_RUNTIME_ERROR)
        }
        ("missing", Some(r)) => Ok(r),
        _ => {
            // ⛔ The transcript goes to STDERR here, unlike `podbox pull` where
            // it is the output. The payload owns stdout.
            let mut err = std::io::stderr().lock();
            match podbox_image::pull::pull(store, image, platform, policy, &mut err) {
                Ok(done) => Ok(done.record),
                Err(e) => {
                    let _ = writeln!(err, "podbox {verb}: {e}");
                    Err(e.exit_code())
                }
            }
        }
    }
}

/// Ensure the record is extracted, holding nothing: the caller holds the
/// image lock across the check and the extract (T-0204), because the
/// rootfs below must not be deleted between them. Shared by `run`'s
/// prepare and `cp`'s image addressing (TODO/cli.md T-1323): one path
/// for "the extracted rootfs in the store".
pub(crate) fn extract_now(
    verb: &str,
    store: &podbox_image::Store,
    record: &podbox_image::Record,
) -> std::result::Result<(), i32> {
    let d = podbox_image::digest::Digest::parse(&record.manifest_digest).map_err(|e| {
        eprintln!("podbox {verb}: {e}");
        e.exit_code()
    })?;
    let bytes = store.read_blob(&d).map_err(|e| {
        eprintln!("podbox {verb}: {e}");
        e.exit_code()
    })?;
    let manifest: podbox_image::oci::Manifest = serde_json::from_slice(&bytes).map_err(|e| {
        eprintln!("podbox {verb}: the manifest does not parse: {e}");
        podbox_image::error::EXIT_RUNTIME_ERROR
    })?;
    let mut err = std::io::stderr().lock();
    let done = podbox_extract::extract(store, &manifest, &record.manifest_digest, &mut err)
        .map_err(|e| {
            let _ = writeln!(err, "podbox {verb}: {e}");
            podbox_image::error::EXIT_RUNTIME_ERROR
        })?;
    crate::diagnose::report_dropped(&mut err, &done, &podbox_probe::identity::read());
    Ok(())
}

/// The image's config blob.
///
/// ⚠ `pub(crate)` because `exec` reads the same blob for the same reason, and a
/// second copy of it is the copy that drifts: `docs/conventions/code.md`.
pub(crate) fn config_of(
    verb: &str,
    store: &podbox_image::Store,
    record: &podbox_image::Record,
) -> std::result::Result<podbox_image::oci::Config, i32> {
    let d = podbox_image::digest::Digest::parse(&record.config_digest).map_err(|e| {
        eprintln!("podbox {verb}: {e}");
        e.exit_code()
    })?;
    let bytes = store.read_blob(&d).map_err(|e| {
        eprintln!("podbox {verb}: {e}");
        e.exit_code()
    })?;
    serde_json::from_slice(&bytes).map_err(|e| {
        eprintln!("podbox {verb}: the image config does not parse: {e}");
        podbox_image::error::EXIT_RUNTIME_ERROR
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn every_usage_string_is_plain_ascii() {
        // TODO/cli.md T-1336: every `--help` path prints bytes 0x00-0x7F
        // only. `run`'s usage is the shared one (`create` and `exec`
        // print it too), so holding this one holds all three.
        let text = usage("run") + &usage("create") + RUN_OPTIONS;
        let bad = text.bytes().filter(|b| *b > 0x7F).count();
        assert_eq!(bad, 0, "run usage carries {bad} non-ASCII bytes");
    }

    /// ⛔ The rule a caller cannot work around if podbox gets it wrong.
    #[test]
    fn parsing_stops_at_the_image_name() {
        let o = parse("run", &v(&["--rm", "alpine", "ls", "-l", "--rm"])).unwrap();
        assert!(o.rm);
        assert_eq!(o.image.as_deref(), Some("alpine"));
        // ⛔ `-l` and the SECOND `--rm` belong to `ls`, not to podbox.
        assert_eq!(o.command, v(&["ls", "-l", "--rm"]));
    }

    #[test]
    fn a_flag_needing_a_value_does_not_swallow_the_image() {
        assert_eq!(
            parse("run", &v(&["--platform"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(parse("run", &v(&["-e"])).unwrap_err(), EXIT_FLAG_ERROR);
    }

    #[test]
    fn both_spellings_of_every_valued_flag_reach_the_same_place() {
        let a = parse(
            "run",
            &v(&["-e", "A=1", "-w", "/w", "--entrypoint", "/e", "img"]),
        )
        .unwrap();
        let b = parse(
            "run",
            &v(&["--env=A=1", "--workdir=/w", "--entrypoint=/e", "img"]),
        )
        .unwrap();
        assert_eq!(a.env, b.env);
        assert_eq!(a.workdir, b.workdir);
        assert_eq!(a.entrypoint, b.entrypoint);
        assert_eq!(a.image, b.image);
    }

    #[test]
    fn an_unknown_pull_mode_is_invalid_input_and_not_a_silent_default() {
        assert_eq!(
            parse("run", &v(&["--pull", "sometimes", "img"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            parse("run", &v(&["--pull", "always", "img"])).unwrap().pull,
            "always"
        );
        assert_eq!(parse("run", &v(&["img"])).unwrap().pull, "missing");
    }

    /// ⭐ TODO/cli.md T-0801. Every flag the table ADMITS for this verb reaches
    /// an arm of this parser. The other direction is held by construction: an
    /// argument starting with `-` goes through `parity::admit` first, so an arm
    /// for a flag with no row is unreachable rather than a second surface.
    #[test]
    fn every_flag_the_table_admits_is_handled_by_this_parser() {
        for r in crate::parity::TABLE
            .iter()
            .filter(|r| r.verb == "run" && r.flag.is_some())
        {
            if r.status == crate::parity::Status::None {
                // ⚠ A `None` row is refused BY `admit`, so reaching an arm is
                // exactly what it must not do. Asserted the other way round.
                let got = parse(
                    "run",
                    &v(&[
                        r.flag.unwrap().split(',').next().unwrap().trim(),
                        "img",
                        "true",
                    ]),
                );
                assert_eq!(
                    got.unwrap_err(),
                    EXIT_FLAG_ERROR,
                    "{:?} was not refused",
                    r.flag
                );
                continue;
            }
            for spelling in r.flag.unwrap().split(',') {
                let f = spelling.trim();
                if f == "-h" || f == "--help" {
                    assert_eq!(
                        parse("run", &v(&[f])).unwrap_err(),
                        0,
                        "{f} did not print usage"
                    );
                    continue;
                }
                // ⚠ THE ASSERTION IS `parity::no_arm`'s PANIC, not a code
                // comparison. It used to compare against EXIT_RUNTIME_ERROR,
                // which was the fallback arm's own code and distinguishable
                // from a legitimate refusal's 2; T-0802 measured docker and
                // made a flag error 125 as well, so that comparison started
                // asserting `125 != 125` and could no longer fail. Both shapes
                // are tried because a valued flag needs a value, a boolean one
                // does not, and one with a closed set of values (`--pull`)
                // refuses any value this test could invent: all three are
                // legitimate, and only reaching the fallback arm is not.
                for shape in [v(&[f, "V", "img", "true"]), v(&[f, "img", "true"])] {
                    let _ = parse("run", &shape);
                }
            }
        }
    }

    #[test]
    /// ⚠ docker's own code for a required argument that is not there is 1 and
    /// not 125: measured, `docker run` with no image exits 1. T-0802.
    fn no_image_is_a_cli_error() {
        assert_eq!(parse("run", &v(&["--rm"])).unwrap_err(), EXIT_CLI_ERROR);
    }

    /// ⭐ TODO/podvm.md T-1302. The tier flag takes two values, and the
    /// emulator arguments collect whole: one token per occurrence, and a
    /// value with a space stays one token rather than splitting.
    #[test]
    fn the_tier_flag_takes_two_values_and_qemu_args_collect_whole() {
        assert_eq!(
            parse("run", &v(&["--podbox-tier=machine", "img"]))
                .unwrap()
                .tier,
            Some(crate::tier::Want::Machine)
        );
        assert_eq!(
            parse("run", &v(&["--podbox-tier", "chroot", "img"]))
                .unwrap()
                .tier,
            Some(crate::tier::Want::Chroot)
        );
        assert_eq!(
            parse("run", &v(&["--podbox-tier=sandbox", "img"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            parse("run", &v(&["--podbox-tier", "img"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        let o = parse(
            "run",
            &v(&["--podbox-qemu-arg", "a", "--podbox-qemu-arg=b c", "img"]),
        )
        .unwrap();
        assert_eq!(o.qemu_args, v(&["a", "b c"]));
    }

    /// ⭐ TODO/podvm.md T-1305: the memory spelling parses to bytes in both
    /// forms, and anything that is not a size is a flag error.
    #[test]
    fn the_mem_flag_parses_to_bytes_in_both_forms() {
        assert_eq!(
            parse("run", &v(&["--podbox-mem", "512M", "img"]))
                .unwrap()
                .mem,
            Some(512 << 20)
        );
        assert_eq!(
            parse("run", &v(&["--podbox-mem=1G", "img"])).unwrap().mem,
            Some(1 << 30)
        );
        assert_eq!(
            parse("run", &v(&["--podbox-mem=bogus", "img"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            parse("run", &v(&["--podbox-mem", "0", "img"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
    }

    /// ⭐ TODO/cli.md T-0801, issue 60: `--env-file` loads docker's
    /// `KEY=VALUE` lines at the flag's position, and the stub trio
    /// (`--label`, `--attach`, `--expose`) parses without reaching an arm
    /// that acts on them.
    #[test]
    fn env_file_loads_and_stubs_parse() {
        let dir = std::env::temp_dir().join(format!("pb-envfile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("test.env");
        std::fs::write(
            &file,
            "# a comment\n\nA=1\nB = two words\nC=\"quoted\"\nD='single'\n",
        )
        .unwrap();
        let path = file.to_str().unwrap().to_string();
        // Both spellings land in the same place, at the flag's position:
        // a later -e wins over the file.
        let o = parse("run", &v(&["--env-file", &path, "-e", "A=9", "img"])).unwrap();
        assert_eq!(
            o.env,
            v(&["A=1", "B=two words", "C=quoted", "D=single", "A=9"])
        );
        let o = parse("run", &v(&[&format!("--env-file={path}"), "img"])).unwrap();
        assert_eq!(o.env, v(&["A=1", "B=two words", "C=quoted", "D=single"]));
        // A line without `=` is refused naming the file, never skipped.
        let bad = dir.join("bad.env");
        std::fs::write(&bad, "A=1\nNOEQUALS\n").unwrap();
        assert_eq!(
            parse("run", &v(&["--env-file", bad.to_str().unwrap(), "img"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        // A missing file is refused naming the path, never empty env.
        assert_eq!(
            parse(
                "run",
                &v(&[
                    "--env-file",
                    dir.join("absent.env").to_str().unwrap(),
                    "img"
                ])
            )
            .unwrap_err(),
            EXIT_FLAG_ERROR
        );
        std::fs::remove_dir_all(&dir).ok();
        // The stub trio parses and changes nothing about the run.
        let o = parse(
            "run",
            &v(&["--label", "k=v", "--attach", "--expose", "img"]),
        )
        .unwrap();
        assert_eq!(o.image.as_deref(), Some("img"));
        assert!(o.env.is_empty());
    }

    /// ⭐ TODO/supervise.md T-0605, issue 56: `--log-driver` takes
    /// `json-file` and nothing else. The accepted value parses in both
    /// spellings; any other value, or none, is a flag error naming the
    /// value rather than a silent substitution.
    #[test]
    fn log_driver_takes_json_file_and_nothing_else() {
        assert!(parse("run", &v(&["--log-driver", "json-file", "img"])).is_ok());
        assert!(parse("run", &v(&["--log-driver=json-file", "img"])).is_ok());
        assert_eq!(
            parse("run", &v(&["--log-driver", "syslog", "img"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            parse("run", &v(&["--log-driver=syslog", "img"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            parse("run", &v(&["--log-driver", "img"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
    }
}
