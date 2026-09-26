//! `podbox exec`: [`TODO/enter.md`](../../../TODO/enter.md) T-0505.
//!
//! ⛔ **`docker exec` enters the container's namespaces. podbox has none to
//! enter.** Its `exec` re-runs the `TOOL.md` section 6.5 sequence against the
//! same rootfs, so it shares the filesystem tree with whatever else is in that
//! rootfs and **nothing else**: not the process table, not `/proc`, not
//! signals, not the original process's environment. A caller that assumes
//! otherwise gets a process that cannot see any of them.
//!
//! ⭐ That difference is said in three places a caller can reach, and the three
//! agree because they read one pair of constants: the banner on every run
//! ([`degradation`]), `podbox inspect --format '{{.Exec.Shares}}'`
//! ([`crate::images::EXEC_SHARES`]), and the `Degraded` row for the verb in the
//! parity table.
//!
//! ⚠ **The target is an image reference today, and a container name at M4.**
//! podbox has no containers yet ([`TODO/supervise.md`](../../../TODO/supervise.md)
//! T-1105), and the mechanism does not change when it does: a container name
//! resolves to the same rootfs and enters it the same way.

use std::io::Write;

use podbox_enter::{Fds, Plan, RootDir};
use podbox_image::error::{EXIT_CLI_ERROR, EXIT_FLAG_ERROR};
use podbox_image::platform::Platform;

pub const EXEC_USAGE: &str = "\
usage: podbox exec [options] <image> <command> [arg...]

  -e, --env K=V    set an environment variable. Repeatable; a later one wins
  -w, --workdir D  working directory inside the container
  -u, --user U:G   as in run: fake the identity through the interposer memo
                   (TODO/interpose.md T-0711)
  --platform P     which platform of a multi-platform image to enter
  --add-host N:IP  add a name to the container's /etc/hosts. Repeatable
  --no-source-fixup
                   leave the image's package sources exactly as extracted, and
                   undo a rewrite an earlier run made (TODO/complete.md T-0411)
  --no-host-cas    as in run: leave the image's trust store alone
  --no-steps       as in run: run no COMMAND inside the rootfs before the
                   command asked for (TODO/complete.md T-0412)
  --strict         refused: refuse to re-enter at all where anything about this
                   invocation is Degraded or Stub (TODO/cli.md T-0804)
  -t, --tty        refused: REFUSED BY NAME where /dev/ptmx is unusable, rather
                   than silently degraded (TODO/enter.md T-0503)
  --podbox-tier T  as in run: machine selects the machine tier, chroot the
                   chroot tier, and an explicit flag wins over the podvm
                   default with the tier stated (TODO/podvm.md T-1302)
  --podbox-qemu-arg A
                   as in run: machine tier only and refused elsewhere, one
                   emulator token per occurrence, repeatable, never split
  --podbox-mem S   as in run: machine tier only and refused elsewhere, the
                   guest memory in bytes with an optional K/M/G/T suffix
                   (TODO/podvm.md T-1305)

  refused: This is a FRESH CHROOT re-entry, not an entry into a running container.
    It shares the filesystem tree and nothing else. `podbox inspect --format
    '{{.Exec.Mode}}'` says the same thing to a program.

  refused: The image must already be extracted. `podbox exec` never pulls and never
    extracts: there would be nothing to re-enter, and a verb that creates what
    it claims to attach to is the lie TOOL.md section 4.1 forbids.
";

/// The one sentence that makes the degradation true rather than implied.
///
/// ⛔ Built from the same constants `inspect` reports, so the banner and the
/// machine-readable field cannot say different things.
pub fn degradation() -> String {
    format!(
        "podbox exec: this is a {} re-entry and not an entry into a running \
         container. It shares {} with anything else in this rootfs and nothing \
         else: not the process table, not /proc, not signals, not the original \
         process's environment (TODO/enter.md T-0505)\n",
        crate::images::EXEC_MODE,
        crate::images::EXEC_SHARES,
    )
}

