//! `podvm exec`: [`TODO/enter.md`](../../../TODO/enter.md) T-0505.
//!
//! ⛔ **`docker exec` enters the container's namespaces. podvm has none to
//! enter.** Its `exec` re-runs the `TOOL.md` section 6.5 sequence against the
//! same rootfs, so it shares the filesystem tree with whatever else is in that
//! rootfs and **nothing else**: not the process table, not `/proc`, not
//! signals. A caller that assumes otherwise gets a process that cannot see
//! any of them. It takes the environment that the container started with,
//! from the record, as docker's `exec` does (TODO/cli.md T-1765).
//!
//! ⭐ That difference is said in three places a caller can reach, and the three
//! agree because they read one pair of constants: the banner on every run
//! ([`degradation`]), `podvm inspect --format '{{.Exec.Shares}}'`
//! ([`crate::images::EXEC_SHARES`]), and the `Degraded` row for the verb in the
//! parity table.
//!
//! ⚠ **The target is an image reference today, and a container name at M4.**
//! podvm has no containers yet ([`TODO/supervise.md`](../../../TODO/supervise.md)
//! T-1105), and the mechanism does not change when it does: a container name
//! resolves to the same rootfs and enters it the same way.

use std::io::Write;

use podvm_enter::{Fds, Plan, RootDir};
use podvm_image::error::{EXIT_CLI_ERROR, EXIT_FLAG_ERROR};
use podvm_image::platform::Platform;

pub const EXEC_USAGE: &str = "\
usage: podvm exec [options] <image> <command> [arg...]

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
  --unsafe-host-paths
                   as in run: userland rung only, opt into the pass-through
                   with no PODVM_MAPS anchoring, and admit a static payload
                   to the memfd family where chroot is denied
                   (TODO/enter.md T-1407, T-1648)
  --strict         refused: refuse to re-enter at all where anything about this
                   invocation is Degraded or Stub (TODO/cli.md T-0804).
                   Kind decides (TODO/cli.md T-1415): safety-relevant
                   differences still refuse, dev-shim substitutions warn,
                   and --strict=all keeps refusing every one
  -q, --quiet      as in run: force the banner off (TODO/cli.md T-1416)
  --verbose        as in run: print the banner; long-only, because -v is
                   docker's --volume (TODO/cli.md T-1416)
  -t, --tty        refused: REFUSED BY NAME where /dev/ptmx is unusable, rather
                   than silently degraded (TODO/enter.md T-0503)
  --podvm-tier T  as in run: machine selects the machine tier, incus and
                   emulate are refused (run only), and each
                   other value is a flag error; with no flag the ladder
                   selects (TODO/machine.md T-1302, TODO/cli.md T-1712)
  --podvm-qemu-arg A
                   as in run: machine tier only and refused elsewhere, one
                   emulator token per occurrence, repeatable, never split
  --podvm-mem S   as in run: machine tier only and refused elsewhere, the
                   guest memory in bytes with an optional K/M/G/T suffix
                   (TODO/machine.md T-1305)

  refused: This is a FRESH CHROOT re-entry, not an entry into a running container.
    It shares the filesystem tree and nothing else. `podvm inspect --format
    '{{.Exec.Mode}}'` says the same thing to a program.

  refused: The image must already be extracted. `podvm exec` never pulls and never
    extracts: there would be nothing to re-enter, and a verb that creates what
    it claims to attach to is the lie TOOL.md section 4.1 forbids.
";

