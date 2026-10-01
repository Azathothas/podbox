//! podbox-plant: break each of the gate's checks on purpose and assert red.
//!
//! A behaviour-preserving port of the retired `plant.sh` harness (T-1554).
//! The gate reports success identically whether its assertions passed or
//! whether it examined nothing, and this binary tells those apart: it plants
//! one defect per check into tracked files, asserts the gate goes red with
//! that defect's own message, then restores the tree from a copy.
//!
//! Read the finding, not the exit code. A gate already red for another reason
//! exits 1 either way, so every case asserts that the planted defect's own
//! message appears and that it did not appear on the clean tree.
//!
//! Four guards, each of which exists because the harness shape without it
//! reports success while planting nothing:
//!
//!   1. THE MUTATION MUST LAND. A rewrite that matches nothing leaves the
//!      source correct, the gate green, and the case reads as a missing
//!      assertion when what is missing is the plant. Asserted by hashing the
//!      files before and after.
//!   2. RESTORE FROM A COPY, NEVER A CHECKOUT. Checkout restores from the
//!      index, so anything a plant staged survives it, and a run started
//!      while work is staged restores the plant instead of the source.
//!      Measured: with a plant staged, restoring one file from the index
//!      leaves the plant in the worktree.
//!   3. ONE FILE LIST. A restore that carries its own second copy of the list
//!      stops putting back the first file a new case learns to touch, and
//!      every later case then measures the leftovers. Everything any case may
//!      touch is named in FILES once, and backup and restore iterate it.
//!   4. CONTROLS ARE COUNTED APART FROM PLANTS. A control stays quiet; a
//!      plant goes red. Adding them together reports more defects caught
//!      than were.
//!
//! Exit: 0 every plant was caught and every control stayed quiet, 1 a plant
//! was missed or a control fired, 2 could not run.
//!
//! Arguments are parsed by hand; the crate carries no dependencies. The
//! binary takes no arguments and extras are ignored, exactly as the script
//! ignored them. The gate under test is the `podbox-gate` binary beside this
//! binary's own path, which is `./target/release/podbox-gate` in CI after
//! the todo job's build step copies it there.
//!
//! Two deliberate divergences from the script, both recorded rather than
//! silent. The script located the checkout from its own path, with a special
//! case for a `/in/job.sh` delivery; a binary is never delivered that way,
//! so this binary walks up from its own directory for `TODO/INDEX.md`, the
//! same walk the gate and the counter use. The script ran the gate and the
//! test tools with no bound; this binary bounds every child process (gate
//! 300 s, cargo 900 s) and a bound that fires reads as could-not-run.
//!
//! T-1579's pins live beside the ported cases: one case per wave-2 source
//! and per batch-4 test file. Those cases drive `cargo test`, not the gate:
//! each inserts a failing assertion naming its own file into that file's own
//! test, asserts the run goes red with that message, then restores. With
//! `PODBOX_PLANT_TRANSCRIPT` set to a path, each of those red runs is
//! recorded to that file as evidence, truncated fresh at the start of the
//! run; unset, the binary prints only the
//! verdict lines, exactly like the script. The transcript path must not name
//! a FILES entry. It is written after each case restores, so it never moves
//! the hashes guard 1 compares.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

// Guard 3: ONE LIST. Everything any case may touch is named here once, and
// both the backup and the restore iterate this and nothing else. The wave-2
// sources and the batch-4 test files joined this list under D-3 (T-1578):
// each T-1579 pin mutates its file, so the dirty-tree guard and the
// backup/restore must cover it, the same as every older case's file.
const FILES: &[&str] = &[
    "TODO/INDEX.md",
    "TODO/PROGRESS.md",
    "TODO/probe.md",
    "TODO/enter.md",
    "TODO/RULES.md",
    "TODO/reference-map.md",
    "README.md",
    "docs/conventions/prose.md",
    "docs/runtime-state.md",
    "experiments/110-bloat-delta.sh",
    "experiments/results/bloat-baseline.txt",
    "experiments/results/bloat-image.txt",
    "experiments/results/bloat-interpose.txt",
    "experiments/360-perf-harness.sh",
    "experiments/perf-ceilings.tsv",
    "experiments/results/perf-lane.txt",
    "experiments/results/perf-seeds.tsv",
    ".github/workflows/gate.yml",
    "crates/podbox-supervise/src/lib.rs",
    "crates/podbox-cli/src/parity.rs",
    "crates/podbox-cli/src/run.rs",
    "scripts/build-interpose.sh",
    "crates/podbox-gate/src/dev.rs",
    "crates/podbox-gate/src/interpose_build.rs",
    "crates/podbox-cli/src/images.rs",
    "crates/podbox-cli/src/system.rs",
    "crates/podbox-complete/src/write.rs",
    "crates/podbox-interpose/src/memo.rs",
    "crates/podbox-interpose/src/identity.rs",
    "crates/podbox-probe/src/exit.rs",
    "crates/podbox-cli/tests/curated_refusals.rs",
    "crates/podbox-cli/tests/detached_stdio.rs",
    "crates/podbox-cli/tests/qol.rs",
    "crates/podbox-cli/tests/store_gates.rs",
    "crates/podbox-enter/tests/tty_refusal.rs",
    "crates/podbox-image/tests/acquisition.rs",
    "crates/podbox-image/tests/across_distributions.rs",
    "crates/podbox-image/tests/common/mod.rs",
    "crates/podbox-image/tests/common/registry.rs",
    "crates/podbox-image/tests/parallel_layers.rs",
    "crates/podbox-image/tests/registry_fixture.rs",
    "crates/podbox-image/tests/space_precheck.rs",
    "crates/podbox-image/tests/store_digest.rs",
    "crates/podbox-probe/tests/namespace.rs",
    "crates/podbox-probe/tests/namespace_base.rs",
];

const GATE_TIMEOUT_SECS: u64 = 300;
const CARGO_TIMEOUT_SECS: u64 = 900;

struct ChildOut {
    code: Option<i32>,
    combined: String,
}

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

fn gate_path() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;
    let name = format!("podbox-gate{}", env::consts::EXE_SUFFIX);
    Some(dir.join(name))
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match fs::metadata(path) {
            Ok(meta) => meta.permissions().mode() & 0o111 != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

// Every child process is bounded. A bound that fires reads as could-not-run;
// the caller turns that into exit 2. Standard output and standard error are
// concatenated in that order; substring matching cannot tell the orders
// apart, which is all the verdicts below need.
fn run_bounded(
    program: &str,
    args: &[String],
    cwd: &Path,
    extra_env: &[(String, String)],
    timeout_secs: u64,
) -> Result<ChildOut, String> {
    let mut cmd = Command::new(program);
    cmd.args(args).current_dir(cwd);
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start {program}: {e}"))?;
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stdout = String::from_utf8_lossy(&buf).into_owned();
                }
                if let Some(mut pipe) = child.stderr.take() {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stderr = String::from_utf8_lossy(&buf).into_owned();
                }
                let mut combined = stdout;
                combined.push_str(&stderr);
                return Ok(ChildOut {
                    code: status.code(),
                    combined,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{program} timed out after {timeout_secs}s"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("could not wait for {program}: {e}")),
        }
    }
}

fn git(root: &Path, args: &[&str]) -> Result<ChildOut, String> {
    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run_bounded("git", &owned, root, &[], 120)
}

struct Plant {
    root: PathBuf,
    gate: PathBuf,
    backup: PathBuf,
    base_out: String,
    plants_caught: u32,
    plants_missed: u32,
    controls_quiet: u32,
    controls_fired: u32,
    transcript: Option<PathBuf>,
}

