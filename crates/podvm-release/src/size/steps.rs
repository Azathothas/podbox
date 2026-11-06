use super::*;

/// Run to completion, returning success plus stdout and stderr separately.
/// The build log keeps both streams like the script's `2>&1`; the bloat
/// table keeps stdout alone, because stderr diagnostics are not the
/// breakdown. A nonzero exit still returns its streams: the script reads a
/// partial table the same way.
pub(super) fn run_split(
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

pub(super) fn capture(
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

pub(super) fn repo_root() -> Option<PathBuf> {
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
pub(super) fn read_ceiling(root: &Path) -> u64 {
    let text = fs::read_to_string(root.join(DECLARATION)).unwrap_or_default();
    for line in text.split('\n') {
        let line = line.trim();
        if let Some(digits) = line.strip_prefix(&format!("{DECL_NAME}="))
            && !digits.is_empty()
            && digits.bytes().all(|b| b.is_ascii_digit())
            && let Ok(value) = digits.parse::<u64>()
        {
            return value;
        }
    }
    eprintln!("podvm-size: the ceiling declaration is absent from {DECLARATION}");
    std::process::exit(2);
}

/// The breakdown, from an unstripped build in its own directory: what
/// `cargo bloat` says, printed, and the words the record carries for it.
pub(super) fn breakdown(root: &Path, work: &Path, manifest: &str) -> (String, PathBuf) {
    println!();
    println!("== what is in it");
    let mut bloat_status = String::new();
    let bloat_txt = work.join("bloat.txt");
    if capture("cargo", &["bloat", "--version"], root, &[], 30).is_none() {
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
                manifest,
                "--bin",
                "podvm",
                "-n",
                "20",
            ],
            root,
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
        if bloat_txt.is_file()
            && let Ok(table) = fs::read_to_string(&bloat_txt)
            && !table.trim().is_empty()
        {
            for line in table.split('\n') {
                println!("  {line}");
            }
            bloat_status = "taken from an unstripped build of the release profile".to_string();
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
    (bloat_status, bloat_txt)
}

/// The third-party crates in the normal dependency graph, printed.
pub(super) fn dependency_count(root: &Path, manifest: &str) -> u64 {
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
            manifest,
        ],
        root,
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
            .filter(|line| !line.starts_with("podvm-"))
            .count() as u64;
    }
    println!("  third-party crates in the normal dependency graph: {deps}");
    deps
}
