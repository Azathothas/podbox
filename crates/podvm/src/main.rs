//! TOOL.md section 6.8: the docker and podman argument surface, the parity table, exit codes.
//!
//! Each verb dispatches here; [`TODO/cli.md`](../../../TODO/cli.md) holds
//! their work.
//!
//! ⚠ The full verb and flag parity table is [`TODO/cli.md`](../../../TODO/cli.md)
//! T-0801 and docker's own exit codes are T-0802, both of which land with M3.
//! Until then this file's contract is the one
//! [`TODO/probe.md`](../../../TODO/probe.md) T-0110 settled: invalid input
//! exits 2, and a runtime failure exits 125.
#![forbid(unsafe_op_in_unsafe_fn)]

mod assets;
mod broker;
mod complete;
mod diagnose;
mod doctor;
mod exec;
mod format;
#[cfg(test)]
mod hosts_tests; // TODO/deps.md T-1679
mod images;
mod incus;
mod interpose;
mod lab; // TODO/net.md T-1662
mod ladder;
mod lifecycle;
mod machine;
mod man;
mod names;
mod notebook;
mod parity;
mod profile;
mod remote;
mod run;
mod runsc;
mod sandprobe;
mod system;
mod tier;
mod version;
mod windows;

use std::io::Write;

use podvm_probe::report;
use podvm_probe::select::{Selection, Strictness};

// ⛔ **THE CODES ARE `podvm_image::error`'S, AND THEY ARE NOT RE-DECLARED
// HERE.** This file carried its own `const EXIT_RUNTIME_ERROR` and its own
// `const EXIT_USAGE`, which is the two-declarations-of-one-value shape
// [`TODO/RULES.md`](../../../TODO/RULES.md) names: on 2026-09-09 T-0802
// corrected the usage code against a measurement of docker and the copy here
// stayed at the old value, so `podvm probe --json --rows` and
// `podvm run --badflag` disagreed about what a flag error is.
use podvm_image::error::{EXIT_CLI_ERROR, EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};

const USAGE: &str = "\
usage: podvm <command> [options]

  run          extract an image if needed and run a command inside it
  exec         run a command in an already extracted image. refused: A FRESH CHROOT,
               sharing only the filesystem, never a namespace entry
  probe        report what this machine permits, and the rung podvm selects
  remote       verbs that act across a machine boundary: ssh into another one
  doctor       check what this machine can run, with one fix line per missing piece
  windows      a disposable Windows guest: doctor, fetch, setup and run
  machine      a guest podvm runs: ssh into it (podman parity)
  man          render this manual from the binary's own usage strings
  profile      list and show the profiles that `run --profile` takes
  notebook     run cycles of a notebook that keep only one directory
  lab          plan a network lab in one guest: scenarios, steps, verdicts
  probe-inside run podvm's probe inside an image and write what it gets there
  compare      compare a report of probe-inside with a reference
  pull         fetch an image into the content-addressed store
  save         write one image as an OCI-layout tarball
  load         read an OCI-layout tarball into the store
  import       build an image record from a plain rootfs tar
  extract      unpack a pulled image's layers into a rootfs
  images       list what the store holds
  rmi          remove images, and every blob no image reaches
  tag          point a second name at one manifest digest
  image        ls | rm | prune | tag
  inspect      print one image's record
  verify       hash the store's blobs against their digests
  create       write a container record, and start nothing
  start        start a created container, and return when its payload is running
  ps           list containers, from podvm's own table and never from /proc
  logs         a container's captured output
  stop         SIGTERM, then SIGKILL after a bounded grace
  restart      stop, then start, naming which half failed
  kill         send one signal to a container's payload
  wait         block until a container ends, and print its exit code
  rm           remove a container
  cp           copy one file into or out of a container's rootfs
  system info  what this podvm is, and the verb and flag parity table AS DATA
  system install-names
               install `docker` and `podman` as symlinks to this binary
  version      print the version

  Every other docker verb is named in that table with the reason podvm does
  not have it, and says so with exit 125 rather than being silently ignored.
  A verb neither podvm nor docker has exits 1, which is docker's own code for
  it (TODO/cli.md T-0802).
