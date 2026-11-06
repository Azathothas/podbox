//! podvm-buildstate: record build inputs and verify the output bytes.
//!
//! A behaviour-preserving port of scripts/build-state.py (T-1559). The script
//! stays as a compat shim that execs this binary; the logic lives here. The
//! record format is byte-identical: `{"inputs": ..., "outputs": {...},
//! "schema": "podvm-build/1"}` with sorted keys and CPython separators,
//! plus a trailing newline, written atomically through a `.new` sibling.
//!
//! Usage: podvm-buildstate {inputs|record|status|commit}
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

#[path = "buildstate/state.rs"]
mod state;
use state::*;

fn main() {
    let options = match parse_args() {
        Ok(options) => options,
        Err(e) => fail(e),
    };
    let root = options.root;
    let binary = options.binary.unwrap_or_else(|| {
        root.join("target")
            .join(&options.target)
            .join("release/podvm")
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
            if let Some(parent) = destination.parent()
                && let Err(e) = fs::create_dir_all(parent)
            {
                fail(format!("cannot create {}: {e}", parent.display()));
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
mod tests;
