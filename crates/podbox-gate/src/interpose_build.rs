//! podbox-interpose-build: both interposer objects, as a binary.
//!
//! A behaviour-preserving port of `scripts/build-interpose.sh` (T-1557),
//! plus the `struct stat` and `struct statx` offset check the ownership
//! experiment carries as its check B. Arguments are parsed by hand; `clap`
//! was ruled out at T-0908. The crate carries no dependencies.
//!
//! One object per libc: a preloaded object is loaded by the payload's own
//! dynamic loader and resolves its own imports against the payload's libc,
//! so a musl-linked object cannot be preloaded into a glibc payload.
//! podbox embeds both and selects on the payload's PT_INTERP.
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
// The object loads into payloads back to this glibc, so no needed version
// above it may enter. A new import the build host stamps higher fails here
// with the version named, rather than refusing somebody's container.
const GLIBC_CEILING: &str = "GLIBC_2.27";
// The numbers `crates/podbox-interpose/src/lib.rs` carries, asserted by
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
            if let Some(open) = line.find('[') {
                if let Some(close) = line[open..].find(']') {
                    names.push(line[open + 1..open + close].to_string());
                }
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

fn set_skip(rc: &mut i32) {
    // A failure outranks a skip, so a skip never clears one.
    if *rc != 1 {
        *rc = 2;
    }
}

fn build_one(root: &Path, krate: &Path, target: &str, ceiling: &str, tmp: &Path) -> i32 {
    let mut rc = 0;
    if !target_installed(target, root) {
        eprintln!("SKIP {target}: target not installed");
        return 2;
    }
    println!("== {target}");
    let zig_linker = root.join("scripts/zig-cc.sh");
    let mut rustflags = "-C target-feature=-crt-static".to_string();
    let mut stub_env: Vec<(String, String)> = Vec::new();
    if target.contains("musl") {
        if command_present("zig") && zig_linker.is_file() {
            rustflags.push_str(&format!(" -C linker={}", zig_linker.display()));
            let zig_version = capture("zig", &["version".to_string()], root, &[])
                .map(|(_, stdout, _)| stdout.trim().to_string())
                .unwrap_or_default();
            println!("   linker: scripts/zig-cc.sh (zig {zig_version})");
        } else {
            eprintln!("SKIP {target}: no zig, and the default linker produces a GLIBC object");
            eprintln!("      ./scripts/common/bootstrap-env.sh zig");
            return 2;
        }
    } else if target.contains("gnu") {
        // THE `libdl` IMPORT LIBRARY. The `dlsym` reference must bind the
        // library that defines it on old systems, and on a modern host that
        // is not `libc.so.6`. The stub carries SONAME libdl.so.2 with no
        // implementation, so the reference binds libdl.so.2, which glibc
        // keeps as a compatibility object on both sides of the merge. musl
        // has no `libdl`, so this stays on the glibc arm the way `zig cc`
        // stays on the musl one.
        let stub_dir = krate.join("target/dl-stub");
        if fs::create_dir_all(&stub_dir).is_err() {
            eprintln!("SKIP {target}: no stub directory");
            return 2;
        }
        let stub = stub_dir.join("libdl-stub.so");
        let stub_rc = Command::new("cc")
            .args([
                "-shared",
                "-nostdlib",
                &format!("-Wl,--version-script={}", krate.join(DL_STUB_MAP).display()),
                "-Wl,-soname,libdl.so.2",
                "-o",
                &stub.to_string_lossy(),
                &krate.join(DL_STUB_SRC).to_string_lossy(),
            ])
            .current_dir(root)
            .stdin(Stdio::null())
            .status()
            .map(|status| status.code().unwrap_or(1))
            .unwrap_or(127);
        if stub_rc != 0 {
            eprintln!("FAIL {target}: the libdl stub did not build");
            return 1;
        }
        let soname = capture(
            "readelf",
            &["-dW".to_string(), stub.to_string_lossy().into_owned()],
            root,
            &[],
        )
        .and_then(|(_, stdout, _)| {
            stdout
                .lines()
                .find(|line| line.contains("SONAME"))
                .and_then(|line| {
                    let open = line.find('[')?;
                    let close = line[open..].find(']')?;
                    Some(line[open + 1..open + close].to_string())
                })
        })
        .unwrap_or_default();
        println!("   stub: {} (SONAME {soname})", stub.display());
        // PREPENDED THROUGH A LINKER WRAPPER, not a late link argument: user
        // link arguments travel after the objects, so by the time the stub
        // is seen the `dlsym` reference is already bound.
        rustflags.push_str(&format!(
            " -C linker={}",
            root.join("scripts/gnu-link-stub.sh").display()
        ));
        stub_env.push((
            "PODBOX_DL_STUB".to_string(),
            stub.to_string_lossy().into_owned(),
        ));
    }
    // THE VERSION SCRIPT. Without it a Rust cdylib exports its runtime
    // into every process it is loaded into, and this object is loaded into
    // every process of a container. An ABSOLUTE path: the build runs with
    // the crate as its working directory and the linker is invoked from
    // somewhere else again.
    rustflags.push_str(&format!(
        " -C link-arg=-Wl,--version-script={}",
        krate.join(MAP).display()
    ));
    let so = krate
        .join("target")
        .join(target)
        .join("release")
        .join("libpodbox_interpose.so");
    let build_rc = Command::new("cargo")
        .args(["build", "--release", "--target", target])
        .env("RUSTFLAGS", &rustflags)
        .envs(stub_env.iter().cloned())
        .current_dir(krate)
        .status()
        .map(|status| status.code().unwrap_or(1))
        .unwrap_or(127);
    if build_rc != 0 {
        eprintln!("FAIL {target}");
        return 1;
    }
    if let Ok(meta) = fs::metadata(&so) {
        println!("   {} {}", meta.len(), so.display());
    }
    let needed = dt_needed(&so, root);
    println!("   DT_NEEDED: {}", needed.join(" "));
    let want = want_libc(target);
    if needed.iter().any(|name| name == want) {
        println!(
            "   ok: it names {want}, so it is {}-linked",
            libc_kind(target)
        );
    } else {
        eprintln!(
            "FAIL {target}: DT_NEEDED is [{}] and must name {want}",
            needed.join(" ")
        );
        rc = 1;
    }
    // THE GLIBC CEILING. No needed version above it may enter.
    let newest = newest_need(&glibc_needs(&so, root));
    match newest {
        Some(top) => {
            if version_at_or_below(&top, GLIBC_CEILING) {
                println!("   ok: newest need {top}, within {GLIBC_CEILING}");
            } else {
                eprintln!("FAIL {target}: needs {top}, above the {GLIBC_CEILING} ceiling");
                rc = 1;
            }
        }
        None => println!("   ok: no versioned needs"),
    }
    // `gettid` MUST NOT LEAVE. The crate defines the number itself as a
    // bare assembly label: local binding by default, no dynamic entry, no
    // payload-visible export. Uppercase only: a localized entry stays in
    // `.dynsym` as `t`, which the loader binds inside the object and no
    // payload resolves.
    let leaked = dynamic_names(&so, root).into_iter().any(|(kind, name)| {
        name == "gettid" && kind.len() == 1 && kind.chars().all(|ch| ch.is_ascii_uppercase())
    });
    if leaked {
        eprintln!("FAIL {target}: gettid is dynamically exported");
        rc = 1;
    } else {
        println!("   ok: gettid is not dynamically exported");
    }
    // `libdl` MUST BE NEEDED on glibc. The SONAME carries its own version,
    // which has nothing to do with the `libc.so.6` beside it.
    if target.contains("gnu") {
        if needed.iter().any(|name| name == "libdl.so.2") {
            println!("   ok: libdl.so.2 is needed");
        } else {
            eprintln!(
                "FAIL {target}: DT_NEEDED is [{}] and names no libdl",
                needed.join(" ")
            );
            rc = 1;
        }
    }
    // THE EXPORTED SET. The version script is the source of this list and
    // the object is what is compared with it: a name in the map that the
    // object does not export is a silent non-interposition, and one the
    // object exports that the map does not list is a symbol some other
    // library in the payload's process resolves to podbox. Without this a
    // link that stopped applying the version script produces a working
    // object exporting hundreds of names and exits 0.
    let declared = declared_exports(&krate.join(MAP));
    let safe_target = target.replace(|ch: char| !ch.is_ascii_alphanumeric(), "_");
    let declared_file = tmp.join(format!("podbox-declared-{safe_target}.txt"));
    let exported_file = tmp.join(format!("podbox-exported-{safe_target}.txt"));
    let _ = fs::write(&declared_file, declared.join("\n") + "\n");
    match exported_symbols(&so, root) {
        Some(exported) => {
            let _ = fs::write(&exported_file, exported.join("\n") + "\n");
            if declared == exported {
                println!(
                    "   ok: exports {} names, exactly what interpose.map declares",
                    exported.len()
                );
            } else {
                eprintln!(
                    "FAIL {target}: exports differ from interpose.map (declared {}, exported {})",
                    declared.len(),
                    exported.len()
                );
                let mut only_declared: Vec<&String> = declared
                    .iter()
                    .filter(|name| !exported.contains(name))
                    .collect();
                let mut only_exported: Vec<&String> = exported
                    .iter()
                    .filter(|name| !declared.contains(name))
                    .collect();
                only_declared.sort();
                only_exported.sort();
                for name in only_declared {
                    eprintln!("      < {name}");
                }
                for name in only_exported {
                    eprintln!("      > {name}");
                }
                rc = 1;
            }
        }
        None => {
            eprintln!(
                "FAIL {target}: nm could not read the exports of {}",
                so.display()
            );
            rc = 1;
        }
    }
    let personalities = dynamic_names(&so, root)
        .into_iter()
        .filter(|(_, name)| name == "rust_eh_personality")
        .count();
    if personalities > 0 {
        eprintln!("FAIL {target}: rust_eh_personality is dynamically exported");
        rc = 1;
    } else {
        println!("   ok: rust_eh_personality is not exported");
    }
    // THE SIZE. This asserts the same ceiling where the object is linked,
    // so every dev check and CI build holds it rather than only a clone
    // reading the record.
    let bytes = fs::metadata(&so).map(|meta| meta.len()).unwrap_or(u64::MAX);
    let limit: u64 = ceiling.parse().unwrap_or(u64::MAX);
    if bytes >= limit {
        eprintln!("FAIL {target}: {bytes} bytes is at or over the interpose ceiling of {ceiling}");
        rc = 1;
    } else {
        println!("   ok: {bytes} bytes, under the interpose ceiling of {ceiling}");
    }
    rc
}

const OFF_C: &str = r#"#define _GNU_SOURCE
#include <stdio.h>
#include <sys/stat.h>
#include <stddef.h>
/* NOT <linux/stat.h>. musl's own <sys/stat.h> defines `struct statx` and the
 * kernel header then redefines it; glibc since 2.28 defines it there too under
 * _GNU_SOURCE. Two libcs declaring one kernel struct in two different headers
 * is exactly why podbox's object carries OFFSETS rather than a struct. */
int main(void){
  printf("sizeof=%zu dev=%zu ino=%zu mode=%zu uid=%zu gid=%zu\n",
    sizeof(struct stat), offsetof(struct stat, st_dev), offsetof(struct stat, st_ino),
    offsetof(struct stat, st_mode), offsetof(struct stat, st_uid),
    offsetof(struct stat, st_gid));
  /* statx too, and it is not a duplicate: coreutils' `stat` on a modern
   * glibc asks statx(2) and never reaches `stat`. src/lib.rs carries these
   * six numbers as well, and a check that asserted only the first struct
   * would have left the ones that actually answer a glibc payload unmeasured. */
  printf("statx sizeof=%zu mask=%zu uid=%zu gid=%zu ino=%zu devmaj=%zu devmin=%zu\n",
    sizeof(struct statx), offsetof(struct statx, stx_mask),
    offsetof(struct statx, stx_uid), offsetof(struct statx, stx_gid),
    offsetof(struct statx, stx_ino), offsetof(struct statx, stx_dev_major),
    offsetof(struct statx, stx_dev_minor));
  return 0;
}
"#;

// Check B. `struct stat` AND `struct statx` OFFSETS, under both libcs, by
// `offsetof` rather than by reading a header. A Linux host runs both
// directly; anywhere else this check could not run, and it says so rather
// than answering from one libc twice.
fn check_offsets(root: &Path, tmp: &Path) -> i32 {
    println!("== B. struct stat offsets, by offsetof and under both libcs");
    // NTFS carries no POSIX mode, and chmod on such a checkout is a silent
    // no-op, so the check builds beside the tree rather than assuming the
    // temporary directory honours modes. The binaries built here run on
    // this host directly.
    let source = tmp.join("off.c");
    if fs::write(&source, OFF_C).is_err() {
        println!("  COULD NOT BUILD OR RUN");
        return 2;
    }
    let src = source.to_string_lossy().into_owned();
    let off_g = tmp.join("off_g");
    let off_m = tmp.join("off_m");
    let glibc_built = Command::new("cc")
        .args(["-O1", "-o", &off_g.to_string_lossy(), &src])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
        || (command_present("zig")
            && Command::new(root.join("scripts/zig-cc.sh"))
                .args(["-O1", "-o", &off_g.to_string_lossy(), &src])
                .env("ZIG_TARGET", "x86_64-linux-gnu.2.17")
                .current_dir(root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false));
    let musl_built = command_present("zig")
        && Command::new(root.join("scripts/zig-cc.sh"))
            .args(["-O1", "-o", &off_m.to_string_lossy(), &src])
            .env("ZIG_TARGET", "x86_64-linux-musl")
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    if !glibc_built {
        eprintln!("  the glibc offsetof program did not build (needs cc or zig)");
    }
    if !musl_built {
        eprintln!("  the musl offsetof program did not build (needs zig)");
    }
    // A Linux host runs both directly. Anywhere else they cannot execute
    // here, and the check says so.
    let uname = Command::new("uname")
        .arg("-s")
        .stdin(Stdio::null())
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default();
    let mut g_off = String::new();
    let mut m_off = String::new();
    if uname == "Linux" {
        if glibc_built {
            g_off = run_program(&off_g);
        }
        if musl_built {
            m_off = run_program(&off_m);
        }
    }
    println!(
        "  glibc  {}",
        if g_off.is_empty() {
            "COULD NOT BUILD OR RUN".to_string()
        } else {
            g_off.clone()
        }
    );
    println!(
        "  musl   {}",
        if m_off.is_empty() {
            "COULD NOT BUILD OR RUN".to_string()
        } else {
            m_off.clone()
        }
    );
    // The numbers the interposer carries, asserted here rather than
    // believed. They agree on x86_64 and that is this architecture's
    // property, not a general one.
    if g_off.is_empty() || m_off.is_empty() {
        eprintln!("  one of the two offsetof programs did not build or run");
        2
    } else if g_off == WANT_OFFSETS && m_off == WANT_OFFSETS {
        println!("  B: both libcs agree on BOTH structs, and with the offsets src/lib.rs carries");
        0
    } else {
        eprintln!("  B: the offsets differ from the ones src/lib.rs carries [{WANT_OFFSETS}]");
        1
    }
}

fn run_program(path: &Path) -> String {
    let start = Instant::now();
    let mut child = match Command::new(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return String::new(),
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return String::new();
                }
                let mut buf = Vec::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let _ = pipe.read_to_end(&mut buf);
                }
                return String::from_utf8_lossy(&buf).replace('\n', "|");
            }
            Ok(None) => {
                if start.elapsed() >= Duration::from_secs(60) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return String::new();
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return String::new(),
        }
    }
}

fn print_help() {
    println!("podbox-interpose-build: both interposer objects, exports, and size checks.");
    println!();
    println!("  TARGETS=\"...\" podbox-interpose-build   one object per libc, then check B");
    println!();
    println!("Exit: 0 every requested target built, 1 a build failed, 2 could not run.");
}

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
        eprintln!("podbox-interpose-build takes no arguments; TARGETS selects the pair");
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
    let krate = root.join("crates/podbox-interpose");
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
    let tmp = env::temp_dir().join(format!("podbox-interpose-{}", std::process::id()));
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