";

const PROBE_USAGE: &str = "\
usage: podvm probe [--json | --rows] [--strict] [--cached]

  (no flag)   print the selected rung on stdout and the evidence on stderr
  --json      print the findings as one JSON document on stdout
  --rows      print every probe row on stdout, in the format
              verification/probe emits, so two runs diff line by line
  --strict    exit non-zero when the selected rung is below `namespace`, or
              when a control of the bogus-argument discriminator could not
              answer. A caller gates on this without parsing anything.
  --cached    the hot path: serve $store/probe.json when its key still holds,
              and measure and rewrite it when it does not. Without this flag
              podvm probe measures and touches no store, because a diagnostic
              that can answer from a file is not a diagnostic.

  --probe-child <name>
              internal. Runs one named probe in this process and reports it on
              stdout and in the exit status. podvm re-executes itself with
              this for every probe, because a successful unshare, chroot or
              setuid mutates the prober.
";

fn main() -> std::process::ExitCode {
    names::start(); // T-1652 (the native-static rung) and SIGPIPE (T-1769)
    let argv: Vec<String> = std::env::args().collect();

    // TODO/net.md T-1665: one connection of a guest, relayed to the broker.
    if argv.get(1).map(String::as_str) == Some(broker::RELAY_ARG) {
        return exit(broker::relay(argv.get(2).map_or("", String::as_str)));
    }
    // TODO/probe.md T-1666: the probe inside a payload prints its report.
    if argv.get(1).map(String::as_str) == Some(podvm_probe::inside::ARG) {
        print!("{}", podvm_probe::inside::run("podvm"));
        return exit(0);
    }
    // ⛔ First, before the ordinary argument surface. A probe child must never
    // fall through into a verb, and nothing here may print before it does.
    if argv.get(1).map(String::as_str) == Some(podvm_probe::CHILD_FLAG) {
        let code = match argv.get(2) {
            Some(name) => podvm_probe::run_child(name),
            None => {
                eprintln!("podvm: {} needs a probe name", podvm_probe::CHILD_FLAG);
                EXIT_FLAG_ERROR
            }
        };
        return exit(code);
    }

    let argv0 = argv.first().cloned().unwrap_or_else(|| "podvm".into());
    let rest = if argv.len() > 2 { &argv[2..] } else { &[] };
    // TODO/machine.md T-1722: `tier = incus` of the configuration, and the
    // verbs of an incus instance that stays.
    let verb = argv.get(1).map(String::as_str);
    let routed = match incus::config::routed(verb, rest) {
        Ok(r) => r,
        Err(code) => return exit(code),
    };
    let rest: &[String] = &routed;
    if let Some(code) = incus::intercept(verb, rest) {
        return exit(code);
    }
    match argv.get(1).map(String::as_str) {
        Some("run") => exit(run::run(rest)),
        Some("exec") => exit(exec::exec(rest)),
        Some("cp") => exit(lifecycle::cp(rest)),
        Some("rm") => exit(lifecycle::rm(rest)),
        Some("wait") => exit(lifecycle::wait(rest)),
        Some("kill") => exit(lifecycle::kill(rest)),
        Some("stop") => exit(lifecycle::stop(rest)),
        Some("restart") => exit(lifecycle::restart(rest)),
        Some("logs") => exit(lifecycle::logs(rest)),
        Some("ps") => exit(lifecycle::ps(rest)),
        Some("start") => exit(lifecycle::start(rest)),
        Some("create") => exit(lifecycle::create(rest)),
        Some("probe") => exit(probe(rest)),
        // ⭐ `podvm remote ssh`, decided 2026-09-27 and recorded in
        // docs/decisions/remote-verb.md. `remote` is a NAMESPACE and not a
        // synonym for `ssh`: `remote fetch`, `remote download` and
        // `remote wget` are all "reach a machine that is not this one" and
        // each would otherwise want its own top-level verb. `local` is its
        // counterpart so the axis is visible, and `machine ssh` is a
        // DIFFERENT axis (a guest podvm itself runs) and stays separate.
        Some("remote") => exit(remote::remote(rest)),
        Some("ssh") => ssh_refusal(),
        Some("doctor") => exit(doctor::doctor("doctor", rest)),
        Some("machine") => exit(machine::machine(rest)),
        Some("guest-net") => exit(machine::guest::net::main()), // in a guest, T-1656
        Some("guest-channel") => exit(machine::guest::channel::guest_main(rest)), // T-1751
        Some("guest-hold") => exit(incus::hold(rest)),          // an incus init, T-1718
        Some("guest-9p") => exit(machine::guest::vol::main(rest)), // in a guest, T-1723
        Some("guest-tty") => exit(machine::guest::tty::guest_main(rest)), // in a guest, T-1723
        Some("windows") => exit(windows::windows(rest)),
        Some("man") => exit(man::man(rest)),
        Some("profile") => exit(profile::main(rest)), // TODO/cli.md T-1654
        Some("notebook") => exit(notebook::main(rest)), // TODO/cli.md T-1660
        Some("lab") => exit(lab::main(rest)),         // TODO/net.md T-1662
        Some("probe-inside") => exit(sandprobe::probe_inside(rest)), // T-1666
        Some("compare") => exit(sandprobe::compare(rest)), // T-1666
        Some("pull") => exit(images::pull("pull", rest)),
        Some("save") => exit(images::save("save", rest)),
        Some("load") => exit(images::load("load", rest)),
        Some("import") => exit(images::import("import", rest)),
        Some("login") => exit(images::login("login", rest)),
        Some("logout") => exit(images::logout("logout", rest)),
        Some("extract") => exit(images::extract("extract", rest)),
        Some("images") => exit(images::images("images", rest)),
        Some("rmi") => exit(images::rmi("rmi", rest)),
        Some("tag") => exit(images::tag("tag", rest)),
        Some("inspect") => exit(images::inspect("inspect", rest)),
        Some("verify") => exit(images::verify("verify", rest)),
        Some("image") => exit(images::group(rest)),
        Some("system") => exit(system::system(rest)),
        Some("info") => exit(system::info("info", rest)),
        Some("version") | Some("--version") | Some("-v") => {
            // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
            let rest: &[String] = &match crate::parity::expand("version", rest) {
                Ok(a) => a,
                Err(member) => {
                    return exit(crate::parity::refuse_member(
                        "version",
                        &member,
                        "usage: podvm version [--verbose]",
                    ))
                }
            };
            let mut verbose = false;
            for a in rest {
                if a == "-h" || a == "--help" {
                    println!("usage: podvm version [--verbose]");
                    return std::process::ExitCode::SUCCESS;
                }
                if a == "--verbose" {
                    verbose = true;
                    continue;
                }
                if a.starts_with('-') {
                    // ⛔ TODO/cli.md T-0801. The table decides: a flag the
                    // table has no row for is refused rather than silently
                    // accepted with the version number.
                    if let Err(c) =
                        crate::parity::admit("version", a, "usage: podvm version [--verbose]")
                    {
                        return exit(c);
                    }
                    return exit(crate::parity::no_arm("version", a));
                }
            }
            if verbose {
                print!("{}", crate::version::info().render_verbose());
            } else {
                println!("podvm {}", env!("CARGO_PKG_VERSION"));
            }
            std::process::ExitCode::SUCCESS
        }
        Some("-h") | Some("--help") | Some("help") => {
            print!("{USAGE}");
            std::process::ExitCode::SUCCESS
        }
        // ⭐ T-0802, measured: `docker` with no arguments prints its help and
        // exits **0**. podvm does the same, because a script that runs the
        // bare name to see whether the tool is there reads that status.
        None => {
            print!("{USAGE}");
            std::process::ExitCode::SUCCESS
        }
        other => {
            let mut err = std::io::stderr().lock();
            let name = other.unwrap_or("");
            // ⛔ TODO/cli.md T-0801. The parity table answers, so a docker verb
            // podvm does not have gets the REASON it does not, from the one
            // place that records it, rather than a milestone number that ages.
            match parity::verb(name) {
                Some(row) => {
                    let _ = writeln!(err, "podvm: {name}: {}", row.note);
                    let _ = writeln!(
                        err,
                        "podvm: `podvm system info --format '{{{{json .Parity}}}}'` is \
                         every verb and flag podvm has and every one it does not"
                    );
                }
                None => {
                    let _ = writeln!(
                        err,
                        "podvm: {name}: podvm has no such command, and neither does \
                         the parity table (TOOL.md section 6.8); `podvm --help` lists the commands"
                    );
                }
            }
            // T-1769: the name matters where podvm runs under another one.
            if !argv0.ends_with("podvm") {
                let _ = writeln!(err, "podvm: invoked as {argv0}");
            }
            // ⭐ T-0802, and the two halves are different codes because
            // docker's are. A verb DOCKER HAS and podvm refuses is podvm
            // failing to run the caller's command: 125. A verb neither has is
            // a name docker itself answers with 1.
            exit(match parity::verb(name) {
                Some(_) => EXIT_RUNTIME_ERROR,
                None => EXIT_CLI_ERROR,
            })
        }
    }
}

