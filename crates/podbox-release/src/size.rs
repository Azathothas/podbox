//! podbox-size: how large is the release artefact, and what is in it.
//!
//! A behaviour-preserving port of experiments/110-bloat-delta.sh (T-1562).
//! The script stays as a declaration stub that execs this binary: the
//! ceiling keeps its one home in the stub (`CEILING_BYTES=<n>`), and the
//! binary reads it at runtime, because the gate refuses the digits anywhere
//! else. The measurement logic lives here.
//!
//! Usage: podbox-size [AREA]  (letters, digits and dashes; default baseline)
//!
//! Exit: 0 measured and under the ceiling, 1 over the ceiling or the build
//! failed, 2 it could not run.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const TARGET: &str = "x86_64-unknown-linux-musl";
const DECLARATION: &str = "experiments/110-bloat-delta.sh";
const DECL_NAME: &str = "CEILING_BYTES";

fn fail(text: &str) -> ! {
    eprintln!("{text}");
    std::process::exit(2);
}

/// Run to completion, returning success plus stdout and stderr separately.
/// The build log keeps both streams like the script's `2>&1`; the bloat
/// table keeps stdout alone, because stderr diagnostics are not the
/// breakdown. A nonzero exit still returns its streams: the script reads a
/// partial table the same way.
fn run_split(
    prog: &str,
    args: &[&str],
    cwd: &Path,
    env_extra: &[(&str, &str)],
    secs: u64,
) -> Result<(bool, String, String), String> {
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new(prog);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env_extra {
        command.env(key, value);
    }
    let child = command
        .spawn()
        .map_err(|e| format!("could not run {prog}: {e}"))?;
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Err(_) => Err(format!("{prog} timed out after {secs}s")),
        Ok(Err(e)) => Err(format!("could not read {prog}: {e}")),
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            Ok((output.status.success(), stdout, stderr))
        }
    }
}

fn capture(
    prog: &str,
    args: &[&str],
    cwd: &Path,
    env_extra: &[(&str, &str)],
    secs: u64,
) -> Option<String> {
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new(prog);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (key, value) in env_extra {
        command.env(key, value);
    }
    let child = command.spawn().ok()?;
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(Ok(output)) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        _ => None,
    }
}

fn repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// The ceiling from its one home: the `CEILING_BYTES=<n>` line in the stub.
/// Absent or unreadable is exit 2, never a guess.
fn read_ceiling(root: &Path) -> u64 {
    let text = fs::read_to_string(root.join(DECLARATION)).unwrap_or_default();
    for line in text.split('\n') {
        let line = line.trim();
        if let Some(digits) = line.strip_prefix(&format!("{DECL_NAME}=")) {
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                if let Ok(value) = digits.parse::<u64>() {
                    return value;
                }
            }
        }
    }
    eprintln!("podbox-size: the ceiling declaration is absent from {DECLARATION}");
    std::process::exit(2);
}

