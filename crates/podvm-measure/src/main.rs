//! podvm-parity: `podvm --help`, `docker --help` and `podman --help` list
//! one set of verbs: one binary, answering to every name it is installed as.
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
//! Usage: podvm-parity [--root DIR] [--bin PATH]
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

#[path = "parity/drive.rs"]
mod drive;
use drive::*;

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
            eprintln!("podvm-parity: unexpected argument: {arg}");
            std::process::exit(2);
        }
    }
    let root = match root_flag {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        Some(_) => {
            eprintln!("podvm-parity: --root needs a directory");
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
            eprintln!("podvm-parity: --bin needs a path");
            std::process::exit(2);
        }
        None => None,
    };
    let code = drive(&root, bin);
    std::process::exit(code);
}

#[cfg(test)]
mod tests;
