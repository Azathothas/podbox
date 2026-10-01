//! podbox-buildstate: record build inputs and verify the output bytes.
//!
//! A behaviour-preserving port of scripts/build-state.py (T-1559). The script
//! stays as a compat shim that execs this binary; the logic lives here. The
//! record format is byte-identical: `{"inputs": ..., "outputs": {...},
//! "schema": "podbox-build/1"}` with sorted keys and CPython separators,
//! plus a trailing newline, written atomically through a `.new` sibling.
//!
//! Usage: podbox-buildstate {inputs|record|status|commit}
//!        [--root DIR] [--target TRIPLE] [--binary PATH]
//!
//! Exit: 0 ready (or the value printed), 1 stale, 2 could not run. The
//! messages name the same states the script named.

#[path = "common.rs"]
mod common;

use common::{file_digest, json_string, repo_root, run_captured, sha256_hex};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Compact `{"k": "v", ...}` with keys sorted and CPython's default
/// separators (`", "` between items, `"': '"` after a key).
fn json_object_sorted(map: &BTreeMap<String, String>, out: &mut String) {
    out.push('{');
    for (i, (key, value)) in map.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        json_string(key, out);
        out.push_str(": ");
        json_string(value, out);
    }
    out.push('}');
}

const SCHEMA: &str = "podbox-build/1";
const OUTPUT_NAMES: [&str; 5] = ["podbox", "node", "operator", "proxy", "shell"];
const DEFAULT_TARGET: &str = "x86_64-unknown-linux-musl";

fn fail(message: String) -> ! {
    eprintln!("build-state: cannot measure the build: {message}");
    std::process::exit(2);
}

fn build_commit(root: &Path) -> String {
    if let Ok(pinned) = env::var("PODBOX_BUILD_COMMIT") {
        return pinned;
    }
    let head = match run_captured("git", &["rev-parse", "HEAD"], root, 10) {
        Ok(text) => text.trim().to_string(),
        Err(_) => return "unknown".to_string(),
    };
    match run_captured("git", &["status", "--porcelain"], root, 10) {
        Ok(dirty) if dirty.trim().is_empty() => head,
        Ok(_) => format!("{head}-dirty"),
        // The script used check=True here, so a status failure is exit 2.
        Err(e) => fail(e),
    }
}

/// Hash names and bytes. File timestamps do not establish freshness.
fn input_paths(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut paths = BTreeSet::new();
    for name in ["crates", "vendor", "scripts", ".cargo"] {
        let base = root.join(name);
        if base.is_dir() {
            walk(&base, root, &mut paths)?;
        }
    }
    for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
        if root.join(name).is_file() {
            paths.insert(name.to_string());
        }
    }
    // These bytes are inputs to the CLI build script.
    for libc in ["gnu", "musl"] {
        let name = format!(
            "crates/podbox-interpose/target/x86_64-unknown-linux-{libc}/release/libpodbox_interpose.so"
        );
        if root.join(&name).is_file() {
            paths.insert(name);
        }
    }
    Ok(paths)
}

fn walk(dir: &Path, root: &Path, out: &mut BTreeSet<String>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("cannot list {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot list {}: {e}", dir.display()))?;
        let path = entry.path();
        // The script never follows a symlinked directory: rglob without
        // followlinks. A symlinked file is hashed through its target.
        let kind = fs::symlink_metadata(&path)
            .map_err(|e| format!("cannot stat {}: {e}", path.display()))?;
        if kind.is_dir() {
            walk(&path, root, out)?;
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|e| format!("cannot relativize: {e}"))?;
            // The script drops any path with a target or __pycache__ part.
            if relative
                .components()
                .any(|c| c.as_os_str() == "target" || c.as_os_str() == "__pycache__")
            {
                continue;
            }
            let text = relative
                .to_str()
                .ok_or_else(|| "non-UTF-8 input path".to_string())?;
            out.insert(text.replace('\\', "/"));
        }
    }
    Ok(())
}