#[derive(Debug)]
struct Opts {
    env: Vec<String>,
    workdir: Option<String>,
    platform: Option<String>,
    tty: bool,
    image: Option<String>,
    command: Vec<String>,
    /// `--user`, resolved against the re-entered rootfs like `run` does.
    user: Option<String>,
    /// `--podbox-tier`, resolved against argv[0] after parse like `run` does.
    tier: Option<crate::tier::Want>,
    /// `--podbox-qemu-arg`, one emulator token per occurrence, refused
    /// outside the machine tier rather than silently dropped.
    qemu_args: Vec<String>,
    /// `--podbox-mem`, the validated guest memory in bytes, refused outside
    /// the machine tier rather than silently dropped. TODO/podvm.md T-1305.
    mem: Option<u64>,
    ask: crate::complete::Ask,
}

/// ⛔ Parsing stops at the image name, exactly as `run`'s does: everything
/// after it is the payload's, dashes and all.
fn parse(args: &[String]) -> std::result::Result<Opts, i32> {
    // ⭐ TODO/cli.md T-1330, as in `run`'s parser: bundled shorts expand
    // before admission sees them.
    let expanded;
    let args = match crate::parity::expand("exec", args) {
        Ok(a) => {
            expanded = a;
            &expanded
        }
        Err(member) => return Err(crate::parity::refuse_member("exec", &member, EXEC_USAGE)),
    };
    let mut o = Opts {
        env: Vec::new(),
        workdir: None,
        platform: None,
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
    for a in args {
        if let Some(flag) = expecting.take() {
            match flag {
                "-e" => o.env.push(a.clone()),
                "-w" => o.workdir = Some(a.clone()),
                "--add-host" => crate::complete::add_host(&mut o.ask, "exec", a)?,
                "--user" => o.user = Some(a.clone()),
                "--podbox-tier" => match crate::tier::want(a) {
                    Some(w) => o.tier = Some(w),
                    None => {
                        eprintln!("podbox exec: --podbox-tier takes machine or chroot, not {a:?}");
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                "--podbox-qemu-arg" => o.qemu_args.push(a.clone()),
                "--podbox-mem" => match crate::tier::parse_mem(a) {
                    Some(m) => o.mem = Some(m),
                    None => {
                        eprintln!("podbox exec: --podbox-mem takes a byte count with an optional K/M/G/T suffix, not {a:?}");
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                _ => o.platform = Some(a.clone()),
            }
            continue;
        }
        if o.image.is_some() {
            o.command.push(a.clone());
            continue;
        }
        // ⛔ TODO/cli.md T-0801. The table decides, exactly as it does for
        // `run`: an unlisted flag never reaches an arm, and a `None` row is
        // refused with its own reason.
        if a.starts_with('-') {
            crate::parity::admit("exec", a, EXEC_USAGE)?;
            o.ask
                .seen_flags
                .push(a.split('=').next().unwrap_or(a).to_string());
        }
        match a.as_str() {
            "-h" | "--help" => {
                print!("{EXEC_USAGE}");
                return Err(0);
            }
            "-t" | "--tty" => o.tty = true,
            "-i" | "--interactive" => {
                // ⚠ Accepted and a no-op, as in `run`: podbox does not detach
                // stdin, so it is already interactive when the caller's is.
            }
            "-e" | "--env" => expecting = Some("-e"),
            "-w" | "--workdir" => expecting = Some("-w"),
            "-u" | "--user" => expecting = Some("--user"),
            "--platform" => expecting = Some("--platform"),
            "--add-host" => expecting = Some("--add-host"),
            "--podbox-tier" => expecting = Some("--podbox-tier"),
            "--podbox-qemu-arg" => expecting = Some("--podbox-qemu-arg"),
            "--podbox-mem" => expecting = Some("--podbox-mem"),
            other if other.starts_with("--add-host=") => {
                crate::complete::add_host(&mut o.ask, "exec", &other[11..])?
            }
            "--no-source-fixup" => o.ask.no_source_fixup = true,
            "--no-host-cas" => o.ask.no_host_cas = true,
            "--no-steps" => o.ask.no_steps = true,
            "--strict" => o.ask.strict = true,
            other if other.starts_with("--env=") => o.env.push(other[6..].to_string()),
            other if other.starts_with("--user=") => o.user = Some(other[7..].to_string()),
            other if other.starts_with("--workdir=") => o.workdir = Some(other[10..].to_string()),
            other if other.starts_with("--platform=") => o.platform = Some(other[11..].to_string()),
            other if other.starts_with("--podbox-tier=") => match crate::tier::want(&other[14..]) {
                Some(w) => o.tier = Some(w),
                None => {
                    eprintln!(
                        "podbox exec: --podbox-tier takes machine or chroot, not {:?}",
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
                            "podbox exec: --podbox-mem takes a byte count with an optional K/M/G/T suffix, not {:?}",
                            &other[13..]
                        );
                        return Err(EXIT_FLAG_ERROR);
                    }
                }
            }
            other if other.starts_with('-') => {
                // ⛔ Unreachable through the table above; an assertion, not a
                // fallback. See `run`'s own arm for why.
                return Err(crate::parity::no_arm("exec", other));
            }
            other => o.image = Some(other.to_string()),
        }
    }
    if let Some(flag) = expecting {
        eprintln!("podbox exec: {flag} needs a value");
        return Err(EXIT_FLAG_ERROR);
    }
    if o.image.is_none() {
        eprint!("{EXEC_USAGE}");
        return Err(EXIT_CLI_ERROR);
    }
    // ⛔ docker's rule, and podbox's for the same reason: `exec` has no default
    // command. An image's Cmd is what `run` starts, not what a second entry
    // re-runs, and silently re-running it is a process the caller did not ask
    // for.
    if o.command.is_empty() {
        eprintln!("podbox exec: a command is required. `exec` never falls back to the image's Cmd");
        eprint!("{EXEC_USAGE}");
        return Err(EXIT_CLI_ERROR);
    }
    Ok(o)
}

/// Enter an existing rootfs. ⭐ ONE PATH for a container and for an image: the
/// only difference is where the rootfs came from, and duplicating the entry
/// would be a second implementation of the thing this verb exists to be.
fn enter(
    target: &str,
    rootfs: &str,
    state: podbox_supervise::table::State,
    o: &Opts,
    store: &podbox_image::Store,
) -> i32 {
    use podbox_supervise::table::State;
    if state == State::Created {
        eprintln!(
            "podbox exec: {target} has been created and never started, so its rootfs              is there and nothing is running in it. note: podbox will enter it anyway,              because a fresh chroot shares only the filesystem and needs nothing to              be running (TODO/enter.md T-0505)"
        );
    }
    // ⚠ The container's own environment is not inherited: T-0505's whole
    // subject is that this shares the filesystem and NOTHING else, and silently
    // copying the original's environment would be the implication it refuses.
    let mut env = podbox_enter::Plan::env_for(&[], &o.env);
    // ⭐ T-0711: the requested identity, resolved against the re-entered
    // rootfs exactly as `run` resolves it against the extracted one.
    if let Err(code) = crate::lifecycle::apply_user("exec", rootfs, o.user.as_ref(), &mut env) {
        return code;
    }
    // ⭐ T-0710: the container's host memo, re-handed on this fresh re-entry.
    // A fresh chroot shares only the filesystem; without the same descriptor
    // a second process answers `stat` with the real uid and contradicts the
    // first, so an entry that cannot be handed one refuses.
    let memo_path = match podbox_supervise::get(store, target) {
        Ok(c) => podbox_supervise::table::memo_path(store, &c.id),
        Err(e) => {
            eprintln!("podbox exec: {e}");
            return podbox_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    if let Err(e) = podbox_supervise::table::ensure_memo_file(&memo_path) {
        eprintln!("podbox exec: the ownership memo could not be created: {e}");
        return podbox_image::error::EXIT_RUNTIME_ERROR;
    }
    env.retain(|e| e.split('=').next().unwrap_or("") != podbox_supervise::table::MEMO_FD_VAR);
    env.push(podbox_supervise::table::memo_fd_env());
    // ⭐ T-0702 and T-0706, as in `run`: a fresh chroot re-entry is a fresh
    // payload, so it is classified and placed again rather than inheriting
    // the first entry's answer.
    let mut interpose_note = String::new();
    if let Err(e) =
        crate::interpose::apply("exec", rootfs, &o.command, &mut env, &mut interpose_note)
    {
        eprintln!("{e}");
        return podbox_image::error::EXIT_RUNTIME_ERROR;
    }
    let path_dirs = podbox_enter::Plan::path_from(&env);
    // ⭐ TODO/complete.md T-0413: the re-entered payload's guest path, as in
    // `run`'s `prepare`. A fresh chroot re-entry is a fresh payload.
    crate::run::push_guest_exe(
        &mut env,
        rootfs,
        o.command.first().map(String::as_str).unwrap_or(""),
        &path_dirs,
    );
    let working_dir = o.workdir.clone().unwrap_or_else(|| "/".to_string());
    let findings = podbox_probe::run();
    let selection = podbox_probe::select::Selection::choose(&findings);
    let entered = podbox_enter::ENTERED_RUNG;
    let mut banner = podbox_probe::report::entry_banner(&findings, &selection, entered);
    if let Some(note) = crate::names::alias_note() {
        banner.push_str(&note);
    }
    banner.push_str(&degradation());
    banner.push_str(&interpose_note);
    // ⭐ M5. `exec` completes the rootfs exactly as `run` does, and for the same
    // reason: a fresh chroot re-entry is a fresh payload, and the `/dev/null` a
    // previous one turned into a file is still a file.
    let mut ask = o.ask.clone();
    ask.container_name = Some(target.to_string());
    let mut completion = match crate::complete::prepare("exec", rootfs, &ask, &mut banner) {
        Ok(r) => r,
        Err(code) => return code,
    };
    let mut err = std::io::stderr().lock();
    // ⭐ T-0804 rule 1, and ahead of every refusal below for `run`'s own
    // reasons: a refused caller gets the banner too, and T-0412's steps have to
    // be named before they run.
    let quiet = crate::complete::banner_quiet(store);
    if !quiet {
        let _ = write!(err, "{banner}");
    }
    if let Err(code) =
        crate::complete::strict_refusal("exec", &ask, entered.word(), &completion, &mut err)
    {
        return code;
    }
    if o.tty && !podbox_probe::probes::ptmx_usable(&findings) {
        let _ = writeln!(err, "{TTY_REFUSAL}");
        return podbox_enter::EXIT_RUNTIME_ERROR;
    }
    // ⭐ T-0412, and `exec` runs them for the same reason it completes the
    // rootfs at all: a fresh chroot re-entry is a fresh payload, and a keyring
    // or a CA index a previous one destroyed is still destroyed.
    // ⭐ T-0710: steps share the container's memo, opened once for the steps
    // and again for the payload below.
    {
        use std::os::fd::AsRawFd;
        let memo_for_steps = podbox_supervise::table::open_memo(&memo_path).ok();
        let memo_host = memo_for_steps.as_ref().map(|f| f.as_raw_fd() as i64);
        crate::complete::run_steps_with_memo(
            "exec",
            rootfs,
            &env,
            &mut completion,
            quiet,
            &mut err,
            memo_host,
        );
    }
    let memo = match podbox_supervise::table::open_memo(&memo_path) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(
                err,
                "podbox exec: the ownership memo could not be opened: {e}"
            );
            return podbox_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    use std::os::fd::AsRawFd;
    let memo_host = memo.as_raw_fd() as i64;
    let plan = Plan {
        argv: o.command.clone(),
        env,
        working_dir,
        fds: Fds {
            pass: vec![(podbox_supervise::table::MEMO_CHILD_FD, memo_host)],
        },
        // ⚠ Empty: the banner was printed above, before the steps.
        banner: String::new(),
        path_dirs,
    };
    let root = match RootDir::open(rootfs) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(err, "podbox exec: {e}");
            return e.exit_code();
        }
    };
    match podbox_enter::run(&root, &plan, &mut err) {
        Ok(c) => c,
        Err(e) => {
            let _ = writeln!(err, "podbox exec: {e}");
            e.exit_code()
        }
    }
}

/// The `-t` refusal, with T-0503's entry named. The usability predicate
/// lives in `podbox-probe`, beside the rows it reads; `run` asks the same
/// one.
const TTY_REFUSAL: &str = "podbox exec: -t was asked for and /dev/ptmx is not usable on this \
machine, so podbox cannot allocate a pty. It refuses rather than running without one and \
letting the payload discover it (TODO/enter.md T-0503)";

pub fn exec(args: &[String]) -> i32 {
    let o = match parse(args) {
        Ok(o) => o,
        Err(code) => return code,
    };
    // ⭐ TODO/packaging.md T-1003. `exec` re-enters and never reaches the
    // ladder, so a force refused here names that rather than falling through
    // it silently.
    if let Err(text) = crate::ladder::refuse_where_undriven("exec", false, false) {
        eprintln!("podbox exec: {text}");
        return podbox_image::error::EXIT_RUNTIME_ERROR;
    }
    // ⭐ TODO/podvm.md T-1302, as in `run`'s `prepare`: the tier is decided
    // before the store is touched, so a machine-tier refusal names the legs
    // with nothing pulled for a tier that cannot run.
    let tier = crate::tier::resolve(&crate::names::invoked_as(), o.tier);
    if let Some(note) = &tier.note {
        eprintln!("podbox exec: {note}");
    }
    let platform = match Platform::wanted(o.platform.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("podbox exec: {e}");
            return e.exit_code();
        }
    };
    // ⭐ TODO/milestones.md T-1112, as in `run`'s `prepare`: the OS gate
    // sits before the tier dispatch. `exec` never pulls, so a platform
    // no tier can enter is refused before the store is even opened,
    // rather than reaching the machine tier's leg refusal.
    if let Err(c) = crate::lifecycle::ensure_linux_guest("exec", &platform.os, &platform.arch) {
        return c;
    };
    if tier.tier == crate::tier::Tier::Machine {
        return crate::tier::enter_machine("exec", o.mem);
    }
    if !o.qemu_args.is_empty() {
        eprintln!("podbox exec: --podbox-qemu-arg needs --podbox-tier=machine");
        return EXIT_FLAG_ERROR;
    }
    if o.mem.is_some() {
        eprintln!("podbox exec: --podbox-mem needs --podbox-tier=machine");
        return EXIT_FLAG_ERROR;
    }
    let image = o.image.clone().expect("checked in parse");

    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("podbox exec: {e}");
            return e.exit_code();
        }
    };

    // ⭐ M4. A CONTAINER NAME FIRST, then an image reference. `docker exec` takes
    // a container and podbox's takes either, because until M4 there were no
    // containers and an image was the only thing to re-enter. ⚠ The mechanism is
    // the same either way: a container's rootfs is a directory in the store and
    // so is an image's.
    if let Some((rootfs, state)) = crate::lifecycle::rootfs_of(&store, &image) {
        return enter(&image, &rootfs, state, &o, &store);
    }

    // ⛔ Never pulls. `exec` re-enters something that is already there, so a
    // reference the store does not hold is a refusal that names `run`, not a
    // fetch the caller did not ask for.
    let found = store.find_for(&image, Some(&platform)).unwrap_or_default();
    let others = found.other_platforms.clone();
    let Some(record) = found.one() else {
        if others.is_empty() {
            eprintln!(
                "podbox exec: {image} is not in the store. `podbox exec` never pulls: \
                 `podbox run {image} ...` is what puts it there"
            );
        } else {
            eprintln!(
                "podbox exec: the store holds {image} for {}, and {platform} was asked \
                 for. `podbox exec` never pulls",
                others.join(", ")
            );
        }
        return podbox_image::error::EXIT_RUNTIME_ERROR;
    };

    // ⛔ The lock BEFORE the extraction check, so a concurrent `rmi` cannot
    // delete the rootfs between podbox deciding it is there and entering it.
    // TODO/image.md T-0204, the same ordering `run` takes.
    let held = match store.hold(&record) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("podbox exec: {e}");
            return podbox_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    if !podbox_extract::is_extracted(&store, &record.manifest_digest) {
        eprintln!(
            "podbox exec: {image} is in the store and has never been extracted, so \
             there is no rootfs to re-enter. `podbox run {image} ...` or `podbox \
             extract {image}` is what puts one there"
        );
        return podbox_image::error::EXIT_RUNTIME_ERROR;
    };
    // ⭐ TODO/milestones.md T-1112. The stored record is what gets entered.
    if let Err(c) = crate::lifecycle::ensure_linux_guest("exec", &record.os, &record.architecture) {
        return c;
    }
    let (rootfs, _) = podbox_extract::paths(&store, &record.manifest_digest);
    let rootfs = rootfs.to_string_lossy().to_string();

    // --------------------------------------------------------------- the plan
    let cfg = match crate::run::config_of("exec", &store, &record) {
        Ok(c) => c,
        Err(code) => return code,
    };
    // ⛔ The image's Entrypoint is NOT prepended. docker's `exec` runs the
    // command given and nothing around it, and an entrypoint that wraps a
    // second entry is a process the caller did not write.
    let argv = o.command.clone();
    let mut env = Plan::env_for(&cfg.config.env, &o.env);
    // ⭐ T-0711: the one call, as on the container path above: a second
    // spelling of the same resolve-and-set would be the second copy
    // `docs/conventions/code.md` refuses.
    if let Err(code) = crate::lifecycle::apply_user("exec", &rootfs, o.user.as_ref(), &mut env) {
        return code;
    }
    // ⭐ T-0710: an ephemeral host memo, as foreground `run` holds. No
    // container record exists on this path, but the payload still cannot reach
    // the file by path; it is handed the descriptor at spawn and deleted on
    // exit.
    let memo_host_path = crate::interpose::ephemeral_memo_path(&store);
    if let Err(e) = podbox_supervise::table::ensure_memo_file(&memo_host_path) {
        eprintln!("podbox exec: the ownership memo could not be created: {e}");
        return podbox_image::error::EXIT_RUNTIME_ERROR;
    }
    env.retain(|e| e.split('=').next().unwrap_or("") != podbox_supervise::table::MEMO_FD_VAR);
    env.push(podbox_supervise::table::memo_fd_env());
    // ⭐ T-0702 and T-0706, as on the container path above: the image path is
    // a second way to reach the same entry, not a second answer to it.
    let mut interpose_note = String::new();
    if let Err(e) = crate::interpose::apply("exec", &rootfs, &argv, &mut env, &mut interpose_note) {
        eprintln!("{e}");
        let _ = std::fs::remove_file(&memo_host_path);
        return podbox_image::error::EXIT_RUNTIME_ERROR;
    }
    let path_dirs = Plan::path_from(&env);
    // ⭐ TODO/complete.md T-0413: the re-entered payload's guest path, as in
    // `run`'s `prepare` and the container path above.
    crate::run::push_guest_exe(
        &mut env,
        &rootfs,
        argv.first().map(String::as_str).unwrap_or(""),
        &path_dirs,
    );
    let working_dir = o
        .workdir
        .clone()
        .unwrap_or_else(|| cfg.config.working_dir.clone());

    // ------------------------------------------------------------- the banner
    let findings = podbox_probe::run();
    let selection = podbox_probe::select::Selection::choose(&findings);
    let entered = podbox_enter::ENTERED_RUNG;
    let mut banner = podbox_probe::report::entry_banner(&findings, &selection, entered);
    // ⭐ TODO/cli.md T-0803. Where podbox was reached under somebody else's
    // name, the banner says which name was used and that this is podbox.
    // Taking the name is the product requirement; taking it silently is what
    // TOOL.md section 4.1 forbids.
    if let Some(note) = crate::names::alias_note() {
        banner.push_str(&note);
    }
    banner.push_str(&degradation());
    banner.push_str(&interpose_note);
    // ⭐ M5. The same completion the container path takes, from the same
    // function: two entry paths that complete a rootfs differently would be two
    // answers to one question, and the one nobody exercises is the one that
    // diverges.
    let mut ask = o.ask.clone();
    ask.container_name = Some(image.clone());
    let mut completion = match crate::complete::prepare("exec", &rootfs, &ask, &mut banner) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let mut err = std::io::stderr().lock();
    // ⭐ T-0804 rule 1, ahead of the refusals: see the container path above.
    let quiet = crate::complete::banner_quiet(&store);
    if !quiet {
        let _ = write!(err, "{banner}");
    }
    if let Err(code) =
        crate::complete::strict_refusal("exec", &ask, entered.word(), &completion, &mut err)
    {
        return code;
    }
    if o.tty {
        // ⛔ T-0503, and the same shared predicate `enter` asks above: `Ok`
        // and nothing else, because a skip is not a pass.
        if !podbox_probe::probes::ptmx_usable(&findings) {
            let _ = writeln!(err, "{TTY_REFUSAL}");
            return podbox_enter::EXIT_RUNTIME_ERROR;
        }
    }
    // ⭐ T-0412's steps, on the image path as on the container one.
    // ⭐ T-0710: steps share the ephemeral memo, opened once for the steps and
    // again for the payload below.
    {
        use std::os::fd::AsRawFd;
        let memo_for_steps = podbox_supervise::table::open_memo(&memo_host_path).ok();
        let memo_host = memo_for_steps.as_ref().map(|f| f.as_raw_fd() as i64);
        crate::complete::run_steps_with_memo(
            "exec",
            &rootfs,
            &env,
            &mut completion,
            quiet,
            &mut err,
            memo_host,
        );
    }

    let memo = match podbox_supervise::table::open_memo(&memo_host_path) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(
                err,
                "podbox exec: the ownership memo could not be opened: {e}"
            );
            let _ = std::fs::remove_file(&memo_host_path);
            return podbox_image::error::EXIT_RUNTIME_ERROR;
        }
    };
    use std::os::fd::AsRawFd;
    let memo_host = memo.as_raw_fd() as i64;
    let plan = Plan {
        argv,
        env,
        working_dir,
        fds: Fds {
            pass: vec![(podbox_supervise::table::MEMO_CHILD_FD, memo_host)],
        },
        // ⚠ Empty: the banner was printed above, before the steps.
        banner: String::new(),
        path_dirs,
    };

    // -------------------------------------------------------------- the entry
    let root = match RootDir::open(&rootfs) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(err, "podbox exec: {e}");
            drop(memo);
            let _ = std::fs::remove_file(&memo_host_path);
            return e.exit_code();
        }
    };
    if let Err(e) = held.hand_to_payload() {
        let _ = writeln!(err, "podbox exec: {e}");
        drop(memo);
        let _ = std::fs::remove_file(&memo_host_path);
        return podbox_image::error::EXIT_RUNTIME_ERROR;
    }
    let code = match podbox_enter::run(&root, &plan, &mut err) {
        Ok(c) => c,
        Err(e) => {
            let _ = writeln!(err, "podbox exec: {e}");
            e.exit_code()
        }
    };
    drop(memo);
    let _ = std::fs::remove_file(&memo_host_path);
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn exec_usage_is_plain_ascii() {
        // TODO/cli.md T-1336: `exec --help` prints on a constrained host
        // with no terminal; a glyph there is unrenderable bytes.
        let bad = EXEC_USAGE.bytes().filter(|b| *b > 0x7F).count();
        assert_eq!(bad, 0, "exec usage carries {bad} non-ASCII bytes");
    }

    #[test]
    fn parsing_stops_at_the_image_name() {
        let o = parse(&v(&["-e", "A=1", "alpine", "ls", "-l", "-e"])).unwrap();
        assert_eq!(o.env, v(&["A=1"]));
        assert_eq!(o.image.as_deref(), Some("alpine"));
        assert_eq!(o.command, v(&["ls", "-l", "-e"]));
    }

    /// ⛔ The difference from `run`, and it is deliberate rather than an
    /// omission: an image's Cmd is what `run` starts.
    #[test]
    fn exec_has_no_default_command() {
        assert_eq!(parse(&v(&["alpine"])).unwrap_err(), EXIT_CLI_ERROR);
        assert!(parse(&v(&["alpine", "true"])).is_ok());
    }

    /// ⭐ TODO/cli.md T-0801. Every flag the table ADMITS for this verb reaches
    /// an arm of this parser. The other direction is held by construction: an
    /// argument starting with `-` goes through `parity::admit` first, so an arm
    /// for a flag with no row is unreachable rather than a second surface.
    #[test]
    fn every_flag_the_table_admits_is_handled_by_this_parser() {
        for r in crate::parity::TABLE
            .iter()
            .filter(|r| r.verb == "exec" && r.flag.is_some())
        {
            if r.status == crate::parity::Status::None {
                // ⚠ A `None` row is refused BY `admit`, so reaching an arm is
                // exactly what it must not do. Asserted the other way round.
                let got = parse(&v(&[
                    r.flag.unwrap().split(',').next().unwrap().trim(),
                    "img",
                    "true",
                ]));
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
                    assert_eq!(parse(&v(&[f])).unwrap_err(), 0, "{f} did not print usage");
                    continue;
                }
                // ⚠ ASSERTED ON THE EXIT CODE, not on a shape parsing. A
                // valued flag needs a value, a boolean one does not, and one
                // with a closed set of values (`--pull`) rejects any value this
                // test could invent: all three are legitimate and only one of
                // them parses. What no legitimate arm ever returns is
                // EXIT_RUNTIME_ERROR, which is the fallback arm's own code and
                // means exactly "the table admits this flag and nothing
                // implements it".
                // ⚠ The assertion is `parity::no_arm`'s panic. See the same
                // test in `run.rs` for why the code comparison it replaced
                // stopped being able to fail.
                for shape in [v(&[f, "V", "img", "true"]), v(&[f, "img", "true"])] {
                    let _ = parse(&shape);
                }
            }
        }
    }

    #[test]
    fn a_flag_needing_a_value_does_not_swallow_the_image() {
        assert_eq!(parse(&v(&["--platform"])).unwrap_err(), EXIT_FLAG_ERROR);
        assert_eq!(parse(&v(&["-w"])).unwrap_err(), EXIT_FLAG_ERROR);
    }

    /// ⭐ TODO/podvm.md T-1302, as in `run`: two tier values, and emulator
    /// arguments that collect whole rather than splitting on whitespace.
    #[test]
    fn the_tier_flag_takes_two_values_and_qemu_args_collect_whole() {
        assert_eq!(
            parse(&v(&["--podbox-tier=machine", "img", "true"]))
                .unwrap()
                .tier,
            Some(crate::tier::Want::Machine)
        );
        assert_eq!(
            parse(&v(&["--podbox-tier", "chroot", "img", "true"]))
                .unwrap()
                .tier,
            Some(crate::tier::Want::Chroot)
        );
        assert_eq!(
            parse(&v(&["--podbox-tier=sandbox", "img", "true"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        let o = parse(&v(&["--podbox-qemu-arg", "a b", "img", "true"])).unwrap();
        assert_eq!(o.qemu_args, v(&["a b"]));
    }

    /// ⭐ TODO/podvm.md T-1305, as in `run`: the memory spelling parses to
    /// bytes in both forms, and anything that is not a size is a flag error.
    #[test]
    fn the_mem_flag_parses_to_bytes_in_both_forms() {
        assert_eq!(
            parse(&v(&["--podbox-mem", "512M", "img", "true"]))
                .unwrap()
                .mem,
            Some(512 << 20)
        );
        assert_eq!(
            parse(&v(&["--podbox-mem=1G", "img", "true"])).unwrap().mem,
            Some(1 << 30)
        );
        assert_eq!(
            parse(&v(&["--podbox-mem=bogus", "img", "true"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            parse(&v(&["--podbox-mem", "0", "img", "true"])).unwrap_err(),
            EXIT_FLAG_ERROR
        );
    }

    /// ⭐ The banner and the machine-readable field are one pair of constants,
    /// so a caller reading either gets the same answer.
    #[test]
    fn the_banner_says_what_inspect_reports() {
        let d = degradation();
        assert!(d.contains(crate::images::EXEC_MODE), "{d}");
        assert!(d.contains(crate::images::EXEC_SHARES), "{d}");
        assert!(d.contains("T-0505"), "{d}");
    }
}
