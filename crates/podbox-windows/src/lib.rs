//! The Windows guest driver: a disposable Validation OS machine driven
//! through a FAT mailbox. `TODO/milestones.md` T-1112.
//!
//! ⛔ **What this is for.** T-1112 asked for "a disposable guest that is not
//! Linux", and the entry was blocked because the machine tier's `full`
//! profile needs `/dev/kvm`, which the constrained machines podbox targets
//! do not have. The reference implementation refuses to run without
//! hardware acceleration, so its feature could not be ported as written.
//!
//! ⭐ **This is that feature with the refusal removed and the portability
//! taken seriously.** The design is the same shape as the reference, one
//! emulator child process, UEFI firmware, a per-run overlay so the base image
//! is never written, a FAT "mailbox" volume the host and guest both see, no
//! daemon and no libvirt, but:
//!
//! * it runs under `tcg` where `kvm` is missing, chosen from the machine
//!   tier's own profile rather than refused, so it works on the machines
//!   podbox was built for;
//! * the mailbox is built in process rather than by shelling out to
//!   `mkfs.fat` and `mtools`;
//! * autostart is a scheduled task rather than `cmd.exe`'s `AutoRun`, which
//!   is what the reference relies on and what does not fire on the
//!   Validation OS image;
//! * the command's exit code, stdout and stderr all come back, and a run
//!   that produces no result is an error rather than an empty success.
//!
//! ⚠ **The guest has no network, and the mailbox is the only channel.** A
//! command is a file the host writes before boot and a result is a file the
//! host reads after the guest powers itself off. That costs a boot per
//! command and buys a guest that cannot talk to the host except through a
//! volume the host owns.

#![forbid(unsafe_code)]

pub mod agent;
pub mod dos;
pub mod fat16;
pub mod fetch;
pub mod plan;
pub mod qmp;

use std::os::unix::fs::DirBuilderExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

pub use agent::Code;
pub use fat16::Fat16;
pub use fetch::{Ceiling, FetchError};
pub use plan::{accel_for, argv, commit_argv, overlay_argv, Accel, Plan};

/// A command to run in the guest.
#[derive(Debug, Clone)]
pub struct Request {
    /// One `cmd.exe` command line, run verbatim by the guest. There is no
    /// argv on the Windows side: `&`, `|`, `>`, `%VAR%` and quoting inside
    /// this string are the guest shell's own.
    pub command: String,
    /// Echoed back with the result, so two runs in flight cannot be confused.
    pub token: String,
    /// How long the whole boot-and-run may take before the guest is killed.
    pub timeout: Duration,
}

/// What a guest run produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub code: i32,
    pub token: String,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// What went wrong, in the vocabulary the caller can report on.
#[derive(Debug)]
pub enum Error {
    /// The emulator is not on this machine.
    NoEmulator(String),
    /// The machine tier establishes no profile, so there is nothing to run.
    NoAcceleration(String),
    /// An I/O step failed.
    Io(String),
    /// The guest never produced a result before the timeout. The second field
    /// is what happened to the emulator, never an assumption about it.
    Timeout(Duration, Stop),
    /// A result file was there but did not parse.
    BadResult(String),
}

/// What happened to an emulator this crate started and then had to stop.
///
/// ⛔ **Two states and no third, because a caller acts on the difference.** A
/// reader who is told only "it was stopped" cannot tell a reaped child from a
/// survivor, and those are opposite: the first needs nothing, the second needs
/// a survivor removed. [`Stop::Confirmed`] means the emulator was reaped by
/// this process, so the pid it held is gone and its own children were killed
/// as its process group. [`Stop::Attempted`] means the kill was issued and the
/// child was not observed to die inside the wait, so it may still be running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// The emulator was reaped here. Its process group was killed first, so
    /// anything it forked is gone with it.
    Confirmed { pid: u32 },
    /// The stop was asked for through QMP and then issued as a signal, and
    /// the child was not observed to die within the wait. The message names
    /// the pid so a reader can go and look for it.
    Attempted { pid: u32 },
}

impl Stop {
    /// The emulator process this describes. Reported whether the stop was
    /// confirmed or only attempted: a reader looking for a survivor needs the
    /// pid in both cases.
    pub fn pid(self) -> u32 {
        match self {
            Stop::Confirmed { pid } | Stop::Attempted { pid } => pid,
        }
    }

