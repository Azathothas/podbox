use super::*;

fn word_count(path: &Path) -> Option<usize> {
    let text = fs::read_to_string(path).ok()?;
    Some(text.split_whitespace().count())
}

pub(super) fn do_startup(root: &Path, _target: &str) -> ExitCode {
    let cargo_here = Command::new("cargo")
        .arg("--version")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !cargo_here {
        eprintln!("SKIP: cargo is not on PATH");
        return ExitCode::from(2);
    }
    // Checked before writing: a cold target directory is hundreds of
    // megabytes and the writable allowance is fixed while `df` misleads.
    let avail_mb: u64 = capture("df", &["-Pm", &root.to_string_lossy()], root)
        .and_then(|(_, text)| {
            text.lines()
                .nth(1)?
                .split_whitespace()
                .nth(3)?
                .parse::<u64>()
                .ok()
        })
        .unwrap_or(0);
    if avail_mb < 2048 {
        eprintln!(
            "SKIP: {avail_mb}MB free under {}, and a cold target needs more",
            root.display()
        );
        return ExitCode::from(2);
    }
    let work = env::temp_dir().join(format!("podvm-startup-{}", std::process::id()));
    if fs::create_dir_all(&work).is_err() {
        eprintln!("SKIP: no temp dir");
        return ExitCode::from(2);
    }
    let mut report = String::new();
    let mut push = |line: &str| {
        report.push_str(line);
        report.push('\n');
    };
    let date = date_utc();
    let kernel = capture("uname", &["-r"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    let cpus = capture("nproc", &[], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    let rustc = capture("rustc", &["--version"], root)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_else(|| "-".to_string());
    let registry = if env::var("HOME")
        .map(|home| PathBuf::from(home).join(".cargo/registry").is_dir())
        .unwrap_or(false)
    {
        "present, so no crate is downloaded below"
    } else {
        "absent"
    };
    push("== conditions");
    push(&format!("date              {date}"));
    push(&format!("host kernel       {kernel}"));
    push(&format!("cpus              {cpus}"));
    push(&format!("rustc             {rustc}"));
    push(&format!("cargo registry    {registry}"));
    push("");

    // 1. the compile, against an EMPTY target directory. A separate target
    // dir, so the repository's own is not destroyed to take a reading.
    push("== 1. the compile, against an EMPTY target directory");
    let cold = work.join("coldtarget");
    let cold_log = work.join("cold.log");
    let t0 = epoch_secs();
    let cold_out = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "x86_64-unknown-linux-musl",
            "--target-dir",
        ])
        .arg(&cold)
        .current_dir(root)
        .stdin(Stdio::null())
        .output();
    let cold_rc = match &cold_out {
        Ok(output) => {
            let mut combined = output.stdout.clone();
            combined.extend_from_slice(&output.stderr);
            let _ = fs::write(&cold_log, combined);
            output.status.code().unwrap_or(1)
        }
        Err(_) => 127,
    };
    let t1 = epoch_secs();
    let cold_s = t1.saturating_sub(t0);
    let cold_size = capture("du", &["-sh", &cold.to_string_lossy()], root)
        .map(|(_, text)| text.split_whitespace().next().unwrap_or("-").to_string())
        .unwrap_or_else(|| "-".to_string());
    let cold_crates = fs::read_to_string(&cold_log)
        .map(|text| {
            text.lines()
                .filter(|line| line.starts_with("   Compiling"))
                .count()
        })
        .unwrap_or(0);
    push(&format!("  exit              {cold_rc}"));
    push(&format!("  wall clock        {cold_s}s"));
    push(&format!("  target size       {cold_size}"));
    push(&format!("  crates compiled   {cold_crates}"));

    // 2. the same build, warm.
    push("");
    push("== 2. the same build, warm");
    let t0 = epoch_secs();
    let _ = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "x86_64-unknown-linux-musl",
            "--target-dir",
        ])
        .arg(&cold)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let t1 = epoch_secs();
    push(&format!("  wall clock        {}s", t1.saturating_sub(t0)));

    // 3. what the environment costs when it is already there.
    push("");
    push("== 3. what the environment costs when it is already there");
    let t0 = epoch_secs();
    let _ = Command::new("./scripts/common/bootstrap-env.sh")
        .arg("--check")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let t1 = epoch_secs();
    push(&format!("  bootstrap --check {}s", t1.saturating_sub(t0)));
    push("  the INSTALL path is not measured here: this container already has");
    push("    every component, and re-measuring would mean removing them. What a");
    push("    genuinely fresh machine pays for apt and the zig download is");
    push("    not measured rather than estimated.");

    // 4. what a session can read while that happens. Words rather than a
    // guess at a reading speed: the number a reader can check.
    push("");
    push("== 4. what a session can read while that happens");
    let mut words: usize = 0;
    for file in ["TODO/PROGRESS.md", "AGENTS.md"] {
        let count = word_count(&root.join(file)).unwrap_or(0);
        push(&format!("  {file:<22} {count} words"));
        words += count;
    }
    push(&format!("  together              {words} words"));
    push("  That is the reading AGENTS.md opens with, and it needs no");
    push(&format!(
        "    toolchain. podvm-dev start starts the {cold_s}s of clause 1 behind it."
    ));

    // 5. the driver returns immediately rather than blocking.
    push("");
    push("== 5. podvm-dev returns immediately rather than blocking");
    let exe = env::current_exe().unwrap_or_else(|_| root.join("target/release/podvm-dev"));
    let t0 = epoch_secs();
    let _ = Command::new(&exe)
        .arg("start")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let t1 = epoch_secs();
    push(&format!(
        "  podvm-dev start  {}s to return",
        t1.saturating_sub(t0)
    ));
    let _ = Command::new(&exe)
        .arg("wait")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let status_first = Command::new(&exe)
        .arg("status")
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .ok()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .unwrap_or_default();
    push(&format!("  then wait says    {status_first}"));

    print!("{report}");
    let out = root.join("experiments/results/session-startup.txt");
    if let Some(parent) = out.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&out, &report).is_err() {
        eprintln!("podvm-dev: cannot write {}", out.display());
        let _ = fs::remove_dir_all(&work);
        return ExitCode::from(2);
    }
    println!();
    println!("written to experiments/results/session-startup.txt");
    let diff = root.join("scripts/common/result-diff.sh");
    if diff.is_file() {
        let _ = Command::new(diff)
            .arg(&out)
            .current_dir(root)
            .stdin(Stdio::null())
            .status();
    }
    let _ = fs::remove_dir_all(&work);
    ExitCode::from(0)
}
