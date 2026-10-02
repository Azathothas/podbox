//! podbox-smoke: the release smoke and the gate diagnostics, as a binary.
//!
//! A behaviour-preserving port of two scripts (T-1556):
//! `nightly-smoke.sh` (the per-arch smoke TODO/packaging.md T-1314
//! promises, extended by T-1329) and `398-gate-diagnostics.py`
//! (the failed-gate diagnostic drive TODO/gate.md T-1351 owns).
//! Arguments are parsed by hand; `clap` was ruled out at T-0908. The crate
//! carries no dependencies.
//!
//!   podbox-smoke BINARY TRIPLE [QEMU]   the per-arch smoke, script-compatible
//!   podbox-smoke --diagnostics          the gate-diagnostics drive
//!   podbox-smoke --diagnostics --powershell
//!                                        the same, with the Windows runner too
//!
//! Exit: 0 the smoke is green, 1 it ran and something failed, 2 it could
//! not run. The smoke half keeps the script's six assertion groups, each
//! read from the process that produced it; the diagnostics half keeps the
//! fixture shape (fixed stub checks, a late message after 20 control lines)
//! and the verdict word GATE-DIAGNOSTICS-OK.
//!
//! Documented approximations. The script cleans its scratch through a trap
//! that also fires on a signal; this binary removes the scratch on every
//! return path, which a kill cannot run, exactly like every other ported
//! tool. Waits are bounded by polling the child (90 s per diagnostics run,
//! like the script's timeout), because the standard library offers no
//! blocking wait with a deadline.

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DIAGNOSTIC_CHECKS: &[&str] = &[
    "check-docs",
    "check-markers",
    "check-one-home",
    "check-placeholders",
    "check-control-bytes",
    "check-changelog",
    "check-attribution",
    "check-remote-items",
];
const MARKER: &str = "FIXTURE-LATE-DIAGNOSTIC";
const RUN_TIMEOUT_SECS: u64 = 90;

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

fn which(program: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

struct Run {
    rc: i32,
    stdout: String,
    stderr: String,
}

// A bounded run: poll the child and kill it past the deadline. Pipes are
// drained after exit, the same shape the gate and plant binaries use, and
// adequate here because every driven output is small.
fn run_bounded(
    prog: &str,
    args: &[String],
    cwd: &Path,
    extra_env: &[(String, String)],
    timeout_secs: u64,
) -> Result<Run, String> {
    let mut cmd = Command::new(prog);
    cmd.args(args).current_dir(cwd);
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start {prog}: {e}"))?;
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stdout = String::from_utf8_lossy(&buf).into_owned();
                }
                if let Some(mut pipe) = child.stderr.take() {
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stderr = String::from_utf8_lossy(&buf).into_owned();
                }
                return Ok(Run {
                    rc: status.code().unwrap_or(1),
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{prog} timed out after {timeout_secs}s"));
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("could not wait for {prog}: {e}")),
        }
    }
}

// ------------------------------------------------------------ the smoke half

fn smoke_fail(triple: &str, message: &str) -> ExitCode {
    eprintln!("SMOKE-FAIL {triple}: {message}");
    ExitCode::from(1)
}

fn smoke_unrun(triple: &str, message: &str) -> ExitCode {
    eprintln!("SMOKE-FAIL {triple}: {message}");
    ExitCode::from(2)
}

fn doc_line(doc: &str, key: &str) -> String {
    for line in doc.lines() {
        if let Some((name, value)) = line.split_once(": ") {
            if name == key {
                return value.to_string();
            }
        }
    }
    String::new()
}