/// ⛔ THE OLD SPELLING IS A USEFUL ANSWER, NOT "UNKNOWN COMMAND". An
/// agent that typed `podvm ssh` is asking a real question, and the
/// house rule is that a tool must not need its user to learn its
/// differences by guessing. So the message names the group and lists
/// what is in it, and the exit is 125: a verb podvm has and refuses
/// is podvm failing to run the caller's command.
///
/// `local` is the decided counterpart, not a command yet: the line says
/// so, pointing at the decision rather than at an arm that does not
/// exist.
fn ssh_refusal() -> std::process::ExitCode {
    eprintln!(
        "podvm: ssh: the verb is `podvm remote ssh`. The remote group has:\n  \
         podvm remote ssh        reach another machine over a relay\n  \
         podvm machine ssh       a shell in a guest podvm itself runs\n  \
         podvm local <verb>      act on THIS machine: planned, not a command yet \
         (see docs/decisions/remote-verb.md)"
    );
    exit(EXIT_RUNTIME_ERROR)
}

fn probe(args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand("probe", args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member("probe", &member, PROBE_USAGE),
    };
    let args: &[String] = &expanded;
    let mut json = false;
    let mut rows = false;
    let mut cached = false;
    let mut strictness = Strictness::Warn;
    for a in args {
        // ⛔ TODO/cli.md T-0801. The table decides, as in every other parser:
        // an unlisted flag never reaches an arm.
        if a.starts_with('-')
            && let Err(c) = crate::parity::admit("probe", a, PROBE_USAGE)
        {
            return c;
        }
        match a.as_str() {
            "--json" => json = true,
            "--rows" => rows = true,
            "--strict" => strictness = Strictness::Refuse,
            "--cached" => cached = true,
            "-h" | "--help" => {
                print!("{PROBE_USAGE}");
                return 0;
            }
            other if other.starts_with('-') => {
                return crate::parity::no_arm("probe", other);
            }
            other => {
                eprintln!("podvm probe: unknown option {other:?}");
                eprint!("{PROBE_USAGE}");
                return EXIT_FLAG_ERROR;
            }
        }
    }
    if json && rows {
        eprintln!("podvm probe: --json and --rows both write stdout; pick one");
        return EXIT_FLAG_ERROR;
    }

    // ⭐ TODO/probe.md T-0111. `--cached` is the hot path `run` and `exec` take
    // at M3; `--rows` is a per-probe rendering the cached document does not
    // carry, so the two are refused together rather than one silently winning.
    if cached && rows {
        eprintln!(
            "podvm probe: --cached serves the stored --json document, which \
             carries no per-probe rows; pick one"
        );
        return EXIT_FLAG_ERROR;
    }
    if cached {
        return cached_probe(json, strictness);
    }

    let findings = podvm_probe::run();
    let mut selection = Selection::choose(&findings);
    crate::system::to_entered(&findings, &mut selection);

    // ⛔ The rung goes to stdout, the evidence to stderr. A payload's stdout is
    // data to whatever consumes it.
    let mut err = std::io::stderr().lock();
    if json {
        print!("{}", report::document(&findings, &selection));
    } else if rows {
        print!("{}", report::rows(&findings));
    } else {
        println!("{}", selection.rung.word());
        let _ = write!(err, "{}", report::evidence(&findings, &selection));
    }

    let code = selection.exit_code(strictness);
    if code != 0 {
        let _ = writeln!(
            err,
            "podvm probe: --strict, and this machine is {} rather than {}{}",
            selection.rung.word(),
            Selection::STRICT_FLOOR.word(),
            if selection.controls_answered {
                ""
            } else {
                ", and a control of the bogus-argument discriminator could not answer"
            }
        );
    }
    code
}

