//! podvm-interpose-build: both interposer objects, as a binary.
//!
//! A behaviour-preserving port of `scripts/build-interpose.sh` (T-1557),
//! plus the `struct stat` and `struct statx` offset check the ownership
//! experiment carries as its check B. Arguments are parsed by hand; `clap`
//! was ruled out at T-0908. The crate carries no dependencies.
//!
//! One object per libc: a preloaded object is loaded by the payload's own
//! dynamic loader and resolves its own imports against the payload's libc,
//! so a musl-linked object cannot be preloaded into a glibc payload.
//! podvm embeds both and selects on the payload's PT_INTERP.
//!
//! Exit: 0 every requested target built, 1 a build failed, 2 could not run.
//! `TARGETS` selects the pair, exactly as the script's did. A target that
//! cannot build here is the third state, never a silent pass, and a failure
//! outranks a skip, so a skip never clears one.
//!
//! D-1 option (c): the per-object byte ceiling is declared once in the
//! shell stub, and this binary reads that declaration at runtime. No Rust
//! source repeats its digits: the gate scans for them, so a literal turns
//! the gate red. The same holds for the release binary's own ceiling, which
//! this binary never names.
//!
//! The musl target needs `zig cc` as the linker, and without it this binary
//! produces no musl object at all: with the default linker the musl object
//! would record glibc names in DT_NEEDED, which is byte-for-byte the wrong
//! answer to "one object per libc" and reads as success because the build
//! exits 0. `rustc` passes `-lgcc_s` on the musl target even under
//! `panic = "abort"`, and `musl-tools` ships no musl-linked libgcc, so the
//! host `cc` links it against the host's glibc. `zig cc` carries its own
//! compiler-rt. Where zig is absent the musl arm skips honestly rather than
//! reporting a glibc object under a musl name.
//!
//! RUSTFLAGS, not a crate-local cargo config: cargo merges config files up
//! the directory tree and appends the parent's rustflags after the child's,
//! so a crate-local negative crt-static flag is followed by the root's
//! positive one and loses. The RUSTFLAGS environment variable replaces the
//! config value outright.

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const CEILING_DECL: &str = "INTERPOSE_CEILING_BYTES";
const STUB: &str = "scripts/build-interpose.sh";
const MAP: &str = "interpose.map";
const DL_STUB_MAP: &str = "dl-stub.map";
const DL_STUB_SRC: &str = "dl-stub.S";
const DL_STUB_MAP_A64: &str = "dl-stub-aarch64.map";
const DL_STUB_SRC_A64: &str = "dl-stub-aarch64.S";
// The object loads into payloads back to this glibc, so no needed version
// above it may enter. A new import the build host stamps higher fails here
// with the version named, rather than refusing somebody's container.
const GLIBC_CEILING: &str = "GLIBC_2.27";
// The numbers `crates/podvm-interpose/src/lib.rs` carries, asserted by
// check B rather than believed. They agree on x86_64 and that is this
// architecture's property, not a general one.
const WANT_OFFSETS: &str = "sizeof=144 dev=0 ino=8 mode=24 uid=28 gid=32|statx sizeof=256 mask=0 uid=20 gid=24 ino=32 devmaj=136 devmin=140|";

fn find_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let profile_dir = exe.parent()?;
    let mut candidate = profile_dir.join("..").join("..");
    for _ in 0..8 {
        if candidate.join("TODO").join("INDEX.md").is_file() {
            return Some(candidate);
        }
        candidate = candidate.join("..");
    }
    None
}