fn do_smoke(bin: &str, triple: &str, qemu: &str, root: &Path) -> ExitCode {
    if bin.is_empty() || triple.is_empty() {
        eprintln!("SMOKE-FAIL usage: podbox-smoke BINARY TRIPLE [QEMU]");
        return ExitCode::from(2);
    }
    if !Path::new(bin).is_file() {
        return smoke_unrun(triple, &format!("no binary at {bin}"));
    }
    if !qemu.is_empty() && which(qemu).is_none() {
        return smoke_unrun(triple, &format!("no emulator {qemu} on PATH"));
    }
    // Run the binary under test, directly or through the emulator, with
    // both streams merged the way the script's 2>&1 merged them.
    let invoke = |args: &[&str]| -> Result<Run, String> {
        let mut full: Vec<String> = Vec::new();
        if !qemu.is_empty() {
            full.push(qemu.to_string());
        }
        full.push(bin.to_string());
        for arg in args {
            full.push(arg.to_string());
        }
        let (prog, rest) = full.split_first().unwrap();
        run_bounded(prog, rest, root, &[], 300)
    };

    // Group 1: `version` exits 0 and prints the artefact's version line.
    let first = match invoke(&["version"]) {
        Ok(run) => run,
        Err(e) => return smoke_unrun(triple, &e),
    };
    let out = format!("{}{}", first.stdout, first.stderr);
    let out_trimmed = out.trim_end().to_string();
    if first.rc != 0 {
        return smoke_fail(
            triple,
            &format!("version exits {}, not 0: {out_trimmed}", first.rc),
        );
    }
    if !out_trimmed.starts_with("podbox ") {
        return smoke_fail(
            triple,
            &format!("version prints [{out_trimmed}], not the version line"),
        );
    }

    // Group 2: `version --verbose` names the triple.
    let verbose = match invoke(&["version", "--verbose"]) {
        Ok(run) => run,
        Err(e) => return smoke_unrun(triple, &e),
    };
    if verbose.rc != 0 {
        return smoke_fail(
            triple,
            &format!("version --verbose exits {}, not 0", verbose.rc),
        );
    }
    let doc = format!("{}{}", verbose.stdout, verbose.stderr);
    print!("{doc}");
    if doc_line(&doc, "target") != triple {
        return smoke_fail(
            triple,
            &format!("verbose names target [{}]", doc_line(&doc, "target")),
        );
    }
    if doc_line(&doc, "crt-static") != "yes" {
        return smoke_fail(
            triple,
            &format!(
                "verbose names crt-static [{}], not yes",
                doc_line(&doc, "crt-static")
            ),
        );
    }

    // Groups 3: both interposer digests are 64 lowercase hex digits.
    for key in ["interpose-gnu", "interpose-musl"] {
        let value = doc_line(&doc, key);
        if value.len() != 64 {
            return smoke_fail(
                triple,
                &format!("{key} is [{value}], not a 64-digit digest"),
            );
        }
        if !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return smoke_fail(triple, &format!("{key} is [{value}], not lowercase hex"));
        }
    }

    // Group 4: no PT_INTERP.
    if which("readelf").is_none() {
        return smoke_unrun(
            triple,
            "readelf is not installed, so static-ness is unchecked",
        );
    }
    let elf = match run_bounded(
        "readelf",
        &["-l".to_string(), bin.to_string()],
        root,
        &[],
        60,
    ) {
        Ok(run) => run,
        Err(e) => return smoke_unrun(triple, &e),
    };
    let interp = elf
        .stdout
        .lines()
        .filter(|line| line.contains("INTERP"))
        .count();
    if interp != 0 {
        return smoke_fail(
            triple,
            &format!("{interp} PT_INTERP entries, so the binary is not static"),
        );
    }

    // Group 6: the embedded pair's e_machine beside the artefact's own. A
    // present object with no machine is a build that did not record; an
    // x86_64 artefact embedding anything but 0x3e is the wrong-arch embed
    // this group exists to catch. Off x86_64 the mismatch is the decided
    // decline, printed with both values and expected, never silent.
    if which("od").is_none() {
        return smoke_unrun(triple, "od is not installed, so e_machine is unchecked");
    }
    let bytes = fs::read(bin).unwrap_or_default();
    if bytes.len() < 20 {
        return smoke_unrun(triple, "e_machine unreadable");
    }
    let machine = u16::from_le_bytes([bytes[18], bytes[19]]);
    if machine == 0 {
        return smoke_unrun(triple, "e_machine unreadable");
    }
    let outer = format!("{machine:x}");
    let gnu_m = doc_line(&doc, "interpose-gnu-machine");
    let musl_m = doc_line(&doc, "interpose-musl-machine");
    if gnu_m.is_empty() {
        return smoke_fail(triple, "no interpose-gnu-machine line in version --verbose");
    }
    if musl_m.is_empty() {
        return smoke_fail(
            triple,
            "no interpose-musl-machine line in version --verbose",
        );
    }
    println!("SMOKE-EMBED-MACHINE {triple} outer=0x{outer} gnu={gnu_m} musl={musl_m}");
    if triple.starts_with("x86_64-") {
        if gnu_m != format!("0x{outer}") {
            return smoke_fail(
                triple,
                &format!("gnu embed {gnu_m} is not the artefact 0x{outer}"),
            );
        }
        if musl_m != format!("0x{outer}") {
            return smoke_fail(
                triple,
                &format!("musl embed {musl_m} is not the artefact 0x{outer}"),
            );
        }
    } else if gnu_m == format!("0x{outer}") && musl_m == format!("0x{outer}") {
        println!("SMOKE-EMBED-MACHINE {triple} embeds match the artefact");
    } else {
        println!("SMOKE-EMBED-MACHINE {triple} embeds differ (expected: x86_64-only pair, honest decline stands)");
    }

    // Group 5: the binary pulls and extracts for its actual job, not only
    // versions. A synthetic one-file image travels by save/load through the
    // binary under test; then the tarball's layer bytes are flipped and the
    // same load must refuse. Native legs only.
    if qemu.is_empty() {
        let code = group_five(bin, triple, qemu, root);
        if code != 0 {
            return ExitCode::from(code);
        }
    }

    let size = fs::metadata(bin).map(|meta| meta.len()).unwrap_or(0);
    println!(
        "SMOKE-OK {triple} qemu={} bytes={size}",
        if qemu.is_empty() { "native" } else { qemu }
    );
    ExitCode::from(0)
}

