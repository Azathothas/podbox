//! podbox-release-licenses: retain licence text for the locked package set.
//!
//! A behaviour-preserving port of scripts/release-licenses.py (T-1560). The
//! script stays as a compat shim that execs this binary; the logic lives
//! here. `inventory.json` is byte-identical to the script's: `json.dumps`
//! with `indent=2`, insertion-ordered keys, plus a trailing newline.
//!
//! Usage: podbox-release-licenses --output DIR [--root DIR]
//!
//! Exit: 0 the inventory is written, 2 it could not be produced. The error
//! line keeps the script's prefix.

#[path = "common.rs"]
mod common;

use common::{file_digest, json_string, repo_root, run_captured};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn fail(message: String) -> ! {
    eprintln!("release-licenses: cannot produce inventory: {message}");
    std::process::exit(2);
}

// ------------------------------------------------------------ JSON model ----

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
    /// One line naming the shape, for errors where the metadata is not the
    /// shape the port expects. Reading every payload here keeps the scalar
    /// variants live in every build, test or not.
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
        // Consumes the opening quote first.
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
                            // A high surrogate pairs with the following low one.
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
                    // UTF-8 text passes through; lone continuation bytes fail.
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

// ------------------------------------------------------- indent=2 writer ---

/// One inventory record in insertion order: name, version, licence, files.
/// Files hold path then sha256. This order is the script's dict order, kept
/// so the bytes agree with the retired output.
struct FileEntry {
    path: String,
    sha256: String,
}

struct Record {
    name: String,
    version: String,
    licence: Option<String>,
    files: Vec<FileEntry>,
}

fn pad(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn write_record(record: &Record, out: &mut String, level: usize) {
    out.push_str("{\n");
    pad(out, level + 1);
    out.push_str("\"name\": ");
    json_string(&record.name, out);
    out.push_str(",\n");
    pad(out, level + 1);
    out.push_str("\"version\": ");
    json_string(&record.version, out);
    out.push_str(",\n");
    pad(out, level + 1);
    out.push_str("\"licence\": ");
    match &record.licence {
        Some(licence) => json_string(licence, out),
        None => out.push_str("null"),
    }
    out.push_str(",\n");
    pad(out, level + 1);
    out.push_str("\"files\": ");
    if record.files.is_empty() {
        out.push_str("[]");
    } else {
        out.push_str("[\n");
        for (i, file) in record.files.iter().enumerate() {
            pad(out, level + 2);
            out.push_str("{\n");
            pad(out, level + 3);
            out.push_str("\"path\": ");
            json_string(&file.path, out);
            out.push_str(",\n");
            pad(out, level + 3);
            out.push_str("\"sha256\": ");
            json_string(&file.sha256, out);
            out.push('\n');
            pad(out, level + 2);
            out.push('}');
            if i + 1 < record.files.len() {
                out.push(',');
            }
            out.push('\n');
        }
        pad(out, level + 1);
        out.push(']');
    }
    out.push('\n');
    pad(out, level);
    out.push('}');
}

fn inventory_json(records: &[Record]) -> String {
    let mut out = String::from("[\n");
    for (i, record) in records.iter().enumerate() {
        pad(&mut out, 1);
        write_record(record, &mut out, 1);
        if i + 1 < records.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("]\n");
    out
}

// ----------------------------------------------------------------- driver ---

fn valid_identifier(text: &str) -> bool {
    // The script fullmatches [A-Za-z0-9_.+-]+ over name+version concatenated.
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'+' | b'-'))
}

fn licence_hit(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["license", "licence", "copying", "notice"]
        .iter()
        .any(|stem| lower.starts_with(stem))
}

fn gather_texts(source: &Path, texts: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(source).map_err(|e| format!("cannot list {}: {e}", source.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot list {}: {e}", source.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if !licence_hit(&name) {
            continue;
        }
        let kind = fs::symlink_metadata(&path)
            .map_err(|e| format!("cannot stat {}: {e}", path.display()))?;
        if kind.is_file() {
            texts.push(path);
        } else if kind.is_dir() {
            gather_tree(&path, texts)?;
        }
    }
    Ok(())
}

fn gather_tree(dir: &Path, texts: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("cannot list {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot list {}: {e}", dir.display()))?;
        let path = entry.path();
        let kind = fs::symlink_metadata(&path)
            .map_err(|e| format!("cannot stat {}: {e}", path.display()))?;
        if kind.is_file() {
            texts.push(path);
        } else if kind.is_dir() {
            gather_tree(&path, texts)?;
        }
    }
    Ok(())
}