fn inputs_digest(root: &Path, target: &str, tools: bool) -> Result<String, String> {
    let mut values = BTreeMap::new();
    for name in input_paths(root)? {
        values.insert(
            name.clone(),
            file_digest(&root.join(&name)).map_err(|e| format!("{}: {e}", name))?,
        );
    }
    let mut conditions = BTreeMap::new();
    conditions.insert("target".to_string(), target.to_string());
    let mut keys = BTreeSet::from([
        "RUSTFLAGS".to_string(),
        "CARGO_ENCODED_RUSTFLAGS".to_string(),
        "CC".to_string(),
        "AR".to_string(),
        "TARGETS".to_string(),
    ]);
    for (key, _) in env::vars() {
        if key.starts_with("CARGO_TARGET_") || key.starts_with("CC_") || key.starts_with("AR_") {
            keys.insert(key);
        }
    }
    for key in keys {
        conditions.insert(key.clone(), env::var(&key).unwrap_or_default());
    }
    if tools {
        conditions.insert("build_commit".to_string(), build_commit(root));
        // The script probes tool versions in its invocation directory (no
        // cwd= argument), not in --root: rustup resolves the toolchain from
        // the working directory, so probing anywhere else records versions
        // the script never saw. Measured in the lane: 1.99.0 under /work,
        // 1.98.1 default elsewhere.
        let here = env::current_dir().unwrap_or_else(|_| root.to_path_buf());
        for (tool, flag) in [
            ("rustc", "--version"),
            ("cargo", "--version"),
            ("zig", "version"),
        ] {
            let text = run_captured(tool, &[flag], &here, 10)?;
            conditions.insert(tool.to_string(), text.trim().to_string());
        }
    }
    let mut values_json = String::new();
    json_object_sorted(&values, &mut values_json);
    let mut conditions_json = String::new();
    json_object_sorted(&conditions, &mut conditions_json);
    Ok(sha256_hex(
        format!("[{values_json}, {conditions_json}]").as_bytes(),
    ))
}

fn outputs_digest(binary: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut digests = BTreeMap::new();
    let dir = binary
        .parent()
        .ok_or_else(|| "binary has no parent".to_string())?;
    for name in OUTPUT_NAMES {
        let path = if name == "podbox" {
            binary.to_path_buf()
        } else {
            dir.join(name)
        };
        match file_digest(&path) {
            Ok(digest) => {
                digests.insert(name.to_string(), digest);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err("missing".to_string());
            }
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
    }
    Ok(digests)
}

fn record_json(inputs: &str, outputs: &BTreeMap<String, String>) -> String {
    let mut out = String::from("{");
    let mut k = String::from("\"inputs\": ");
    json_string(inputs, &mut k);
    out.push_str(&k);
    out.push_str(", \"outputs\": {");
    for (i, (name, digest)) in outputs.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        json_string(name, &mut out);
        out.push_str(": ");
        json_string(digest, &mut out);
    }
    out.push_str("}, \"schema\": ");
    let mut schema = String::new();
    json_string(SCHEMA, &mut schema);
    out.push_str(&schema);
    out.push('}');
    out
}

fn status(root: &Path, target: &str, binary: &Path) -> (String, i32) {
    if !binary.is_file() {
        return ("absent".to_string(), 2);
    }
    let record = root.join(".dev/build-state.json");
    let text = match fs::read_to_string(&record) {
        Ok(text) => text,
        Err(_) => return ("stale: no build record".to_string(), 1),
    };
    // The script loads the record as JSON and refuses a non-dict there.
    // Anything else compares field by field below, in the same order.
    let trimmed = text.trim();
    if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
        return ("stale: unreadable build record".to_string(), 1);
    }
    if field_string(trimmed, "\"schema\"").as_deref() != Some(SCHEMA) {
        return ("stale: build record schema differs".to_string(), 1);
    }
    let current_inputs = match inputs_digest(root, target, true) {
        Ok(digest) => digest,
        Err(e) => fail(e),
    };
    if field_string(trimmed, "\"inputs\"").as_deref() != Some(current_inputs.as_str()) {
        return ("stale: build inputs changed".to_string(), 1);
    }
    let saved_outputs = match field_raw(trimmed, "\"outputs\"").and_then(|raw| parse_strmap(&raw)) {
        Some(map) => map,
        None => return ("stale: binary bytes changed".to_string(), 1),
    };
    match outputs_digest(binary) {
        Err(e) if e == "missing" => ("absent: a workspace executable is missing".to_string(), 2),
        Err(e) => fail(e),
        Ok(current) if current != saved_outputs => ("stale: binary bytes changed".to_string(), 1),
        Ok(_) => ("ready".to_string(), 0),
    }
}

/// The raw JSON value after `"field":`, string-aware so a brace inside a
/// string never counts. None where the field is absent or the value ends
/// before it starts.
fn field_raw(text: &str, field: &str) -> Option<String> {
    let start = text.find(field)? + field.len();
    let rest = text[start..].trim_start();
    let body = rest.strip_prefix(':')?.trim_start();
    let mut chars = body.char_indices();
    let (_, first) = chars.next()?;
    if first == '"' {
        let mut escaped = false;
        for (i, ch) in chars {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                return Some(body[..=i].to_string());
            }
        }
        return None;
    }
    if first == '{' || first == '[' {
        let open = first;
        let close = if open == '{' { '}' } else { ']' };
        let mut depth = 0u32;
        let mut in_string = false;
        let mut escaped = false;
        for (i, ch) in body.char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
            } else if ch == '"' {
                in_string = true;
            } else if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
                if depth == 0 {
                    return Some(body[..=i].to_string());
                }
            }
        }
        return None;
    }
    let end = body.find([',', '}', ']']).unwrap_or(body.len());
    Some(body[..end].trim_end().to_string())
}