fn bin_run(bin: &str, qemu: &str, root: &Path, store: &Path, args: &[&str]) -> Result<Run, String> {
    let mut full: Vec<String> = Vec::new();
    if !qemu.is_empty() {
        full.push(qemu.to_string());
    }
    full.push(bin.to_string());
    for arg in args {
        full.push(arg.to_string());
    }
    let (prog, rest) = full.split_first().unwrap();
    let store_str = store.to_string_lossy().into_owned();
    run_bounded(
        prog,
        rest,
        root,
        &[("PODBOX_STORE".to_string(), store_str)],
        300,
    )
}

fn group_five(bin: &str, triple: &str, qemu: &str, root: &Path) -> u8 {
    let work = env::temp_dir().join(format!("podbox-smoke-{}", std::process::id()));
    let cleanup = |code: u8| -> u8 {
        let _ = fs::remove_dir_all(&work);
        code
    };
    if fs::create_dir_all(&work).is_err() {
        smoke_unrun(triple, "no temp dir");
        return 2;
    }
    let store = work.join("store");
    let seed = work.join("seed");
    if fs::create_dir_all(&seed).is_err() {
        smoke_unrun(triple, "no seed dir");
        return cleanup(2);
    }
    if fs::write(seed.join("payload.txt"), "smoke-payload\n").is_err() {
        smoke_unrun(triple, "no seed file");
        return cleanup(2);
    }
    let rootfs_tar = work.join("rootfs.tar");
    if run_bounded(
        "tar",
        &[
            "-cf".to_string(),
            rootfs_tar.to_string_lossy().into_owned(),
            "payload.txt".to_string(),
        ],
        &seed,
        &[],
        60,
    )
    .is_err()
    {
        smoke_unrun(triple, "no seed tar");
        return cleanup(2);
    }
    let seed_tar = work.join("seed.tar");
    let evil_tar = work.join("evil.tar");
    let steps: &[(&str, Vec<String>)] = &[
        (
            "import",
            vec![
                bin.to_string(),
                "import".to_string(),
                rootfs_tar.to_string_lossy().into_owned(),
                "smoke:t1".to_string(),
            ],
        ),
        (
            "save",
            vec![
                bin.to_string(),
                "save".to_string(),
                "-o".to_string(),
                seed_tar.to_string_lossy().into_owned(),
                "smoke:t1".to_string(),
            ],
        ),
        (
            "load",
            vec![
                bin.to_string(),
                "load".to_string(),
                "-i".to_string(),
                seed_tar.to_string_lossy().into_owned(),
            ],
        ),
        (
            "extract",
            vec![
                bin.to_string(),
                "extract".to_string(),
                "smoke:t1".to_string(),
            ],
        ),
    ];
    for (name, full) in steps {
        let (prog, rest) = full.split_first().unwrap();
        let store_str = store.to_string_lossy().into_owned();
        match run_bounded(
            prog,
            rest,
            root,
            &[("PODBOX_STORE".to_string(), store_str)],
            300,
        ) {
            Ok(run) if run.rc == 0 => {}
            _ => {
                smoke_fail(triple, &format!("{name} of the seed image exits non-zero"));
                return cleanup(1);
            }
        }
    }
    let inspect = match bin_run(
        bin,
        qemu,
        root,
        &store,
        &["inspect", "--format", "{{.RootfsPath}}", "smoke:t1"],
    ) {
        Ok(run) if run.rc == 0 => run,
        _ => {
            smoke_fail(triple, "inspect of the loaded image exits non-zero");
            return cleanup(1);
        }
    };
    let rootfs = inspect.stdout.trim().to_string();
    let payload = fs::read_to_string(Path::new(&rootfs).join("payload.txt")).unwrap_or_default();
    if payload != "smoke-payload\n" {
        smoke_fail(triple, "payload bytes differ after extract");
        return cleanup(1);
    }
    println!("SMOKE-PULL-EXTRACT-OK {triple} honest leg: pull-by-load plus extract reads the payload bytes");
    if fs::copy(&seed_tar, &evil_tar).is_err() {
        smoke_fail(triple, "no repack copy");
        return cleanup(1);
    }
    // Flip the layer blob inside the store-format tarball, not the outer
    // tar framing: `load` hashes each entry's bytes against the descriptor
    // that names it, so a flipped ENTRY fails closed with a digest
    // mismatch.
    let list = match run_bounded(
        "tar",
        &["-tf".to_string(), seed_tar.to_string_lossy().into_owned()],
        root,
        &[],
        60,
    ) {
        Ok(run) => run.stdout,
        Err(_) => {
            smoke_fail(triple, "no blob entry in the seed tarball");
            return cleanup(1);
        }
    };
    let blob = list
        .lines()
        .find(|line| line.contains("blobs/sha256/"))
        .unwrap_or("")
        .to_string();
    if blob.is_empty() {
        smoke_fail(triple, "no blob entry in the seed tarball");
        return cleanup(1);
    }
    let evil_dir = work.join("evil");
    if fs::create_dir_all(&evil_dir).is_err()
        || run_bounded(
            "tar",
            &[
                "-xf".to_string(),
                seed_tar.to_string_lossy().into_owned(),
                "-C".to_string(),
                evil_dir.to_string_lossy().into_owned(),
            ],
            root,
            &[],
            60,
        )
        .is_err()
    {
        smoke_fail(triple, "no repack dir");
        return cleanup(1);
    }
    let blob_path = evil_dir.join(&blob);
    let mut blob_bytes = fs::read(&blob_path).unwrap_or_default();
    if blob_bytes.len() < 101 {
        smoke_unrun(triple, "no dd");
        return cleanup(2);
    }
    blob_bytes[100] = b'X';
    if fs::write(&blob_path, &blob_bytes).is_err() {
        smoke_unrun(triple, "no dd");
        return cleanup(2);
    }
    if run_bounded(
        "tar",
        &[
            "-cf".to_string(),
            evil_tar.to_string_lossy().into_owned(),
            "oci-layout".to_string(),
            "index.json".to_string(),
            "blobs".to_string(),
        ],
        &evil_dir,
        &[],
        60,
    )
    .is_err()
    {
        smoke_fail(triple, "no repack");
        return cleanup(1);
    }
    let evil = bin_run(
        bin,
        qemu,
        root,
        &store,
        &["load", "-i", &evil_tar.to_string_lossy()],
    );
    match evil {
        Ok(run) if run.rc == 0 => {
            smoke_fail(triple, "load of the flipped tarball exits 0");
            return cleanup(1);
        }
        Err(_) => {
            smoke_unrun(triple, "evil load could not run");
            return cleanup(2);
        }
        _ => {}
    }
    println!("SMOKE-PULL-EXTRACT-OK {triple} flipped leg: the corrupted fixture refuses");
    cleanup(0)
}

