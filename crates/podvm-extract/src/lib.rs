//! `TOOL.md` section 6.3: layers, whiteouts, ownership sidecar, path safety.
//! Milestone M2, [`TODO/milestones.md`](../../../TODO/milestones.md) T-1103 and
//! [`TODO/extract.md`](../../../TODO/extract.md) T-0301 to T-0307.
//!
//! `pod-extract` (PodUtils/podcrates) applies the layers: the entry-level
//! extraction, the ownership sidecar, the basename whiteouts, the
//! containment, the rebased symlink targets, the owner bits between layers,
//! the whiteouts before their layer's entries, the digest of each blob while
//! it streams (T-1315) and the completion marker. This crate gives it
//! podvm's store: the layers of a manifest as blob paths, the layout of an
//! image's rootfs under the store, and podvm's settings (`$PODVM_STORE` in
//! the space message).
//!
//! ⛔ **The sidecar changes no kernel permission check.** A file recorded as
//! `gid 42` is owned on disk by the id that extracted it, and every access
//! check the kernel makes uses that one.

#![forbid(unsafe_op_in_unsafe_fn)]

pub mod error;
pub mod tree;

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use podvm_image::{oci, Store};

pub use error::{Error, Result};
pub use pod_extract::{
    apply, layer, remove, safety, sidecar, whiteout, Extracted, DONE_MARKER, EXTRACT_HEADROOM_BYTES,
};

/// Where an image's rootfs and its sidecar live, given the store root.
///
/// ⛔ The sidecar sits BESIDE the rootfs, not inside it. A file inside the
/// rootfs is a file the payload can read, write and be confused by, and
/// T-0302's `Prove` reads it at `<rootfs>/../.meta.jsonl` for that reason.
pub fn paths(store: &Store, digest: &str) -> (PathBuf, PathBuf) {
    let p = place(store, digest);
    (p.rootfs, p.sidecar)
}

/// The rootfs, the sidecar and the completion marker of an image, under
/// `<store>/rootfs/<digest>/`.
fn place(store: &Store, digest: &str) -> pod_extract::Place {
    pod_extract::Place::under(&store.root().join("rootfs").join(digest.replace(':', "_")))
}

/// Extract every layer of `manifest` into the store, in order.
///
/// The space precheck (T-0203's second call site) is in `pod_extract::extract`,
/// before its per-layer loop. Each layer's line (`<digest>: Extracted (N
/// entries, M whiteouts)`) reaches `out` when the extraction ends, in order,
/// also when it fails.
pub fn extract(
    store: &Store,
    manifest: &oci::Manifest,
    digest: &str,
    out: &mut dyn Write,
) -> Result<Extracted> {
    let mut layers = Vec::new();
    for d in &manifest.layers {
        // `?`: the error of the digest is the store's or `pod-oci`'s, and
        // `Error` takes both.
        let parsed = d.parsed_digest()?;
        layers.push(pod_extract::Layer {
            digest: parsed.to_string(),
            media_type: d.media_type.clone(),
            size: d.size,
            blob: store.blob_path(&parsed),
        });
    }
    let lines = Arc::new(Mutex::new(Vec::new()));
    let keep = Arc::clone(&lines);
    let settings = pod_extract::Settings {
        host: podvm_probe::settings(),
        report: pod_core::Report {
            events: Arc::new(move |e: pod_core::Event| {
                if let Ok(mut l) = keep.lock() {
                    l.push(e.text);
                }
            }),
            ..pod_core::Report::default()
        },
        ..pod_extract::Settings::default()
    };
    let done = pod_extract::extract(&settings, digest, &layers, &place(store, digest));
    if let Ok(l) = lines.lock() {
        for line in l.iter() {
            let _ = writeln!(out, "{line}");
        }
    }
    Ok(done?)
}

/// Has this image already been extracted, completely?
///
/// ⛔ The marker, not the directory. See [`DONE_MARKER`].
pub fn is_extracted(store: &Store, digest: &str) -> bool {
    place(store, digest).is_extracted()
}

/// Remove an extracted rootfs and its sidecar.
///
/// ⛔ The removal never follows a symlink out of the tree it is clearing:
/// this tree was written from a hostile archive.
pub fn remove_extracted(store: &Store, digest: &str) -> Result<bool> {
    let p = place(store, digest);
    let Some(base) = p.rootfs.parent() else {
        return Ok(false);
    };
    Ok(pod_extract::remove_tree(base)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sidecar_sits_beside_the_rootfs_not_inside_it() {
        let s = Store::open("/tmp/podvm-test-store-paths").unwrap();
        let (rootfs, side) = paths(&s, "sha256:abc123");
        // T-0302's Prove reads `<rootfs>/../.meta.jsonl`.
        assert_eq!(rootfs.parent().unwrap().join(".meta.jsonl"), side);
        // ⛔ A colon is not a portable path character and the digest carries
        // one.
        assert!(!rootfs.to_string_lossy().contains(':'));
    }
}
