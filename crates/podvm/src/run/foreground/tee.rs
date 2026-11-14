//! The payload's stdout, teed to the container log of a foreground run.

/// The payload's stdout, teed to the container log.
///
/// fd 1 is redirected into a pipe whose reader is a pump thread writing
/// every byte to the saved stdout and to the log file, so the payload
/// still owns stdout while `logs` reads the same bytes afterwards.
/// Restored by [`untee`] before the record is finalized, so no byte is
/// still in flight when `logs` runs.
///
/// ⚠ The payload's stdout becomes a pipe, which `isatty` answers
/// differently from a terminal. That is the cost of foreground records,
/// stated here rather than hidden: docker pays it too, for the same
/// reason.
pub(super) struct Tee {
    /// The descriptor the tee took over: fd 1 for a run.
    pub(super) fd: i64,
    pub(super) saved: i64,
    pub(super) handle: std::thread::JoinHandle<()>,
}

/// TODO/supervise.md T-1806: stdout AND stderr, each through its own pump
/// into the one log, which each pump opens for append, so each write lands
/// whole and the two streams interleave in the order of their writes, as
/// the parity row of `logs` says. stderr only reached the terminal, and a
/// payload that wrote to stderr left nothing in the log.
pub(super) fn tee_output_to(log_path: &std::path::Path) -> Result<Vec<Tee>, String> {
    let out = tee_fd_to(1, log_path)?;
    match tee_fd_to(2, log_path) {
        Ok(err) => Ok(vec![out, err]),
        Err(why) => {
            untee(out);
            Err(why)
        }
    }
}

/// The tee of [`tee_stdout_to`] on any descriptor `fd`. The run tees fd
/// 1; the tests tee a pipe of their own, so they never take the stdout
/// of the test process.
pub(super) fn tee_fd_to(fd: i64, log_path: &std::path::Path) -> Result<Tee, String> {
    use std::os::unix::io::FromRawFd;
    let mut pipe = [0i32; 2];
    podvm_probe::sys::pipe2(&mut pipe, podvm_probe::sys::O_CLOEXEC)
        .map_err(|e| format!("pipe: {} ({})", e.name(), e.0))?;
    // ⛔ CLOEXEC on both ends: the payload must not inherit the reader
    // (it would hold the pipe open past its own stdout and the pump
    // would never see EOF), and the saved stdout must not leak into it
    // either. `dup2` below clears the flag on fd 1 itself, which is the
    // one descriptor the payload is meant to keep.
    let saved = podvm_probe::sys::dup_cloexec(fd)
        .map_err(|e| format!("dup of stdout: {} ({})", e.name(), e.0))?;
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| format!("{}: {e}", log_path.display()))?;
    // SAFETY: both descriptors were just created and are owned here; the
    // reader moves into the pump thread, the writer is closed below once
    // fd 1 carries the pipe.
    let reader = unsafe { std::fs::File::from_raw_fd(pipe[0]) };
    let writer = pipe[1] as i64;
    if let Err(e) = podvm_probe::sys::dup2(writer, fd) {
        let _ = podvm_probe::sys::close(writer);
        let _ = podvm_probe::sys::close(saved);
        return Err(format!("dup2 of stdout: {} ({})", e.name(), e.0));
    }
    let _ = podvm_probe::sys::close(writer);
    // SAFETY: as above; wrapped so the pump never closes it (the restore
    // below owns that close, and a double close would take an unrelated
    // reused descriptor with it). ⛔ TODO/enter.md T-1700: the restore
    // closes it only after the pump ends.
    let out = unsafe { std::mem::ManuallyDrop::new(std::fs::File::from_raw_fd(saved as i32)) };
    // ⛔ TODO/supervise.md T-1708: a signal mask is per thread, and the
    // waiter of `pod-enter` blocks SIGINT and SIGTERM in its own thread to
    // read them from a signalfd. A thread that leaves them unblocked takes
    // the signal with the default action: podvm ended 143, and the tree ran on.
    let handle = std::thread::spawn(move || {
        let mut old = 0u64;
        let _ = podvm_probe::sys::sigblock_shutdown(&mut old);
        pump(reader, out, log)
    });
    Ok(Tee { fd, saved, handle })
}

/// The pump behind [`tee_stdout_to`]: every byte to the saved stdout and
/// to the log, until EOF or a read error. Best-effort by design: the
/// payload's exit code is the verdict, and a logging failure must never
/// move it.
fn pump(
    mut reader: std::fs::File,
    mut out: std::mem::ManuallyDrop<std::fs::File>,
    mut log: std::fs::File,
) {
    use std::io::Read;
    let mut buf = [0u8; 65536];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let _ = std::io::Write::write_all(&mut *out, &buf[..n]);
                let _ = std::io::Write::write_all(&mut log, &buf[..n]);
            }
            Err(_) => break,
        }
    }
    let _ = std::io::Write::flush(&mut *out);
    let _ = std::io::Write::flush(&mut log);
}

/// Restore stdout and join the pump. Runs before the record is
/// finalized, so `logs` never races bytes still in flight.
///
/// ⛔ TODO/enter.md T-1700. The order is the contract: the `dup2` closes
/// the last writer of the pipe that podvm holds, so the pump reads the
/// bytes that remain and then EOF; the join waits for those writes; the
/// close of the saved descriptor comes last. The pump writes to the saved
/// descriptor. A close before the join made each write after it fail
/// with EBADF, which the pump ignores: a payload that wrote and exited
/// just before the restore lost its output, and the run exited 0.
pub(super) fn untee(tee: Tee) {
    let _ = podvm_probe::sys::dup2(tee.saved, tee.fd);
    let _ = tee.handle.join();
    let _ = podvm_probe::sys::close(tee.saved);
}