// ------------------------------------------------------ the diagnostics half

fn extract_json_int(text: &str, key: &str) -> Option<i64> {
    let needle = format!("\"{key}\"");
    let mut search: &str = text;
    loop {
        let at = search.find(&needle)?;
        let mut rest = search[at + needle.len()..].trim_start();
        if !rest.starts_with(':') {
            search = &search[at + needle.len()..];
            continue;
        }
        rest = rest[1..].trim_start();
        let mut digits = 0;
        let mut negative = false;
        if rest.starts_with('-') {
            negative = true;
            rest = &rest[1..];
        }
        let mut value: i64 = 0;
        for byte in rest.bytes() {
            if !byte.is_ascii_digit() {
                break;
            }
            value = value
                .saturating_mul(10)
                .saturating_add((byte - b'0') as i64);
            digits += 1;
        }
        if digits == 0 {
            search = &search[at + needle.len()..];
            continue;
        }
        return Some(if negative { -value } else { value });
    }
}

fn do_diagnostics(root: &Path, with_powershell: bool) -> ExitCode {
    use std::time::{SystemTime, UNIX_EPOCH};
    println!("== conditions");
    let date = run_bounded(
        "date",
        &["-u".to_string(), "+%Y-%m-%dT%H:%M:%SZ".to_string()],
        root,
        &[],
        30,
    )
    .map(|run| run.stdout.trim().to_string())
    .unwrap_or_else(|_| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "-".to_string())
    });
    println!("{date}");
    let host = run_bounded("uname", &["-s".to_string()], root, &[], 30)
        .map(|run| run.stdout.trim().to_string())
        .unwrap_or_else(|_| "-".to_string());
    let python = run_bounded(
        "python3",
        &[
            "-c".to_string(),
            "import sys; print(sys.version.split()[0])".to_string(),
        ],
        root,
        &[],
        30,
    )
    .map(|run| run.stdout.trim().to_string())
    .unwrap_or_else(|_| "-".to_string());
    println!("host={host} python={python}");
    println!("Inputs: fixed stub-v1 checks; late message after 20 control lines.");
    let shell = which("sh");
    let powershell = which("pwsh");
    if shell.is_none() || (with_powershell && powershell.is_none()) {
        println!("cannot run: required interpreter is absent");
        return ExitCode::from(2);
    }
    let work = env::temp_dir().join(format!("podbox-gate-output-{}", std::process::id()));
    if fs::create_dir_all(&work).is_err() {
        println!("cannot run: required interpreter is absent");
        return ExitCode::from(2);
    }
    let cleanup = |code: ExitCode| -> ExitCode {
        let _ = fs::remove_dir_all(&work);
        code
    };
    for name in DIAGNOSTIC_CHECKS {
        if fs::write(work.join(format!("{name}.sh")), "exit 0\n").is_err()
            || fs::write(work.join(format!("{name}.ps1")), "exit 0\n").is_err()
        {
            println!("cannot run: required interpreter is absent");
            return cleanup(ExitCode::from(2));
        }
    }
    let profiles: &[(&str, bool)] = &[("sh", true), ("ps1", with_powershell)];
    for (kind, enabled) in profiles {
        if !enabled {
            if *kind == "ps1" {
                println!("PowerShell profile: not requested; Windows proof uses --powershell");
            }
            continue;
        }
        let interpreter = if *kind == "sh" {
            shell.clone().unwrap()
        } else {
            powershell.clone().unwrap()
        };
        let source_path = root.join(format!("scripts/common/check-gate.{kind}"));
        let source = match fs::read_to_string(&source_path) {
            Ok(text) => text,
            Err(_) => {
                println!("cannot run: required interpreter is absent");
                return cleanup(ExitCode::from(2));
            }
        };
        let runner = work.join(format!("check-gate.{kind}"));
        // The runner is a copy the fixture may mutate; the tree is never
        // written. Newlines stay LF exactly as the script wrote them.
        if fs::write(&runner, source.replace("\r\n", "\n")).is_err() {
            println!("cannot run: required interpreter is absent");
            return cleanup(ExitCode::from(2));
        }
        let stub = work.join(format!("check-docs.{kind}"));
        let noise = (0..20)
            .map(|_| {
                if *kind == "sh" {
                    "echo control".to_string()
                } else {
                    "Write-Output 'control'".to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let fail_body = if *kind == "sh" {
            format!("{noise}\necho {MARKER}\nexit 1\n")
        } else {
            format!("{noise}\nWrite-Output '{MARKER}'\nexit 1\n")
        };
        if fs::write(&stub, &fail_body).is_err() {
            println!("cannot run: required interpreter is absent");
            return cleanup(ExitCode::from(2));
        }
        let (flags, json_flag, strict) = if *kind == "sh" {
            ("--fast", "--json", "--strict")
        } else {
            ("-Fast", "-Json", "-Strict")
        };
        let prefix: Vec<String> = if *kind == "sh" {
            vec![interpreter.to_string_lossy().into_owned()]
        } else {
            vec![
                interpreter.to_string_lossy().into_owned(),
                "-NoProfile".to_string(),
                "-File".to_string(),
            ]
        };
        let run_with = |extra: &[&str]| -> Result<Run, String> {
            let mut args: Vec<String> = prefix.clone();
            args.push(runner.to_string_lossy().into_owned());
            for flag in extra {
                args.push(flag.to_string());
            }
            let (prog, rest) = args.split_first().unwrap();
            run_bounded(prog, rest, &work, &[], RUN_TIMEOUT_SECS)
        };
        // The late failure is printed, and the JSON carries it as a count.
        match run_with(&[flags]) {
            Ok(run) if run.rc == 1 && run.stdout.contains(MARKER) => {}
            _ => {
                eprintln!("SMOKE-FAIL diagnostics: the late failure did not print on {kind}");
                return cleanup(ExitCode::from(1));
            }
        }
        match run_with(&[flags, json_flag]) {
            Ok(run) => {
                if run.rc != 1 || extract_json_int(&run.stdout, "failed") != Some(1) {
                    eprintln!("SMOKE-FAIL diagnostics: the JSON verdict is wrong on {kind}");
                    return cleanup(ExitCode::from(1));
                }
                if run.stdout.contains(MARKER) {
                    eprintln!("SMOKE-FAIL diagnostics: the marker leaked into JSON on {kind}");
                    return cleanup(ExitCode::from(1));
                }
            }
            Err(e) => {
                eprintln!("SMOKE-FAIL diagnostics: {e}");
                return cleanup(ExitCode::from(1));
            }
        }
        // The lost-output mutation: truncate the failure log and the late
        // message must vanish while the run still fails.
        let mutated = if *kind == "sh" {
            let current = fs::read_to_string(&runner).unwrap_or_default();
            current.replacen(
                "sed 's/^/          /' \"$OUT/log\" ;;",
                "sed 's/^/          /' \"$OUT/log\" | head -12 ;;",
                1,
            )
        } else {
            let current = fs::read_to_string(&runner).unwrap_or_default();
            current.replacen(
                "Get-Content -LiteralPath $logFile -ErrorAction",
                "Get-Content -LiteralPath $logFile -TotalCount 12 -ErrorAction",
                1,
            )
        };
        let current = fs::read_to_string(&runner).unwrap_or_default();
        if mutated == current {
            eprintln!("SMOKE-FAIL diagnostics: mutation did not reach its runner on {kind}");
            return cleanup(ExitCode::from(1));
        }
        if fs::write(&runner, &mutated).is_err() {
            eprintln!("SMOKE-FAIL diagnostics: cannot write the runner on {kind}");
            return cleanup(ExitCode::from(1));
        }
        match run_with(&[flags]) {
            Ok(run) if run.rc == 1 && !run.stdout.contains(MARKER) => {}
            _ => {
                eprintln!("SMOKE-FAIL diagnostics: the truncated run kept the message on {kind}");
                return cleanup(ExitCode::from(1));
            }
        }
        if fs::write(&runner, &current).is_err() {
            eprintln!("SMOKE-FAIL diagnostics: cannot restore the runner on {kind}");
            return cleanup(ExitCode::from(1));
        }
        // The clean control passes, and the unavailable twin skips.
        if fs::write(&stub, "exit 0\n").is_err() {
            eprintln!("SMOKE-FAIL diagnostics: cannot write the stub on {kind}");
            return cleanup(ExitCode::from(1));
        }
        match run_with(&[flags]) {
            Ok(run) if run.rc == 0 && !run.stdout.contains(MARKER) => {}
            _ => {
                eprintln!("SMOKE-FAIL diagnostics: the clean control failed on {kind}");
                return cleanup(ExitCode::from(1));
            }
        }
        if fs::write(work.join("check-twins.sh"), "echo unavailable\nexit 2\n").is_err() {
            eprintln!("SMOKE-FAIL diagnostics: cannot write the twin on {kind}");
            return cleanup(ExitCode::from(1));
        }
        match run_with(&[json_flag]) {
            Ok(run) => {
                if run.rc != 0
                    || extract_json_int(&run.stdout, "skipped") != Some(1)
                    || extract_json_int(&run.stdout, "failed") != Some(0)
                {
                    eprintln!("SMOKE-FAIL diagnostics: the twin did not skip on {kind}");
                    return cleanup(ExitCode::from(1));
                }
            }
            Err(e) => {
                eprintln!("SMOKE-FAIL diagnostics: {e}");
                return cleanup(ExitCode::from(1));
            }
        }
        match run_with(&[json_flag, strict]) {
            Ok(run) => {
                if run.rc != 1 || extract_json_int(&run.stdout, "skipped") != Some(1) {
                    eprintln!("SMOKE-FAIL diagnostics: strict did not fail the skip on {kind}");
                    return cleanup(ExitCode::from(1));
                }
            }
            Err(e) => {
                eprintln!("SMOKE-FAIL diagnostics: {e}");
                return cleanup(ExitCode::from(1));
            }
        }
        println!("ok: {kind} late failure, lost-output mutation, JSON, clean control, and unavailable twin");
    }
    println!("verdict GATE-DIAGNOSTICS-OK");
    cleanup(ExitCode::from(0))
}

fn print_help() {
    println!("podbox-smoke: the per-arch release smoke and the gate diagnostics.");
    println!();
    println!("  podbox-smoke BINARY TRIPLE [QEMU]   version, digests, static-ness,");
    println!("                                      embeds, and pull plus extract");
    println!("  podbox-smoke --diagnostics          the failed-gate diagnostic drive");
    println!("  podbox-smoke --diagnostics --powershell");
    println!("                                      the same, with the Windows runner too");
    println!();
    println!("Exit: 0 green, 1 something failed, 2 it could not run.");
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    if argv.first().map(|s| s.as_str()) == Some("--diagnostics") {
        let root = match find_root() {
            Some(root) => root,
            None => {
                println!("cannot run: required interpreter is absent");
                return ExitCode::from(2);
            }
        };
        let mut powershell = false;
        for arg in &argv[1..] {
            match arg.as_str() {
                "--powershell" => powershell = true,
                _ => {
                    eprintln!("SMOKE-FAIL usage: podbox-smoke --diagnostics [--powershell]");
                    return ExitCode::from(2);
                }
            }
        }
        return do_diagnostics(&root, powershell);
    }
    if argv.first().map(|s| s.as_str()) == Some("-h")
        || argv.first().map(|s| s.as_str()) == Some("--help")
    {
        print_help();
        return ExitCode::from(0);
    }
    if argv.len() > 3 {
        eprintln!("SMOKE-FAIL usage: podbox-smoke BINARY TRIPLE [QEMU]");
        return ExitCode::from(2);
    }
    let root = match find_root() {
        Some(root) => root,
        None => {
            eprintln!("SMOKE-FAIL cannot locate the repository root");
            return ExitCode::from(2);
        }
    };
    let bin = argv.first().cloned().unwrap_or_default();
    let triple = argv.get(1).cloned().unwrap_or_default();
    let qemu = argv.get(2).cloned().unwrap_or_default();
    do_smoke(&bin, &triple, &qemu, &root)
}