fn command_present(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {name}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn capture(
    prog: &str,
    args: &[String],
    cwd: &Path,
    extra_env: &[(String, String)],
) -> Option<(i32, String, String)> {
    let mut cmd = Command::new(prog);
    cmd.args(args).current_dir(cwd);
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let output = cmd.stdin(Stdio::null()).output().ok()?;
    Some((
        output.status.code().unwrap_or(1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

// The ceiling's one home is the stub. Without that line every other file
// naming a size is unanchored.
fn read_ceiling(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root.join(STUB)).ok()?;
    let prefix = format!("{CEILING_DECL}=");
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix(&prefix) {
            let value = rest.trim().trim_matches('"').to_string();
            if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
                return Some(value);
            }
        }
    }
    None
}

fn want_libc(target: &str) -> &str {
    if target.contains("musl") {
        "libc.so"
    } else {
        "libc.so.6"
    }
}

fn libc_kind(target: &str) -> &str {
    if target.contains("musl") {
        "musl"
    } else {
        "glibc"
    }
}

// The exact words of DT_NEEDED, one per line. An exact word: `libc.so.6`
// CONTAINS `libc.so`, so a substring test would call a glibc object
// musl-linked, which is the very mistake being caught.
fn dt_needed(obj: &Path, root: &Path) -> Vec<String> {
    let args = vec!["-dW".to_string(), obj.to_string_lossy().into_owned()];
    let mut names = Vec::new();
    if let Some((_, stdout, _)) = capture("readelf", &args, root, &[]) {
        for line in stdout.lines() {
            if !line.contains("NEEDED") {
                continue;
            }
            if let Some(open) = line.find('[')
                && let Some(close) = line[open..].find(']')
            {
                names.push(line[open + 1..open + close].to_string());
            }
        }
    }
    names
}

fn glibc_needs(obj: &Path, root: &Path) -> Vec<String> {
    let args = vec!["-VW".to_string(), obj.to_string_lossy().into_owned()];
    let mut needs = Vec::new();
    if let Some((_, stdout, _)) = capture("readelf", &args, root, &[]) {
        let mut at = 0;
        let bytes = stdout.as_bytes();
        while at < bytes.len() {
            if stdout[at..].starts_with("GLIBC_") {
                let mut end = at + 6;
                while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
                    end += 1;
                }
                if end > at + 6 {
                    needs.push(stdout[at..end].to_string());
                }
                at = end;
            } else {
                at += 1;
            }
        }
    }
    needs.sort();
    needs.dedup();
    needs
}

// The newest need, by numeric version order: `sort -V` ranks GLIBC_2.14
// above GLIBC_2.3.4, and a lexicographic maximum reports the wrong one.
fn newest_need(needs: &[String]) -> Option<String> {
    needs
        .iter()
        .max_by(|left, right| {
            let left_parts: Vec<u64> = left
                .split('_')
                .next_back()
                .unwrap_or("")
                .split('.')
                .map(|piece| piece.parse::<u64>().unwrap_or(0))
                .collect();
            let right_parts: Vec<u64> = right
                .split('_')
                .next_back()
                .unwrap_or("")
                .split('.')
                .map(|piece| piece.parse::<u64>().unwrap_or(0))
                .collect();
            let width = left_parts.len().max(right_parts.len());
            for index in 0..width {
                let a = *left_parts.get(index).unwrap_or(&0);
                let b = *right_parts.get(index).unwrap_or(&0);
                if a != b {
                    return a.cmp(&b);
                }
            }
            std::cmp::Ordering::Equal
        })
        .cloned()
}

fn version_at_or_below(version: &str, ceiling: &str) -> bool {
    // Compare dotted numeric tails: GLIBC_2.27 against GLIBC_2.30.
    fn parts(text: &str) -> Vec<u64> {
        text.split('_')
            .next_back()
            .unwrap_or("")
            .split('.')
            .map(|piece| piece.parse::<u64>().unwrap_or(u64::MAX))
            .collect()
    }
    let left = parts(version);
    let right = parts(ceiling);
    let width = left.len().max(right.len());
    for index in 0..width {
        let a = *left.get(index).unwrap_or(&0);
        let b = *right.get(index).unwrap_or(&0);
        if a != b {
            return a < b;
        }
    }
    true
}

// The exported set, exactly the entry points: `nm -D --defined-only` text
// symbols beside the map's globals.
fn declared_exports(map: &Path) -> Vec<String> {
    let text = fs::read_to_string(map).unwrap_or_default();
    let mut global = false;
    let mut names = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("global:") {
            global = true;
            continue;
        }
        if trimmed.starts_with("local:") {
            global = false;
            continue;
        }
        if global && trimmed.contains(';') {
            let name: String = trimmed
                .chars()
                .filter(|ch| !ch.is_whitespace() && *ch != ';')
                .collect();
            if !name.is_empty() {
                names.push(name);
            }
        }
    }
    names.sort();
    names
}