fn main() {
    let mut argv = std::env::args().skip(1);
    let area = argv.next().unwrap_or_else(|| "baseline".to_string());
    if area.is_empty()
        || !area
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        eprintln!("SKIP: <area> must be lowercase letters, digits and dashes");
        std::process::exit(2);
    }
    if Command::new("cargo")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        eprintln!("SKIP: cargo is not on PATH");
        std::process::exit(2);
    }
    let root = repo_root().unwrap_or_else(|| fail("SKIP: not the podbox tree"));
    let ceiling = read_ceiling(&root);
    let out = root.join(format!("experiments/results/bloat-{area}.txt"));
    let base = root.join("experiments/results/bloat-baseline.txt");
    let work = std::env::temp_dir().join(format!("podbox-size-{}", std::process::id()));
    if fs::create_dir_all(&work).is_err() {
        fail("SKIP: cannot create scratch");
    }
    struct Guard(PathBuf);
    impl Drop for Guard {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }
    let _guard = Guard(work.clone());

    let date = capture("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], &root, &[], 10).unwrap_or_default();
    let kernel = capture("uname", &["-r"], &root, &[], 10).unwrap_or_default();
    let rustc = capture("rustc", &["--version"], &root, &[], 10).unwrap_or_default();
    let cargo = capture("cargo", &["--version"], &root, &[], 10).unwrap_or_default();
    println!("== conditions");
    println!("date              {}", date.trim());
    println!("host kernel       {}", kernel.trim());
    println!("rustc             {}", rustc.trim());
    println!("cargo             {}", cargo.trim());
    println!("target            {TARGET}");
    println!("area              {area}");
    println!("ceiling           {ceiling} bytes, declared in the stub and nowhere else");
    println!();

    // The total, from the shipping profile unmodified.
    println!("== the total, from the shipping profile");
    let manifest = root.join("Cargo.toml").to_string_lossy().into_owned();
    let build_log = work.join("build.log");
    match run_split(
        "cargo",
        &[
            "build",
            "--release",
            "--target",
            TARGET,
            "--manifest-path",
            &manifest,
        ],
        &root,
        &[],
        1800,
    ) {
        Ok((true, stdout, stderr)) => {
            let _ = fs::write(&build_log, format!("{stdout}{stderr}"));
        }
        Ok((false, stdout, stderr)) => {
            let _ = fs::write(&build_log, format!("{stdout}{stderr}"));
            eprintln!("FAIL: the release build did not succeed");
            if let Ok(log) = fs::read_to_string(&build_log) {
                let mut shown = 0;
                let lines: Vec<&str> = log.split('\n').collect();
                let mut i = 0;
                while i < lines.len() && shown < 40 {
                    if lines[i].starts_with("error") || lines[i].starts_with("warning: unused") {
                        for line in lines.iter().skip(i).take(7) {
                            eprintln!("{line}");
                            shown += 1;
                            if shown >= 40 {
                                break;
                            }
                        }
                    }
                    i += 1;
                }
            }
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("FAIL: the release build did not succeed");
            std::process::exit(1);
        }
    }
    if let Ok(log) = fs::read_to_string(&build_log) {
        let lines: Vec<&str> = log.split('\n').collect();
        let tail: Vec<&str> = lines.iter().rev().take(2).rev().copied().collect();
        for line in tail {
            println!("{line}");
        }
    }
    let bin = root.join(format!("target/{TARGET}/release/podbox"));
    if !bin.is_file() {
        eprintln!("SKIP: {} was not produced", bin.display());
        std::process::exit(2);
    }
    let total = fs::metadata(&bin).map(|m| m.len()).unwrap_or(0);
    println!("  {}", bin.display());
    println!("  total_bytes {total}");
    println!("  headroom    {}", ceiling.saturating_sub(total));

    let interp = capture("readelf", &["-l", &bin.to_string_lossy()], &root, &[], 60)
        .map(|text| {
            text.split('\n')
                .filter(|line| line.contains("INTERP"))
                .count()
        })
        .unwrap_or(0);
    println!("  PT_INTERP   {interp}");

    // The control: a zero delta has two causes, and only the scaffold tells
    // them apart. TODO/probe.md T-0102 is the same discipline elsewhere.
    let mut scaffold_said = String::new();
    if let Some(first) = capture(
        &bin.to_string_lossy(),
        &["version"],
        &root,
        &[("PODBOX_SWEEP", "1")],
        60,
    )
    .and_then(|text| text.split('\n').next().map(str::to_string))
    {
        if !first.starts_with("podbox ") {
            scaffold_said = first.trim().to_string();
        }
    }
    println!(
        "  scaffold    {}",
        if scaffold_said.is_empty() {
            "(said nothing)".to_string()
        } else {
            scaffold_said.clone()
        }
    );

    // The interposer pair, per libc.
    println!();
    println!("== the interposer pair, per libc");
    let musl_so = root.join(
        "crates/podbox-interpose/target/x86_64-unknown-linux-musl/release/libpodbox_interpose.so",
    );
    let gnu_so = root.join(
        "crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so",
    );
    let musl_bytes = fs::metadata(&musl_so)
        .map(|m| m.len().to_string())
        .unwrap_or_else(|_| "absent".to_string());
    let gnu_bytes = fs::metadata(&gnu_so)
        .map(|m| m.len().to_string())
        .unwrap_or_else(|_| "absent".to_string());
    println!("  musl {musl_bytes}");
    println!("  gnu {gnu_bytes}");

    // The breakdown, from an unstripped build in its own directory.
    println!();
    println!("== what is in it");
    let mut bloat_status = String::new();
    let bloat_txt = work.join("bloat.txt");
    if capture("cargo", &["bloat", "--version"], &root, &[], 30).is_none() {
        bloat_status =
            "not taken: cargo-bloat is not installed (cargo install cargo-bloat)".to_string();
        println!("  SKIP {bloat_status}");
    } else {
        let target_dir = work.join("bloat-target").to_string_lossy().into_owned();
        let err_log = work.join("bloat.err");
        match run_split(
            "cargo",
            &[
                "bloat",
                "--release",
                "--target",
                TARGET,
                "--manifest-path",
                &manifest,
                "--bin",
                "podbox",
                "-n",
                "20",
            ],
            &root,
            &[
                ("CARGO_TARGET_DIR", &target_dir),
                ("CARGO_PROFILE_RELEASE_STRIP", "none"),
            ],
            1200,
        ) {
            Ok((_, table, stderr)) => {
                let _ = fs::write(&bloat_txt, &table);
                let _ = fs::write(&err_log, &stderr);
            }
            Err(_) => {
                let _ = fs::write(&err_log, "cargo bloat did not run");
            }
        }
        if bloat_txt.is_file() {
            if let Ok(table) = fs::read_to_string(&bloat_txt) {
                if !table.trim().is_empty() {
                    for line in table.split('\n') {
                        println!("  {line}");
                    }
                    bloat_status =
                        "taken from an unstripped build of the release profile".to_string();
                }
            }
        }
        if bloat_status.is_empty() {
            bloat_status = "not taken: cargo bloat produced nothing".to_string();
            println!("  SKIP {bloat_status}");
            if let Ok(err) = fs::read_to_string(&err_log) {
                for line in err.split('\n') {
                    eprintln!("    {line}");
                }
            }
        }
    }

    println!();
    println!("== the dependency tree");
    let mut deps = 0u64;
    if let Some(tree) = capture(
        "cargo",
        &[
            "tree",
            "--edges",
            "normal",
            "--prefix",
            "none",
            "--manifest-path",
            &manifest,
        ],
        &root,
        &[],
        300,
    ) {
        let mut unique = BTreeSet::new();
        for line in tree.split('\n') {
            if !line.trim().is_empty() {
                unique.insert(line.to_string());
            }
        }
        deps = unique
            .iter()
            .filter(|line| !line.starts_with("podbox-"))
            .count() as u64;
    }
    println!("  third-party crates in the normal dependency graph: {deps}");

    // The delta.
    let mut delta_line = "the baseline itself; there is nothing before it".to_string();
    let mut before: Option<u64> = None;
    if area != "baseline" {
        if let Ok(base_text) = fs::read_to_string(&base) {
            let found = base_text
                .split('\n')
                .filter_map(|line| line.strip_prefix("total_bytes "))
                .next()
                .map(str::trim);
            match found.and_then(|digits| digits.parse::<u64>().ok()) {
                Some(value) => {
                    before = Some(value);
                    delta_line = format!(
                        "{} bytes against {value} in {}",
                        total as i64 - value as i64,
                        base.file_name().unwrap_or_default().to_string_lossy()
                    );
                }
                None => {
                    delta_line = "- (the baseline file carries no total_bytes line)".to_string()
                }
            }
        } else {
            delta_line = "- (no committed baseline; run the baseline area first)".to_string();
        }
    }
    println!();
    println!("== the delta");
    println!("  {delta_line}");

    let mut unmeasured = 0;
    if area != "baseline" && deps > 0 && before == Some(total) {
        if scaffold_said.is_empty() {
            unmeasured = 1;
            println!();
            eprintln!("FAIL: {deps} third-party crate(s) are in the graph, the binary did not");
            eprintln!("      move by one byte, AND the scaffold said nothing when run. That is");
            eprintln!("      a scaffold `main` cannot reach, deleted by lto, not a dependency");
            eprintln!("      that is free. Wire it into a reachable path and re-run.");
        } else {
            delta_line = format!(
                "0 bytes against {}: BELOW THIS INSTRUMENT'S RESOLUTION. The scaffold ran and printed \"{scaffold_said}\", so the candidate is linked and reachable and still moved nothing a padded, stripped, lto'd binary can show.",
                before.unwrap_or(0)
            );
        }
    }

    // The record.
    let mut record = format!(
        "# cargo bloat and the release total, area={area}\n# TODO/deps.md T-0910. One machine, one day.\n\
         date              {date}\nhost kernel       {kernel}rustc             {rustc}\
         cargo             {cargo}target            {TARGET}\n\
         profile           release: lto=true opt-level=z codegen-units=1 strip=symbols panic=abort\n\
         total_bytes {total}\nceiling_bytes {ceiling}\nheadroom_bytes {headroom}\npt_interp {interp}\n\
         third_party_crates {deps}\ninterpose_musl_bytes {musl_bytes}\ninterpose_gnu_bytes {gnu_bytes}\n\
         delta             {delta_line}\nunmeasured        {unmeasured}\n\
         scaffold_said     {scaffold}\nbreakdown         {bloat_status}\n\n",
        date = date.trim(),
        kernel = kernel.trim(),
        rustc = rustc.trim(),
        cargo = cargo.trim(),
        headroom = ceiling.saturating_sub(total),
        scaffold = if scaffold_said.is_empty() { "(nothing)".to_string() } else { scaffold_said.clone() },
    );
    record.push_str("## the dependency declaration this was measured with\n");
    if let Ok(manifest_text) = fs::read_to_string(root.join("Cargo.toml")) {
        let mut in_section = false;
        for line in manifest_text.split('\n') {
            if line.starts_with("[workspace.dependencies]") {
                in_section = true;
                continue;
            }
            if in_section {
                if line.starts_with('[') || line.trim().is_empty() {
                    if line.starts_with('[') {
                        break;
                    }
                    continue;
                }
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.is_empty() {
                    continue;
                }
                record.push_str(line);
                record.push('\n');
            }
        }
    }
    record.push_str("\n## the scaffold `main` reached\n");
    if let Ok(lib) = fs::read_to_string(root.join("crates/podbox-image/src/lib.rs")) {
        let mut in_scaffold = false;
        for line in lib.split('\n') {
            if line.contains("sweep_scaffold") {
                in_scaffold = true;
            }
            if in_scaffold {
                record.push_str(line);
                record.push('\n');
                if line == "}" {
                    break;
                }
            }
        }
    }
    record.push_str("\n## cargo bloat -n 20\n");
    if bloat_status.starts_with("taken") {
        if let Ok(table) = fs::read_to_string(&bloat_txt) {
            record.push_str(&table);
        }
    } else {
        record.push_str(&format!("(not taken: {bloat_status})\n"));
    }
    if fs::write(&out, record).is_err() {
        fail("SKIP: cannot write the record");
    }
    println!();
    println!(
        "written to {}",
        out.strip_prefix(&root).unwrap_or(&out).display()
    );

    if root.join("scripts/common/result-diff.sh").is_file() {
        println!();
        println!("== against the committed reading");
        let _ = capture(
            "sh",
            &["scripts/common/result-diff.sh", &out.to_string_lossy()],
            &root,
            &[],
            60,
        );
    }

    if total >= ceiling {
        println!();
        eprintln!("FAIL: {total} bytes is at or over the ceiling declared in the stub.");
        eprintln!("      Raise it deliberately, there, with the delta that justifies it");
        eprintln!("      committed beside the change. TODO/deps.md T-0910.");
        std::process::exit(1);
    }
    if unmeasured == 1 {
        std::process::exit(1);
    }
    if bloat_status.starts_with("not taken:") {
        std::process::exit(2);
    }
    std::process::exit(0);
}
