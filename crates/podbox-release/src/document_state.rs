//! document-state: generate the source state page.
//!
//! A behaviour-preserving port of scripts/document-state.py (T-1564). The
//! script stays as a compat shim that execs this binary; the logic lives
//! here. The page is byte-identical, including the regenerate line that
//! names the shim path, which stays valid. TOML is read with a line parser
//! that covers exactly what the manifests use: section headers, `key =
//! value` lines with string or inline-table values, and `[[package]]`
//! blocks. Anything else is exit 2, never a guess.
//!
//! Usage: document-state [--write] [--root DIR]
//!
//! Exit: 0 the snapshot matches (or was written), 1 it differs, 2 the
//! sources could not be read.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn fail(message: String) -> ! {
    eprintln!("document-state: cannot read source state: {message}");
    std::process::exit(2);
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

fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

/// Split `key = value` lines of one section. Returns (key_head, value) with
/// the key cut at the first dot, so `sha2.workspace = true` reads as the
/// `sha2` dependency the TOML table holds.
fn section_entries(text: &str, header: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut inside = false;
    for raw in text.split('\n') {
        let line = raw.trim();
        if line.starts_with('[') {
            inside = line == header;
            continue;
        }
        if !inside || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().trim_matches('"');
            let head = key.split('.').next().unwrap_or("").trim().to_string();
            let value = line[eq + 1..].trim().to_string();
            if !head.is_empty() {
                entries.push((head, value));
            }
        }
    }
    entries
}

/// The `[workspace]` members list: quoted strings inside the `members = [`
/// array, which spans lines.
fn workspace_members(text: &str) -> Result<Vec<String>, String> {
    let missing = "workspace has no members list".to_string();
    let mut array: Option<String> = None;
    for raw in text.split('\n') {
        let line = raw.trim();
        if array.is_none() {
            if line.starts_with("members") {
                if let Some(eq) = line.find('=') {
                    array = Some(line[eq + 1..].to_string());
                }
            }
            continue;
        }
        let current = array.unwrap_or_default();
        if current.contains(']') {
            array = Some(current);
            break;
        }
        array = Some(current + "\n" + line);
    }
    let array = array.ok_or(missing.clone())?;
    let open = array.find('[').ok_or(missing.clone())?;
    let after = &array[open + 1..];
    let close = after.find(']').ok_or(missing.clone())?;
    let mut members = Vec::new();
    for part in after[..close].split(',') {
        let part = part.trim().trim_matches('"').trim();
        if !part.is_empty() && !part.starts_with('#') {
            members.push(part.to_string());
        }
    }
    if members.is_empty() {
        return Err(missing);
    }
    Ok(members)
}

/// The workspace version: the first `version = "x"` line after
/// `[workspace.package]`.
fn workspace_version(text: &str) -> Result<String, String> {
    let mut inside = false;
    for raw in text.split('\n') {
        let line = raw.trim();
        if line.starts_with('[') {
            inside = line == "[workspace.package]";
            continue;
        }
        if inside && line.starts_with("version") {
            if let Some(eq) = line.find('=') {
                let value = line[eq + 1..].trim().trim_matches('"').to_string();
                if !value.is_empty() {
                    return Ok(value);
                }
            }
        }
    }
    Err("workspace has no package version".to_string())
}

/// A dependency is registry-made where the manifest gives a version string
/// or a table without `path`, exactly like the script's isinstance-or test.
fn is_registry(value: &str) -> bool {
    let value = value.trim();
    if value.starts_with('"') {
        return true;
    }
    if value.starts_with('{') {
        return !value.contains("path");
    }
    true
}

fn lock_versions(text: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut table: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut name = String::new();
    let mut version = String::new();
    let mut source = false;
    let mut inside = false;
    let flush = |table: &mut BTreeMap<String, BTreeSet<String>>,
                 name: &str,
                 version: &str,
                 source: bool| {
        if source && !name.is_empty() && !version.is_empty() {
            table
                .entry(name.to_string())
                .or_default()
                .insert(version.to_string());
        }
    };
    for raw in text.split('\n') {
        let line = raw.trim();
        if line == "[[package]]" {
            flush(&mut table, &name, &version, source);
            name.clear();
            version.clear();
            source = false;
            inside = true;
            continue;
        }
        if line.starts_with('[') {
            flush(&mut table, &name, &version, source);
            name.clear();
            version.clear();
            source = false;
            inside = false;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let value = line[eq + 1..].trim().trim_matches('"').to_string();
            if key == "name" {
                name = value;
            } else if key == "version" {
                version = value;
            } else if key == "source" {
                source = true;
            }
        }
    }
    flush(&mut table, &name, &version, source);
    table
}

