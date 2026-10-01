//! podbox-podvm: `podbox --help` and `podvm --help` list one set of verbs.
//!
//! A behaviour-preserving port of experiments/145-podvm-parity.sh (T-1568).
//! The script stays as a compat shim that execs this binary; the logic
//! lives here. Its live-payload rows stay shell-driven by this binary the
//! same way the script drove them: nothing here pulls an image, and the
//! refusals name the legs without reaching them.
//!
//! The pure half is a unit test in this file: the verb-set comparison over
//! captured help texts, with the table rows as the other input. The binary
//! drives the shipped binary for the live clauses.
//!
//! Usage: podbox-podvm [--root DIR] [--bin PATH]
//!
//! Exit: 0 every row green, 1 a row disagreed, 2 the binary or the table
//! could not run.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

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

fn run_combined(prog: &str, args: &[&str], secs: u64) -> Option<(i32, String)> {
    let (tx, rx) = mpsc::channel();
    let mut command = Command::new(prog);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().ok()?;
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(Ok(output)) => {
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            Some((output.status.code().unwrap_or(1), text))
        }
        _ => None,
    }
}

// ------------------------------------------------------------ JSON reader ---

/// The two shapes this binary reads: the parity table (objects with `verb`
/// and an optional `flag`) and the exit-code table (objects with `case` and
/// `code`). A strict small reader: trailing bytes and unknown escapes are
/// errors, never guesses.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Json {
    Null,
    Bool(bool),
    Number(String),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