/// `podvm probe --cached`.
///
/// ⛔ Where the cache is served, the evidence says so on stderr. A rung read
/// from a file and a rung measured now are different sentences, and printing
/// the first as though it were the second is the failure T-0111 exists to stop.
fn cached_probe(json: bool, strictness: Strictness) -> i32 {
    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("podvm probe: {e}");
            return e.exit_code();
        }
    };
    let probe = podvm_image::probe_cache::resolve(&store);
    let mut err = std::io::stderr().lock();
    match &probe.source {
        podvm_image::probe_cache::Source::Cache => {
            let _ = writeln!(
                err,
                "podvm probe: served from {}, whose key still matches this \
                 machine's confinement",
                probe.path.display()
            );
        }
        podvm_image::probe_cache::Source::Measured(why) => {
            let _ = writeln!(err, "podvm probe: measured now, because {why}");
        }
    }
    if let Some(why) = &probe.not_written {
        let _ = writeln!(err, "podvm probe: note: the answer was not cached: {why}");
    }

    let Some(rung) = probe.rung() else {
        let _ = writeln!(
            err,
            "podvm probe: the document at {} carries no rung",
            probe.path.display()
        );
        return EXIT_RUNTIME_ERROR;
    };
    if json {
        print!("{}", probe.document);
    } else {
        println!("{rung}");
    }

    // ⚠ `--strict` on the cached path reads `strict_ok` out of the document
    // rather than re-deriving it: `Selection` is not reconstructed from JSON,
    // and a second implementation of the rung rules is exactly the copy-paste
    // docs/conventions/code.md forbids.
    if matches!(strictness, Strictness::Refuse) {
        let ok = probe.document.contains("\"strict_ok\":true");
        if !ok {
            let _ = writeln!(
                err,
                "podvm probe: --strict, and this machine is {rung} rather than {}",
                Selection::STRICT_FLOOR.word()
            );
            return 1;
        }
    }
    0
}

fn exit(code: i32) -> std::process::ExitCode {
    // ⚠ An exit status carries eight bits. Anything wider would be truncated
    // into a different code, so it is clamped to one that cannot be confused
    // with a verdict.
    std::process::ExitCode::from(u8::try_from(code).unwrap_or(EXIT_RUNTIME_ERROR as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_ssh_spelling_is_a_runtime_refusal() {
        assert_eq!(ssh_refusal(), exit(EXIT_RUNTIME_ERROR));
    }
}
