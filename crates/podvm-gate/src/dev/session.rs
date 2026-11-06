use super::*;

pub(super) fn print_help() {
    println!("podvm-dev: get a fresh session to writing code as fast as possible.");
    println!();
    println!("  podvm-dev                  start the build behind the reading");
    println!("  podvm-dev start            the same, explicit");
    println!("  podvm-dev status           running, ready, failed, stale or unknown");
    println!("  podvm-dev wait             block until the background work finishes");
    println!("  podvm-dev build            one foreground build, after a source change");
    println!("  podvm-dev check            fmt, clippy, build, tests and both gates");
    println!("  podvm-dev session          the host report, then the lane setup");
    println!("  podvm-dev startup          the cold-start measurement T-1005 cites");
    println!();
    println!("Exit: 0 the requested thing succeeded, 1 it failed, 2 it could not run.");
}
// ------------------------------------------------------------ the session
//
// The one command a session runs first. It answers four questions and then
// starts the environment: where am I, what time is it, what is installed,
// and which lane does this host use. It creates nothing that a second run
// would create again.

fn say(quiet: bool, text: &str) {
    if !quiet {
        println!("{text}");
    }
}

fn row(quiet: bool, key: &str, value: &str) {
    if !quiet {
        println!("  {key:<14} {value}");
    }
}

fn tool_version(tool: &str, root: &Path) -> Option<String> {
    let _ = root;
    let (rc, text) = match tool {
        "python3" | "py" => capture(
            tool,
            &["-c", "import sys; print(sys.version.split()[0])"],
            Path::new("."),
        )?,
        "zig" => capture("zig", &["version"], Path::new("."))?,
        "go" => capture("go", &["version"], Path::new("."))?,
        "wsl-toolkit" => capture("wsl-toolkit", &["version"], Path::new("."))?,
        _ => capture(tool, &["--version"], Path::new("."))?,
    };
    let _ = rc;
    Some(text.lines().next().unwrap_or("").trim().to_string())
}