fn exported_symbols(obj: &Path, root: &Path) -> Option<Vec<String>> {
    let args = vec![
        "-D".to_string(),
        "--defined-only".to_string(),
        obj.to_string_lossy().into_owned(),
    ];
    let (_, stdout, _) = capture("nm", &args, root, &[])?;
    let mut names = Vec::new();
    for line in stdout.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() == 3 && fields[1] == "T" {
            names.push(fields[2].to_string());
        }
    }
    names.sort();
    Some(names)
}

fn dynamic_names(obj: &Path, root: &Path) -> Vec<(String, String)> {
    let args = vec!["-D".to_string(), obj.to_string_lossy().into_owned()];
    let mut out = Vec::new();
    if let Some((_, stdout, _)) = capture("nm", &args, root, &[]) {
        for line in stdout.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() == 3 {
                out.push((fields[1].to_string(), fields[2].to_string()));
            }
        }
    }
    out
}

fn target_installed(target: &str, root: &Path) -> bool {
    match capture(
        "rustup",
        &[
            "target".to_string(),
            "list".to_string(),
            "--installed".to_string(),
        ],
        root,
        &[],
    ) {
        Some((0, stdout, _)) => stdout.lines().any(|line| line.trim() == target),
        _ => false,
    }
}

#[path = "interpose_build/build.rs"]
mod build;
use build::*;

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    if argv
        .iter()
        .any(|arg| arg == "-h" || arg == "--help" || arg == "help")
    {
        print_help();
        return ExitCode::from(0);
    }
    if !argv.is_empty() {
        eprintln!("podvm-interpose-build takes no arguments; TARGETS selects the pair");
        return ExitCode::from(2);
    }
    let root = match find_root() {
        Some(root) => root,
        None => {
            eprintln!("SKIP: cannot locate the repository root");
            return ExitCode::from(2);
        }
    };
    if !command_present("cargo") {
        eprintln!("SKIP: cargo not on PATH");
        return ExitCode::from(2);
    }
    let krate = root.join("crates/podvm-interpose");
    if !krate.is_dir() {
        eprintln!("SKIP: {} is absent", krate.display());
        return ExitCode::from(2);
    }
    let ceiling = match read_ceiling(&root) {
        Some(ceiling) => ceiling,
        None => {
            eprintln!("SKIP: {STUB} declares no {CEILING_DECL}=<n> line");
            return ExitCode::from(2);
        }
    };
    let targets = env::var("TARGETS")
        .unwrap_or_else(|_| "x86_64-unknown-linux-musl x86_64-unknown-linux-gnu".to_string());
    let tmp = env::temp_dir().join(format!("podvm-interpose-{}", std::process::id()));
    if fs::create_dir_all(&tmp).is_err() {
        eprintln!("SKIP: no temp dir");
        return ExitCode::from(2);
    }
    let mut rc = 0;
    for target in targets.split_whitespace() {
        let one = build_one(&root, &krate, target, &ceiling, &tmp);
        if one == 1 {
            rc = 1;
        } else if one == 2 {
            set_skip(&mut rc);
        }
    }
    let check_b = check_offsets(&root, &tmp);
    if check_b == 1 {
        rc = 1;
    } else if check_b == 2 {
        set_skip(&mut rc);
    }
    let _ = fs::remove_dir_all(&tmp);
    ExitCode::from(rc as u8)
}
