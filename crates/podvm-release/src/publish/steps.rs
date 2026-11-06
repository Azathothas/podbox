use super::*;

pub(super) fn parse_json(text: &str) -> Option<Json> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        pos: 0,
    };
    let value = parser.value()?;
    parser.skip();
    if parser.pos == parser.bytes.len() {
        Some(value)
    } else {
        None
    }
}

// ----------------------------------------------------------------- driver ---

// The script's error taxonomy: a failed expectation is exit 1, and
// anything the machine refused is exit 2. A JSON value of an unexpected
// shape is a failed expectation about the answer received.
enum Problem {
    Failed(String),
    CouldNotRun(String),
}

fn workflow(root: &Path, name: &str, commit: &str, branch: &str) -> Result<(), Problem> {
    let runs = command(
        &[
            "gh",
            "run",
            "list",
            "--repo",
            REPO,
            "--workflow",
            name,
            "--limit",
            "30",
            "--json",
            "headSha,headBranch,status,conclusion,url",
        ],
        root,
        120,
    )
    .map_err(Problem::CouldNotRun)?;
    let parsed = parse_json(&runs).ok_or(Problem::Failed(format!(
        "the exact-commit workflow is unreadable: {name}"
    )))?;
    let items = parsed.as_array().ok_or(Problem::Failed(format!(
        "the exact-commit workflow is unreadable: {name}"
    )))?;
    let mut matching = None;
    for run in items {
        if run.get("headSha").and_then(Json::as_str) == Some(commit)
            && run.get("headBranch").and_then(Json::as_str) == Some(branch)
        {
            matching = Some(run);
            break;
        }
    }
    let run = matching.ok_or(Problem::Failed(format!(
        "the exact-commit workflow is not complete: {name}"
    )))?;
    if run.get("status").and_then(Json::as_str) != Some("completed") {
        return Err(Problem::Failed(format!(
            "the exact-commit workflow is not complete: {name}"
        )));
    }
    let conclusion = run
        .get("conclusion")
        .and_then(Json::as_str)
        .unwrap_or("none");
    let url = run.get("url").and_then(Json::as_str).unwrap_or("");
    println!("{name}: {conclusion} {url}");
    if conclusion != "success" {
        return Err(Problem::Failed(format!(
            "the exact-commit workflow failed: {name}"
        )));
    }
    Ok(())
}

// The script matches the whole tag against `vN.N.N-beta.N`.
pub(super) fn valid_release(tag: &str) -> bool {
    // Split the `-beta.N` tail first: a naive dot split counts the beta
    // number as a fourth version part and refuses every release tag.
    let mut tag_parts = tag.split("-beta.");
    let version = tag_parts.next().unwrap_or("");
    let beta = tag_parts.next().unwrap_or("");
    if tag_parts.next().is_some() {
        return false;
    }
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or("");
    let minor = parts.next().unwrap_or("");
    let patch = parts.next().unwrap_or("");
    if parts.next().is_some() {
        return false;
    }
    if !major.starts_with('v') || major.len() < 2 || !major[1..].bytes().all(|b| b.is_ascii_digit())
    {
        return false;
    }
    if minor.is_empty() || !minor.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    !patch.is_empty()
        && patch.bytes().all(|b| b.is_ascii_digit())
        && !beta.is_empty()
        && beta.bytes().all(|b| b.is_ascii_digit())
}

/// The architecture matrix from the tagged workflow recipe: one `arch:`
/// per line, unique. `re.findall(r'^\s+- arch:\s*(\S+)\s*$', recipe, re.M)`.
pub(super) fn recipe_arches(recipe: &str) -> Option<Vec<String>> {
    let mut arches = Vec::new();
    for line in recipe.split('\n') {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("- arch:") {
            let arch = rest.trim();
            if arch.is_empty() || arch.contains(char::is_whitespace) {
                return None;
            }
            arches.push(arch.to_string());
        }
    }
    if arches.is_empty() {
        return None;
    }
    let mut unique = arches.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != arches.len() {
        return None;
    }
    Some(arches)
}

fn date_iso(root: &Path) -> String {
    let (tx, rx) = mpsc::channel();
    let mut child_process = Command::new("date");
    child_process
        .arg("+%Y-%m-%dT%H:%M:%S%z")
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let child = match child_process.spawn() {
        Ok(child) => child,
        Err(_) => return "-".to_string(),
    };
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(output)) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "-".to_string(),
    }
}

pub(super) fn run(root: &Path, release: Option<&str>, deleted_branch: Option<&str>) -> i32 {
    println!("== conditions");
    println!("{}", date_iso(root));
    println!("Scope: remote refs, workflow conclusions, and complete asset names.");
    println!("Signature content verification uses scripts/verify-release.sh separately.");
    let outcome = check(root, release, deleted_branch);
    match outcome {
        Ok(()) => {
            println!("verdict PUBLICATION-OK");
            0
        }
        Err(Problem::Failed(message)) => {
            println!("FAIL: {message}");
            1
        }
        Err(Problem::CouldNotRun(message)) => {
            println!("cannot run: {message}");
            2
        }
    }
}

