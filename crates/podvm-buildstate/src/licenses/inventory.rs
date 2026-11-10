use super::*;

#[derive(Debug, Clone)]
pub(super) enum Json {
    Null,
    Bool(bool),
    Number(String),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub(super) fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub(super) fn as_str(&self) -> Option<&str> {
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

// ------------------------------------------------------- indent=2 writer ---

/// One inventory record in insertion order: name, version, licence, files.
/// Files hold path then sha256. This order is the script's dict order, kept
/// so the bytes agree with the retired output.
pub(super) struct FileEntry {
    pub(super) path: String,
    pub(super) sha256: String,
}

pub(super) struct Record {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) licence: Option<String>,
    pub(super) files: Vec<FileEntry>,
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

pub(super) fn inventory_json(records: &[Record]) -> String {
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

pub(super) fn valid_identifier(text: &str) -> bool {
    // The script fullmatches [A-Za-z0-9_.+-]+ over name+version concatenated.
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'+' | b'-'))
}

pub(super) fn licence_hit(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["license", "licence", "copying", "notice"]
        .iter()
        .any(|stem| lower.starts_with(stem))
}

pub(super) fn gather_texts(source: &Path, texts: &mut Vec<PathBuf>) -> Result<(), String> {
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

pub(super) struct Package {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) manifest_dir: PathBuf,
    pub(super) licence: Option<String>,
    pub(super) licence_file: Option<String>,
}

/// The triples of the release matrix (`.github/workflows/nightly.yml`). The
/// inventory holds the packages that one of them builds; a package that only
/// another platform pulls in, such as `r-efi` for UEFI, is in no released
/// binary. A test keeps this list equal to the matrix.
pub(super) const RELEASE_TRIPLES: &[&str] = &[
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "riscv64gc-unknown-linux-musl",
    "loongarch64-unknown-linux-musl",
    "armv7-unknown-linux-musleabihf",
    "i686-unknown-linux-musl",
    "powerpc64le-unknown-linux-musl",
];

/// The registry and git packages that `manifest` builds for each triple of
/// [`RELEASE_TRIPLES`], once each.
pub(super) fn metadata_packages(root: &Path, manifest: &str) -> Result<Vec<Package>, String> {
    let mut out: Vec<Package> = Vec::new();
    for triple in RELEASE_TRIPLES {
        for package in triple_packages(root, manifest, triple)? {
            if !out
                .iter()
                .any(|p| p.name == package.name && p.version == package.version)
            {
                out.push(package);
            }
        }
    }
    Ok(out)
}

fn triple_packages(root: &Path, manifest: &str, triple: &str) -> Result<Vec<Package>, String> {
    let text = run_captured(
        "cargo",
        &[
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            triple,
            "--manifest-path",
            manifest,
        ],
        root,
        180,
    )
    .map_err(|e| format!("cargo metadata for {manifest} ({triple}): {e}"))?;
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
