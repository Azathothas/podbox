//! The wall bound of a foreground run: `--timeout`, TODO/cli.md T-1655.
//! A detached run has its bound in the launcher (`set_wall`).

use std::path::Path;
use std::time::Duration;

/// The wall bound of a foreground run (`--timeout`, TODO/cli.md T-1655):
/// at the bound, a thread kills each descendant of this process with
/// SIGKILL, as the launcher of a detached run does at its bound.
pub struct Wall {
    stop: std::sync::mpsc::Sender<()>,
    fired: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: std::thread::JoinHandle<()>,
}

/// Arm the wall bound. The thread blocks SIGINT and SIGTERM, so that the
/// waiter of the payload takes them (a mask is per thread).
pub fn arm_wall(wall_ms: u64) -> Wall {
    let (stop, rx) = std::sync::mpsc::channel::<()>();
    let fired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = fired.clone();
    let handle = std::thread::spawn(move || {
        let mut old = 0u64;
        let _ = podvm_probe::sys::sigblock_shutdown(&mut old);
        if rx.recv_timeout(Duration::from_millis(wall_ms))
            != Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        {
            return;
        }
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        let me = i64::from(std::process::id());
        if let Some(tree) = podvm_supervise::tree::descendants(Path::new("/proc"), me) {
            for pid in tree {
                let _ = podvm_probe::sys::kill(pid, 9);
            }
        }
    });
    Wall {
        stop,
        fired,
        handle,
    }
}

impl Wall {
    /// Stop the thread; true where the bound came first.
    pub fn disarm(self) -> bool {
        let _ = self.stop.send(());
        let _ = self.handle.join();
        self.fired.load(std::sync::atomic::Ordering::SeqCst)
    }
}
