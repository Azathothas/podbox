//! Embed the two interposer objects, or embed nothing and say so.
//!
//! [`TODO/interpose.md`](../../TODO/interpose.md) T-0702. `podbox-interpose` is
//! not a workspace member and its musl object needs `zig cc` as its linker, so
//! `scripts/build-interpose.sh` builds it and this script only carries what
//! that script left behind.
//!
//! ⛔ **IT DOES NOT INVOKE CARGO, AND THAT IS A RULING RATHER THAN AN
//! OVERSIGHT.** T-0702's `Status note` recommended a `build.rs` that runs the
//! script, which means a cargo inside a cargo. `experiments/158-interpose-embedding.sh`
//! measured that on 2026-09-12 and it completed in both shapes it tried, so the
//! hazard did not fire here. ⚠ It is still refused: the deadlock is a property
//! of whoever's machine runs it, the objects are ALREADY built outside cargo by
//! `scripts/dev.sh` and by the gate workflow, and a build step that only copies
//! cannot deadlock anywhere.
//!
//! ⛔ **A MISSING OBJECT IS NOT AN ERROR HERE.** A plain `cargo build` on a
//! fresh clone, on a machine with no zig and no musl target, has to work:
//! `cargo test --workspace`, the acceptance and every contributor's first
//! command all run through it. So an absent object becomes an EMPTY file in
//! `OUT_DIR`, `include_bytes!` still compiles, and the run-time path refuses
//! the payload with a named reason rather than pretending to interpose. ⚠ That
//! refusal is [`TODO/interpose.md`](../../TODO/interpose.md) T-0706's channel
//! and it is the whole reason an empty object is allowed to exist.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Hex sha256 of the embedded object bytes.
fn hex_digest(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut s = String::with_capacity(out.len() * 2);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// The ELF `e_machine` of the embedded object bytes, as `0x` hex, or
/// `absent` where the bytes are no ELF at all. The smoke reports what
/// the binary embedded, per arch (TODO/interpose.md T-1327): magic at
/// 0..4, little-endian `e_machine` at 18..20.
fn machine_of(bytes: &[u8]) -> String {
    if bytes.len() >= 20 && bytes[0..4] == [0x7f, b'E', b'L', b'F'] {
        format!("0x{:x}", u16::from_le_bytes([bytes[18], bytes[19]]))
    } else {
        "absent".to_string()
    }
}

/// The two objects, by the libc they are linked against.
///
/// ⚠ The triples are the ones `scripts/build-interpose.sh` builds by default.
/// A tree that overrides its `TARGETS` gets placeholders here, which is the
/// same honest outcome as a tree that never ran it.
const OBJECTS: &[(&str, &str)] = &[
    ("gnu", "x86_64-unknown-linux-gnu"),
    ("musl", "x86_64-unknown-linux-musl"),
];

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    // `crates/podbox-cli` -> `crates` -> the repository root.
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits two levels under the root")
        .to_path_buf();

    let mut embedded = 0;
    for (name, triple) in OBJECTS {
        let from = root
            .join("crates/podbox-interpose/target")
            .join(triple)
            .join("release/libpodbox_interpose.so");
        // ⛔ Declared whether or not it exists. Cargo reruns this script when a
        // watched path APPEARS, which is what makes a later
        // `scripts/build-interpose.sh` reach a binary that was built without it.
        println!("cargo::rerun-if-changed={}", from.display());
        let to = out.join(format!("interpose-{name}.so"));
        let bytes = match std::fs::read(&from) {
            Ok(bytes) if !bytes.is_empty() => bytes,
            _ => {
                // ⚠ Zero bytes, and the run-time path reads that as absent.
                // Not a stub object: a stub that loads and does nothing is the
                // silent degradation `TOOL.md` section 4.1 forbids.
                std::fs::write(&to, b"").expect("writing the placeholder");
                println!(
                    "cargo::warning=no {name} interposer at {}. \
                     Run ./scripts/build-interpose.sh; until then podbox \
                     declines to interpose and says so (TODO/interpose.md T-0702)",
                    from.display()
                );
                Vec::new()
            }
        };
        if !bytes.is_empty() {
            std::fs::write(&to, &bytes).expect("writing the embedded object");
            embedded += 1;
        }
        // The digest `version --verbose` reports (T-1004), or `absent`
        // where the placeholder went in. A digest of nothing would be a
        // value that looks measured and is not.
        println!(
            "cargo::rustc-env=PODBOX_INTERPOSE_{}={}",
            name.to_ascii_uppercase(),
            if bytes.is_empty() {
                "absent".to_string()
            } else {
                hex_digest(&bytes)
            }
        );
        // The object's architecture beside its digest (T-1327): the
        // smoke asserts the embed matches the artefact, per arch.
        println!(
            "cargo::rustc-env=PODBOX_INTERPOSE_{}_MACHINE={}",
            name.to_ascii_uppercase(),
            if bytes.is_empty() {
                "absent".to_string()
            } else {
                machine_of(&bytes)
            }
        );
    }
    // ⚠ Reported for the record, and nothing conditions on the count: the
    // run-time path reads the bytes it has rather than a compile-time flag,
    // so one object present and one absent is a state podbox can describe.
    println!("cargo::rustc-env=PODBOX_INTERPOSE_EMBEDDED={embedded}");

    build_info(&root);
}

/// Record the inputs that produced this binary for `version --verbose`
/// ([`TODO/packaging.md`](../../../TODO/packaging.md) T-1004). Every value
/// is emitted unconditionally: [`crate::version`] reads them with `env!`,
/// which is a compile error on a missing variable, so `unknown` is a
/// value and never an absence.
fn build_info(root: &Path) {
    println!(
        "cargo::rustc-env=PODBOX_BUILD_COMMIT={}",
        build_commit(root)
    );
    println!("cargo::rustc-env=PODBOX_BUILD_RUSTC={}", build_rustc());
    println!(
        "cargo::rustc-env=PODBOX_BUILD_TARGET={}",
        std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string())
    );
    println!(
        "cargo::rustc-env=PODBOX_BUILD_CRT_STATIC={}",
        match std::env::var("CARGO_CFG_TARGET_FEATURE") {
            Ok(f) if f.split(',').any(|s| s == "crt-static") => "yes",
            _ => "no",
        }
    );
}

/// The git commit built, with `-dirty` where the tree is modified, or
/// `unknown` where no commit is readable. `PODBOX_BUILD_COMMIT` overrides
/// (a byte-exact rebuild names its own input); otherwise the build reads
/// the checkout it runs in, which the lane keeps beside the sources.
fn build_commit(root: &Path) -> String {
    println!("cargo::rerun-if-env-changed=PODBOX_BUILD_COMMIT");
    if let Ok(pinned) = std::env::var("PODBOX_BUILD_COMMIT") {
        return pinned;
    }
    let head = root.join(".git/HEAD");
    // ⚠ Declared whether or not it exists: a tree without `.git` builds
    // with `unknown` rather than rebuilding pointlessly on every edit.
    println!("cargo::rerun-if-changed={}", head.display());
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(root)
        .output();
    let commit = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => return "unknown".to_string(),
    };
    let dirty = std::process::Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(root)
        .output()
        .map(|o| !o.stdout.iter().all(|b| b.is_ascii_whitespace()))
        .unwrap_or(false);
    if dirty {
        format!("{commit}-dirty")
    } else {
        commit
    }
}

/// `rustc --version` of the building toolchain, or `unknown`. Cargo names
/// the compiler in `RUSTC`; nothing is probed beyond reading it.
fn build_rustc() -> String {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}