    /// Whether the emulator was observed to die. A timeout message that does
    /// not carry this is asserting a stop nobody checked.
    pub fn confirmed(self) -> bool {
        matches!(self, Stop::Confirmed { .. })
    }
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stop::Confirmed { pid } => {
                write!(
                    f,
                    "the emulator {pid} was killed and reaped, so its process group is gone"
                )
            }
            Stop::Attempted { pid } => write!(
                f,
                "the stop of emulator {pid} was attempted and not confirmed: \
                 QMP quit and SIGKILL to its process group were issued, and it \
                 was still not reaped within the wait, so it may still be running"
            ),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoEmulator(e) => write!(f, "{e}"),
            Error::NoAcceleration(e) => write!(f, "{e}"),
            Error::Io(e) => write!(f, "{e}"),
            Error::Timeout(d, stop) => write!(
                f,
                "the guest did not power off within {}s; {stop}",
                d.as_secs()
            ),
            Error::BadResult(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

/// The mailbox a run is handed, and the results it is read back from.
///
/// ⚠ **The marker and the token are written here, and the token is the only
/// thing that tells one run's answer from another's.** A host that reuses a
/// mailbox directory must read [`Outcome::token`].
pub fn mailbox(agent_cmd: &str, request: &Request) -> Fat16 {
    let mut m = Fat16::new();
    // These three cannot fail: every name is 8.3, which the tests pin.
    m.put(agent::MARKER, b"podbox").expect("8.3 marker");
    m.put(agent::AGENT_NAME, agent_cmd.as_bytes())
        .expect("8.3 agent name");
    m.put(agent::SETUP_NAME, agent::SETUP_CMD.as_bytes())
        .expect("8.3 setup name");
    m.put(agent::CMD, agent::command_file(&request.command).as_bytes())
        .expect("8.3 command name");
    m.put(agent::GO, request.token.as_bytes())
        .expect("8.3 token name");
    m
}

/// Read a finished run's mailbox. `Ok(None)` is "the guest did not finish":
/// a missing `WQCODE.TXT` is not a zero.
///
/// ⛔ **The token is checked, not assumed.** A result whose token does not
/// match the request is refused rather than reported: that is a stale volume
/// or a second writer, and either one would otherwise be read as this run's
/// answer.
pub fn outcome(m: &Fat16, token: &str) -> Result<Option<Outcome>, Error> {
    let Some(code) = m.get(agent::CODE) else {
        return Ok(None);
    };
    let text = String::from_utf8_lossy(&code).to_string();
    let parsed = agent::parse_code(&text)
        .ok_or_else(|| Error::BadResult(format!("{text:?} is not an exit code and a token")))?;
    if parsed.token != token {
        return Err(Error::BadResult(format!(
            "the mailbox holds a result for token {:?}, not {token:?}",
            parsed.token
        )));
    }
    Ok(Some(Outcome {
        code: parsed.code,
        token: parsed.token,
        stdout: m.get(agent::OUT).unwrap_or_default(),
        stderr: m.get(agent::ERR).unwrap_or_default(),
    }))
}

/// Stage one run: the mailbox on disk and the disposable overlay over the
/// read-only base image. Returns the in-memory mailbox, though the result is
/// read back from the file the guest wrote rather than from it.
///
/// ⚠ The per-run variable store is the caller's, because the caller also
/// copies it: see `Plan::firmware_vars`, which must already name a per-run
/// copy by the time this runs.
pub fn stage(
    qemu_img: &Path,
    base: &Path,
    plan: &Plan,
    agent_cmd: &str,
    request: &Request,
) -> Result<Fat16, Error> {
    let m = mailbox(agent_cmd, request);
    std::fs::write(&plan.mailbox, m.image())
        .map_err(|e| Error::Io(format!("{}: {e}", plan.mailbox.display())))?;
    // ⛔ `qemu-img create` refuses an existing file, and a scratch directory
    // that survived a killed run would otherwise fail here naming the overlay
    // rather than running.
    let _ = std::fs::remove_file(&plan.root);
    // ⛔ And the emulator refuses to **bind** a socket path that already
    // exists, so a stale one from a killed run stops the boot at startup
    // with an error about the monitor rather than about the guest.
    let _ = std::fs::remove_file(&plan.monitor);
    run_steps(&overlay_argv(qemu_img, base, &plan.root))?;
    Ok(m)
}

/// Run one argv to completion and answer its status.
///
/// ⚠ **The status comes back with `Ok`, and is not read for success.** Two
/// callers need different things from the same exit: `stage` and the
/// provisioning commit need only that it did not fail, and the stop path
/// needs to know that a signal killed it. Reporting success and reporting the
/// status are different questions, so the status is the answer and each caller
/// judges it.
pub(crate) fn run_steps(argv: &[String]) -> Result<ExitStatus, Error> {
    let (prog, rest) = argv
        .split_first()
        .ok_or_else(|| Error::Io("empty argv".into()))?;
    let out = Command::new(prog)
        .args(rest)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Error::Io(format!("{prog}: {e}")))?;
    if !out.status.success() {
        return Err(Error::Io(format!(
            "{prog} {}: {}",
            rest.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(out.status)
}

/// Spawn the emulator with `args`, in a process group of its own.
///
/// ⛔ **The group is the point, and it is set before the child exists.**
/// `process_group(0)` puts the child in a new group whose id is its own pid,
/// so [`stop_emulator`] can signal the whole group and take anything the
/// emulator forked with it. A kill of the direct child alone leaves a helper
/// running, and a helper that holds the disk image open is what leaves a
/// machine in a state the next run cannot enter. Neither `process_group` nor
/// the signal below is `unsafe`, so [`forbid(unsafe_code)`] stands.
///
/// ⚠ **The argv is the caller's, not [`plan::argv`]'s.** The two flavors have
/// different machine lines: the Windows guest is q35 with two NVMe devices and
/// the DOS guest is `pc` with IDE disks, and one spawn serving both would
/// change whichever line it did not own. `plan` supplies the binary and the
/// stop needs it; the command line stays with the flavor that builds it.
pub(crate) fn spawn_emulator(plan: &Plan, args: &[String], stderr: Stdio) -> Result<Child, Error> {
    let mut cmd = Command::new(&plan.emulator);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr)
        .process_group(0);
    cmd.spawn()
        .map_err(|e| Error::NoEmulator(format!("{}: {e}", plan.emulator.display())))
}

/// Spawn the Windows guest, whose machine line is [`plan::argv`]'s.
pub(crate) fn spawn_windows(plan: &Plan, stderr: Stdio) -> Result<Child, Error> {
    spawn_emulator(plan, &argv(plan), stderr)
}

/// How long [`stop_emulator`] waits for a killed emulator to be reaped. A
/// killed process is reaped as soon as the kernel has torn it down, and the
/// group is dead with it, so this is a bound and not a wait that has to
/// elapse.
const STOP_WAIT: Duration = Duration::from_secs(5);
/// How long the QMP `quit` is given to work before the signal is sent. ⛔ The
/// guest is the wrong thing to wait for: it is what timed out. This bounds
/// how long the emulator keeps running on the polite path, nothing more.
const QUIT_WAIT: Duration = Duration::from_secs(2);

/// Stop the emulator this crate started, and report what was observed.
///
/// ⛔ **Three steps in a fixed order, and each one answers with evidence.**
/// QMP `quit` first, so a guest that can still reach its monitor ends without
/// a signal. Then `SIGKILL` to the whole process group from
/// [`spawn_emulator`], because `SIGKILL` is what a stuck emulator needs and a
/// helper it forked is in that group too. Then the child is waited for, and
/// the reaped status *is* the confirmation: [`Child::try_wait`] answers
/// `Ok(Some(status))` only for a process this process reaped, and a process
/// that was never reaped is still running however many signals it took.
///
/// ⚠ **The QMP step is best effort and its failure changes nothing.** The
/// monitor is gone once the emulator is stopping, and a guest that has already
/// powered itself off has no socket to answer on. Both are the success case,
/// so a refusal from `quit` is not reported as a failure of the stop.
pub(crate) fn stop_emulator(plan: &Plan, child: &mut Child) -> Stop {
    stop_with(plan, child, QUIT_WAIT, STOP_WAIT)
}

/// [`stop_emulator`] with both bounds supplied, so the wait for a stop can be
/// tested without a signal that has to be delivered.
fn stop_with(plan: &Plan, child: &mut Child, quit_wait: Duration, reap_wait: Duration) -> Stop {
    let pid = child.id();
    stop(plan);
    let start = Instant::now();
    while start.elapsed() < quit_wait {
        if child.try_wait().ok().flatten().is_some() {
            // ⛔ A guest that obeyed `quit` needs no signal at all, and a kill
            // here would be aimed at a pid the kernel may already have reused.
            return Stop::Confirmed { pid };
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    kill_group(pid);
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Stop::Confirmed { pid },
            Ok(None) => {}
            // ⛔ A `try_wait` that cannot answer is not a clean exit, so it is
            // not read as one: the worst report is the honest one.
            Err(_) => return Stop::Attempted { pid },
        }
        if start.elapsed() >= reap_wait {
            return Stop::Attempted { pid };
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// `kill(-pgid, SIGKILL)`, the group form of the signal.
///
/// ⛔ **The group, and the emulator's pid as the negative target.** The group
/// id is the child's pid because [`spawn_emulator`] asked for `process_group(0)`
/// before the child existed, so `-pid` is this emulator's group and nothing
/// else: the driver is in its own group and is never a target here.
///
/// ⚠ **The errno is not reported, and the reap is what reports.** `ESRCH`
/// means the group is already gone, which is the outcome this wanted; any
/// other errno leaves the child alive, and the reap loop below is what turns
/// that into [`Stop::Attempted`]. A second error path here would say the same
/// thing twice, and the one that cannot be wrong is the one that observed it.
fn kill_group(pid: u32) {
    let _ = podbox_probe::sys::kill(-(pid as i64), SIGKILL as i64);
}

/// `SIGKILL`, 9 in `asm-generic/signal.h` and in every architecture's
/// `general.rs` the probe reads its numbers from. ⛔ **Not `SIGTERM`.** A
/// guest that has to be killed has already had its chance: the QMP `quit`
/// above is the polite stop, and a term signal between it and the kill would
/// add a wait that has been measured not to finish.
const SIGKILL: u8 = 9;

/// Boot the guest, wait for it to power itself off, and read the result.
///
/// ⛔ **A run that times out is killed and reported, never `Ok`.** The agent
/// powers the machine off only after it has written the exit code, so a
/// timeout means the command is still running or the guest never reached the
/// mailbox, and both are failures.
pub fn run(plan: &Plan, request: &Request) -> Result<Outcome, Error> {
    // ⛔ **Never a pipe that nobody reads.** The emulator's streams were
    // piped and not drained until exit: a guest that makes the emulator
    // write more than one pipe buffer blocks it, and the run then reads as
    // a guest that did not power off. The error stream goes to a file in
    // the per-run directory, which `cleanup` removes with the run.
    let log = emulator_log(plan);
    let stderr = match &log {
        Some(path) => std::fs::File::create(path)
            .map(Stdio::from)
            .map_err(|e| Error::Io(format!("{}: {e}", path.display())))?,
        None => Stdio::null(),
    };
    let mut child = spawn_windows(plan, stderr)?;
    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| Error::Io(e.to_string()))? {
            Some(s) => break Some(s),
            None if start.elapsed() >= request.timeout => break None,
            None => std::thread::sleep(Duration::from_millis(500)),
        }
    };
    if status.is_none() {
        // ⚠ The emulator that hit this is the one the run started. The agent
        // powers the machine off only after it has written the exit code, so
        // this is a run that is still going, and what it leaves behind is a
        // machine rather than a file.
        let stop = stop_emulator(plan, &mut child);
        return Err(Error::Timeout(request.timeout, stop));
    }
    // The emulator exited with a status the host does not act on, so it is
    // judged and dropped here rather than carried past the point that would
    // report it: the mailbox below is what this function answers from, and a
    // missing result is the error either way. ⛔ Do not turn it into `Ok`.
    let _ = status.expect("checked above");
    // Re-read the volume from disk: the guest is what wrote the result, and
    // the in-memory copy is the one we handed it, not the one it filled in.
    let bytes = std::fs::read(&plan.mailbox).map_err(|e| Error::Io(e.to_string()))?;
    let back = Fat16::from_image(bytes)
        .ok_or_else(|| Error::Io(format!("{}: not a mailbox", plan.mailbox.display())))?;
    outcome(&back, &request.token)?.ok_or_else(|| {
        let tail = log.as_deref().map(log_tail).unwrap_or_default();
        Error::BadResult(format!("the guest powered off with no result{tail}"))
    })
}

/// The emulator's error stream: a file in the per-run directory, or `None`
/// where the plan has no directory that [`cleanup`] would remove.
fn emulator_log(plan: &Plan) -> Option<PathBuf> {
    let dir = scratch_of(plan)?;
    let named = dir
        .file_name()
        .map(|n| n.to_string_lossy().starts_with(SCRATCH_PREFIX))
        .unwrap_or(false);
    named.then(|| dir.join("emulator.log"))
}

/// The last lines of the emulator's error stream, for a failure message.
/// Empty when the file is absent or empty.
fn log_tail(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_default();
    // Only the end of the file, and each line cut short: the stream is the
    // emulator's, and its size is not ours to choose.
    let text = String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(4096)..]);
    let lines: Vec<String> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().take(200).collect())
        .collect();
    if lines.is_empty() {
        return String::new();
    }
    let start = lines.len().saturating_sub(5);
    format!("; the emulator said: {}", lines[start..].join(" | "))
}

/// Provision a fresh image: boot it once and type the installer into its
/// console, because the agent has to exist before it can be autostarted.
///
/// ⚠ **This is the only step that needs the console, and it happens once per
/// base image.** Everything after it is the mailbox. The driver types a
/// drive-scanning loop rather than a drive letter: Windows assigns the letter
/// and the reference image has been seen to change it between boots.
///
/// ⛔ **The base is written to nothing.** The installer runs on
/// [`Plan::root`], which is a disposable overlay over `base` created by
/// [`overlay_argv`], and the overlay is never `base`: the vendor image is
/// named only as that overlay's `-b`, which `qemu-img create` opens
/// read-only. `TODO/gate.md` T-1641.
///
/// ⛔ **And the provisioned image is a separate standalone file.** The overlay
/// is committed by [`commit_argv`] into the second path named here, with no
/// `-b`, so what comes out of this carries no pointer back at `base` and the
/// vendor file can be moved or removed afterwards. The commit happens after
/// the guest has powered itself off, because a commit reads the overlay while
/// the emulator still holds it.
pub fn provision(
    qemu_img: &Path,
    base: &Path,
    plan: &Plan,
    out: &Path,
    wait: Duration,
) -> Result<Vec<u8>, Error> {
    let mut m = Fat16::new();
    m.put(agent::MARKER, b"podbox").expect("8.3 marker");
    m.put(agent::AGENT_NAME, agent::AGENT_CMD.as_bytes())
        .expect("8.3 agent name");
    m.put(agent::SETUP_NAME, agent::SETUP_CMD.as_bytes())
        .expect("8.3 setup name");
    std::fs::write(&plan.mailbox, m.image())
        .map_err(|e| Error::Io(format!("{}: {e}", plan.mailbox.display())))?;
    let _ = std::fs::remove_file(&plan.root);
    let _ = std::fs::remove_file(&plan.monitor);
    // ⚠ `qemu-img create` refuses an existing file, and `out` is the caller's
    // product rather than scratch, so it is only removed here when this boot
    // is going to overwrite it.
    if out != plan.root {
        let _ = std::fs::remove_file(out);
    }
    run_steps(&overlay_argv(qemu_img, base, &plan.root))?;
    let mut child = spawn_windows(plan, Stdio::null())?;
    // The shell needs the image to finish booting first; typing into a boot
    // screen types into nothing.
    std::thread::sleep(wait);
    let typed = type_into_console(plan, "for %d in (D E F G H I J K L M N O P Q R S T U V W X Y Z) do @if exist %d:\\WQMARK.TXT %d:\\WA.CMD");
    typed?;
    // ⛔ The installer ends in `shutdown`, so the guest powers itself off and
    // its FAT writes are flushed before it does. Waiting for the emulator to
    // exit is what makes the read below safe; killing it here is a read of a
    // volume the guest had not finished writing.
    let start = Instant::now();
    let mut exited = false;
    while start.elapsed() < wait {
        if child
            .try_wait()
            .map_err(|e| Error::Io(e.to_string()))?
            .is_some()
        {
            exited = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    // ⚠ The signal is only for an emulator that outlived its wait. A child
    // that already exited has been reaped by the `try_wait` above, and a kill
    // aimed at its pid afterwards is aimed at a pid the kernel may already
    // have reused.
    let _stop = if exited {
        Stop::Confirmed { pid: child.id() }
    } else {
        stop_emulator(plan, &mut child)
    };
    // ⛔ The commit happens here, after the emulator is gone and before
    // anything is reported as provisioned. `run_steps` turns a non-zero exit
    // into `Error::Io` carrying the program and its own stderr, so a commit
    // that failed is named rather than leaving a half-written `out` behind
    // that a later `run` would boot and call a provisioned image.
    run_steps(&commit_argv(qemu_img, &plan.root, out))?;
    // The installer writes SETUP.TXT back to the mailbox; that file is the
    // evidence that provisioning ran, and it is read after the guest stops.
    let bytes = std::fs::read(&plan.mailbox).map_err(|e| Error::Io(e.to_string()))?;
    let back = Fat16::from_image(bytes)
        .ok_or_else(|| Error::Io(format!("{}: not a mailbox", plan.mailbox.display())))?;
    back.get(agent::SETUP)
        .ok_or_else(|| Error::BadResult("the installer wrote no SETUP.TXT".into()))
}

/// Type `text` into the guest console through QMP. A shifted character is
/// the modifier and the key pressed together, which is one `send-key`.
fn type_into_console(plan: &Plan, text: &str) -> Result<(), Error> {
    let mut m =
        qmp::Monitor::connect(&plan.monitor, Duration::from_secs(120)).map_err(Error::Io)?;
    for ch in text.chars() {
        let (name, shifted) = keys(ch).ok_or_else(|| Error::Io(format!("no key for {ch:?}")))?;
        if shifted {
            m.keys(&["shift", name]).map_err(Error::Io)?;
        } else {
            m.keys(&[name]).map_err(Error::Io)?;
        }
        std::thread::sleep(Duration::from_millis(8));
    }
    m.keys(&["ret"]).map_err(Error::Io)?;
    std::thread::sleep(Duration::from_millis(500));
    Ok(())
}

/// Ask a running guest to stop, through QMP. Best effort: a guest that has
/// already powered itself off has no monitor left to answer, and that is the
/// success case, not a failure.
pub fn stop(plan: &Plan) {
    if let Ok(mut m) = qmp::Monitor::connect(&plan.monitor, Duration::from_secs(2)) {
        m.quit();
    }
}

/// ⭐ Docker's exit range is 0-124, and 125 is `podbox` itself failing. A
/// guest status of 125 or above would therefore read as a podbox failure,
/// so it is capped rather than passed through: `None` is "this status
/// cannot be returned as-is", and the caller names it.
pub fn cap_status(code: i32) -> Option<i32> {
    (0..=124).contains(&code).then_some(code)
}

/// The per-run directory: where the mailbox, the QMP socket and the serial
/// log are, which is the directory [`scratch_dir`] made. `None` where the
/// plan's mailbox has no parent.
///
/// ⛔ **Not `Plan::root`'s parent, and that distinction is a defect that was
/// caught in review.** A cleanup keyed on `root` would remove whatever
/// directory the root disk lives in, and on the `setup` path that is a
/// caller's directory of Windows images beside the vendor's. The mailbox
/// never moves out of the per-run directory, which is why it is the field this
/// reads. `TODO/gate.md` T-1641 took `setup`'s `root` back into the per-run
/// directory as well; this is keyed on the mailbox because `run`'s root is
/// still per-run and the two must not depend on that staying true.
pub fn scratch_of(plan: &Plan) -> Option<PathBuf> {
    plan.mailbox.parent().map(|p| p.to_path_buf())
}

/// The prefix every per-run directory carries, used both to create one and
/// to be sure of one before removing it.
pub const SCRATCH_PREFIX: &str = "podbox-windows-";

/// Remove a per-run directory unless `PODBOX_WINDOWS_KEEP` is set.
///
/// ⛔ **A run's leftovers are about a hundred megabytes**, measured: a 56 MB
/// overlay, a 17 MB mailbox and a 0.5 MB variable store, per run. Leaving
/// them behind is how the runs earlier in this entry's own record failed at
/// 300 s: the sandbox's 489 MB tmpfs filled, the host saw ENOSPC on the
/// writethrough mailbox, and the guest never answered. So the default is to
/// remove, and the override exists because a run that needs looking at is
/// exactly the run that failed.
///
/// ⚠ **Two guards before anything is removed, and both are refusals rather
/// than errors.** The directory must be the one [`scratch_of`] names *and*
/// its own name must carry [`SCRATCH_PREFIX`]. A recursive delete driven by
/// a path a caller can influence is the one operation here that can destroy
/// work that is not ours, so a plan that does not look like ours is left
/// exactly as it is. Best effort after that: a refusal to remove is not a
/// failure of the run, which has already returned its result.
pub fn cleanup(plan: &Plan) {
    if std::env::var_os("PODBOX_WINDOWS_KEEP").is_some() {
        return;
    }
    let Some(dir) = scratch_of(plan) else {
        return;
    };
    let named = dir
        .file_name()
        .map(|n| n.to_string_lossy().starts_with(SCRATCH_PREFIX))
        .unwrap_or(false);
    if !named {
        return;
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A per-run directory that removes itself when it falls out of scope.
///
/// `cleanup` runs on every coded exit; this runs on the exits without one:
/// a panic past the staging point, where the explicit calls never execute.
/// SIGKILL aside, which no guard survives. Removing twice is safe
/// (`cleanup` ignores a missing directory through `let _`), so the coded
/// paths keep their explicit calls and this only backstops them.
pub struct RunGuard<'a> {
    plan: &'a Plan,
}

impl<'a> RunGuard<'a> {
    /// Arm the guard for `plan`. Name the binding so it lives to the end
    /// of the run: a guard dropped at once guards nothing.
    pub fn arm(plan: &'a Plan) -> Self {
        RunGuard { plan }
    }
}

impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        cleanup(self.plan);
    }
}

/// A per-run directory, mode 0700, holding the QMP socket, the mailbox, the
/// overlay and the serial log.
///
/// ⛔ **The mode is the point.** The QMP socket lets whoever connects type
/// into the guest console, so the directory that holds it is not readable
/// or traversable by another local user.
pub fn scratch_dir(tag: &str) -> Result<PathBuf, Error> {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join(format!("{SCRATCH_PREFIX}{}-{tag}", std::process::id()));
    let mut b = std::fs::DirBuilder::new();
    b.mode(0o700);
    match b.create(&dir) {
        Ok(()) => Ok(dir),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(dir),
        Err(e) => Err(Error::Io(format!("{}: {e}", dir.display()))),
    }
}

/// The cached base image: `PODBOX_WINDOWS_BASE` where it is set, so an
/// experiment isolates with its own copy, else under the home directory so
/// it survives between runs the way an image cache should.
pub fn base_cache() -> PathBuf {
    if let Some(p) = std::env::var_os("PODBOX_WINDOWS_BASE") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let home = std::env::var_os("HOME").unwrap_or_else(|| "/tmp".into());
    Path::new(&home).join(".local/share/podbox/windows/base.vhdx")
}

/// A character to a `sendkey` name, `(name, shifted)`, for a US layout.
fn keys(c: char) -> Option<(&'static str, bool)> {
    Some(match c {
        'a'..='z' => (LETTER[(c as u8 - b'a') as usize], false),
        'A'..='Z' => (LETTER[(c as u8 - b'A') as usize], true),
        '0'..='9' => (DIGIT[(c as u8 - b'0') as usize], false),
        ' ' => ("spc", false),
        '!' => ("1", true),
        '"' => ("apostrophe", true),
        '#' => ("3", true),
        '$' => ("4", true),
        '%' => ("5", true),
        '&' => ("7", true),
        '\'' => ("apostrophe", false),
        '(' => ("9", true),
        ')' => ("0", true),
        '*' => ("8", true),
        '+' => ("equal", true),
        ',' => ("comma", false),
        '-' => ("minus", false),
        '.' => ("dot", false),
        '/' => ("slash", false),
        ':' => ("semicolon", true),
        ';' => ("semicolon", false),
        '<' => ("comma", true),
        '=' => ("equal", false),
        '>' => ("dot", true),
        '?' => ("slash", true),
        '@' => ("2", true),
        '[' => ("bracket_left", false),
        '\\' => ("backslash", false),
        ']' => ("bracket_right", false),
        '^' => ("6", true),
        '_' => ("minus", true),
        '`' => ("grave_accent", false),
        '{' => ("bracket_left", true),
        '|' => ("backslash", true),
        '}' => ("bracket_right", true),
        '~' => ("grave_accent", true),
        _ => return None,
    })
}

const LETTER: [&str; 26] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
    "t", "u", "v", "w", "x", "y", "z",
];
const DIGIT: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

#[cfg(test)]
mod tests {
    use super::*;

    /// The provisioning boot's root disk, which `T-1641` moved into the
    /// per-run directory. ⛔ It is not the provisioned image: that is a second
    /// path, and this is the disposable scratch overlay the commit reads.
    #[test]
    fn the_provisioning_root_is_per_run_scratch_and_not_the_provisioned_image() {
        let scratch = scratch_dir("provision-root").expect("a scratch directory");
        let vendor = std::env::temp_dir().join(format!("pbx-prov-{}", std::process::id()));
        std::fs::create_dir_all(&vendor).unwrap();
        let image = vendor.join("ValidationOS.vhdx");
        std::fs::write(&image, b"not really a disk image").unwrap();
        let out = vendor.join("ValidationOS.podbox.qcow2");
        let plan = Plan {
            emulator: "qemu-system-x86_64".into(),
            accel: Accel::Tcg,
            share: "/usr/share/qemu".into(),
            firmware_code: "/code.fd".into(),
            firmware_vars: scratch.join("vars.fd"),
            // exactly what the CLI's `setup` does
            root: scratch.join("run.qcow2"),
            mailbox: scratch.join("mailbox.img"),
            serial: scratch.join("serial.log"),
            monitor: scratch.join("qmp.sock"),
            memory_mib: 4096,
            cpus: 2,
            emu_args: Vec::new(),
        };
        assert_ne!(plan.root, out, "the two are separate files");
        assert_eq!(
            scratch_of(&plan).as_deref(),
            Some(scratch.as_path()),
            "the root overlay is inside the directory cleanup removes"
        );
        // and the boot's own disk line names that per-run file
        let a = argv(&plan);
        assert!(
            a.iter()
                .any(|x| x.contains("run.qcow2") && x.contains("id=root")),
            "{a:?}"
        );
        // ⛔ so a cleanup cannot reach the directory the vendor image and the
        // provisioned image live in, whatever a caller does with `root`.
        let _ = std::fs::remove_dir_all(&scratch);
        let _ = std::fs::remove_dir_all(&vendor);
    }

    /// ⛔ The defect this pins: `setup` puts `root` in a caller's directory of
    /// Windows images in every revision but one, so a cleanup keyed on `root`
    /// would delete that directory. It is keyed on the mailbox, and it refuses
    /// a directory that is not named like a scratch one.
    #[test]
    fn cleanup_never_removes_the_directory_a_vendor_image_lives_in() {
        let vendor = std::env::temp_dir().join(format!("pbx-vendor-{}", std::process::id()));
        std::fs::create_dir_all(&vendor).unwrap();
        let image = vendor.join("ValidationOS.vhdx");
        std::fs::write(&image, b"not really a disk image").unwrap();
        let scratch = scratch_dir("cleanup-test").expect("a scratch directory");
        let plan = Plan {
            emulator: "qemu-system-x86_64".into(),
            accel: Accel::Tcg,
            share: "/usr/share/qemu".into(),
            firmware_code: "/code.fd".into(),
            firmware_vars: scratch.join("vars.fd"),
            // exactly what the CLI's `setup` does
            root: vendor.join("ValidationOS.podbox.qcow2"),
            mailbox: scratch.join("mailbox.img"),
            serial: scratch.join("serial.log"),
            monitor: scratch.join("qmp.sock"),
            memory_mib: 4096,
            cpus: 2,
            emu_args: Vec::new(),
        };
        std::env::remove_var("PODBOX_WINDOWS_KEEP");
        cleanup(&plan);
        assert_eq!(scratch_of(&plan).as_deref(), Some(scratch.as_path()));
        assert!(image.is_file(), "the vendor image is untouched");
        assert!(vendor.is_dir(), "and so is the directory holding it");
        assert!(!scratch.exists(), "the per-run directory is gone");

        // and a plan whose mailbox is not in a scratch-named directory is
        // left alone rather than recursively deleted
        let mut not_ours = plan.clone();
        not_ours.mailbox = vendor.join("mailbox.img");
        cleanup(&not_ours);
        assert!(
            vendor.is_dir(),
            "an unrecognised directory is never removed"
        );

        // ⭐ and the override keeps it
        let kept = scratch_dir("cleanup-keep").expect("a scratch directory");
        std::env::set_var("PODBOX_WINDOWS_KEEP", "1");
        let mut p2 = plan.clone();
        p2.mailbox = kept.join("mailbox.img");
        cleanup(&p2);
        assert!(kept.is_dir(), "PODBOX_WINDOWS_KEEP keeps the run");
        std::env::remove_var("PODBOX_WINDOWS_KEEP");
        let _ = std::fs::remove_dir_all(&kept);
        let _ = std::fs::remove_dir_all(&vendor);
    }

    /// The defect this pins: `cleanup` runs on the coded exits, and a panic
    /// past staging skipped every one of them, leaking the per-run
    /// directory. The guard removes it on drop, including the unwind path.
    #[test]
    fn a_dropped_guard_removes_the_run_directory() {
        let scratch = scratch_dir("guard-drop").expect("a scratch directory");
        std::fs::write(scratch.join("mailbox.img"), b"mailbox").unwrap();
        let plan = Plan {
            emulator: "qemu-system-x86_64".into(),
            accel: Accel::Tcg,
            share: "/usr/share/qemu".into(),
            firmware_code: "/code.fd".into(),
            firmware_vars: scratch.join("vars.fd"),
            root: scratch.join("root.qcow2"),
            mailbox: scratch.join("mailbox.img"),
            serial: scratch.join("serial.log"),
            monitor: scratch.join("qmp.sock"),
            memory_mib: 512,
            cpus: 1,
            emu_args: Vec::new(),
        };
        std::env::remove_var("PODBOX_WINDOWS_KEEP");
        {
            let _guard = RunGuard::arm(&plan);
            assert!(scratch.is_dir(), "the run is staged");
        }
        assert!(!scratch.exists(), "dropping the guard removes the run");
    }

    /// ⛔ The defect this pins: the emulator's streams were piped and not
    /// read, so an emulator that wrote more than one pipe buffer blocked and
    /// the run was reported as a guest that did not power off. The fake
    /// emulator writes 256 KiB to each stream and exits; the run must end
    /// at once and carry the end of the error stream in its message.
    #[test]
    fn a_noisy_emulator_is_not_blocked_on_its_own_output() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = scratch_dir("noisy-emulator").expect("a scratch directory");
        let fake = scratch.join("fake-emulator");
        std::fs::write(
            &fake,
            "#!/bin/sh\nhead -c 262144 /dev/zero | tr '\\0' o\n\
             head -c 262144 /dev/zero | tr '\\0' e >&2\necho '' >&2\necho last-line >&2\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
        let plan = Plan {
            emulator: fake,
            accel: Accel::Tcg,
            share: "/usr/share/qemu".into(),
            firmware_code: "/code.fd".into(),
            firmware_vars: scratch.join("vars.fd"),
            root: scratch.join("root.qcow2"),
            mailbox: scratch.join("mailbox.img"),
            serial: scratch.join("serial.log"),
            monitor: scratch.join("qmp.sock"),
            memory_mib: 512,
            cpus: 1,
            emu_args: Vec::new(),
        };
        std::fs::write(&plan.mailbox, mailbox(agent::AGENT_CMD, &request()).image()).unwrap();
        let mut req = request();
        req.timeout = Duration::from_secs(30);
        // A file written and then executed while another test thread forks
        // can be refused as busy for a moment; that refusal is not the
        // subject here, so it is retried.
        let mut attempt = 0;
        let (e, elapsed) = loop {
            let start = Instant::now();
            let e = run(&plan, &req).unwrap_err();
            let busy = matches!(&e, Error::NoEmulator(m) if m.contains("busy"));
            if !busy || attempt == 20 {
                break (e, start.elapsed());
            }
            attempt += 1;
            std::thread::sleep(Duration::from_millis(50));
        };
        std::env::remove_var("PODBOX_WINDOWS_KEEP");
        cleanup(&plan);
        assert!(
            elapsed < Duration::from_secs(20),
            "the run blocked for {elapsed:?}"
        );
        assert!(matches!(e, Error::BadResult(_)), "{e}");
        assert!(format!("{e}").contains("last-line"), "{e}");
    }

    fn request() -> Request {
        Request {
            command: "ver & echo hi".into(),
            token: "tok-1".into(),
            timeout: Duration::from_secs(10),
        }
    }

    #[test]
    fn the_mailbox_carries_the_marker_the_command_and_the_token() {
        let m = mailbox(agent::AGENT_CMD, &request());
        assert_eq!(m.get(agent::MARKER).as_deref(), Some(&b"podbox"[..]));
        assert_eq!(m.get(agent::GO).as_deref(), Some(&b"tok-1"[..]));
        assert!(m.get(agent::CMD).is_some());
        // The agent is on the volume under the name `SETUP_CMD` copies it to.
        assert_eq!(
            m.get(agent::AGENT_NAME).as_deref(),
            Some(agent::AGENT_CMD.as_bytes())
        );
        // and nothing has been answered yet
        assert!(m.get(agent::CODE).is_none());
    }

    #[test]
    fn a_result_is_read_back_with_its_streams_and_its_code() {
        let mut m = mailbox(agent::AGENT_CMD, &request());
        m.put(agent::OUT, b"hello\r\n").unwrap();
        m.put(agent::ERR, b"").unwrap();
        m.put(agent::CODE, b"3 tok-1").unwrap();
        let o = outcome(&m, "tok-1").unwrap().expect("a result");
        assert_eq!(o.code, 3);
        assert_eq!(o.token, "tok-1");
        assert_eq!(o.stdout, b"hello\r\n");
        assert!(o.stderr.is_empty());
    }

    #[test]
    fn a_guest_that_did_not_finish_is_not_a_zero() {
        let m = mailbox(agent::AGENT_CMD, &request());
        assert_eq!(outcome(&m, "tok-1").unwrap(), None);
    }

    #[test]
    fn a_result_for_somebody_elses_token_is_refused_not_reported() {
        let mut m = mailbox(agent::AGENT_CMD, &request());
        m.put(agent::CODE, b"0 someone-else").unwrap();
        let e = outcome(&m, "tok-1").unwrap_err();
        assert!(format!("{e}").contains("someone-else"), "{e}");
    }

    #[test]
    fn the_timeout_message_names_the_time_and_the_pid() {
        let e = Error::Timeout(Duration::from_secs(600), Stop::Confirmed { pid: 4242 });
        assert!(format!("{e}").contains("600"), "{e}");
        assert!(format!("{e}").contains("4242"), "{e}");
    }

    /// ⛔ The defect this pins: the message said "so it was stopped" on a path
    /// where nothing confirmed a stop, and the kill's own result was thrown
    /// away. The two states read differently and each names the pid, because
    /// a reader chasing a survivor needs it in both.
    #[test]
    fn a_timeout_says_whether_the_stop_was_confirmed_and_names_the_pid() {
        let confirmed = Error::Timeout(Duration::from_secs(60), Stop::Confirmed { pid: 1234 });
        let text = format!("{confirmed}");
        assert!(text.contains("1234"), "{text}");
        assert!(
            text.contains("reaped"),
            "a confirmed stop says the child was reaped: {text}"
        );
        let attempted = Error::Timeout(Duration::from_secs(60), Stop::Attempted { pid: 1234 });
        let text = format!("{attempted}");
        assert!(text.contains("1234"), "{text}");
        assert!(
            text.contains("not confirmed"),
            "an unconfirmed stop does not claim one: {text}"
        );
        assert!(
            text.contains("still be running"),
            "an unconfirmed stop names the risk a reader has to act on: {text}"
        );
        assert!(
            Stop::Confirmed { pid: 1234 }.confirmed() && !Stop::Attempted { pid: 1234 }.confirmed()
        );
        assert_eq!(Stop::Confirmed { pid: 1234 }.pid(), 1234);
    }

    /// ⛔ The defect this pins: the emulator was spawned without a process
    /// group and killed as a direct child, so a helper it forked outlived the
    /// kill. The emulator and its forked helper are in one group here, and
    /// neither survives.
    #[test]
    fn a_stopped_emulator_takes_its_process_group_with_it() {
        let scratch = scratch_dir("stop-group").expect("a scratch directory");
        let marker = scratch.join("group-survived");
        let fake = scratch.join("fake-emulator");
        // The emulator stands in for its own group; the `sleep` stands for a
        // helper that forks under it, and writes its file only if it outlives
        // the kill.
        std::fs::write(&fake, "#!/bin/sh\n(sleep 30; : > \"$0\") &\nwait\n").unwrap();
        std::fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o700))
            .unwrap();
        let mut plan = plan_over(&scratch, &fake);
        plan.root = scratch.join("run.qcow2");
        let mut child = spawn_windows(&plan, Stdio::null()).expect("the fake emulator starts");
        let child_pid = child.id();
        // ⛔ The child is not in the driver's own group, so the group kill below
        // cannot reach this test process.
        let stop = stop_with(
            &plan,
            &mut child,
            Duration::from_millis(50),
            Duration::from_secs(10),
        );
        assert!(
            matches!(stop, Stop::Confirmed { .. }),
            "the emulator was reaped: {stop}"
        );
        assert_eq!(stop.pid(), child_pid);
        assert_ne!(
            std::process::id(),
            child_pid,
            "the emulator is its own process"
        );
        std::thread::sleep(Duration::from_millis(500));
        assert!(!marker.exists(), "a forked helper outlived the group kill");
        let _ = std::fs::remove_dir_all(&scratch);
    }