impl Plant {
    fn backup_all(&self) -> Result<(), String> {
        for file in FILES {
            let dest = self.backup.join(file);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
            }
            fs::copy(self.root.join(file), &dest)
                .map_err(|e| format!("could not back up {file}: {e}"))?;
        }
        Ok(())
    }

    // Restore undoes every kind of plant, not only an edit to a file in the
    // list. A case that stages a scratch file must unstage it too, or the
    // leftover makes every later case measure a dirty tree. The duplicate
    // experiment script is found by name rather than passed in: only the
    // plant creates that name, so finding it here cannot touch real work,
    // and a run killed mid-case still scrubs on the next restore.
    fn restore_all(&self, scratch: &str) {
        for file in FILES {
            let _ = fs::copy(self.backup.join(file), self.root.join(file));
        }
        let _ = git(
            &self.root,
            &[
                "rm",
                "-q",
                "--cached",
                "-f",
                "--ignore-unmatch",
                "-r",
                scratch,
            ],
        );
        let _ = fs::remove_dir_all(self.root.join(scratch));
        if let Ok(dir) = fs::read_dir(self.root.join("experiments")) {
            for entry in dir.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with("-plantdup.sh") {
                    let rel = format!("experiments/{name}");
                    let _ = git(
                        &self.root,
                        &["rm", "-q", "--cached", "-f", "--ignore-unmatch", &rel],
                    );
                    let _ = fs::remove_file(self.root.join(&rel));
                }
            }
        }
    }

    // Hash the whole working state, not just the listed files. A plant that
    // creates a new file changes nothing in the list, so a list-only hash
    // would report "did not land" on a plant that did.
    fn hashes(&self) -> String {
        let mut out = String::new();
        for file in FILES {
            match git(&self.root, &["hash-object", file]) {
                Ok(child) => out.push_str(child.combined.trim_end()),
                Err(_) => out.push_str("missing"),
            }
            out.push('\n');
        }
        match git(&self.root, &["status", "--porcelain"]) {
            Ok(child) => out.push_str(&child.combined),
            Err(_) => out.push_str("no-status"),
        }
        out
    }

    fn run_gate(&self, extra_env: &[(String, String)]) -> Result<ChildOut, String> {
        let gate = self.gate.to_string_lossy().into_owned();
        run_bounded(&gate, &[], &self.root, extra_env, GATE_TIMEOUT_SECS)
    }

    // A plant case: the expected substring is the planted defect's own
    // message. A case that matched only the exit code would pass on any
    // unrelated failure.
    fn case_plant(&mut self, name: &str, want: &str, scratch: &str, mutate: &dyn Fn() -> bool) {
        let before = self.hashes();
        mutate();
        let after = self.hashes();
        // Guard 1: did the mutation land?
        if before == after {
            println!("  MISS {name:<34} the mutation did not land; the files are byte-identical");
            self.plants_missed += 1;
            self.restore_all(scratch);
            return;
        }
        let (rc, out) = match self.run_gate(&[]) {
            Ok(child) => (child.code.unwrap_or(1), child.combined),
            Err(e) => {
                println!("  MISS {name:<34} the gate could not run: {e}");
                self.plants_missed += 1;
                self.restore_all(scratch);
                return;
            }
        };
        self.restore_all(scratch);
        if rc == 0 {
            println!("  MISS {name:<34} the gate stayed green");
            self.plants_missed += 1;
            return;
        }
        // The finding, not the exit code: this message, and not on the clean
        // tree.
        if out.contains(want) {
            if self.base_out.contains(want) {
                println!("  MISS {name:<34} that message is already on the clean tree");
                self.plants_missed += 1;
            } else {
                println!("  ok   {name:<34} red, and it named it");
                self.plants_caught += 1;
            }
        } else {
            println!("  MISS {name:<34} red for another reason, not this one");
            for line in out.lines().skip(1).take(3) {
                println!("         {line}");
            }
            self.plants_missed += 1;
        }
    }

    // A control changes something the gate must not object to. It proves the
    // gate is discriminating rather than merely noisy.
    fn case_control(&mut self, name: &str, scratch: &str, mutate: &dyn Fn()) {
        mutate();
        let (rc, out) = match self.run_gate(&[]) {
            Ok(child) => (child.code.unwrap_or(1), child.combined),
            Err(e) => {
                println!("  FIRE {name:<34} the gate could not run: {e}");
                self.controls_fired += 1;
                self.restore_all(scratch);
                return;
            }
        };
        self.restore_all(scratch);
        if rc == 0 {
            println!("  ok   {name:<34} quiet, as a control must be");
            self.controls_quiet += 1;
        } else {
            println!("  FIRE {name:<34} the gate objected to a legitimate edit");
            for line in out.lines().skip(1).take(3) {
                println!("         {line}");
            }
            self.controls_fired += 1;
        }
    }
}

// Small rewrites, each mirroring one shell substitution exactly: without a
// `g` flag the shell replaces the first match on every line, so these map
// over lines and replace once per line.
fn sub_first_per_line(text: &str, find: &str, repl: &str) -> String {
    text.split('\n')
        .map(|line| {
            if line.contains(find) {
                line.replacen(find, repl, 1)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_file(root: &Path, file: &str) -> String {
    fs::read_to_string(root.join(file)).unwrap_or_default()
}

fn write_file(root: &Path, file: &str, text: &str) {
    let _ = fs::write(root.join(file), text);
}

fn append_file(root: &Path, file: &str, text: &str) {
    let mut old = read_file(root, file);
    old.push_str(text);
    write_file(root, file, &old);
}

// The first line starting with `prefix` gets that prefix swapped, once.
fn swap_first_prefixed_line(root: &Path, file: &str, prefix: &str, replacement: &str) {
    let text = read_file(root, file);
    let mut done = false;
    let out: Vec<String> = text
        .split('\n')
        .map(|line| {
            if !done && line.starts_with(prefix) {
                done = true;
                format!("{replacement}{}", &line[prefix.len()..])
            } else {
                line.to_string()
            }
        })
        .collect();
    write_file(root, file, &out.join("\n"));
}

// A bare `name <digits>` reading line gets a new value, whole-line
// anchored like the shell substitution. A line carrying anything past the
// digits is left alone: planting into the wrong shape is the wrong defect.
fn set_bare_reading(root: &Path, file: &str, name: &str, value: &str) {
    let text = read_file(root, file);
    let head = format!("{name} ");
    let out: Vec<String> = text
        .split('\n')
        .map(|line| {
            if let Some(rest) = line.strip_prefix(&head) {
                if !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()) {
                    return format!("{head}{value}");
                }
            }
            line.to_string()
        })
        .collect();
    write_file(root, file, &out.join("\n"));
}

// The planted citations are assembled at run time, never written out here.
// The gate reads this file like any other, so a literal bad citation in
// this source is a permanent gate failure rather than a plant. The same
// holds for the exit-code declaration below and for both ceiling numbers:
// each is read out of the file that owns it, at run time.
// The past-EOF citation names the gate's own source, read at run time.
// The retired script is gone, so citing it reports "does not exist"
// rather than the over-long citation these cases must plant.
fn gate_source_name() -> String {
    ["crates/podbox-gate/src/main", ".rs"].concat()
}

fn bad_bare_name() -> String {
    ["TODO/no-such-", "category.md"].concat()
}

fn exit_declaration() -> String {
    ["pub", " const EXIT_", "RUNTIME_ERROR: i32 = 125;"].concat()
}

fn ceiling_from_file(root: &Path, file: &str, name: &str) -> Option<String> {
    for line in read_file(root, file).split('\n') {
        if let Some(rest) = line.strip_prefix(name) {
            if let Some(digits) = rest.strip_prefix('=') {
                if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                    return Some(digits.to_string());
                }
            }
        }
    }
    None
}

// The first numbered experiment script, alphabetically, like the shell glob.
fn taken_experiment(root: &Path) -> Option<String> {
    let dir = fs::read_dir(root.join("experiments")).ok()?;
    let mut names: Vec<String> = Vec::new();
    for entry in dir.flatten() {
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    for name in names {
        if !name.ends_with(".sh") {
            continue;
        }
        let stem = &name[..name.len() - 3];
        if let Some(dash) = stem.find('-') {
            let (num, rest) = stem.split_at(dash);
            if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) && rest.len() > 1 {
                return Some(num.to_string());
            }
        }
    }
    None
}

// The cargo config names a wrapper program; that program says which
// bootstrap component installs it. Neither word is written out here, for the
// same reason the ceiling is not: naming it here would be a second
// declaration of the value the check exists to keep in one place.
fn cc_component(root: &Path) -> Option<(String, String)> {
    let config = read_file(root, ".cargo/config.toml");
    let mut wrapper: Option<String> = None;
    for line in config.split('\n') {
        let trimmed = line.trim_start();
        if !(trimmed.starts_with("CC_")) {
            continue;
        }
        let Some(eq) = line.find('=') else {
            continue;
        };
        let Some(open) = line[eq..].find('{') else {
            continue;
        };
        if line[eq..eq + open].contains('}') {
            continue;
        }
        let after = &line[eq + open..];
        let Some(value_at) = after.find('"') else {
            continue;
        };
        let rest = &after[value_at + 1..];
        let Some(end) = rest.find('"') else {
            continue;
        };
        wrapper = Some(rest[..end].to_string());
        break;
    }
    let wrapper = wrapper?;
    let body = read_file(root, &wrapper);
    if body.is_empty() {
        return None;
    }
    let marker = "bootstrap-env.sh";
    let mut at = 0;
    while let Some(found) = body[at..].find(marker) {
        let mut pos = at + found + marker.len();
        let bytes = body.as_bytes();
        let base = at + found;
        if pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
            while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                pos += 1;
            }
            let start = pos;
            if pos < bytes.len() && bytes[pos].is_ascii_lowercase() {
                pos += 1;
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_lowercase()
                        || bytes[pos].is_ascii_digit()
                        || bytes[pos] == b'-')
                {
                    pos += 1;
                }
                return Some((wrapper, body[start..pos].to_string()));
            }
        }
        at = base + 1;
    }
    None
}

