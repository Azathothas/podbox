//! podbox-publish: read exact-commit CI and optional release acceptance.
//!
//! A behaviour-preserving port of experiments/399-publication.py (T-1567).
//! The script stays as a compat shim that execs this binary; the logic
//! lives here. No remote writes: remote refs, workflow conclusions, and
//! complete asset names are read, and signature content verification stays
//! with `podbox-verify`.
//!
//! Usage: podbox-publish [--release TAG] [--deleted-branch NAME] [--root DIR]
//!
//! Exit: 0 the publication reads clean, 1 a check failed, 2 it could not
//! run. `FAIL: ` names a failed expectation; `cannot run: ` names a missing
//! tool, ref, or answer.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const REPO: &str = "Azathothas/podbox";

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

fn command(args: &[&str], cwd: &Path, secs: u64) -> Result<String, String> {
    if args.is_empty() {
        return Err("required command failed: no command".to_string());
    }
    let (tx, rx) = mpsc::channel();
    let mut child_process = Command::new(args[0]);
    child_process
        .args(&args[1..])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    child_process.env("GIT_TERMINAL_PROMPT", "0");
    child_process.env("GH_PROMPT_DISABLED", "1");
    let child = child_process
        .spawn()
        .map_err(|_| format!("required command failed: {}", args[0]))?;
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Err(_) => Err(format!("required command failed: {}", args[0])),
        Ok(Err(_)) => Err(format!("required command failed: {}", args[0])),
        Ok(Ok(output)) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                Err(format!("required command failed: {}", args[0]))
            }
        }
    }
}

// ------------------------------------------------------------ JSON reader ---

#[derive(Debug, Clone)]
enum Json {
    Null,
    Bool(bool),
    Number(String),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }
    fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(value) => Some(*value),
            _ => None,
        }
    }
    fn describe(&self) -> String {
        match self {
            Json::Null => "null".to_string(),
            Json::Bool(value) => format!("boolean {value}"),
            Json::Number(value) => format!("number {value}"),
            Json::Str(value) => format!("string of {} chars", value.len()),
            Json::Array(items) => format!("array of {} items", items.len()),
            Json::Object(pairs) => format!("object of {} pairs", pairs.len()),
        }
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        while self.pos < self.bytes.len() && is_space(self.bytes[self.pos]) {
            self.pos += 1;
        }
    }
    fn value(&mut self) -> Option<Json> {
        self.skip();
        let b = *self.bytes.get(self.pos)?;
        match b {
            b'"' => Some(Json::Str(self.string()?)),
            b'{' => self.object(),
            b'[' => self.array(),
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'n' => self.literal("null", Json::Null),
            b'-' | b'0'..=b'9' => Some(Json::Number(self.number())),
            _ => None,
        }
    }
    fn literal(&mut self, word: &str, value: Json) -> Option<Json> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Some(value)
        } else {
            None
        }
    }
    fn number(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.bytes.len()
            && !matches!(
                self.bytes[self.pos],
                b',' | b']' | b'}' | b' ' | b'\t' | b'\n' | b'\r'
            )
        {
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned()
    }
    fn string(&mut self) -> Option<String> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let b = *self.bytes.get(self.pos)?;
            self.pos += 1;
            match b {
                b'"' => return Some(out),
                b'\\' => {
                    let e = *self.bytes.get(self.pos)?;
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0c}'),
                        b'u' => {
                            let hex = std::str::from_utf8(self.bytes.get(self.pos..self.pos + 4)?)
                                .ok()?;
                            let code = u32::from_str_radix(hex, 16).ok()?;
                            self.pos += 4;
                            if (0xd800..0xdc00).contains(&code) {
                                if self.bytes.get(self.pos..self.pos + 2) == Some(b"\\u") {
                                    let low = std::str::from_utf8(
                                        self.bytes.get(self.pos + 2..self.pos + 6)?,
                                    )
                                    .ok()?;
                                    let low = u32::from_str_radix(low, 16).ok()?;
                                    if (0xdc00..0xe000).contains(&low) {
                                        self.pos += 6;
                                        out.push(char::from_u32(
                                            0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00),
                                        )?);
                                        continue;
                                    }
                                }
                                return None;
                            }
                            out.push(char::from_u32(code)?);
                        }
                        _ => return None,
                    }
                }
                _ => {
                    let rest = std::str::from_utf8(&self.bytes[self.pos - 1..]).ok()?;
                    let ch = rest.chars().next()?;
                    out.push(ch);
                    self.pos += ch.len_utf8() - 1;
                }
            }
        }
    }
    fn object(&mut self) -> Option<Json> {
        self.pos += 1;
        let mut pairs = Vec::new();
        loop {
            self.skip();
            if self.bytes.get(self.pos) == Some(&b'}') {
                self.pos += 1;
                return Some(Json::Object(pairs));
            }
            if self.bytes.get(self.pos) != Some(&b'"') {
                return None;
            }
            let key = self.string()?;
            self.skip();
            if self.bytes.get(self.pos) != Some(&b':') {
                return None;
            }
            self.pos += 1;
            let value = self.value()?;
            pairs.push((key, value));
            self.skip();
            match self.bytes.get(self.pos) {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => continue,
                _ => return None,
            }
        }
    }
    fn array(&mut self) -> Option<Json> {
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip();
            if self.bytes.get(self.pos) == Some(&b']') {
                self.pos += 1;
                return Some(Json::Array(items));
            }
            items.push(self.value()?);
            self.skip();
            match self.bytes.get(self.pos) {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => continue,
                _ => return None,
            }
        }
    }
}