/// The field as a JSON string, or None where it is absent or not a string.
fn field_string(text: &str, field: &str) -> Option<String> {
    unescape(&field_raw(text, field)?)
}

/// Parse `{"k": "v", ...}` with string keys and values. The writer only ever
/// stores digests there, so anything else is a changed record, not a parse
/// to argue with.
fn parse_strmap(raw: &str) -> Option<BTreeMap<String, String>> {
    let body = raw.trim().strip_prefix('{')?.strip_suffix('}')?;
    let mut map = BTreeMap::new();
    for (key, value) in split_fields(body)? {
        map.insert(unescape(&key)?, unescape(&value)?);
    }
    Some(map)
}

fn split_fields(body: &str) -> Option<Vec<(String, String)>> {
    let mut fields = Vec::new();
    let mut depth = 0u32;
    let mut in_string = false;
    let mut escaped = false;
    let mut start = 0usize;
    for (i, ch) in body.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' | '[' => depth += 1,
            '}' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                fields.push(parse_pair(body[start..i].trim())?);
                start = i + 1;
            }
            _ => {}
        }
    }
    let tail = body[start..].trim();
    if !tail.is_empty() {
        fields.push(parse_pair(tail)?);
    }
    Some(fields)
}

fn parse_pair(field: &str) -> Option<(String, String)> {
    let mut in_string = false;
    let mut escaped = false;
    for (i, ch) in field.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
        } else if ch == ':' {
            return Some((
                field[..i].trim().to_string(),
                field[i + 1..].trim().to_string(),
            ));
        }
    }
    None
}

fn unescape(text: &str) -> Option<String> {
    let text = text.trim().strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'b' => out.push('\u{08}'),
                'f' => out.push('\u{0c}'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if hex.len() != 4 {
                        return None;
                    }
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                _ => return None,
            },
            c => out.push(c),
        }
    }
    Some(out)
}

struct Options {
    action: String,
    root: PathBuf,
    target: String,
    binary: Option<PathBuf>,
}

fn parse_args() -> Result<Options, String> {
    let mut action = None;
    let mut root = None;
    let mut target = DEFAULT_TARGET.to_string();
    let mut binary = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--root" {
            root = Some(args.next().ok_or("--root needs a directory".to_string())?);
        } else if arg == "--target" {
            target = args.next().ok_or("--target needs a triple".to_string())?;
        } else if arg == "--binary" {
            binary = Some(args.next().ok_or("--binary needs a path".to_string())?);
        } else if let Some(dir) = arg.strip_prefix("--root=") {
            root = Some(dir.to_string());
        } else if let Some(triple) = arg.strip_prefix("--target=") {
            target = triple.to_string();
        } else if let Some(path) = arg.strip_prefix("--binary=") {
            binary = Some(path.to_string());
        } else if action.is_none() && !arg.starts_with('-') {
            action = Some(arg);
        } else {
            return Err(format!("unexpected argument: {arg}"));
        }
    }
    let action =
        action.ok_or("usage: podbox-buildstate {inputs|record|status|commit}".to_string())?;
    if !["inputs", "record", "status", "commit"].contains(&action.as_str()) {
        return Err(format!("unknown action: {action}"));
    }
    let root = match root {
        Some(dir) => PathBuf::from(dir),
        None => repo_root().ok_or("cannot locate the checkout root".to_string())?,
    };
    Ok(Options {
        action,
        root,
        target,
        binary: binary.map(PathBuf::from),
    })
}