/// The variant list of `pub enum NAME`, mirroring the script's two
/// expressions: the body up to the closing brace on its own line, then one
/// leading-capital word per line before a comma.
fn enumeration(text: &str, path: &str, name: &str) -> Result<Vec<String>, String> {
    let error = format!("cannot read {name} from {path}");
    let start = text
        .find(&format!("pub enum {name}"))
        .ok_or(error.clone())?;
    let after = &text[start..];
    let open = after.find('{').ok_or(error.clone())?;
    let body = &after[open + 1..];
    let end = body.find("\n}").ok_or(error.clone())?;
    let mut values = Vec::new();
    for line in body[..end].split('\n') {
        let trimmed = line.trim_start();
        let mut chars = trimmed.chars();
        match chars.next() {
            Some(first) if first.is_ascii_uppercase() => {}
            _ => continue,
        }
        let mut word = String::new();
        for ch in trimmed.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                word.push(ch);
            } else {
                break;
            }
        }
        if word.is_empty() {
            continue;
        }
        let rest = trimmed[word.len()..].trim_start();
        if rest.starts_with(',') {
            values.push(word);
        }
    }
    if values.is_empty() {
        return Err(format!("no values for {name}"));
    }
    Ok(values)
}

fn render(root: &Path) -> Result<String, String> {
    let manifest = read_text(&root.join("Cargo.toml"))?;
    let lock = read_text(&root.join("Cargo.lock"))?;
    let members = workspace_members(&manifest)?;
    let version = workspace_version(&manifest)?;
    let mut registry = BTreeSet::new();
    for member in &members {
        let text = read_text(&root.join(member).join("Cargo.toml"))?;
        for (name, value) in section_entries(&text, "[dependencies]") {
            if is_registry(&value) {
                registry.insert(name);
            }
        }
    }
    let locked = lock_versions(&lock);
    let mut lines = vec![
        "# Source state".to_string(),
        String::new(),
        "This page is generated from the tracked manifests and source declarations.".to_string(),
        "It records the build surface. Runtime proof and limits live in".to_string(),
        "[PROGRESS](../TODO/PROGRESS.md) and [Limits](limits.md).".to_string(),
        String::new(),
        "Regenerate with `py scripts/document-state.py --write` on Windows.".to_string(),
        "Use `python3` on Linux. The record gate rejects a changed snapshot.".to_string(),
        String::new(),
        "## Workspace".to_string(),
        String::new(),
        format!("Declared version: `{version}`."),
        String::new(),
        "| Member | Manifest |".to_string(),
        "| --- | --- |".to_string(),
    ];
    for member in &members {
        let short = member.rsplit('/').next().unwrap_or(member);
        lines.push(format!(
            "| `{short}` | [{member}/Cargo.toml](../{member}/Cargo.toml) |"
        ));
    }
    lines.push(String::new());
    lines.push("Separate crate: `crates/podbox-interpose`.".to_string());
    lines.push(String::new());
    lines.push("## Mechanism declarations".to_string());
    lines.push(String::new());
    for (path, name) in [
        ("crates/podbox-probe/src/select.rs", "Rung"),
        ("crates/podbox-cli/src/tier.rs", "Tier"),
        ("crates/podbox-enter/src/ladder.rs", "Mode"),
    ] {
        let text = read_text(&root.join(path))?;
        let values = enumeration(&text, path, name)?;
        let rendered: Vec<String> = values.iter().map(|value| format!("`{value}`")).collect();
        lines.push(format!(
            "`{name}` in [{path}](../{path}): {}.",
            rendered.join(", ")
        ));
        lines.push(String::new());
    }
    lines.push("## SSH helper sources".to_string());
    lines.push(String::new());
    lines.push("| Executable | Source |".to_string());
    lines.push("| --- | --- |".to_string());
    let mut bins = Vec::new();
    let bin_dir = root.join("crates/podbox-ssh/src/bin");
    let entries =
        fs::read_dir(&bin_dir).map_err(|e| format!("cannot list {}: {e}", bin_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot list bins: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            bins.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    bins.sort();
    for path in bins {
        let stem = path
            .rsplit('/')
            .next()
            .unwrap_or(&path)
            .strip_suffix(".rs")
            .unwrap_or(&path);
        lines.push(format!("| `{stem}` | [{path}](../{path}) |"));
    }
    lines.push(String::new());
    lines.push("## Declared direct registry dependencies".to_string());
    lines.push(String::new());
    lines.push("| Dependency | Locked versions |".to_string());
    lines.push("| --- | --- |".to_string());
    for name in &registry {
        let empty = BTreeSet::new();
        let versions = locked.get(name).unwrap_or(&empty);
        if versions.is_empty() {
            return Err(format!("no registry lock entry for {name}"));
        }
        let rendered: Vec<String> = versions
            .iter()
            .map(|version| format!("`{version}`"))
            .collect();
        lines.push(format!("| `{name}` | {} |", rendered.join(", ")));
    }
    Ok(lines.join("\n") + "\n")
}

fn main() {
    let mut write = false;
    let mut root_flag: Option<String> = None;
    // The retired script derived the root from its own path and took no
    // --root flag; the binary has no script path, so it takes one, in both
    // the `--root DIR` and `--root=DIR` forms.
    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        if arg == "--write" {
            write = true;
        } else if arg == "--root" {
            root_flag = Some(argv.next().unwrap_or_default());
        } else if let Some(dir) = arg.strip_prefix("--root=") {
            root_flag = Some(dir.to_string());
        } else {
            eprintln!("document-state: unexpected argument: {arg}");
            std::process::exit(2);
        }
    }
    let root = match root_flag {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        Some(_) => {
            eprintln!("document-state: --root needs a directory");
            std::process::exit(2);
        }
        None => repo_root().unwrap_or_else(|| fail("cannot locate the checkout root".to_string())),
    };
    let destination = root.join("docs/runtime-state.md");
    let expected = match render(&root) {
        Ok(page) => page,
        Err(e) => fail(e),
    };
    if write {
        match fs::write(&destination, &expected) {
            Ok(()) => println!("document-state: wrote docs/runtime-state.md"),
            Err(e) => fail(format!("cannot write {}: {e}", destination.display())),
        }
    } else if !destination.is_file()
        || fs::read_to_string(&destination)
            .map(|text| text != expected)
            .unwrap_or(true)
    {
        eprintln!("document-state: source state differs; regenerate docs/runtime-state.md");
        std::process::exit(1);
    } else {
        println!("document-state: ok");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_version_and_deps_read_the_manifest_shape() {
        let text = "[workspace]\nmembers = [\n    \"crates/a\",\n    \"crates/b\",\n]\n[workspace.package]\nversion = \"0.2.0\"\n";
        assert_eq!(
            workspace_members(text).unwrap(),
            vec!["crates/a".to_string(), "crates/b".to_string()]
        );
        assert_eq!(workspace_version(text).unwrap(), "0.2.0");
        let member = "[dependencies]\nfoo = \"1\"\nbar = { path = \"../bar\" }\nbar2 = { version = \"2\" }\n[dev-dependencies]\nbaz = \"3\"\n";
        let entries = section_entries(member, "[dependencies]");
        assert_eq!(entries.len(), 3);
        let registry: Vec<&str> = entries
            .iter()
            .filter(|(_, value)| is_registry(value))
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(registry, vec!["foo", "bar2"]);
    }

    #[test]
    fn lock_versions_keep_registry_sources_only() {
        let lock = "[[package]]\nname = \"a\"\nversion = \"1\"\nsource = \"registry+x\"\n\n[[package]]\nname = \"b\"\nversion = \"2\"\n";
        let table = lock_versions(lock);
        assert_eq!(table.len(), 1);
        assert!(table["a"].contains("1"));
    }

    #[test]
    fn enumerations_match_the_script_expressions() {
        let text = "pub enum Mode {\n    Chroot,\n    Userland,\n}\n";
        assert_eq!(
            enumeration(text, "p", "Mode").unwrap(),
            vec!["Chroot".to_string(), "Userland".to_string()]
        );
        assert!(enumeration(text, "p", "Missing").is_err());
        assert!(enumeration("pub enum Empty {\n}\n", "p", "Empty").is_err());
    }
}
