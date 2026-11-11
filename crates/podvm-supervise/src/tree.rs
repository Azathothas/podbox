//! The tree of a foreground run. TODO/supervise.md T-1708.
//!
//! A detached run has its launcher, which `pod-supervise` makes the child
//! subreaper of the payload's tree. A foreground run has podvm's own
//! process in that place: it becomes the child subreaper before the
//! payload starts, so a process of the tree whose parent ends (after a
//! `setsid` or a double fork) becomes a child of podvm, not of init. At the
//! end of the payload podvm kills each descendant that stays, as the
//! launcher does.

use std::path::Path;
use std::time::{Duration, Instant};

use podvm_probe::sys;

pub use pod_supervise::launcher::tree::TREE_KILL_MS;

/// Make this process the child subreaper of its descendants. False where
/// the kernel refused: the end then counts `unknown` where a child stays.
pub fn become_subreaper() -> bool {
    sys::set_child_subreaper().is_ok()
}

/// Kill each descendant of this process and reap it, within
/// [`TREE_KILL_MS`]. The count of the processes that stayed: 0 only where
/// `wait4` says that no child is left, `unknown` where `/proc` cannot list
/// the descendants.
pub fn end_own_tree() -> crate::Residual {
    let me = i64::from(std::process::id());
    let deadline = Instant::now() + Duration::from_millis(TREE_KILL_MS);
    loop {
        loop {
            let mut status = 0i32;
            match sys::reap_any(&mut status) {
                Ok(Some(_)) => continue,
                Ok(None) => break,
                Err(e) if e == sys::EINTR => continue,
                Err(e) if e == sys::ECHILD => return crate::Residual::Count(0),
                Err(_) => break,
            }
        }
        let Some(left) = pod_supervise::launcher::tree::descendants(Path::new("/proc"), me) else {
            return crate::Residual::Unknown;
        };
        if Instant::now() >= deadline {
            return crate::Residual::Count(left.len() as u64);
        }
        for pid in &left {
            let _ = sys::kill(*pid, 9);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub use pod_supervise::launcher::tree::descendants;