// Strip the component from every bootstrap line, the way the shell's
// unguided substitution does: the longest match wins, so the last ` comp`
// on the line goes.
fn strip_component_line(line: &str, component: &str) -> String {
    let needle = format!(" {component}");
    if let Some(boot) = line.find("bootstrap-env.sh") {
        if let Some(rel) = line[boot..].rfind(&needle) {
            let at = boot + rel;
            let mut out = line.to_string();
            out.replace_range(at..at + needle.len(), "");
            return out;
        }
    }
    line.to_string()
}

fn run() -> i32 {
    let root = match find_root() {
        Some(root) => root,
        None => {
            eprintln!("SKIP: cannot locate the repository root");
            return 2;
        }
    };
    let gate = match gate_path() {
        Some(gate) => gate,
        None => {
            eprintln!("SKIP: cannot locate the gate binary beside this binary");
            return 2;
        }
    };
    if !is_executable(&gate) {
        eprintln!("SKIP: {} is not executable", gate.display());
        return 2;
    }
    if run_bounded("git", &["--version".to_string()], &root, &[], 30).is_err() {
        eprintln!("SKIP: no git");
        return 2;
    }

    // Refuse to start over staged work. The restore is safe, but a dirty
    // index means the clean baseline below is not clean, and every case then
    // compares against a tree somebody was mid-edit on.
    let file_args: Vec<String> = FILES.iter().map(|f| f.to_string()).collect();
    let mut diff_args = vec!["diff".to_string(), "--quiet".to_string(), "--".to_string()];
    diff_args.extend(file_args.clone());
    let work_dirty = run_bounded("git", &diff_args, &root, &[], 60)
        .map(|out| out.code.unwrap_or(1) != 0)
        .unwrap_or(true);
    let mut cached_args = vec![
        "diff".to_string(),
        "--cached".to_string(),
        "--quiet".to_string(),
        "--".to_string(),
    ];
    cached_args.extend(file_args);
    let index_dirty = run_bounded("git", &cached_args, &root, &[], 60)
        .map(|out| out.code.unwrap_or(1) != 0)
        .unwrap_or(true);
    if work_dirty || index_dirty {
        eprintln!("SKIP: one of the files this script plants into has uncommitted changes:");
        let mut status_args = vec![
            "status".to_string(),
            "--porcelain".to_string(),
            "--".to_string(),
        ];
        status_args.extend(FILES.iter().map(|f| f.to_string()));
        if let Ok(out) = run_bounded("git", &status_args, &root, &[], 60) {
            eprintln!("{}", out.combined.trim_end());
        }
        eprintln!("Commit or stash them. This script must own the tree while it runs.");
        return 2;
    }

    let pid = std::process::id();
    let backup = env::temp_dir().join(format!("podbox-plant-{pid}"));
    if fs::create_dir_all(&backup).is_err() {
        eprintln!("SKIP: cannot create a backup directory");
        return 2;
    }
    let scratch = "experiments/.plantscratch";

    let mut plant = Plant {
        root: root.clone(),
        gate,
        backup: backup.clone(),
        base_out: String::new(),
        plants_caught: 0,
        plants_missed: 0,
        controls_quiet: 0,
        controls_fired: 0,
        transcript: match env::var("PODBOX_PLANT_TRANSCRIPT") {
            Ok(value) if !value.trim().is_empty() => {
                let path = PathBuf::from(value.trim());
                Some(if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                })
            }
            _ => None,
        },
    };
    if plant.backup_all().is_err() {
        eprintln!("SKIP: cannot back up the listed files");
        let _ = fs::remove_dir_all(&backup);
        return 2;
    }

    println!("== baseline: the gate on the clean tree");
    let (base_rc, base_out) = match plant.run_gate(&[]) {
        Ok(child) => (child.code.unwrap_or(1), child.combined),
        Err(e) => {
            eprintln!("SKIP: the gate could not run: {e}");
            plant.restore_all(scratch);
            let _ = fs::remove_dir_all(&backup);
            return 2;
        }
    };
    println!("  exit {base_rc}");
    if base_rc != 0 {
        eprintln!("SKIP: the gate is already red. Nothing below would be interpretable.");
        for line in base_out.lines() {
            eprintln!("    {line}");
        }
        plant.restore_all(scratch);
        let _ = fs::remove_dir_all(&backup);
        return 2;
    }
    println!();
    plant.base_out = base_out;

    // The needles are assembled or read here, before the first case. The
    // gate reads this source like any other tracked file, so none of these
    // values may appear in it literally.
    let gate_source = gate_source_name();
    let past_eof = read_file(&root, &gate_source).split('\n').count() + 1;
    let bad_a = format!("{gate_source}:{past_eof}");
    let bad_b = format!("{gate_source}:{past_eof}");
    let bad_bare = bad_bare_name();
    let ceiling = match ceiling_from_file(&root, "experiments/110-bloat-delta.sh", "CEILING_BYTES")
    {
        Some(value) => value,
        None => {
            eprintln!("SKIP: experiments/110-bloat-delta.sh declares no CEILING_BYTES");
            plant.restore_all(scratch);
            let _ = fs::remove_dir_all(&backup);
            return 2;
        }
    };
    let interpose_ceiling = match ceiling_from_file(
        &root,
        "scripts/build-interpose.sh",
        "INTERPOSE_CEILING_BYTES",
    ) {
        Some(value) => value,
        None => {
            eprintln!("SKIP: scripts/build-interpose.sh declares no INTERPOSE_CEILING_BYTES");
            plant.restore_all(scratch);
            let _ = fs::remove_dir_all(&backup);
            return 2;
        }
    };
    let taken = match taken_experiment(&root) {
        Some(num) => num,
        None => {
            eprintln!("SKIP: experiments/ carries no numbered script to collide with");
            plant.restore_all(scratch);
            let _ = fs::remove_dir_all(&backup);
            return 2;
        }
    };
    let dup_plant = format!("experiments/{taken}-plantdup.sh");

    // Fresh evidence per run: truncate the transcript before the first case,
    // when the caller asked for one. The write is best-effort and happens
    // outside every case, so it never moves the hashes guard 1 compares and
    // never touches the verdict. Unset, the binary prints only the verdict
    // lines, exactly like the script.
    if let Some(path) = plant.transcript.clone() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, "");
    }

    println!("== plants");

    plant.case_plant(
        "1 row without an entry",
        "has a row and no entry",
        scratch,
        &|| {
            append_file(
                &root,
                "TODO/INDEX.md",
                "| [T-9999](probe.md) | P0 | probe | open | A row naming nothing |\n",
            );
            true
        },
    );

    plant.case_plant(
        "2 entry without a row",
        "has an entry and no row",
        scratch,
        &|| {
            append_file(
                &root,
                "TODO/probe.md",
                "\n---\n\n### T-9998 An entry no row names\n\nStatus:      open\n",
            );
            true
        },
    );

    // Status-agnostic, for the same reason cases 4 and 10 are count-agnostic.
    // A plant must mutate whatever status is there.
    plant.case_plant(
        "3 status disagreement",
        "disagrees with the index",
        scratch,
        &|| {
            let text = read_file(&root, "TODO/probe.md");
            let mut from = String::new();
            for line in text.split('\n') {
                if let Some(rest) = line.strip_prefix("Status:      ") {
                    let word: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_lowercase())
                        .collect();
                    if !word.is_empty() {
                        from = word;
                        break;
                    }
                }
            }
            if from.is_empty() {
                return false;
            }
            let to = if from == "blocked" { "open" } else { "blocked" };
            let text = text.split('\n').collect::<Vec<_>>();
            let mut done = false;
            let out: Vec<String> = text
                .into_iter()
                .map(|line| {
                    if !done && line.starts_with(&format!("Status:      {from}")) {
                        done = true;
                        format!(
                            "Status:      {to}{}",
                            &line["Status:      ".len() + from.len()..]
                        )
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(&root, "TODO/probe.md", &out.join("\n"));
            true
        },
    );

    // Count-agnostic. A plant must mutate whatever number is there.
    plant.case_plant(
        "4 count block is stale",
        "the totals line is not the rows",
        scratch,
        &|| {
            let text = read_file(&root, "TODO/INDEX.md");
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    let mut parts = line.splitn(2, " items: ");
                    if let (Some(head), Some(tail)) = (parts.next(), parts.next()) {
                        if !head.is_empty()
                            && head.bytes().all(|b| b.is_ascii_digit())
                            && tail.contains(" open")
                        {
                            let mut tail_parts = tail.splitn(2, ' ');
                            if let (Some(_), Some(rest)) = (tail_parts.next(), tail_parts.next()) {
                                return format!("{head} items: 999 {rest}");
                            }
                        }
                    }
                    line.to_string()
                })
                .collect();
            write_file(&root, "TODO/INDEX.md", &out.join("\n"));
            true
        },
    );

    plant.case_plant(
        "5 a missing field",
        "has no `Decision:` field",
        scratch,
        &|| {
            let text = read_file(&root, "TODO/probe.md");
            let mut done = false;
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    if !done && line.starts_with("Decision:") {
                        done = true;
                        format!("Decisionx:{}", &line["Decision:".len()..])
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(&root, "TODO/probe.md", &out.join("\n"));
            true
        },
    );

    // Every mention, not the first row. Each tree is named twice in the map,
    // so deleting one row leaves the other and the check never loses it.
    plant.case_plant(
        "6 a corpus tree the map lost",
        "is on disk and the map does not name it",
        scratch,
        &|| {
            let text = read_file(&root, "TODO/reference-map.md");
            let out = text.replace("`references/qaidvoid__onelf`", "the onelf tree");
            write_file(&root, "TODO/reference-map.md", &out);
            true
        },
    );

    plant.case_plant("7 a citation past end of file", "lines", scratch, &|| {
        append_file(
            &root,
            "TODO/probe.md",
            &format!("\nSee `{bad_a}` for this.\n"),
        );
        true
    });

    plant.case_plant(
        "8 a TODO link to nothing",
        "does not resolve",
        scratch,
        &|| {
            append_file(
                &root,
                "TODO/probe.md",
                "\nSee [nothing](no-such-file.md).\n",
            );
            true
        },
    );

    plant.case_plant(
        "9 a T-NNNN naming nothing",
        "which is not an entry",
        scratch,
        &|| {
            append_file(&root, "TODO/probe.md", "\nBlocked behind T-8888.\n");
            true
        },
    );

    plant.case_plant(
        "10 PROGRESS count is stale",
        "the count line is not the rows",
        scratch,
        &|| {
            let text = read_file(&root, "TODO/PROGRESS.md");
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    let mut parts = line.splitn(2, " entries: ");
                    if let (Some(head), Some(tail)) = (parts.next(), parts.next()) {
                        if !head.is_empty()
                            && head.bytes().all(|b| b.is_ascii_digit())
                            && tail.contains(" open")
                        {
                            let mut tail_parts = tail.splitn(2, ' ');
                            if let (Some(_), Some(rest)) = (tail_parts.next(), tail_parts.next()) {
                                return format!("{head} entries: 999 {rest}");
                            }
                        }
                    }
                    line.to_string()
                })
                .collect();
            write_file(&root, "TODO/PROGRESS.md", &out.join("\n"));
            true
        },
    );

    plant.case_plant("11 a tree citation past EOF", "lines", scratch, &|| {
        append_file(
            &root,
            "README.md",
            &format!("\nSee `{bad_b}` for the gate.\n"),
        );
        true
    });

    plant.case_plant(
        "12 a link out of README",
        "does not resolve",
        scratch,
        &|| {
            append_file(&root, "README.md", "\n[gone](docs/no-such-page.md)\n");
            true
        },
    );

    plant.case_plant(
        "13 a tenth dangling link",
        "dangling link",
        scratch,
        &|| {
            append_file(
                &root,
                "docs/conventions/prose.md",
                "\n[gone](../../scripts/no-such-template-script.sh)\n",
            );
            true
        },
    );

    plant.case_plant(
        "14 a bare path naming nothing",
        "git tracks no such file",
        scratch,
        &|| {
            append_file(
                &root,
                "README.md",
                &format!("\nThe work is in `{bad_bare}`.\n"),
            );
            true
        },
    );

    // Case 15 stages a file inside a scratch directory and must force-add it,
    // because the ignore rule is what stops it in normal use.
    plant.case_plant(
        "15 tracked experiment scratch",
        "build artefact or experiment scratch",
        scratch,
        &|| {
            let dir = root.join(scratch);
            let _ = fs::create_dir_all(&dir);
            let _ = fs::write(dir.join("x.bin"), b"");
            let _ = git(&root, &["add", "-f", &format!("{scratch}/x.bin")]);
            true
        },
    );

    plant.case_plant(
        "17a the ceiling in a second file",
        "names the binary size ceiling",
        scratch,
        &|| {
            append_file(
                &root,
                "README.md",
                &format!("\nThe release binary must stay under {ceiling} bytes.\n"),
            );
            true
        },
    );

    plant.case_plant(
        "17b a baseline over the ceiling",
        "at or over the ceiling",
        scratch,
        &|| {
            set_bare_reading(
                &root,
                "experiments/results/bloat-baseline.txt",
                "total_bytes",
                &ceiling,
            );
            true
        },
    );

    plant.case_plant(
        "17c a baseline with no total",
        "carries no `total_bytes <n>` line",
        scratch,
        &|| {
            swap_first_prefixed_line(
                &root,
                "experiments/results/bloat-baseline.txt",
                "total_bytes ",
                "total_bytes_renamed ",
            );
            true
        },
    );

    plant.case_plant(
        "17d a shipped reading over the ceiling",
        "at or over the ceiling",
        scratch,
        &|| {
            set_bare_reading(
                &root,
                "experiments/results/bloat-image.txt",
                "total_bytes",
                &ceiling,
            );
            true
        },
    );

    // No backticks in 18a's planted line, deliberately: in backticks it is
    // also a bare path naming nothing, another check fires too, and the case
    // would pass on the wrong message.
    plant.case_plant(
        "18a a Prove at a taken number",
        "Give the new one a free number",
        scratch,
        &|| {
            append_file(
                &root,
                "README.md",
                &format!(
                    "\nThe loop is driven by experiments/{taken}-plantdup.sh, once it exists.\n"
                ),
            );
            true
        },
    );

    plant.case_plant(
        "18b a second script on disk",
        "Give the new one a free number",
        scratch,
        &|| {
            let _ = fs::write(root.join(&dup_plant), "#!/bin/sh\n# a plant\n");
            let _ = git(&root, &["add", &dup_plant]);
            true
        },
    );
    // 18b's staged file is removed by every restore, which finds the
    // duplicate name rather than taking it as a parameter.
    let code = continue_run(&mut plant, &root, scratch, &interpose_ceiling);
    plant.restore_all(scratch);
    let _ = fs::remove_dir_all(&backup);
    code
}