fn command_present(name: &str) -> bool {
    // `command -v` through `sh`: `command` is a shell builtin, not a
    // binary, so it cannot be executed directly.
    Command::new("sh")
        .args(["-c", &format!("command -v {name}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub(super) fn do_session(root: &Path, target: &str, args: &[String]) -> ExitCode {
    let mut check = false;
    let mut quiet = false;
    for arg in args {
        match arg.as_str() {
            "--check" => check = true,
            "--quiet" => quiet = true,
            "-h" | "--help" => {
                println!("podvm-dev session - the one command a session runs first.");
                return ExitCode::from(0);
            }
            _ => {
                eprintln!("podvm-dev: unknown option {arg}");
                return ExitCode::from(2);
            }
        }
    }
    let mut problems: u32 = 0;

    // 1. when. Read from the machine, never typed.
    let now = capture("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    let local = capture("date", &["+%Y-%m-%dT%H:%M:%S%z"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    say(quiet, "== when");
    row(quiet, "utc", &now);
    row(quiet, "local", &local);

    // 2. where.
    let uname_s = capture("uname", &["-s"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let uname_r = capture("uname", &["-r"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let uname_m = capture("uname", &["-m"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let mut kind = "unknown".to_string();
    match uname_s.as_str() {
        "Linux" => {
            kind = "linux".to_string();
            // A container and a WSL guest are both Linux and neither is the
            // host. Each is named rather than folded into one, because the
            // setup differs.
            let dockerenv = Path::new("/.dockerenv").exists();
            let cgroup = fs::read_to_string("/proc/1/cgroup").unwrap_or_default();
            let grouped = cgroup.contains("docker")
                || cgroup.contains("podman")
                || cgroup.contains("containerd")
                || cgroup.contains("libpod");
            let osrelease = fs::read_to_string("/proc/sys/kernel/osrelease")
                .unwrap_or_default()
                .to_lowercase();
            if dockerenv || grouped {
                kind = "container".to_string();
            } else if osrelease.contains("microsoft") {
                kind = "wsl-guest".to_string();
            }
        }
        other
            if other.starts_with("MINGW")
                || other.starts_with("MSYS")
                || other.starts_with("CYGWIN") =>
        {
            kind = "windows".to_string();
        }
        "Darwin" => kind = "darwin".to_string(),
        _ => {}
    }
    say(quiet, "");
    say(quiet, "== where");
    row(quiet, "kind", &kind);
    row(quiet, "uname", &format!("{uname_s} {uname_r} {uname_m}"));
    row(quiet, "cwd", &root.to_string_lossy());
    let user = capture("id", &["-un"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    let uid = capture("id", &["-u"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    row(quiet, "user", &format!("{user}, uid {uid}"));
    if !root.join("AGENTS.md").is_file() || !root.join("TODO").is_dir() {
        eprintln!("podvm-dev: this is not the podvm tree. AGENTS.md or TODO/ is missing.");
        return ExitCode::from(2);
    }

    // 3. the tree.
    say(quiet, "");
    say(quiet, "== the tree");
    if command_present("git")
        && Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    {
        let branch = capture("git", &["rev-parse", "--abbrev-ref", "HEAD"], root)
            .map(|(_, text)| text.trim().to_string())
            .unwrap_or_else(|| "-".to_string());
        row(quiet, "branch", &branch);
        let head = capture("git", &["log", "-1", "--format=%h %s"], root)
            .map(|(_, text)| text.trim().to_string())
            .unwrap_or_else(|| "-".to_string());
        row(quiet, "head", &head);
        // grep -c exits 1 when it counts zero, which breaks an && chain.
        // The count is read from this process instead.
        let dirty = capture("git", &["status", "--porcelain"], root)
            .map(|(_, text)| text.lines().count().to_string())
            .unwrap_or_else(|| "-".to_string());
        row(quiet, "dirty", &format!("{dirty} file(s)"));
        let gname = capture("git", &["config", "user.name"], root)
            .map(|(_, text)| text.trim().to_string())
            .unwrap_or_default();
        let gmail = capture("git", &["config", "user.email"], root)
            .map(|(_, text)| text.trim().to_string())
            .unwrap_or_default();
        let identity = format!(
            "{} {}",
            if gname.is_empty() { "UNSET" } else { &gname },
            if gmail.is_empty() { "UNSET" } else { &gmail }
        );
        row(quiet, "identity", &identity);
        // The identity is the operator's and is never invented. An unset
        // one is named here rather than at the commit.
        if gname.is_empty() || gmail.is_empty() {
            eprintln!("  ! the git identity is unset. Read it from the history, then set it in this repository.");
            problems += 1;
        }
        // Two different states, and they need two different next steps.
        if branch == "main" {
        } else if branch.starts_with("publish/") {
            eprintln!("  ! on {branch}, which is a publish branch a previous session left open. Merge it or delete it before starting work: gh pr merge --rebase --delete-branch");
            problems += 1;
        } else {
            eprintln!("  ! on {branch}. Work happens on main. TODO/RULES.md section 2 settles this and it is not open.");
            problems += 1;
        }
    } else {
        eprintln!("  ! git is absent, or this directory is not a checkout");
        problems += 1;
    }

    // 4. tools. A NAME ON PATH IS NOT A WORKING PROGRAM: each version is
    // read by RUNNING the tool rather than by finding it.
    say(quiet, "");
    say(quiet, "== tools");
    for tool in [
        "git",
        "cargo",
        "rustc",
        "zig",
        "python3",
        "py",
        "jq",
        "curl",
        "tar",
        "podman",
        "docker",
        "go",
        "scc",
        "codegraph",
        "wsl-toolkit",
    ] {
        if command_present(tool) {
            let version = tool_version(tool, root)
                .filter(|version| !version.is_empty())
                .unwrap_or_else(|| "on PATH and answered nothing".to_string());
            row(quiet, tool, &version);
        } else {
            row(quiet, tool, "-");
        }
    }

    // 5. codegraph. The index is per machine and is not tracked, so it is
    // built here and synced every session.
    say(quiet, "");
    say(quiet, "== codegraph");
    if !command_present("codegraph") {
        row(
            quiet,
            "index",
            "codegraph is absent. grep is the fallback and it is slower.",
        );
    } else if check {
        if root.join(".codegraph").is_dir() {
            row(quiet, "index", "present, and --check builds nothing");
        } else {
            row(quiet, "index", "absent, and --check builds nothing");
        }
    } else if root.join(".codegraph").is_dir() {
        let ok = Command::new("codegraph")
            .arg("sync")
            .arg(".")
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if ok {
            row(quiet, "index", "synced");
        } else {
            row(quiet, "index", "sync failed. Run codegraph sync");
        }
    } else {
        let ok = Command::new("codegraph")
            .arg("init")
            .arg(".")
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if ok {
            row(quiet, "index", "built");
        } else {
            row(quiet, "index", "init failed. Run codegraph init");
        }
    }

    // 6. the lane. THE LANE IS THIS COMMAND'S DECISION AND NOT THE
    // SESSION'S. A session that picks the lane by hand runs the Debian
    // bootstrap against an Arch base, or calls wsl.exe on a host where
    // docs/containers.md forbids it.
    say(quiet, "");
    say(quiet, "== the lane");
    let lane = match kind.as_str() {
        "linux" | "container" | "wsl-guest" => "native",
        "windows" => "wsl-toolkit",
        _ => "none",
    };
    row(quiet, "lane", lane);
    match lane {
        "native" => {
            row(
                quiet,
                "setup",
                "scripts/common/bootstrap-env.sh, started by podvm-dev",
            );
            if check {
                let rc = Command::new("./scripts/common/bootstrap-env.sh")
                    .arg("--check")
                    .current_dir(root)
                    .status()
                    .map(|status| status.code().unwrap_or(1))
                    .unwrap_or(1);
                if rc != 0 {
                    eprintln!("  ! bootstrap --check exits {rc}");
                    problems += 1;
                }
            } else {
                // In the background on purpose. It compiles behind the
                // reading, and the reading needs no toolchain.
                let exe =
                    env::current_exe().unwrap_or_else(|_| root.join("target/release/podvm-dev"));
                let rc = Command::new(exe)
                    .arg("start")
                    .env("PODVM_TARGET", target)
                    .current_dir(root)
                    .status()
                    .map(|status| status.code().unwrap_or(1))
                    .unwrap_or(1);
                if rc != 0 {
                    eprintln!("  ! podvm-dev start did not start");
                    problems += 1;
                }
            }
        }
        "wsl-toolkit" => {
            if !command_present("wsl-toolkit") {
                eprintln!(
                    "  ! wsl-toolkit is not on PATH. docs/agent-tooling.md says where it lives."
                );
                problems += 1;
            } else {
                row(
                    quiet,
                    "instance",
                    "podvm, which is the distribution wsl-toolkit-podvm",
                );
                if check {
                    let rc = Command::new("wsl-toolkit")
                        .args(["--instance", "podvm", "base", "status", "--probe"])
                        .current_dir(root)
                        .status()
                        .map(|status| status.code().unwrap_or(1))
                        .unwrap_or(1);
                    if rc != 0 {
                        eprintln!("  ! the base is not usable. Run this command without --check.");
                        problems += 1;
                    }
                } else {
                    // Reconciles. It can re-provision and restart the base; from a
                    // redirected app shell it is refused and installs nothing.
                    let rc = Command::new("wsl-toolkit")
                        .args(["--instance", "podvm", "base", "ensure", "--probe"])
                        .current_dir(root)
                        .status()
                        .map(|status| status.code().unwrap_or(1))
                        .unwrap_or(1);
                    if rc != 0 {
                        eprintln!("  ! base ensure failed. It builds from an OCI image, so it needs a container engine on this host: podman machine start");
                        problems += 1;
                    }
                }
                row(quiet, "host half", "sh scripts/common/check-gate.sh --fast");
                row(
                    quiet,
                    "linux half",
                    "sh scripts/windows/run-in-base.sh JOB.sh",
                );
            }
        }
        _ => {
            eprintln!("  ! this host has no lane. docs/containers.md names the three that exist.");
            problems += 1;
        }
    }

    // 7. the reading.
    say(quiet, "");
    say(quiet, "== read these, in this order");
    say(
        quiet,
        "  1. AGENTS.md           the router and the absolutes",
    );
    say(
        quiet,
        "  2. TODO/PROGRESS.md    the state, the work order, the open questions",
    );
    say(
        quiet,
        "  3. TODO/RESUME.md      what the last session left in flight",
    );
    say(quiet, "  4. the TODO/ entry that your task names, in full");

    say(quiet, "");
    if problems > 0 {
        say(
            quiet,
            &format!("== {problems} thing(s) need attention. Each is named above."),
        );
        ExitCode::from(1)
    } else {
        say(
            quiet,
            &format!("== ready. The session start instant is {now}."),
        );
        ExitCode::from(0)
    }
}

// ------------------------------------------------------------ the startup
//
// How long a fresh session waits before it can run podvm, and how much of
// that wait can be spent reading instead. WHAT THIS CAN AND CANNOT MEASURE
// HERE: the container this runs in already has its tools and its cargo
// registry cache, so the apt and download halves cannot be re-measured
// without destroying them. They are reported as `not measured here` rather
// than estimated, and the compile half, which is the one that dominates on
// a warm registry, is measured properly against an empty target directory.
