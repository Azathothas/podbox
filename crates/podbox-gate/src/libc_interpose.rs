//! podbox-libc-interpose: one static binary beside one preload object per libc.
//!
//! A behaviour-preserving port of `60-interposer-libc.sh`
//! (T-1577) as a check in the gate crate. It stays out of the workspace
//! members: no new crate, no member row, one more binary beside the gate.
//! Arguments are parsed by hand; `clap` was ruled out at T-0908. The crate
//! carries no dependencies.
//!
//! Two claims are checked, both of which the specification leaves open:
//!
//!   A. can `crates/podbox-interpose` be built as a cdylib under the
//!      workspace's own `-C target-feature=+crt-static`?
//!   B. can a musl-linked preload object be loaded into a glibc payload?
//!
//! B needs a musl libc on the build host. Without one the musl-target
//! object records `libc.so.6` in DT_NEEDED and is a glibc object under a
//! musl target name. This binary reads DT_NEEDED and exits 2 rather than
//! answering B from that object: an object that is not musl-linked cannot
//! measure whether a musl-linked one loads. A musl toolchain alone is not
//! enough for this crate: `rustc` passes `-lgcc_s` on the musl target even
//! under `panic = "abort"`, and `musl-tools` ships no musl-linked
//! libgcc. What is missing then is a musl-linked libgcc, not musl itself.
//!
//! Inputs pinned: the toolchain named by rust-toolchain.toml, the two
//! targets below, and this host's own /usr/bin/env as the glibc payload.
//! No network, no container, no registry.
//!
//! Exit: 0 every question this host can answer was answered and matched,
//! 1 an answer contradicts what is expected, 2 a question could not run
//! here.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

const TARGETS: &[&str] = &["x86_64-unknown-linux-musl", "x86_64-unknown-linux-gnu"];
const PAYLOAD: &str = "/usr/bin/env";

fn find_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let profile_dir = exe.parent()?;
    let mut candidate = profile_dir.join("..").join("..");
    for _ in 0..8 {
        if candidate.join("TODO").join("INDEX.md").is_file() {
            return Some(candidate);
        }
        candidate = candidate.join("..");
    }
    None
}