impl Plant {
    // A T-1579 pin: the file's own test, broken on purpose. The mutation
    // inserts a failing assertion naming the file as the test's first
    // statement, so the test panics before any fixture matters and the
    // failure names the file. The run must go red naming both the test and
    // the planted message; red for any other reason is a miss, exactly like
    // a gate case that reddens on the wrong message.
    #[allow(clippy::too_many_arguments)]
    fn case_rust_test(
        &mut self,
        name: &str,
        file: &str,
        test: &str,
        cargo: &[String],
        extra_env: &[(String, String)],
        scratch: &str,
        mutate: &dyn Fn(),
    ) {
        let before = self.hashes();
        mutate();
        let after = self.hashes();
        if before == after {
            println!("  MISS {name:<34} the mutation did not land; the files are byte-identical");
            self.plants_missed += 1;
            self.restore_all(scratch);
            return;
        }
        let marker = format!("plant {file}");
        let (rc, out) = match run_bounded("cargo", cargo, &self.root, extra_env, CARGO_TIMEOUT_SECS)
        {
            Ok(child) => (child.code.unwrap_or(1), child.combined),
            Err(e) => {
                println!("  MISS {name:<34} the test could not run: {e}");
                self.plants_missed += 1;
                self.restore_all(scratch);
                return;
            }
        };
        self.restore_all(scratch);
        // The marker is constructed, not observed: no clean tree can carry
        // `plant <file>`, so there is no clean-tree arm to check. The test
        // name arm stays: a red run that names neither is red for another
        // reason.
        if rc != 0 && out.contains(test) && out.contains(&marker) {
            println!("  ok   {name:<34} red, and it named it");
            self.plants_caught += 1;
        } else if rc == 0 {
            println!("  MISS {name:<34} the test stayed green");
            self.plants_missed += 1;
        } else {
            println!("  MISS {name:<34} red for another reason, not this one");
            for line in out.lines().skip(1).take(3) {
                println!("         {line}");
            }
            self.plants_missed += 1;
        }
        // The transcript is evidence, not behaviour: it is written after the
        // restore, so it never moves the hashes above, and only when the
        // caller asked for it by name.
        if let Some(path) = self.transcript.clone() {
            let mut section = format!(
                "--- {name} {file} ---\ntest: {test}\nmutation: insert \
                 `assert!(false, \"{marker}\");` into the test\ncommand: cargo {}\nexit: {rc}\noutput:\n",
                cargo.join(" ")
            );
            section.push_str(&out);
            if !out.ends_with('\n') {
                section.push('\n');
            }
            section.push_str("restored from the backup copy\n\n");
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut old = fs::read_to_string(&path).unwrap_or_default();
            old.push_str(&section);
            let _ = fs::write(&path, old);
        }
    }
}

