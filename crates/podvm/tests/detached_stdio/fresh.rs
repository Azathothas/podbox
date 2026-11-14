//! The stores of `detached_stdio.rs` (TODO/gate.md T-1859).

use super::NEXT;
use std::path::{Path, PathBuf};

/// A fresh store directory, unique across the parallel tests in this file.
pub(crate) fn fresh_store(tag: &str) -> Fresh {
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("podvm-{}-{}-{n}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("could not create store dir {}: {e}", dir.display()));
    Fresh(dir)
}

/// TODO/gate.md T-1859: a store that goes at the drop, also after a
/// panic. A failed test left its store of about 1.3 GB in the temp
/// directory.
pub(crate) struct Fresh(PathBuf);

impl std::ops::Deref for Fresh {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Fresh {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn cleanup(store: &Path) {
    let _ = std::fs::remove_dir_all(store);
}