fn command_present(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {name}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn capture(
    prog: &str,
    args: &[&str],
    cwd: &Path,
    extra_env: &[(String, String)],
) -> Option<(i32, String)> {
    let mut cmd = Command::new(prog);
    cmd.args(args).current_dir(cwd);
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let output = cmd.stdin(Stdio::null()).output().ok()?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Some((output.status.code().unwrap_or(1), text))
}

fn print_help() {
    println!("podbox-libc-interpose: the one-binary one-object-per-libc check.");
    println!();
    println!("  podbox-libc-interpose   build both objects, refuse the workspace");
    println!("                          crt-static, and load each where it belongs");
    println!();
    println!("Exit: 0 every answer matched, 1 one contradicts, 2 it could not run.");
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    if argv
        .iter()
        .any(|arg| arg == "-h" || arg == "--help" || arg == "help")
    {
        print_help();
        return ExitCode::from(0);
    }
    if !argv.is_empty() {
        eprintln!("podbox-libc-interpose takes no arguments");
        return ExitCode::from(2);
    }
    let root = match find_root() {
        Some(root) => root,
        None => {
            eprintln!("SKIP: cannot locate the repository root");
            return ExitCode::from(2);
        }
    };
    let krate = root.join("crates/podbox-interpose");
    let out = env::var("OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("experiments/.interpose"));
    if fs::create_dir_all(&out).is_err() {
        eprintln!("SKIP: cannot create {}", out.display());
        return ExitCode::from(2);
    }

    println!("== conditions");
    let date = capture("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], &root, &[])
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    println!("date              {date}");
    let kernel = capture("uname", &["-r"], &root, &[])
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    println!("host kernel       {kernel}");
    let rustc = capture("rustc", &["--version"], &root, &[])
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "MISSING".to_string());
    println!("rustc             {rustc}");
    let cargo = capture("cargo", &["--version"], &root, &[])
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "MISSING".to_string());
    println!("cargo             {cargo}");
    let installed = capture("rustup", &["target", "list", "--installed"], &root, &[])
        .map(|(_, text)| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    println!("targets installed {installed}");
    println!();

    if !command_present("cargo") {
        eprintln!("SKIP: cargo not on PATH");
        return ExitCode::from(2);
    }
    if !krate.is_dir() {
        eprintln!("SKIP: {} is absent", krate.display());
        return ExitCode::from(2);
    }
    for target in TARGETS {
        if !installed.split_whitespace().any(|line| line == *target) {
            eprintln!("SKIP: target {target} is not installed");
            return ExitCode::from(2);
        }
    }

    let mut rc = 0;

    // A. cdylib under the workspace's +crt-static. RUSTFLAGS is deliberately
    // UNSET here so the crate picks up the root cargo config, which is the
    // condition being measured.
    println!("== A. cdylib under the workspace's +crt-static");
    let a_log = out.join("a-crt-static.txt");
    let mut a_cmd = Command::new("cargo");
    a_cmd
        .args(["build", "--release", "--target", TARGETS[0]])
        .current_dir(&krate)
        .env_remove("RUSTFLAGS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let a_out = a_cmd.output();
    match a_out {
        Ok(output) => {
            let code = output.status.code().unwrap_or(1);
            let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
            combined.push_str(&String::from_utf8_lossy(&output.stderr));
            let _ = fs::write(&a_log, &combined);
            if code == 0 {
                println!(
                    "UNEXPECTED: the build succeeded. The root config no longer sets +crt-static,"
                );
                println!(
                    "            or cargo's cdylib rule changed. Re-read the interpose builder."
                );
                rc = 1;
            } else if combined.contains("does not support these crate types") {
                println!("REFUSED, as expected:");
                for line in combined.lines() {
                    if line.contains("does not support these crate types") {
                        println!("  {line}");
                        break;
                    }
                }
            } else {
                println!("FAILED for another reason; see {}", a_log.display());
                for line in combined
                    .lines()
                    .rev()
                    .take(3)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                {
                    println!("  {line}");
                }
                rc = 1;
            }
        }
        Err(e) => {
            println!("FAILED for another reason: {e}");
            rc = 1;
        }
    }
    println!();

    // A2. the same crate with -crt-static, one object per libc. THE MUSL ARM
    // NEEDS A LINKER THAT CARRIES ITS OWN libgcc, and the default `cc`
    // linked the "musl" object against the HOST's glibc. `zig cc` carries
    // `compiler-rt`. Where zig is absent the arm falls back to the default
    // linker and says so rather than reporting a musl object that is not
    // one.
    println!("== A2. the same crate with -crt-static, one object per libc");
    let zig_linker = root.join("scripts/zig-cc.sh");
    let mut musl_link = String::new();
    if command_present("zig") && zig_linker.is_file() {
        musl_link = format!("-C linker={}", zig_linker.display());
        let zig_version = capture("zig", &["version"], &root, &[])
            .map(|(_, text)| text.trim().to_string())
            .unwrap_or_default();
        println!("  linker for musl: scripts/zig-cc.sh (zig {zig_version})");
    } else {
        println!("  zig is absent, so the musl arm uses the default linker and will");
        println!("    produce a glibc-linked object. Install it with");
        println!("    ./scripts/common/bootstrap-env.sh zig");
    }
    let a2_log = out.join("a2-build.txt");
    for target in TARGETS {
        let link = if *target == TARGETS[0] {
            musl_link.clone()
        } else {
            String::new()
        };
        let flags = format!("-C target-feature=-crt-static {link}");
        let flags = flags.trim().to_string();
        let code = {
            let mut cmd = Command::new("cargo");
            let output = cmd
                .args(["build", "--release", "--target", target])
                .env("RUSTFLAGS", &flags)
                .current_dir(&krate)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();
            match output {
                Ok(output) => {
                    let mut previous = fs::read_to_string(&a2_log).unwrap_or_default();
                    previous.push_str(&String::from_utf8_lossy(&output.stdout));
                    previous.push_str(&String::from_utf8_lossy(&output.stderr));
                    let _ = fs::write(&a2_log, previous);
                    output.status.code().unwrap_or(1)
                }
                Err(_) => 127,
            }
        };
        if code == 0 {
            let so = krate
                .join("target")
                .join(target)
                .join("release")
                .join("libpodbox_interpose.so");
            let bytes = fs::metadata(&so).map(|meta| meta.len()).unwrap_or(0);
            let needed = capture(
                "readelf",
                &["-d", so.to_string_lossy().as_ref()],
                &root,
                &[],
            )
            .map(|(_, text)| text.lines().filter(|line| line.contains("NEEDED")).count())
            .unwrap_or(0);
            println!("  {target:<30} OK   {bytes:>8} bytes  {needed} NEEDED entries");
        } else {
            println!("  {target:<30} FAIL");
            rc = 1;
        }
    }
    println!();

    // B. cross-libc preload reach. The payload is this host's own
    // /usr/bin/env, which is glibc-linked. Its interpreter is read rather
    // than assumed.
    println!("== B. cross-libc preload reach");
    let musl_so = krate
        .join("target")
        .join(TARGETS[0])
        .join("release")
        .join("libpodbox_interpose.so");
    let gnu_so = krate
        .join("target")
        .join(TARGETS[1])
        .join("release")
        .join("libpodbox_interpose.so");
    if !Path::new(PAYLOAD).is_file() {
        eprintln!("SKIP: {PAYLOAD} is absent, so there is no glibc payload to preload into");
        return ExitCode::from(2);
    }
    let needed_of = |obj: &Path| -> String {
        capture(
            "readelf",
            &["-d", obj.to_string_lossy().as_ref()],
            &root,
            &[],
        )
        .map(|(_, text)| {
            text.lines()
                .filter(|line| line.contains("NEEDED"))
                .map(|line| {
                    line.split('[')
                        .nth(1)
                        .and_then(|tail| tail.split(']').next())
                        .unwrap_or("")
                        .trim()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
    };
    let musl_needed = needed_of(&musl_so);
    let gnu_needed = needed_of(&gnu_so);
    println!("  payload           {PAYLOAD}");
    let interp = capture("readelf", &["-l", PAYLOAD], &root, &[])
        .map(|(_, text)| {
            text.lines()
                .filter_map(|line| {
                    line.find("/").and_then(|start| {
                        line[start..]
                            .split(']')
                            .next()
                            .map(|piece| piece.split_whitespace().next().unwrap_or("").to_string())
                    })
                })
                .next()
                .unwrap_or_default()
        })
        .unwrap_or_default();
    println!("  payload PT_INTERP {interp}");
    println!("  musl-target DT_NEEDED   {musl_needed}");
    println!("  gnu-target  DT_NEEDED   {gnu_needed}");
    println!();

    // THE CONTROL COMES FIRST. Without a matching-libc object that does
    // load, a cross-libc object that does not load proves nothing about
    // the libc.
    let control = Command::new(PAYLOAD)
        .arg("true")
        .env("LD_PRELOAD", &gnu_so)
        .stdin(Stdio::null())
        .output();
    match control {
        Ok(output) => {
            let code = output.status.code().unwrap_or(1);
            println!("  glibc object into a glibc payload (control): rc={code}");
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            for line in text.lines() {
                if !line.is_empty() {
                    println!("    {line}");
                }
            }
            if code != 0 {
                println!("FAIL: the control did not load. Nothing below is interpretable.");
                return ExitCode::from(1);
            }
        }
        Err(_) => {
            println!("FAIL: the control did not load. Nothing below is interpretable.");
            return ExitCode::from(1);
        }
    }
    println!();

    if musl_needed.contains("libc.so.6") {
        println!("== B: COULD NOT RUN on this host");
        println!("  The musl-target object records libc.so.6 in DT_NEEDED, so it was");
        println!("  linked against this host's glibc and is not a musl object.");
        let ldd = capture("ldd", &["--version"], &root, &[])
            .map(|(_, text)| text.lines().next().unwrap_or("").to_string())
            .unwrap_or_default();
        println!("    host libc     {ldd}");
        let musl_gcc = which_path("musl-gcc");
        println!("    musl-gcc      {musl_gcc}");
        let ld_musl = if Path::new("/lib/ld-musl-x86_64.so.1").exists() {
            "present"
        } else {
            "absent"
        };
        println!("    ld-musl       {ld_musl}");
        let libgcc = fs::read_dir("/usr/lib/x86_64-linux-musl")
            .ok()
            .and_then(|entries| {
                entries
                    .flatten()
                    .find(|entry| entry.file_name() == "libgcc_s.so.1")
                    .map(|entry| entry.path().to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "absent".to_string());
        println!("    musl libgcc_s {libgcc}");
        // NAME THE ACTUAL SHORTAGE. A musl libc being present is not the
        // same as this crate being buildable against it. Saying "no musl
        // here" when musl-gcc is on PATH sends the next session to install
        // a toolchain it already has.
        println!("  What is missing is a musl-linked libgcc_s.so.1, not musl itself.");
        println!("  This does not leave B unanswered. Run");
        println!("     ./experiments/80-interposer-abi.sh, which answers it against the C");
        println!("     reference interposer and needs only musl-gcc, not a musl libgcc.");
        return ExitCode::from(2);
    }

    let trial = Command::new(PAYLOAD)
        .arg("true")
        .env("LD_PRELOAD", &musl_so)
        .stdin(Stdio::null())
        .output();
    match trial {
        Ok(output) => {
            let code = output.status.code().unwrap_or(1);
            println!("  musl object into a glibc payload: rc={code}");
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            for line in text.lines() {
                if !line.is_empty() {
                    println!("    {line}");
                }
            }
            println!();
            println!("== verdict");
            if code != 0 {
                println!("  one object per libc is REQUIRED: the musl object does not load into a");
                println!(
                    "  glibc payload while the glibc object does. TOOL.md section 6.7 does not say"
                );
                println!("  this; TODO/interpose.md T-0702 carries it.");
            } else {
                println!("  the musl object loaded into a glibc payload. The one-object-per-libc");
                println!("  premise is wrong as stated and the entry takes the correction underneath it.");
                rc = 1;
            }
        }
        Err(_) => {
            println!("FAIL: the trial could not run.");
            rc = 1;
        }
    }

    ExitCode::from(rc as u8)
}

fn which_path(name: &str) -> String {
    if command_present(name) {
        capture(
            "sh",
            &["-c", &format!("command -v {name}")],
            Path::new("."),
            &[],
        )
        .map(|(_, text)| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| name.to_string())
    } else {
        "absent".to_string()
    }
}