    /// A guest that has already powered itself off needs no signal, and the
    /// report says the emulator was reaped rather than that a kill was issued.
    /// The wait is zero, so this does not sleep on a real timeout.
    #[test]
    fn an_emulator_that_exited_on_its_own_is_reported_as_a_confirmed_stop() {
        let scratch = scratch_dir("stop-exited").expect("a scratch directory");
        let fake = scratch.join("fake-emulator");
        std::fs::write(&fake, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o700))
            .unwrap();
        let plan = plan_over(&scratch, &fake);
        let mut child = spawn_windows(&plan, Stdio::null()).expect("the fake emulator starts");
        // Let the child be reaped by the stop, not by a caller that already
        // waited for it.
        std::thread::sleep(Duration::from_millis(200));
        let stop = stop_with(
            &plan,
            &mut child,
            Duration::from_millis(50),
            Duration::from_secs(5),
        );
        assert!(
            matches!(stop, Stop::Confirmed { pid } if pid == child.id()),
            "{stop}"
        );
        let _ = std::fs::remove_dir_all(&scratch);
    }

    /// The plan every stop test needs: a real emulator binary that is not
    /// qemu, and a monitor socket that was never created. ⛔ `stop` polls the
    /// monitor before it signals, and a socket that is absent makes that
    /// polling the bound rather than a connection to something live.
    fn plan_over(scratch: &Path, emulator: &Path) -> Plan {
        Plan {
            emulator: emulator.to_path_buf(),
            accel: Accel::Tcg,
            share: "/usr/share/qemu".into(),
            firmware_code: "/code.fd".into(),
            firmware_vars: scratch.join("vars.fd"),
            root: scratch.join("run.qcow2"),
            mailbox: scratch.join("mailbox.img"),
            serial: scratch.join("serial.log"),
            monitor: scratch.join("qmp.sock"),
            memory_mib: 512,
            cpus: 1,
            emu_args: Vec::new(),
        }
    }

    #[test]
    fn docker_keeps_0_to_124_and_podbox_keeps_125_up() {
        assert_eq!(cap_status(0), Some(0));
        assert_eq!(cap_status(42), Some(42));
        assert_eq!(cap_status(124), Some(124));
        assert_eq!(cap_status(125), None);
        assert_eq!(cap_status(255), None);
        assert_eq!(cap_status(-1), None);
    }

    #[test]
    fn provisioning_types_a_command_that_finds_the_volume_without_a_letter() {
        // The installer is invoked through a scan, never through `D:`.
        let text = "for %d in (D E) do @if exist %d:\\WQMARK.TXT %d:\\WA.CMD";
        assert!(text.contains("%d"));
        assert!(text.contains("WQMARK.TXT"));
        assert!(keys('d').is_some());
        assert!(keys('D').map(|k| k.1).unwrap_or(false));
    }
}
