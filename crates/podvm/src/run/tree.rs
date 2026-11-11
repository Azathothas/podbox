//! The container's own rootfs (TODO/image.md T-1709).
//!
//! `prepare` copies the extracted image into `<store>/trees/<id>/` before
//! anything writes in it: the interposer object, the completion fixups and
//! the payload all write in the copy, never in the image. The guard
//! removes the copy when `prepare` refuses after the fill; once `prepare`
//! succeeds, the container record owns it, and `podvm_supervise::remove`
//! removes it with the record.

use podvm_extract::tree::Fill;

/// A filled tree that is removed on drop, until [`Tree::keep`].
pub(crate) struct Tree<'a> {
    store: &'a podvm_image::Store,
    pub(crate) fill: Fill,
    kept: bool,
}

impl<'a> Tree<'a> {
    /// Fill a tree from the extracted image `digest`. The caller holds the
    /// image lock (T-0204).
    pub(crate) fn fill(
        verb: &str,
        store: &'a podvm_image::Store,
        digest: &str,
    ) -> Result<Self, i32> {
        match podvm_extract::tree::fill(store, digest) {
            Ok(fill) => Ok(Tree {
                store,
                fill,
                kept: false,
            }),
            Err(e) => {
                eprintln!(
                    "podvm {verb}: the container's own rootfs could not be made under {}: {e} \
                     (TODO/image.md T-1709)",
                    podvm_extract::tree::trees(store).display()
                );
                Err(podvm_image::error::EXIT_RUNTIME_ERROR)
            }
        }
    }

    /// The host path of the rootfs.
    pub(crate) fn rootfs(&self) -> String {
        self.fill.rootfs.to_string_lossy().to_string()
    }

    /// The tree outlives this guard: a record owns it now.
    pub(crate) fn keep(mut self) {
        self.kept = true;
    }
}

impl Drop for Tree<'_> {
    fn drop(&mut self) {
        if !self.kept {
            let _ = podvm_extract::tree::remove(self.store, &self.rootfs());
        }
    }
}

/// Remove the tree of a prepared run that no record took (a refused
/// `create` or foreground record).
pub(crate) fn drop_unrecorded(store: &podvm_image::Store, rootfs: &str) {
    let _ = podvm_extract::tree::remove(store, rootfs);
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