fn check(root: &Path, release: Option<&str>, deleted_branch: Option<&str>) -> Result<(), Problem> {
    let remote =
        command(&["git", "remote", "get-url", "origin"], root, 60).map_err(Problem::CouldNotRun)?;
    if remote != format!("https://github.com/{REPO}.git")
        && remote != format!("https://github.com/{REPO}")
    {
        return Err(Problem::Failed(
            "origin differs from the approved HTTPS remote".to_string(),
        ));
    }
    let commit = command(&["git", "rev-parse", "HEAD"], root, 60).map_err(Problem::CouldNotRun)?;
    println!("local commit: {commit}");
    let remote_head = command(
        &["git", "ls-remote", "--heads", "origin", "refs/heads/main"],
        root,
        120,
    )
    .map_err(Problem::CouldNotRun)?;
    let first = remote_head.split_whitespace().next().unwrap_or("");
    if first.is_empty() || first != commit {
        return Err(Problem::Failed(
            "remote main differs from local HEAD".to_string(),
        ));
    }
    workflow(root, "gate.yml", &commit, "main")?;
    if let Some(branch) = deleted_branch {
        let refs = command(
            &[
                "git",
                "ls-remote",
                "--heads",
                "origin",
                &format!("refs/heads/{branch}"),
            ],
            root,
            120,
        )
        .map_err(Problem::CouldNotRun)?;
        if !refs.trim().is_empty() {
            return Err(Problem::Failed(
                "obsolete branch is still present".to_string(),
            ));
        }
        println!("deleted branch absent: {branch}");
    }
    if let Some(tag) = release {
        if !valid_release(tag) {
            return Err(Problem::Failed("release tag format differs".to_string()));
        }
        let tag_commit = command(
            &["git", "rev-parse", &format!("{tag}^{{commit}}")],
            root,
            60,
        )
        .map_err(Problem::CouldNotRun)?;
        println!("release commit: {tag_commit}");
        workflow(root, "nightly.yml", &tag_commit, tag)?;
        let release_text = command(
            &[
                "gh",
                "release",
                "view",
                tag,
                "--repo",
                REPO,
                "--json",
                "tagName,assets,url,isPrerelease",
            ],
            root,
            120,
        )
        .map_err(Problem::CouldNotRun)?;
        let release_json = parse_json(&release_text).ok_or(Problem::Failed(
            "the release listing is unreadable".to_string(),
        ))?;
        let recipe = command(
            &[
                "git",
                "show",
                &format!("{tag}:.github/workflows/nightly.yml"),
            ],
            root,
            60,
        )
        .map_err(Problem::CouldNotRun)?;
        let arches = recipe_arches(&recipe).ok_or(Problem::Failed(
            "cannot read a unique architecture matrix".to_string(),
        ))?;
        let assets_value = release_json.get("assets").ok_or(Problem::Failed(
            "the release listing is unreadable".to_string(),
        ))?;
        let assets = assets_value.as_array().ok_or(Problem::Failed(format!(
            "release assets are not an array: {}",
            assets_value.describe()
        )))?;
        let mut names = std::collections::BTreeMap::new();
        for asset in assets {
            let name = asset
                .get("name")
                .and_then(Json::as_str)
                .ok_or(Problem::Failed(
                    "the release listing is unreadable".to_string(),
                ))?;
            let size = match asset.get("size") {
                Some(Json::Number(value)) => value.clone(),
                _ => String::new(),
            };
            names.insert(name.to_string(), size);
        }
        let mut expected = std::collections::BTreeSet::new();
        let prefix = identity::asset_prefix(tag);
        for arch in &arches {
            for stem in [
                format!("{prefix}-{arch}"),
                format!("{prefix}-ssh-{arch}.tar.gz"),
            ] {
                expected.insert(stem.clone());
                expected.insert(format!("{stem}.sha256"));
                expected.insert(format!("{stem}.sigstore"));
            }
        }
        let tag_name = release_json
            .get("tagName")
            .and_then(Json::as_str)
            .unwrap_or("");
        let prerelease = release_json
            .get("isPrerelease")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        if tag_name != tag || !prerelease {
            return Err(Problem::Failed(
                "release identity or pre-release state differs".to_string(),
            ));
        }
        let missing: Vec<String> = expected
            .iter()
            .filter(|name| {
                names
                    .get(name.as_str())
                    .map(|size| size.is_empty() || size == "0")
                    .unwrap_or(true)
            })
            .map(|name| name.to_string())
            .collect();
        if !missing.is_empty() {
            return Err(Problem::Failed(format!(
                "missing or empty assets: {}",
                missing.join(", ")
            )));
        }
        let url = release_json.get("url").and_then(Json::as_str).unwrap_or("");
        println!(
            "release: {url}; {} architectures; {} required non-empty assets",
            arches.len(),
            expected.len()
        );
    }
    Ok(())
}