struct Package {
    name: String,
    version: String,
    manifest_dir: PathBuf,
    licence: Option<String>,
    licence_file: Option<String>,
}

fn metadata_packages(root: &Path, manifest: &str) -> Result<Vec<Package>, String> {
    let text = run_captured(
        "cargo",
        &[
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--manifest-path",
            manifest,
        ],
        root,
        180,
    )
    .map_err(|e| format!("cargo metadata for {manifest}: {e}"))?;
    let parsed = parse_json(&text)
        .ok_or_else(|| format!("cargo metadata for {manifest}: unreadable output"))?;
    let list = match parsed.get("packages") {
        Some(Json::Array(items)) => items,
        other => {
            let shape = other
                .map(Json::describe)
                .unwrap_or_else(|| "absent".to_string());
            return Err(format!(
                "cargo metadata for {manifest}: packages is not an array: {shape}"
            ));
        }
    };
    let mut packages = Vec::new();
    for item in list {
        if item.get("source").and_then(Json::as_str).is_none() {
            continue;
        }
        let name = item
            .get("name")
            .and_then(Json::as_str)
            .ok_or_else(|| format!("cargo metadata: package without a name: {item:?}"))?;
        let version = item
            .get("version")
            .and_then(Json::as_str)
            .ok_or_else(|| format!("cargo metadata: package without a version: {item:?}"))?;
        let manifest_path = item
            .get("manifest_path")
            .and_then(Json::as_str)
            .ok_or_else(|| format!("cargo metadata: package without a manifest: {item:?}"))?;
        packages.push(Package {
            name: name.to_string(),
            version: version.to_string(),
            manifest_dir: Path::new(manifest_path)
                .parent()
                .ok_or("cargo metadata: manifest without a parent")?
                .to_path_buf(),
            licence: item
                .get("license")
                .and_then(Json::as_str)
                .map(str::to_string),
            licence_file: item
                .get("license_file")
                .and_then(Json::as_str)
                .map(str::to_string),
        });
    }
    Ok(packages)
}