fn continue_run(plant: &mut Plant, root: &Path, scratch: &str, interpose_ceiling: &str) -> i32 {
    // The component is read out of the wrapper at run time, like the
    // ceilings. Naming it here would be a second declaration of the value
    // this check exists to keep in one place.
    let (cc_wrapper, cc_component) = match cc_component(root) {
        Some(pair) => pair,
        None => {
            eprintln!("SKIP: the cargo config names no readable toolchain wrapper");
            return 2;
        }
    };
    let _ = cc_wrapper;

    // The build tool's own name is never written out here, in a case title
    // or in a pattern: this harness is itself a script the check reads, so
    // that name here would read as this harness running it.
    plant.case_plant(
        "19a a build job without its toolchain",
        "does not carry",
        scratch,
        &|| {
            let text = read_file(root, ".github/workflows/gate.yml");
            let out: Vec<String> = text
                .split('\n')
                .map(|line| strip_component_line(line, &cc_component))
                .collect();
            write_file(root, ".github/workflows/gate.yml", &out.join("\n"));
            true
        },
    );

    plant.case_plant(
        "19b the same, reached by a script",
        "runs cargo (through",
        scratch,
        &|| {
            // Both direct build spellings go: the workspace musl line and
            // the gate-crate `-p` lines the wave-4 ports added. What remains
            // reaches cargo only through scripts/build-interpose.sh, which is
            // the arm this case exists to catch.
            let text = read_file(root, ".github/workflows/gate.yml");
            let out: Vec<String> = text
                .split('\n')
                .filter(|line| {
                    !line.contains("--release --target")
                        && !line.contains("cargo build --release -p")
                })
                .map(|line| strip_component_line(line, &cc_component))
                .collect();
            write_file(root, ".github/workflows/gate.yml", &out.join("\n"));
            true
        },
    );

    // The needle is assembled at run time. The check reads every crate
    // source, so a literal copy of that declaration written here would be a
    // second one the day the check's scope widens to this directory.
    let exit_decl = exit_declaration();
    plant.case_plant(
        "20 a second exit-code declaration",
        "already holds docker",
        scratch,
        &|| {
            append_file(
                root,
                "crates/podbox-supervise/src/lib.rs",
                &format!("\n{exit_decl}\n"),
            );
            true
        },
    );

    plant.case_plant(
        "21a an unqualified image in a Prove line",
        "unqualified image reference",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    if line.starts_with("Prove:")
                        && line.contains("public.ecr.aws/docker/library/alpine:3.20")
                    {
                        line.replacen(
                            "public.ecr.aws/docker/library/alpine:3.20",
                            "alpine:latest",
                            1,
                        )
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(root, "TODO/probe.md", &out.join("\n"));
            true
        },
    );

    plant.case_plant(
        "21b a docker.io image in a Prove line",
        "which pulls from Docker Hub",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    if line.starts_with("Prove:")
                        && line.contains("public.ecr.aws/docker/library/alpine:3.20")
                    {
                        line.replacen(
                            "public.ecr.aws/docker/library/alpine:3.20",
                            "docker.io/library/alpine:latest",
                            1,
                        )
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(root, "TODO/probe.md", &out.join("\n"));
            true
        },
    );

    // The plant is global on purpose rather than aimed at one entry: any
    // single entry named here would rot the way cases 3, 4 and 10 did.
    plant.case_plant(
        "22 a closed entry with no recorded run",
        "closes without a recorded run",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    if let Some(rest) = line.strip_prefix("**Done") {
                        format!("*Done{rest}")
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(root, "TODO/probe.md", &out.join("\n"));
            true
        },
    );

    plant.case_plant(
        "23a an interpose object over the ceiling",
        "at or over the interpose ceiling",
        scratch,
        &|| {
            set_bare_reading(
                root,
                "experiments/results/bloat-interpose.txt",
                "interpose_gnu_bytes",
                interpose_ceiling,
            );
            true
        },
    );

    plant.case_plant(
        "23b an interpose reading with no sizes",
        "carries no `interpose_gnu_bytes",
        scratch,
        &|| {
            swap_first_prefixed_line(
                root,
                "experiments/results/bloat-interpose.txt",
                "interpose_gnu_bytes ",
                "interpose_gnu_bytes_renamed ",
            );
            true
        },
    );

    // The needle is a token pair, and the plant removes one half of it.
    // T-1557 moved the comparison into the binary: the mutation breaks the
    // `nm` spelling there, which the export check reads. The old
    // build-interpose.sh target no longer carries the needle, so planting
    // there lands nowhere (guard 1 caught exactly that).
    plant.case_plant(
        "24 the export comparison removed",
        "carries no export-set comparison",
        scratch,
        &|| {
            let text = read_file(root, "crates/podbox-gate/src/interpose_build.rs");
            let out = text.replace("--defined-only", "--defined-onlx");
            write_file(root, "crates/podbox-gate/src/interpose_build.rs", &out);
            true
        },
    );

    // The arm, retargeted. A step that exits 2 then reaches no arm that
    // names it, and the holding check must go red rather than pass a gate
    // that stays silent about skips. T-1555 ported the shell driver to
    // dev.rs: the mutation moves the match's `2 =>` arm, which the devcheck
    // reads, off its status.
    plant.case_plant(
        "25 the SKIP arm retargeted",
        "has no SKIP arm",
        scratch,
        &|| {
            let text = read_file(root, "crates/podbox-gate/src/dev.rs");
            let needle = "\n            2 => {";
            let Some(at) = text.find(needle) else {
                return false;
            };
            let mut out = text.to_string();
            out.replace_range(at + 1..at + needle.len(), "9 => {");
            write_file(root, "crates/podbox-gate/src/dev.rs", &out);
            true
        },
    );

    plant.case_plant(
        "26a a done Prove naming a refused flag",
        "Prove names `--network`",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out = sub_first_per_line(
                &text,
                "`! podbox run --network=none --rm",
                "`podbox run --network=none --rm",
            );
            write_file(root, "TODO/probe.md", &out);
            true
        },
    );

    plant.case_plant(
        "26b a done Prove naming an unlisted flag",
        "Prove names `--frobnicate`",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out = sub_first_per_line(
                &text,
                "podbox probe --strict",
                "podbox probe --strict --frobnicate",
            );
            write_file(root, "TODO/probe.md", &out);
            true
        },
    );

    plant.case_plant(
        "26c a done Prove running a refused verb",
        "refuses outright",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out = sub_first_per_line(&text, "podbox probe --strict", "podbox pause");
            write_file(root, "TODO/probe.md", &out);
            true
        },
    );

    plant.case_plant(
        "26d a done Prove running an unknown verb",
        "is no verb",
        scratch,
        &|| {
            let text = read_file(root, "TODO/probe.md");
            let out =
                sub_first_per_line(&text, "podbox probe --strict", "podbox frobnicate --strict");
            write_file(root, "TODO/probe.md", &out);
            true
        },
    );

    plant.case_plant(
        "26e a done Prove with a bad cluster member",
        "Prove names `-Z`",
        scratch,
        &|| {
            append_file(
                root,
                "TODO/probe.md",
                "\n---\n\n### T-9998 Cluster plant\n\nSource: plant\nCategory: probe\nPriority: P3\nEffort: S\nStatus: done\n\nProblem: plant\nPremise: plant\nApproach: plant\nDecision: plant\nProve: `podbox ps -aZ`\n\n**Done.** plant.\n",
            );
            true
        },
    );

    plant.case_plant(
        "27a a parity note leaning on a shipped milestone",
        "leans on M4",
        scratch,
        &|| {
            let text = read_file(root, "crates/podbox-cli/src/parity.rs");
            let out = sub_first_per_line(
                &text,
                "says which half failed (TODO/cli.md T-1331)",
                "needs a supervisor, which is M4",
            );
            write_file(root, "crates/podbox-cli/src/parity.rs", &out);
            true
        },
    );

    plant.case_plant(
        "27b a parity note missing a present verb",
        "claims `prune`",
        scratch,
        &|| {
            let text = read_file(root, "crates/podbox-cli/src/parity.rs");
            let out = sub_first_per_line(
                &text,
                "events are not implemented",
                "events, prune and df are not implemented",
            );
            write_file(root, "crates/podbox-cli/src/parity.rs", &out);
            true
        },
    );

    plant.case_plant(
        "27c a curated flag with no parity row",
        "names `--read-only`",
        scratch,
        &|| {
            let text = read_file(root, "crates/podbox-cli/src/parity.rs");
            let out: Vec<String> = text
                .split('\n')
                .filter(|line| !line.contains("--read-only"))
                .map(|line| line.to_string())
                .collect();
            write_file(root, "crates/podbox-cli/src/parity.rs", &out.join("\n"));
            true
        },
    );

    // The plant lands in a printed string on purpose, and the octal bytes
    // travel through one variable: a literal backslash-digit run here would
    // read as a regex back reference under some tools and the mutation would
    // land nowhere. The glyph itself is built from bytes, never written out.
    let glyph = String::from_utf8(vec![0xE2u8, 0x9Bu8, 0x94u8]).unwrap_or_default();
    plant.case_plant(
        "28 a glyph in a printed string",
        "prints U+26D4",
        scratch,
        &|| {
            let text = read_file(root, "crates/podbox-cli/src/run.rs");
            let out = sub_first_per_line(
                &text,
                "refused: do not run any COMMAND",
                &format!("refused: do not run any COMMAND {glyph}"),
            );
            write_file(root, "crates/podbox-cli/src/run.rs", &out);
            true
        },
    );

    plant.case_plant(
        "29 a dropped cleanup procedure",
        "post-task cleanup procedure",
        scratch,
        &|| {
            let text = read_file(root, "TODO/RULES.md");
            let out = sub_first_per_line(
                &text,
                "Post-task cleanup, after every task",
                "Post-task cleanup, someday",
            );
            write_file(root, "TODO/RULES.md", &out);
            true
        },
    );

    // The lane report is external state. Supply one kept session through a
    // tool fixture and assert that the check names it. The fixture changes
    // no repo file. Two vehicles: the extensionless script runs where the OS
    // execs shebangs, and the batch file runs where it does not.
    {
        let mockbin = plant.backup.join("mockbin");
        let _ = fs::create_dir_all(&mockbin);
        let _ = fs::write(
            mockbin.join("wsl-toolkit"),
            "#!/bin/sh\n[ \"$*\" = \"--instance podbox gc --json\" ] || exit 2\nprintf '%s\\n' '{\"schema\":\"wsl-toolkit-cleanup/1\",\"dry_run\":true,\"containers\":null,\"guest_dirs\":null,\"host_dirs\":null,\"sessions\":[\"plant-session\"]}'\n",
        );
        let _ = fs::write(
            mockbin.join("wsl-toolkit.bat"),
            "@echo off\r\nif \"%*\"==\"--instance podbox gc --json\" (\r\n  echo {\"schema\":\"wsl-toolkit-cleanup/1\",\"dry_run\":true,\"containers\":null,\"guest_dirs\":null,\"host_dirs\":null,\"sessions\":[\"plant-session\"]}\r\n  exit /b 0\r\n)\r\nexit /b 2\r\n",
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(
                mockbin.join("wsl-toolkit"),
                fs::Permissions::from_mode(0o755),
            );
        }
        let old_path = env::var("PATH").unwrap_or_default();
        let sep = if cfg!(windows) { ";" } else { ":" };
        let mock_env = vec![(
            "PATH".to_string(),
            format!("{}{}{}", mockbin.display(), sep, old_path),
        )];
        let name = "29b a kept lane session";
        let want = "lane job still kept: plant-session";
        match plant.run_gate(&mock_env) {
            Ok(child) => {
                let rc = child.code.unwrap_or(1);
                if rc != 0 && child.combined.contains(want) && !plant.base_out.contains(want) {
                    println!("  ok   {name:<34} red, and it named it");
                    plant.plants_caught += 1;
                } else {
                    println!("  MISS {name:<34} the gate did not name the kept session");
                    plant.plants_missed += 1;
                }
            }
            Err(e) => {
                println!("  MISS {name:<34} the gate could not run: {e}");
                plant.plants_missed += 1;
            }
        }
    }

    plant.case_plant(
        "30 a perf reading over its ceiling",
        "perf regression",
        scratch,
        &|| {
            let text = read_file(root, "experiments/results/perf-lane.txt");
            let needle = "run.repeat\tloopback\t";
            let out: Vec<String> = text
                .split('\n')
                .map(|line| {
                    if let Some(at) = line.find(needle) {
                        let mut end = at + needle.len();
                        let bytes = line.as_bytes();
                        while end < bytes.len()
                            && (bytes[end].is_ascii_digit() || bytes[end] == b'.')
                        {
                            end += 1;
                        }
                        format!("{}99999.0{}", &line[..at + needle.len()], &line[end..])
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            write_file(root, "experiments/results/perf-lane.txt", &out.join("\n"));
            true
        },
    );

    println!();
    plant.case_plant(
        "31 source state differs",
        "source state differs",
        scratch,
        &|| {
            append_file(root, "docs/runtime-state.md", "\nStale generated state.\n");
            true
        },
    );

    run_pins(plant, root, scratch);

    println!();
    println!("== not planted against");
    println!("  16 coverage floor    no case. Planting it means making a check examine");
    println!("                       nothing, which requires editing the gate's own");
    println!("                       matchers rather than the tree. Every other check's");
    println!("                       counter is asserted non-zero on every run instead,");
    println!("                       and a zero is reported as a failure.");
    println!();
    println!("== controls");

    plant.case_control("an ordinary prose edit", scratch, &|| {
        append_file(
            root,
            "README.md",
            "\nOne more sentence that cites nothing.\n",
        );
    });

    // A citation that resolves: this binary's own first line.
    plant.case_control("a citation that does resolve", scratch, &|| {
        append_file(
            root,
            "README.md",
            "\nThe gate is `crates/podbox-gate/src/plant.rs:1`.\n",
        );
    });

    plant.case_control("a forward reference to a result", scratch, &|| {
        append_file(
            root,
            "README.md",
            "\nIt will land in `experiments/results/not-yet-taken.txt`.\n",
        );
    });

    // A payload-side flag after the image is not the tool's flag.
    plant.case_control("a payload-side flag after the image", scratch, &|| {
        let text = read_file(root, "TODO/enter.md");
        let marker = "sh -c 'echo hi'";
        let mut out = text.clone();
        if let Some(at) = text.find(marker) {
            out.replace_range(at..at + marker.len(), "sh -c 'echo hi' --not-a-podbox-flag");
        }
        write_file(root, "TODO/enter.md", &out);
    });

    println!();
    println!("== verdict");
    println!(
        "  plants   {} caught, {} missed",
        plant.plants_caught, plant.plants_missed
    );
    println!(
        "  controls {} quiet, {} fired",
        plant.controls_quiet, plant.controls_fired
    );
    // Guard 4: these two are reported apart. A single "defects caught"
    // number adds the controls in and overstates the coverage by exactly
    // their count.
    if plant.plants_missed == 0 && plant.controls_fired == 0 {
        println!("  every check that was planted against went red with its own message.");
        0
    } else {
        println!("  see the MISS and FIRE lines above.");
        1
    }
}

// Insert the failing pin as the test's first statement, after every line
// opening that test. Each anchor below names exactly one test in its file,
// verified with grep before the change landed; the pin panics before any
// fixture matters, so the run needs no engine, no network, and no binary.
fn pin_test(root: &Path, file: &str, anchor: &str) {
    let marker = format!("plant {file}");
    let needle = format!("fn {anchor}(");
    let text = read_file(root, file);
    let mut out = Vec::new();
    for line in text.split('\n') {
        out.push(line.to_string());
        if line.contains(&needle) {
            let indent: String = line.chars().take_while(|c| *c == ' ').collect();
            out.push(format!("{indent}    assert!(false, \"{marker}\");"));
        }
    }
    write_file(root, file, &out.join("\n"));
}

// The shared module holds no test of its own, so the pin is a test of its
// own: it lives in the file, fails with the file's name, and runs inside
// every test binary that includes the module.
fn pin_module(root: &Path, file: &str, test: &str) {
    let marker = format!("plant {file}");
    append_file(
        root,
        file,
        &format!("\n#[test]\nfn {test}() {{\n    assert!(false, \"{marker}\");\n}}\n"),
    );
}

fn cargo_lib(pkg: &str, test: &str) -> Vec<String> {
    vec![
        "test".to_string(),
        "-p".to_string(),
        pkg.to_string(),
        "--lib".to_string(),
        test.to_string(),
    ]
}

fn cargo_bin(pkg: &str, bin: &str, test: &str) -> Vec<String> {
    vec![
        "test".to_string(),
        "-p".to_string(),
        pkg.to_string(),
        "--bin".to_string(),
        bin.to_string(),
        test.to_string(),
    ]
}

fn cargo_test(pkg: &str, target: &str, test: &str) -> Vec<String> {
    vec![
        "test".to_string(),
        "-p".to_string(),
        pkg.to_string(),
        "--test".to_string(),
        target.to_string(),
        test.to_string(),
    ]
}

// The interposer is excluded from the workspace, so its tests run through
// its own manifest on the gnu target, which is the gate workflow's own
// command for them.
fn cargo_interpose(test: &str) -> Vec<String> {
    vec![
        "test".to_string(),
        "--manifest-path".to_string(),
        "crates/podbox-interpose/Cargo.toml".to_string(),
        "--target".to_string(),
        "x86_64-unknown-linux-gnu".to_string(),
        test.to_string(),
    ]
}

fn interpose_env() -> Vec<(String, String)> {
    vec![(
        "RUSTFLAGS".to_string(),
        "-C target-feature=-crt-static".to_string(),
    )]
}

// T-1579's pins: one case per wave-2 source and per batch-4 test file.
// Each breaks its file's own test and asserts the failure names the file.
fn run_pins(plant: &mut Plant, root: &Path, scratch: &str) {
    let none: Vec<(String, String)> = Vec::new();

    let test = "every_image_usage_is_plain_ascii";
    plant.case_rust_test(
        "32a images",
        "crates/podbox-cli/src/images.rs",
        test,
        &cargo_bin("podbox-cli", "podbox", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-cli/src/images.rs", test),
    );

    let test = "system_usage_is_plain_ascii";
    plant.case_rust_test(
        "32b system",
        "crates/podbox-cli/src/system.rs",
        test,
        &cargo_bin("podbox-cli", "podbox", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-cli/src/system.rs", test),
    );

    let test = "every_parity_note_is_plain_ascii";
    plant.case_rust_test(
        "32c parity",
        "crates/podbox-cli/src/parity.rs",
        test,
        &cargo_bin("podbox-cli", "podbox", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-cli/src/parity.rs", test),
    );

    let test = "a_write_through_an_absolute_symlink_does_not_escape";
    plant.case_rust_test(
        "32d write",
        "crates/podbox-complete/src/write.rs",
        test,
        &cargo_lib("podbox-complete", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-complete/src/write.rs", test),
    );

    let test = "a_decimal_fd_parses_and_anything_else_does_not";
    plant.case_rust_test(
        "32e memo",
        "crates/podbox-interpose/src/memo.rs",
        test,
        &cargo_interpose(test),
        &interpose_env(),
        scratch,
        &|| pin_test(root, "crates/podbox-interpose/src/memo.rs", test),
    );

    let test = "uid_alone_means_its_own_group";
    plant.case_rust_test(
        "32f identity",
        "crates/podbox-interpose/src/identity.rs",
        test,
        &cargo_interpose(test),
        &interpose_env(),
        scratch,
        &|| pin_test(root, "crates/podbox-interpose/src/identity.rs", test),
    );

    let test = "a_sigkilled_payload_is_137";
    plant.case_rust_test(
        "32g exit",
        "crates/podbox-probe/src/exit.rs",
        test,
        &cargo_lib("podbox-probe", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-probe/src/exit.rs", test),
    );

    let test = "refused_run_flags_exit_125_naming_status_none";
    plant.case_rust_test(
        "33a curated_refusals",
        "crates/podbox-cli/tests/curated_refusals.rs",
        test,
        &cargo_test("podbox-cli", "curated_refusals", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-cli/tests/curated_refusals.rs", test),
    );

    let test = "run_help_names_the_detach_flag";
    plant.case_rust_test(
        "33b detached_stdio",
        "crates/podbox-cli/tests/detached_stdio.rs",
        test,
        &cargo_test("podbox-cli", "detached_stdio", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-cli/tests/detached_stdio.rs", test),
    );

    let test = "logs_tail_with_a_non_count_is_a_flag_error";
    plant.case_rust_test(
        "33c qol",
        "crates/podbox-cli/tests/qol.rs",
        test,
        &cargo_test("podbox-cli", "qol", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-cli/tests/qol.rs", test),
    );

    let test = "empty_store_lists_nothing_and_marks_nothing";
    plant.case_rust_test(
        "33d store_gates",
        "crates/podbox-cli/tests/store_gates.rs",
        test,
        &cargo_test("podbox-cli", "store_gates", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-cli/tests/store_gates.rs", test),
    );

    let test = "refusal_triggers_only_where_the_open_row_is_denied";
    plant.case_rust_test(
        "33e tty_refusal",
        "crates/podbox-enter/tests/tty_refusal.rs",
        test,
        &cargo_test("podbox-enter", "tty_refusal", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-enter/tests/tty_refusal.rs", test),
    );

    let test = "pull_the_fixture_then_pull_it_again_for_up_to_date";
    plant.case_rust_test(
        "33f acquisition",
        "crates/podbox-image/tests/acquisition.rs",
        test,
        &cargo_test("podbox-image", "acquisition", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-image/tests/acquisition.rs", test),
    );

    let test = "the_reference_is_qualified_before_it_is_pulled";
    plant.case_rust_test(
        "33g across_distributions",
        "crates/podbox-image/tests/across_distributions.rs",
        test,
        &cargo_test("podbox-image", "across_distributions", test),
        &none,
        scratch,
        &|| {
            pin_test(
                root,
                "crates/podbox-image/tests/across_distributions.rs",
                test,
            )
        },
    );

    let test = "plant_common_mod_pin";
    plant.case_rust_test(
        "33h common mod",
        "crates/podbox-image/tests/common/mod.rs",
        test,
        &cargo_test("podbox-image", "acquisition", test),
        &none,
        scratch,
        &|| pin_module(root, "crates/podbox-image/tests/common/mod.rs", test),
    );

    let test = "the_fixture_serves_manifest_blobs_and_shutdown";
    plant.case_rust_test(
        "33i common registry",
        "crates/podbox-image/tests/common/registry.rs",
        test,
        &cargo_test("podbox-image", "registry_fixture", test),
        &none,
        scratch,
        &|| {
            pin_test(
                root,
                "crates/podbox-image/tests/common/registry.rs",
                "start",
            )
        },
    );

    let test = "concurrent_pulls_against_one_fixture_all_resolve_to_the_seeded_digest";
    plant.case_rust_test(
        "33j parallel_layers",
        "crates/podbox-image/tests/parallel_layers.rs",
        test,
        &cargo_test("podbox-image", "parallel_layers", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-image/tests/parallel_layers.rs", test),
    );

    let test = "the_fixture_serves_manifest_blobs_and_shutdown";
    plant.case_rust_test(
        "33k registry_fixture",
        "crates/podbox-image/tests/registry_fixture.rs",
        test,
        &cargo_test("podbox-image", "registry_fixture", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-image/tests/registry_fixture.rs", test),
    );

    let test = "require_refuses_short_bytes_and_short_inodes_naming_the_destination";
    plant.case_rust_test(
        "33l space_precheck",
        "crates/podbox-image/tests/space_precheck.rs",
        test,
        &cargo_test("podbox-image", "space_precheck", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-image/tests/space_precheck.rs", test),
    );

    let test = "the_record_and_the_blobs_name_the_served_bytes";
    plant.case_rust_test(
        "33m store_digest",
        "crates/podbox-image/tests/store_digest.rs",
        test,
        &cargo_test("podbox-image", "store_digest", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-image/tests/store_digest.rs", test),
    );

    let test = "lane_shape_selects_below_namespace";
    plant.case_rust_test(
        "33n namespace",
        "crates/podbox-probe/tests/namespace.rs",
        test,
        &cargo_test("podbox-probe", "namespace", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-probe/tests/namespace.rs", test),
    );

    let test = "capable_shape_selects_namespace";
    plant.case_rust_test(
        "33o namespace_base",
        "crates/podbox-probe/tests/namespace_base.rs",
        test,
        &cargo_test("podbox-probe", "namespace_base", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podbox-probe/tests/namespace_base.rs", test),
    );
}

fn main() {
    std::process::exit(run());
}