/// The one sentence that makes the degradation true rather than implied.
///
/// ⛔ Built from the same constants `inspect` reports, so the banner and the
/// machine-readable field cannot say different things.
pub fn degradation() -> String {
    format!(
        "podvm exec: this is a {} re-entry and not an entry into a running \
         container. It shares {} with anything else in this rootfs and nothing \
         else: not the process table, not /proc, not signals. It takes the \
         environment that the container started with (TODO/enter.md T-0505, \
         TODO/cli.md T-1765)\n",
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
    /// `--podvm-tier`, resolved against argv[0] after parse like `run` does.
    tier: Option<crate::tier::Want>,
    /// `--podvm-qemu-arg`, one emulator token per occurrence, refused
    /// outside the machine tier rather than silently dropped.
    qemu_args: Vec<String>,
    /// `--podvm-mem`, the validated guest memory in bytes, refused outside
    /// the machine tier rather than silently dropped. TODO/machine.md T-1305.
    mem: Option<u64>,
    /// `--unsafe-host-paths`, as in `run`: the userland opt-out.
    unsafe_host_paths: bool,
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
        unsafe_host_paths: false,
        ask: crate::complete::Ask::default(),
    };
    let mut expecting: Option<&'static str> = None;
    for a in args {
        if let Some(flag) = expecting.take() {
            match flag {
                "-e" => o.env.extend(crate::run::from_host(a)),
                "-w" => o.workdir = Some(a.clone()),
                "--add-host" => crate::complete::add_host(&mut o.ask, "exec", a)?,
                "--user" => o.user = Some(a.clone()),
                "--podvm-tier" => match crate::tier::want(a) {
                    Some(w) => o.tier = Some(w),
                    None => {
                        eprintln!("{}", crate::tier::refusal("exec", a));
                        return Err(EXIT_FLAG_ERROR);
                    }
                },
                "--podvm-qemu-arg" => o.qemu_args.push(a.clone()),
                "--podvm-mem" => match crate::tier::parse_mem(a) {
                    Some(m) => o.mem = Some(m),
                    None => {
                        eprintln!("podvm exec: --podvm-mem takes a byte count with an optional K/M/G/T suffix, not {a:?}");
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
            "-q" | "--quiet" => o.ask.quiet = true,
            "--verbose" => o.ask.verbose = true,
            "-i" | "--interactive" => {
                // ⚠ Accepted and a no-op, as in `run`: podvm does not detach
                // stdin, so it is already interactive when the caller's is.
            }
            "-e" | "--env" => expecting = Some("-e"),
            "-w" | "--workdir" => expecting = Some("-w"),
            "-u" | "--user" => expecting = Some("--user"),
            "--platform" => expecting = Some("--platform"),
            "--add-host" => expecting = Some("--add-host"),
            "--podvm-tier" => expecting = Some("--podvm-tier"),
            "--podvm-qemu-arg" => expecting = Some("--podvm-qemu-arg"),
            "--podvm-mem" => expecting = Some("--podvm-mem"),
            other if other.starts_with("--add-host=") => {
                crate::complete::add_host(&mut o.ask, "exec", &other[11..])?
            }
            "--no-source-fixup" => o.ask.no_source_fixup = true,
            "--no-host-cas" => o.ask.no_host_cas = true,
            "--no-steps" => o.ask.no_steps = true,
            "--strict" => o.ask.strict = true,
            "--unsafe-host-paths" => o.unsafe_host_paths = true,
            other if other.starts_with("--strict=") => match &other[9..] {
                // ⭐ T-1415, as in `run`: `--strict=all` keeps refusing
                // every Degraded and Stub difference.
                "all" => {
                    o.ask.strict = true;
                    o.ask.strict_all = true;
                }
                v => {
                    eprintln!("podvm exec: --strict takes all or nothing, not {v:?}");
                    return Err(EXIT_FLAG_ERROR);
                }
            },
            other if other.starts_with("--env=") => {
                o.env.extend(crate::run::from_host(&other[6..]))
            }
            other if other.starts_with("--user=") => o.user = Some(other[7..].to_string()),
            other if other.starts_with("--workdir=") => o.workdir = Some(other[10..].to_string()),
            other if other.starts_with("--platform=") => o.platform = Some(other[11..].to_string()),
            other if other.starts_with("--podvm-tier=") => {
                match crate::tier::want(&other["--podvm-tier=".len()..]) {
                    Some(w) => o.tier = Some(w),
                    None => {
                        eprintln!(
                            "{}",
                            crate::tier::refusal("exec", &other["--podvm-tier=".len()..])
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
                            "podvm exec: --podvm-mem takes a byte count with an optional K/M/G/T suffix, not {:?}",
                            &other["--podvm-mem=".len()..]
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
        eprintln!("podvm exec: {flag} needs a value");
        return Err(EXIT_FLAG_ERROR);
    }
    if o.image.is_none() {
        eprint!("{EXEC_USAGE}");
        return Err(EXIT_CLI_ERROR);
    }
    // ⛔ docker's rule, and podvm's for the same reason: `exec` has no default
    // command. An image's Cmd is what `run` starts, not what a second entry
    // re-runs, and silently re-running it is a process the caller did not ask
    // for.
    if o.command.is_empty() {
        eprintln!("podvm exec: a command is required. `exec` never falls back to the image's Cmd");
        eprint!("{EXEC_USAGE}");
        return Err(EXIT_CLI_ERROR);
    }
    Ok(o)
}

pub(crate) mod enter;
use enter::*;

mod command;
pub use command::*;

#[cfg(test)]
mod tests;