fn main() {
    let mut output: Option<PathBuf> = None;
    let mut root: Option<PathBuf> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--output" {
            output = Some(
                args.next()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| fail("--output needs a directory".to_string())),
            );
        } else if let Some(dir) = arg.strip_prefix("--output=") {
            output = Some(PathBuf::from(dir));
        } else if arg == "--root" {
            root = Some(
                args.next()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| fail("--root needs a directory".to_string())),
            );
        } else if let Some(dir) = arg.strip_prefix("--root=") {
            root = Some(PathBuf::from(dir));
        } else {
            fail(format!("unexpected argument: {arg}"));
        }
    }
    let output = output.unwrap_or_else(|| fail("--output DIR is required".to_string()));
    let root = root.unwrap_or_else(|| {
        repo_root().unwrap_or_else(|| fail("cannot locate the checkout root".to_string()))
    });
    // Include the separately built interposer as well as the workspace.
    let mut packages: BTreeMap<(String, String), Package> = BTreeMap::new();
    for manifest in ["Cargo.toml", "crates/podbox-interpose/Cargo.toml"] {
        match metadata_packages(&root, manifest) {
            Ok(list) => {
                for package in list {
                    packages.insert((package.name.clone(), package.version.clone()), package);
                }
            }
            Err(e) => fail(e),
        }
    }
    if packages.is_empty() {
        fail("the locked package set is empty".to_string());
    }
    let mut records = Vec::new();
    for ((name, version), package) in &packages {
        if !valid_identifier(&format!("{name}{version}")) {
            fail("invalid package identifier".to_string());
        }
        let source = package.manifest_dir.clone();
        let mut texts = Vec::new();
        if let Err(e) = gather_texts(&source, &mut texts) {
            fail(e);
        }
        if let Some(file) = &package.licence_file {
            texts.push(source.join(file));
        }
        if texts.is_empty() {
            fail(format!("no retained licence text: {name} {version}"));
        }
        texts.sort();
        let destination = output.join(format!("{name}-{version}"));
        if let Err(e) = fs::create_dir_all(&destination) {
            fail(format!("cannot create {}: {e}", destination.display()));
        }
        let canonical_source = source.canonicalize().unwrap_or_else(|_| source.clone());
        let mut files = Vec::new();
        for path in texts {
            // The script resolves strictly, so a dangling licence path is
            // exit 2 rather than a skipped file.
            let resolved = path.canonicalize().unwrap_or_else(|_| {
                fail(format!("cannot resolve {}: dangling path", path.display()))
            });
            if !resolved.starts_with(&canonical_source) {
                fail(format!("licence path leaves package: {name} {version}"));
            }
            let relative = resolved
                .strip_prefix(&canonical_source)
                .unwrap_or_else(|_| fail(format!("licence path leaves package: {name} {version}")));
            let target = destination.join(relative);
            if let Some(parent) = target.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    fail(format!("cannot create {}: {e}", parent.display()));
                }
            }
            if let Err(e) = fs::copy(&resolved, &target) {
                fail(format!("cannot copy {}: {e}", resolved.display()));
            }
            let digest = match file_digest(&target) {
                Ok(digest) => digest,
                Err(e) => fail(format!("cannot hash {}: {e}", target.display())),
            };
            let mut relative_text = relative
                .to_str()
                .unwrap_or_else(|| fail(format!("non-UTF-8 licence path in {name} {version}")))
                .replace('\\', "/");
            if relative_text.is_empty() {
                relative_text = ".".to_string();
            }
            files.push(FileEntry {
                path: relative_text,
                sha256: digest,
            });
        }
        records.push(Record {
            name: name.clone(),
            version: version.clone(),
            licence: package.licence.clone(),
            files,
        });
    }
    if let Err(e) = fs::create_dir_all(&output) {
        fail(format!("cannot create {}: {e}", output.display()));
    }
    if let Err(e) = fs::write(output.join("inventory.json"), inventory_json(&records)) {
        fail(format!("cannot write inventory: {e}"));
    }
    println!(
        "release-licenses: retained texts for {} locked registry packages",
        records.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_matches_the_script_character_class() {
        assert!(valid_identifier("ring0.17.14"));
        assert!(valid_identifier("libc-1.0_0+qux"));
        assert!(!valid_identifier(""));
        assert!(!valid_identifier("has space"));
        assert!(!valid_identifier("uni\u{e9}code"));
    }

    #[test]
    fn parser_reads_metadata_shapes() {
        let parsed = parse_json("{\"packages\": [{\"name\": \"a\", \"version\": \"1\", \"source\": \"x\", \"manifest_path\": \"/m/C.toml\", \"license\": null}]}").unwrap();
        let list = match parsed.get("packages") {
            Some(Json::Array(items)) => items,
            _ => panic!("packages is not an array"),
        };
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].get("name").and_then(Json::as_str), Some("a"));
        assert!(list[0].get("license").and_then(Json::as_str).is_none());
        assert!(parse_json("[1, {\"a\": true}, null]").is_some());
        assert!(parse_json("{\"a\": }").is_none());
        assert!(parse_json("{\"a\": 1} trailing").is_none());
        assert_eq!(
            parse_json("\"a\\u00e9\"").unwrap().as_str(),
            Some("a\u{e9}")
        );
        match parse_json("{\"n\": 3, \"b\": false}").unwrap() {
            Json::Object(pairs) => {
                assert!(matches!(pairs[0].1, Json::Number(ref n) if n == "3"));
                assert!(matches!(pairs[1].1, Json::Bool(false)));
            }
            _ => panic!("object expected"),
        }
    }

    #[test]
    fn inventory_renders_like_cpython_indent_two() {
        let records = vec![Record {
            name: "a".to_string(),
            version: "1".to_string(),
            licence: None,
            files: vec![FileEntry {
                path: "LICENSE".to_string(),
                sha256: "aa".to_string(),
            }],
        }];
        assert_eq!(
            inventory_json(&records),
            "[\n  {\n    \"name\": \"a\",\n    \"version\": \"1\",\n    \"licence\": null,\n    \"files\": [\n      {\n        \"path\": \"LICENSE\",\n        \"sha256\": \"aa\"\n      }\n    ]\n  }\n]\n"
        );
        let empty = vec![Record {
            name: "a".to_string(),
            version: "1".to_string(),
            licence: Some("MIT".to_string()),
            files: vec![],
        }];
        assert!(inventory_json(&empty).contains("\"licence\": \"MIT\""));
        assert!(inventory_json(&empty).contains("\"files\": []"));
    }

    #[test]
    fn licence_stems_match_case_insensitively() {
        assert!(licence_hit("LICENSE-MIT"));
        assert!(licence_hit("licence.txt"));
        assert!(licence_hit("COPYING"));
        assert!(licence_hit("Notice.md"));
        assert!(!licence_hit("README.md"));
    }
}
