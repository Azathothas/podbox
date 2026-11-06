//! podvm-release-licenses: retain licence text for the locked package set.
//!
//! A behaviour-preserving port of scripts/release-licenses.py (T-1560). The
//! script stays as a compat shim that execs this binary; the logic lives
//! here. `inventory.json` is byte-identical to the script's: `json.dumps`
//! with `indent=2`, insertion-ordered keys, plus a trailing newline.
//!
//! Usage: podvm-release-licenses --output DIR [--root DIR]
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

#[path = "licenses/inventory.rs"]
mod inventory;
use inventory::*;

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
    for manifest in ["Cargo.toml", "crates/podvm-interpose/Cargo.toml"] {
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
            if let Some(parent) = target.parent()
                && let Err(e) = fs::create_dir_all(parent)
            {
                fail(format!("cannot create {}: {e}", parent.display()));
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
#[path = "licenses/tests.rs"]
mod tests;
