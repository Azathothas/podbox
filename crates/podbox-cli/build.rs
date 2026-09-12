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
        match std::fs::read(&from) {
            Ok(bytes) if !bytes.is_empty() => {
                std::fs::write(&to, &bytes).expect("writing the embedded object");
                embedded += 1;
            }
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
            }
        }
    }
    // ⚠ Reported for the record, and nothing conditions on the count: the
    // run-time path reads the bytes it has rather than a compile-time flag,
    // so one object present and one absent is a state podbox can describe.
    println!("cargo::rustc-env=PODBOX_INTERPOSE_EMBEDDED={embedded}");
}
