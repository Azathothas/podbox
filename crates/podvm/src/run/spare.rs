//! The spare tree of an image, TODO/image.md T-1781.
//!
//! A run copies the extracted image into its own tree (T-1709), which is
//! the cost of a large image: 66 % of `run --rm golang true` on the cloud
//! host (experiment 442). After a run takes its tree, podvm fills the next
//! one for the same image in a detached `podvm tree-prefill`, and the next
//! run of the image takes it with one rename.
//!
//! ⭐ The prefill starts at podvm's exit ([`spawn_pending`]), not during the
//! run: the launcher of a rung kills each process of its tree at the end
//! of the payload, and a prefill must not compete with the payload. It runs
//! at the lowest CPU priority and in the idle I/O class, and holds the
//! image (T-0204), so a `prune` does not remove it during the copy.
//! `PODVM_SPARE_TREE=0` turns the spare off: a spare takes the disk of one
//! more tree for each image that ran.

use std::path::PathBuf;
use std::sync::Mutex;

/// The image whose spare podvm fills at its exit: the store and the digest.
static PENDING: Mutex<Option<(PathBuf, String)>> = Mutex::new(None);

/// Whether the spare is on (`PODVM_SPARE_TREE` is not `0`).
pub(crate) fn enabled() -> bool {
    std::env::var_os("PODVM_SPARE_TREE").is_none_or(|v| v != "0")
}

/// Fill the spare of `digest` at podvm's exit.
pub(crate) fn note(store: &podvm_image::Store, digest: &str) {
    if let Ok(mut p) = PENDING.lock() {
        *p = Some((store.root().to_path_buf(), digest.to_string()));
    }
}

/// At podvm's exit: start the detached prefill of the noted image.
pub(crate) fn spawn_pending() {
    use std::os::unix::process::CommandExt;
    if cfg!(test) {
        return;
    }
    let pending = PENDING.lock().ok().and_then(|mut p| p.take());
    let trash = podvm_extract::tree::trashed();
    let (root, digest) = match (pending, trash) {
        (Some((root, digest)), _) => (root, digest),
        (None, Some(root)) => (root, String::new()),
        (None, None) => return,
    };
    let Ok(me) = std::env::current_exe() else {
        return;
    };
    let null = std::process::Stdio::null;
    let _ = std::process::Command::new(me)
        .arg("tree-prefill")
        .arg(root)
        .args((!digest.is_empty()).then_some(digest))
        .stdin(null())
        .stdout(null())
        .stderr(null())
        .process_group(0)
        .spawn();
}

/// `podvm tree-prefill STORE [DIGEST]`: empty the trash of the store, then
/// fill the spare of the image, at the lowest priority. Exit 0 where each
/// step held.
pub(crate) fn main(args: &[String]) -> i32 {
    unsafe extern "C" {
        fn setpriority(which: i32, who: u32, prio: i32) -> i32;
        fn syscall(n: i64, ...) -> i64;
    }
    // SAFETY: plain system calls on this process. 251 is ioprio_set on
    // x86_64 and 30 on aarch64; class 3 (idle) << 13.
    unsafe {
        setpriority(0, 0, 19);
        let nr = if cfg!(target_arch = "aarch64") {
            30
        } else {
            251
        };
        syscall(nr, 1i64, 0i64, 3i64 << 13);
    }
    let (root, digest) = match args {
        [root] => (root, None),
        [root, digest] => (root, Some(digest)),
        _ => return podvm_image::error::EXIT_FLAG_ERROR,
    };
    let Ok(store) = podvm_image::Store::open(root.as_str()) else {
        return 1;
    };
    podvm_extract::tree::empty_trash(&store);
    let Some(digest) = digest else {
        return 0;
    };
    let Some(record) = store
        .list()
        .ok()
        .and_then(|l| l.into_iter().find(|r| &r.manifest_digest == digest))
    else {
        return 1;
    };
    let Ok(_held) = store.hold(&record) else {
        return 1;
    };
    if !podvm_extract::is_extracted(&store, digest) {
        return 1;
    }
    match podvm_extract::tree::prefill(&store, digest) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}
