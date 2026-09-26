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
//! taken seriously.** The design is the same shape as the reference — one
//! emulator child process, UEFI firmware, a per-run overlay so the base image
//! is never written, a FAT "mailbox" volume the host and guest both see, no
//! daemon and no libvirt — but:
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
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub use agent::Code;
pub use fat16::Fat16;
pub use fetch::{Ceiling, FetchError};
pub use plan::{accel_for, argv, overlay_argv, Accel, Plan};

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
    /// The guest never produced a result before the timeout.
    Timeout(Duration),
    /// A result file was there but did not parse.
    BadResult(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoEmulator(e) => write!(f, "{e}"),
            Error::NoAcceleration(e) => write!(f, "{e}"),
            Error::Io(e) => write!(f, "{e}"),
            Error::Timeout(d) => write!(
                f,
                "the guest did not power off within {}s, so it was stopped",
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

pub(crate) fn run_steps(argv: &[String]) -> Result<(), Error> {
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
    Ok(())
}

/// Boot the guest, wait for it to power itself off, and read the result.
///
/// ⛔ **A run that times out is killed and reported, never `Ok`.** The agent
/// powers the machine off only after it has written the exit code, so a
/// timeout means the command is still running or the guest never reached the
/// mailbox, and both are failures.
pub fn run(plan: &Plan, request: &Request) -> Result<Outcome, Error> {
    let mut cmd = Command::new(&plan.emulator);
    cmd.args(argv(plan))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| Error::NoEmulator(format!("{}: {e}", plan.emulator.display())))?;
    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| Error::Io(e.to_string()))? {
            Some(s) => break Some(s),
            None if start.elapsed() >= request.timeout => break None,
            None => std::thread::sleep(Duration::from_millis(500)),
        }
    };
    if status.is_none() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Error::Timeout(request.timeout));
    }
    let _ = status.expect("checked above").code();
    // Re-read the volume from disk: the guest is what wrote the result, and
    // the in-memory copy is the one we handed it, not the one it filled in.
    let bytes = std::fs::read(&plan.mailbox).map_err(|e| Error::Io(e.to_string()))?;
    let back = Fat16::from_image(bytes)
        .ok_or_else(|| Error::Io(format!("{}: not a mailbox", plan.mailbox.display())))?;
    outcome(&back, &request.token)?
        .ok_or_else(|| Error::BadResult("the guest powered off with no result".into()))
}

/// Provision a fresh image: boot it once and type the installer into its
/// console, because the agent has to exist before it can be autostarted.
///
/// ⚠ **This is the only step that needs the console, and it happens once per
/// base image.** Everything after it is the mailbox. The driver types a
/// drive-scanning loop rather than a drive letter: Windows assigns the letter
/// and the reference image has been seen to change it between boots.
pub fn provision(
    qemu_img: &Path,
    base: &Path,
    plan: &Plan,
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
    run_steps(&overlay_argv(qemu_img, base, &plan.root))?;
    let mut child = Command::new(&plan.emulator)
        .args(argv(plan))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Error::NoEmulator(format!("{}: {e}", plan.emulator.display())))?;
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
    while start.elapsed() < wait {
        if child
            .try_wait()
            .map_err(|e| Error::Io(e.to_string()))?
            .is_some()
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    let _ = child.kill();
    let _ = child.wait();
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
/// caught in review.** `setup` points `root` at the provisioned image beside
/// the *vendor's*, so a cleanup keyed on `root` would remove the directory a
/// caller keeps their Windows images in. The mailbox never moves out of the
/// per-run directory, which is why it is the field this reads.
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
    /// ⛔ The defect this pins: `setup` puts `root` beside the vendor image,
    /// so a cleanup keyed on `root` would delete the directory a caller
    /// keeps their Windows images in. It is keyed on the mailbox, and it
    /// refuses a directory that is not named like a scratch one.
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

    use super::*;

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
    fn the_timeout_message_names_the_time() {
        let e = Error::Timeout(Duration::from_secs(600));
        assert!(format!("{e}").contains("600"), "{e}");
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