impl<'a> Reader<'a> {
    fn skip(&mut self) {
        while self.pos < self.bytes.len() && is_space(self.bytes[self.pos]) {
            self.pos += 1;
        }
    }
    fn value(&mut self) -> Option<Json> {
        self.skip();
        match *self.bytes.get(self.pos)? {
            b'"' => Some(Json::Str(self.string()?)),
            b'{' => self.object(),
            b'[' => self.array(),
            b't' => self.word("true", Json::Bool(true)),
            b'f' => self.word("false", Json::Bool(false)),
            b'n' => self.word("null", Json::Null),
            b'-' | b'0'..=b'9' => Some(Json::Number(self.number())),
            _ => None,
        }
    }
    fn word(&mut self, word: &str, value: Json) -> Option<Json> {
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
            pairs.push((key, self.value()?));
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
    let mut reader = Reader {
        bytes: text.as_bytes(),
        pos: 0,
    };
    let value = reader.value()?;
    reader.skip();
    if reader.pos == reader.bytes.len() {
        Some(value)
    } else {
        None
    }
}

fn field<'a>(row: &'a Json, key: &str) -> Option<&'a Json> {
    match row {
        Json::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

// ------------------------------------------------------------- pure half ---

/// The words of one help text, for the one-verb-set comparison. Clause 1 of
/// the script compares the two help outputs byte for byte and then greps
/// four words; the set comparison below is the unit-testable half.
fn help_words(help: &str) -> BTreeSet<String> {
    help.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_')
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// True where the parity table carries the row: the verb with a flag
/// starting at the prefix, where a missing flag reads as `-`.
fn table_carries(rows: &[Json], verb: &str, flag: &str) -> bool {
    rows.iter().any(|row| {
        let verb_match = field(row, "verb").and_then(|value| match value {
            Json::Str(text) => Some(text.as_str()),
            _ => None,
        }) == Some(verb);
        let flag_text = match field(row, "flag") {
            Some(Json::Str(text)) => text.clone(),
            _ => "-".to_string(),
        };
        verb_match && flag_text.starts_with(flag)
    })
}

// ----------------------------------------------------------------- driver ---

struct Report {
    text: String,
    driven: u32,
    fails: u32,
    failed: bool,
}

impl Report {
    fn say(&mut self, line: &str) {
        self.text.push_str(line);
        self.text.push('\n');
    }
    fn pass(&mut self, what: &str) {
        self.driven += 1;
        self.say(&format!("  ok: {what}"));
    }
    fn miss(&mut self, what: &str) {
        self.failed = true;
        self.fails += 1;
        self.say(&format!("  FAIL: {what}"));
    }
}

fn main() {
    let mut root_flag: Option<String> = None;
    let mut bin_flag: Option<String> = None;
    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        if arg == "--root" {
            root_flag = Some(argv.next().unwrap_or_default());
        } else if arg == "--bin" {
            bin_flag = Some(argv.next().unwrap_or_default());
        } else if let Some(dir) = arg.strip_prefix("--root=") {
            root_flag = Some(dir.to_string());
        } else if let Some(path) = arg.strip_prefix("--bin=") {
            bin_flag = Some(path.to_string());
        } else {
            eprintln!("podbox-podvm: unexpected argument: {arg}");
            std::process::exit(2);
        }
    }
    let root = match root_flag {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        Some(_) => {
            eprintln!("podbox-podvm: --root needs a directory");
            std::process::exit(2);
        }
        None => repo_root().unwrap_or_else(|| {
            eprintln!("SKIP: cannot locate the checkout root");
            std::process::exit(2);
        }),
    };
    let bin = match bin_flag {
        Some(path) if !path.is_empty() => Some(path),
        Some(_) => {
            eprintln!("podbox-podvm: --bin needs a path");
            std::process::exit(2);
        }
        None => None,
    };
    let code = drive(&root, bin);
    std::process::exit(code);
}

fn drive(root: &Path, bin_flag: Option<String>) -> i32 {
    let out = root.join("experiments/results/podvm-parity.txt");
    let mut report = Report {
        text: String::new(),
        driven: 0,
        fails: 0,
        failed: false,
    };
    let binary = match bin_flag {
        Some(path) => PathBuf::from(path),
        None => std::env::var("PODBOX_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root.join("target/x86_64-unknown-linux-musl/release/podbox")),
    };
    if !binary.is_file() {
        eprintln!("SKIP: {} is not an executable. Build it:", binary.display());
        eprintln!("      cargo build --release --target x86_64-unknown-linux-musl");
        return 2;
    }
    let name = binary.to_string_lossy().into_owned();
    // `podvm` is the binary under another name. The dispatch keys on the
    // basename of argv[0], so a copy named `podvm` routes exactly like the
    // script's symlink, portably.
    let scratch = std::env::temp_dir().join(format!(
        "podbox-podvm-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    if fs::create_dir_all(&scratch).is_err() {
        eprintln!("SKIP: cannot create scratch");
        return 2;
    }
    let twin = scratch.join("podvm");
    if fs::copy(&binary, &twin).is_err() {
        eprintln!("SKIP: cannot stage the podvm name");
        fs::remove_dir_all(&scratch).ok();
        return 2;
    }
    let outcome = drive_inner(&name, &twin.to_string_lossy(), &mut report);
    fs::remove_dir_all(&scratch).ok();
    report.say("");
    report.say(&format!(
        "== counts: {} driven, {} mismatches",
        report.driven, report.fails
    ));
    print!("{}", report.text);
    if fs::create_dir_all(out.parent().unwrap_or_else(|| Path::new("."))).is_ok() {
        fs::write(&out, &report.text).ok();
    }
    // Copy to /out where a lane runner collects evidence, like the
    // lock-inheritance proof.
    let out_dir = Path::new("/out");
    if out_dir.is_dir() {
        fs::copy(&out, out_dir.join("podvm-parity.txt")).ok();
    }
    println!();
    println!(
        "written to {}",
        out.strip_prefix(root).unwrap_or(&out).display()
    );
    if outcome == 2 {
        return 2;
    }
    if report.failed {
        return 1;
    }
    0
}

fn drive_inner(binary: &str, twin: &str, report: &mut Report) -> i32 {
    let version = run_combined(binary, &["version"], 60)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_default();
    report.say("== conditions");
    report.say(&format!(
        "date              {}",
        run_combined("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], 10)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!(
        "host kernel       {}",
        run_combined("uname", &["-r"], 10)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!("podbox            {version}"));
    report.say("json              parsed in-process; no jq needed");
    report.say("");

    // The exit codes are data, read out of the binary like the script reads
    // them out of the exit-codes helper. A case the table does not carry is
    // a failure to read it, never a guess.
    let codes_doc = match run_combined(
        binary,
        &["system", "info", "--format", "{{json .ExitCodes}}"],
        60,
    ) {
        Some((0, text)) if !text.trim().is_empty() => text,
        _ => {
            eprintln!("SKIP: cannot read podbox's exit-code table");
            return 2;
        }
    };
    let mut codes = BTreeMap::new();
    if let Some(Json::Array(rows)) = parse_json(&codes_doc) {
        for row in &rows {
            if let (Some(Json::Str(case)), Some(Json::Number(code))) =
                (field(row, "case"), field(row, "code"))
            {
                if let Ok(number) = code.parse::<i32>() {
                    codes.insert(case.clone(), number);
                }
            }
        }
    }
    let runtime_error = match codes.get("runtime-error") {
        Some(code) => *code,
        None => {
            eprintln!("SKIP: cannot read podbox's exit-code table");
            return 2;
        }
    };
    let flag_error = match codes.get("flag-error") {
        Some(code) => *code,
        None => {
            eprintln!("SKIP: cannot read podbox's exit-code table");
            return 2;
        }
    };

    // The table, read out of the binary. Every clause starts here, so a row
    // the binary does not publish is a row this binary cannot drive.
    let parity_doc = match run_combined(
        binary,
        &["system", "info", "--format", "{{json .Parity}}"],
        60,
    ) {
        Some((0, text)) if !text.trim().is_empty() => text,
        _ => {
            eprintln!("SKIP: the parity table did not print");
            return 2;
        }
    };
    let rows = match parse_json(&parity_doc) {
        Some(Json::Array(rows)) => rows,
        _ => {
            eprintln!("SKIP: the parity table did not parse");
            return 2;
        }
    };

    report.say("== 0. the tier rows are data");
    for (verb, flag) in [
        ("run", "--podbox-tier"),
        ("run", "--podbox-qemu-arg"),
        ("exec", "--podbox-tier"),
        ("exec", "--podbox-qemu-arg"),
        ("podvm", "-"),
    ] {
        if table_carries(&rows, verb, flag) {
            report.pass(&format!("table carries {verb} {flag}"));
        } else {
            report.miss(&format!("table has no {verb} {flag} row"));
        }
    }

    report.say("");
    report.say("== 1. both help texts list one set of verbs");
    let podbox_help = run_combined(binary, &["--help"], 60).map(|(_, text)| text);
    let podvm_help = run_combined(twin, &["--help"], 60).map(|(_, text)| text);
    match (podbox_help, podvm_help) {
        (Some(left), Some(right)) if left == right => {
            report.pass("podbox --help and podvm --help are the same text")
        }
        _ => report.miss("the two help texts differ"),
    }
    // The word grep over the podvm text, like the script.
    let words = podvm_help_words(twin);
    for word in ["run", "exec", "podvm", "--podbox-tier"] {
        if words.contains(word) {
            report.pass(&format!("podvm --help names {word}"));
        } else {
            report.miss(&format!("podvm --help never names {word}"));
        }
    }

    // run VERB ARGS...: the words, the code, and which of two markers the
    // output carries. The flag-error code is also the runtime code
    // (TODO/cli.md T-0802), so the number alone proves nothing. Every call
    // is bounded: a refusal that hangs is a hang with a green label on it.
    let run = |args: &[&str]| run_combined(binary, args, 120).unwrap_or((124, String::new()));
    let pvm = |args: &[&str]| run_combined(twin, args, 120).unwrap_or((124, String::new()));

    report.say("");
    report.say("== 2. the flag routes");
    let (rc, text) = run(&[
        "run",
        "--podbox-tier=machine",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == runtime_error && text.contains("machine tier refused") {
        report.pass("run --podbox-tier=machine refuses naming the legs");
    } else {
        report.miss(&format!("run --podbox-tier=machine (rc={rc}): {text}"));
    }
    let (rc, text) = run(&[
        "run",
        "--podbox-tier=sandbox",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == flag_error && text.contains("takes machine or chroot") {
        report.pass("run --podbox-tier=sandbox is a flag error");
    } else {
        report.miss(&format!("run --podbox-tier=sandbox (rc={rc}): {text}"));
    }
    let (rc, text) = run(&[
        "run",
        "--podbox-qemu-arg=--cpu,host",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == flag_error && text.contains("needs --podbox-tier=machine") {
        report.pass("run --podbox-qemu-arg without the machine tier is a flag error");
    } else {
        report.miss(&format!(
            "run --podbox-qemu-arg without the tier (rc={rc}): {text}"
        ));
    }
    let (rc, text) = run(&[
        "run",
        "--podbox-tier=machine",
        "--podbox-qemu-arg=--cpu,host",
        "--podbox-qemu-arg",
        "a b",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == runtime_error && text.contains("machine tier refused") {
        report.pass("repeated qemu args with a space ride the machine tier to its refusal");
    } else {
        report.miss(&format!(
            "run --podbox-tier=machine with qemu args (rc={rc}): {text}"
        ));
    }

    report.say("");
    report.say("== 3. the name routes");
    let (rc, text) = pvm(&["run", "never-pulled-145:tag", "/bin/true"]);
    if rc == runtime_error
        && text.contains("machine tier refused")
        && text.contains("invoked as `podvm`")
    {
        report.pass("podvm run defaults to the machine tier and says its name");
    } else {
        report.miss(&format!("podvm run (rc={rc}): {text}"));
    }
    let (_, text) = pvm(&[
        "run",
        "--podbox-tier=chroot",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if text.contains("wins over the podvm default") && !text.contains("machine tier refused") {
        report
            .pass("podvm run --podbox-tier=chroot states the override and proceeds past the tier");
    } else {
        report.miss(&format!("podvm run --podbox-tier=chroot: {text}"));
    }

    report.say("");
    report.say("== 4. exec mirrors run, and other verbs refuse the flag as unlisted");
    let (rc, text) = run(&[
        "exec",
        "--podbox-tier=machine",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == runtime_error && text.contains("machine tier refused") {
        report.pass("exec --podbox-tier=machine refuses naming the legs");
    } else {
        report.miss(&format!("exec --podbox-tier=machine (rc={rc}): {text}"));
    }
    let (rc, text) = pvm(&["exec", "never-pulled-145:tag", "/bin/true"]);
    if rc == runtime_error && text.contains("machine tier refused") {
        report.pass("podvm exec defaults to the machine tier");
    } else {
        report.miss(&format!("podvm exec (rc={rc}): {text}"));
    }
    let (rc, text) = run(&["ps", "--podbox-tier=machine"]);
    if rc == flag_error && text.contains("no row in the parity table") {
        report.pass("ps --podbox-tier is refused as unlisted");
    } else {
        report.miss(&format!("ps --podbox-tier (rc={rc}): {text}"));
    }
    0
}

fn podvm_help_words(twin: &str) -> BTreeSet<String> {
    run_combined(twin, &["--help"], 60)
        .map(|(_, text)| help_words(&text))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_rows_match_verb_and_flag_prefix() {
        let rows = vec![
            Json::Object(vec![
                ("verb".to_string(), Json::Str("run".to_string())),
                ("flag".to_string(), Json::Str("--podbox-tier".to_string())),
            ]),
            Json::Object(vec![("verb".to_string(), Json::Str("podvm".to_string()))]),
        ];
        assert!(table_carries(&rows, "run", "--podbox-tier"));
        assert!(table_carries(&rows, "podvm", "-"));
        assert!(!table_carries(&rows, "run", "--podbox-qemu-arg"));
        assert!(!table_carries(&rows, "exec", "--podbox-tier"));
    }

    #[test]
    fn help_words_split_like_the_script_greps() {
        let words = help_words("run exec podvm --podbox-tier\n  spaced words");
        for word in ["run", "exec", "podvm", "--podbox-tier", "spaced", "words"] {
            assert!(words.contains(word));
        }
    }

    #[test]
    fn exit_codes_and_parity_parse() {
        let codes = parse_json("[{\"case\": \"runtime-error\", \"code\": 1}]").unwrap();
        match &codes {
            Json::Array(rows) => {
                assert_eq!(
                    field(&rows[0], "case"),
                    Some(&Json::Str("runtime-error".to_string()))
                );
                assert_eq!(
                    field(&rows[0], "code"),
                    Some(&Json::Number("1".to_string()))
                );
            }
            _ => panic!("array expected"),
        }
        assert!(parse_json("[{\"a\": }]").is_none());
        assert!(parse_json("{\"verb\": \"run\"} trailing").is_none());
    }
}