fn parse_json(text: &str) -> Option<Json> {
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
fn valid_release(tag: &str) -> bool {
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
fn recipe_arches(recipe: &str) -> Option<Vec<String>> {
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

fn run(root: &Path, release: Option<&str>, deleted_branch: Option<&str>) -> i32 {
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
        for arch in &arches {
            for stem in [
                format!("podbox-{arch}"),
                format!("podbox-ssh-{arch}.tar.gz"),
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

fn main() {
    let mut release: Option<String> = None;
    let mut deleted_branch: Option<String> = None;
    let mut root_flag: Option<String> = None;
    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        if arg == "--release" {
            release = Some(argv.next().unwrap_or_default());
        } else if arg == "--deleted-branch" {
            deleted_branch = Some(argv.next().unwrap_or_default());
        } else if arg == "--root" {
            root_flag = Some(argv.next().unwrap_or_default());
        } else if let Some(dir) = arg.strip_prefix("--root=") {
            root_flag = Some(dir.to_string());
        } else {
            eprintln!("podbox-publish: unexpected argument: {arg}");
            std::process::exit(2);
        }
    }
    let root = match root_flag {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        Some(_) => {
            eprintln!("podbox-publish: --root needs a directory");
            std::process::exit(2);
        }
        None => repo_root().unwrap_or_else(|| {
            eprintln!("podbox-publish: cannot locate the checkout root");
            std::process::exit(2);
        }),
    };
    // The script's error taxonomy: ValueError is a failed expectation and
    // anything the machine refused is "could not run". A JSON value of an
    // unexpected shape is a failed expectation about the answer received.
    let code = run(&root, release.as_deref(), deleted_branch.as_deref());
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_tags_match_the_script_expression() {
        assert!(valid_release("v0.1.0-beta.12"));
        assert!(valid_release("v10.20.30-beta.1"));
        assert!(!valid_release("v0.1.0"));
        assert!(!valid_release("v0.1.0-beta."));
        assert!(!valid_release("beta.12"));
        assert!(!valid_release("v0.1.0-beta.12 "));
        assert!(!valid_release("v0.1.0-beta.1.2"));
        assert!(!valid_release("not-a-tag"));
    }

    #[test]
    fn recipe_arches_read_one_per_line_and_unique() {
        let recipe = "jobs:\n  build:\n    strategy:\n      matrix:\n        include:\n          - arch: x86_64\n          - arch: aarch64\n";
        assert_eq!(
            recipe_arches(recipe).unwrap(),
            vec!["x86_64".to_string(), "aarch64".to_string()]
        );
        let dup = "          - arch: x86_64\n          - arch: x86_64\n";
        assert!(recipe_arches(dup).is_none());
        assert!(recipe_arches("no matrix here").is_none());
    }

    #[test]
    fn reader_and_shapes_cover_gh_answers() {
        let parsed = parse_json("{\"tagName\": \"v1\", \"isPrerelease\": true, \"assets\": [{\"name\": \"a\", \"size\": 3}]}").unwrap();
        assert_eq!(parsed.get("tagName").and_then(Json::as_str), Some("v1"));
        assert_eq!(
            parsed.get("isPrerelease").and_then(Json::as_bool),
            Some(true)
        );
        assert_eq!(parsed.describe(), "object of 3 pairs");
    }
}