fn main() {
    let options = match parse_args() {
        Ok(options) => options,
        Err(e) => fail(e),
    };
    let root = options.root;
    let binary = options.binary.unwrap_or_else(|| {
        root.join("target")
            .join(&options.target)
            .join("release/podbox")
    });
    match options.action.as_str() {
        "commit" => println!("{}", build_commit(&root)),
        "inputs" => match inputs_digest(&root, &options.target, true) {
            Ok(digest) => println!("{digest}"),
            Err(e) => fail(e),
        },
        "record" => {
            let inputs = match inputs_digest(&root, &options.target, true) {
                Ok(digest) => digest,
                Err(e) => fail(e),
            };
            let outputs = match outputs_digest(&binary) {
                Ok(digests) => digests,
                Err(e) if e == "missing" => fail("a workspace executable is missing".to_string()),
                Err(e) => fail(e),
            };
            let destination = root.join(".dev/build-state.json");
            if let Some(parent) = destination.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    fail(format!("cannot create {}: {e}", parent.display()));
                }
            }
            let temporary = destination.with_extension("new");
            if let Err(e) = fs::write(&temporary, record_json(&inputs, &outputs) + "\n") {
                fail(format!("cannot write {}: {e}", temporary.display()));
            }
            if let Err(e) = fs::rename(&temporary, &destination) {
                fail(format!("cannot replace {}: {e}", destination.display()));
            }
        }
        _ => {
            let (message, code) = status(&root, &options.target, &binary);
            println!("{message}");
            std::process::exit(code);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = env::temp_dir().join(format!(
            "podbox-buildstate-test-{}-{}",
            std::process::id(),
            stamp
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn objects_sort_keys_with_python_separators() {
        let mut map = BTreeMap::new();
        map.insert("b".to_string(), "2".to_string());
        map.insert("a".to_string(), "1".to_string());
        let mut out = String::new();
        json_object_sorted(&map, &mut out);
        assert_eq!(out, "{\"a\": \"1\", \"b\": \"2\"}");
    }

    #[test]
    fn record_json_matches_python_key_order() {
        let mut outputs = BTreeMap::new();
        outputs.insert("podbox".to_string(), "aa".to_string());
        outputs.insert("node".to_string(), "bb".to_string());
        assert_eq!(
            record_json("in", &outputs),
            "{\"inputs\": \"in\", \"outputs\": {\"node\": \"bb\", \"podbox\": \"aa\"}, \"schema\": \"podbox-build/1\"}"
        );
    }

    #[test]
    fn status_names_every_state_on_a_fixture() {
        let root = scratch();
        let binary = root.join("podbox");
        // Absent binary.
        assert_eq!(status(&root, "t", &binary), ("absent".to_string(), 2));
        fs::write(&binary, b"fixture-output\n").unwrap();
        for helper in ["node", "operator", "proxy", "shell"] {
            fs::write(root.join(helper), b"fixture-output\n").unwrap();
        }
        // No record.
        assert_eq!(status(&root, "t", &binary).1, 1);
        // Unreadable record.
        fs::create_dir_all(root.join(".dev")).unwrap();
        fs::write(root.join(".dev/build-state.json"), b"not json").unwrap();
        assert_eq!(
            status(&root, "t", &binary),
            ("stale: unreadable build record".to_string(), 1)
        );
        // Ready record through the writer itself. status() digests with
        // tools=true, so the fixture record must too, or the comparison
        // is against a different digest.
        let inputs = inputs_digest(&root, "t", true).unwrap();
        let outputs = outputs_digest(&binary).unwrap();
        fs::write(
            root.join(".dev/build-state.json"),
            record_json(&inputs, &outputs) + "\n",
        )
        .unwrap();
        assert_eq!(status(&root, "t", &binary), ("ready".to_string(), 0));
        // Changed output bytes.
        fs::write(&binary, b"different-output\n").unwrap();
        assert_eq!(status(&root, "t", &binary).1, 1);
        // Missing output.
        fs::remove_file(&binary).unwrap();
        assert_eq!(status(&root, "t", &binary).1, 2);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn walk_skips_target_and_pycache_parts() {
        let root = scratch();
        fs::create_dir_all(root.join("crates/a/target")).unwrap();
        fs::create_dir_all(root.join("scripts/__pycache__")).unwrap();
        fs::write(root.join("crates/a/keep.rs"), b"x").unwrap();
        fs::write(root.join("crates/a/target/drop.rs"), b"x").unwrap();
        fs::write(root.join("scripts/__pycache__/drop.pyc"), b"x").unwrap();
        let mut paths = BTreeSet::new();
        walk(&root.join("crates"), &root, &mut paths).unwrap();
        walk(&root.join("scripts"), &root, &mut paths).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths.contains("crates/a/keep.rs"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn record_round_trips_through_the_reader() {
        let mut outputs = BTreeMap::new();
        outputs.insert("podbox".to_string(), "aa".to_string());
        let text = record_json("in", &outputs);
        assert_eq!(field_string(&text, "\"schema\"").as_deref(), Some(SCHEMA));
        assert_eq!(field_string(&text, "\"inputs\"").as_deref(), Some("in"));
        let raw = field_raw(&text, "\"outputs\"").unwrap();
        assert_eq!(parse_strmap(&raw).unwrap(), outputs);
        assert!(field_string("not json", "\"inputs\"").is_none());
        assert!(field_string("{\"inputs\": 1}", "\"inputs\"").is_none());
    }
}
