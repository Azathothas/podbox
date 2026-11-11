//! TODO/cli.md T-1649. The `#!` route of the no-chroot entry.
//!
//! The kernel reads a `#!` line and opens its interpreter on the host root,
//! and the userland rung has no chroot beneath it. So podvm does the step of
//! the kernel itself, before the exec: it reads the line, resolves the
//! interpreter inside the rootfs, and runs the interpreter with the script
//! path as its argument, in the order of the kernel (`interp [arg] script
//! args`). The exec family of the interposer does the same for a child
//! (TODO/interpose.md T-1646). An interpreter that does not resolve inside
//! the image refuses at 126; podvm never looks for it on the host.
//!
//! ⭐ T-137 of PodUtils/podcrates (COORD-018). The `#!` parser is
//! `podvm_enter::memfd::shebang_parts` of `pod-enter`, and the `$0` of the
//! script is `podvm_enter::ladder::guest_invoked`. `head::shebang` of the
//! interposer stays a copy by value, because the interposer takes no
//! dependency; the tests pin the same cases on both.
//!
//! ⭐ TODO/enter.md T-1653. Each guest path resolves through the table of
//! the run (`abi::GuestRoot`), so a script or an interpreter in a volume is
//! the file of the volume.

use super::*;

/// How many `#!` levels one entry follows: the bound of the kernel
/// (`BINPRM_MAX_RECURSION`) and of the exec family of the interposer.
pub(super) const DEPTH: usize = 4;

/// The payload that the entry decides on, after the `#!` route.
pub(super) struct Followed {
    /// The guest argv that runs: the caller's argv, or the argv that the
    /// kernel would build for the script.
    pub argv: Vec<String>,
    /// The program file by host path.
    pub resolved: String,
    /// The head of that file (`podvm_enter::abi::HEAD_BYTES`): the `#!`
    /// line and the ELF headers are there.
    pub bytes: Vec<u8>,
    /// Each script followed: its host path and its `#!` line.
    pub scripts: Vec<(String, String)>,
}

/// A refusal of the route: the exit code, the note for the no-chroot
/// sentence, and whether a `#!` script was followed before it.
pub(super) type Refusal = (i32, String, bool);

/// The argv that the kernel builds for a script: the interpreter, its
/// optional argument, the script path as invoked, then the arguments.
pub(super) fn kernel_argv(
    interp: &str,
    arg: Option<&str>,
    script: &str,
    rest: &[String],
) -> Vec<String> {
    let mut v = Vec::with_capacity(rest.len() + 3);
    v.push(interp.to_string());
    v.extend(arg.map(str::to_string));
    v.push(script.to_string());
    v.extend(rest.iter().cloned());
    v
}

/// The script path that the interpreter receives: as invoked where it has
/// a `/`, else the unresolved join of the `PATH` directory where the name
/// resolves, as the kernel gives it ([`podvm_enter::ladder::guest_invoked`]).
fn invoked_path<G: podvm_enter::abi::GuestRoot + ?Sized>(
    root: &G,
    argv0: &str,
    path_dirs: &[String],
) -> String {
    podvm_enter::ladder::guest_invoked(root, argv0, path_dirs).unwrap_or_else(|| argv0.to_string())
}

/// Resolve the payload, read it, and follow each `#!` level to the program
/// that runs, inside `root` only: the rootfs, or the table of the run with
/// its volumes.
///
/// ⭐ T-1413 and T-1414. The path and the bytes are read once: the path
/// rides the refusals, the resolution sentences decide the code (127
/// absent, 126 found but not invocable), and the loader arm reuses them.
/// Read-only: a refusal writes nothing into the image.
///
/// ⭐ TODO/enter.md T-1707. A relative program resolves against
/// `working_dir` ([`podvm_enter::program_in`]); the argv keeps it as the
/// caller wrote it.
pub(super) fn follow<G: podvm_enter::abi::GuestRoot + ?Sized>(
    root: &G,
    argv: &[String],
    path_dirs: &[String],
    working_dir: &str,
) -> Result<Followed, Refusal> {
    let mut argv = argv.to_vec();
    let mut scripts: Vec<(String, String)> = Vec::new();
    loop {
        let routed = !scripts.is_empty();
        let argv0 = argv.first().cloned().unwrap_or_default();
        let name = podvm_enter::program_in(&argv0, working_dir);
        let resolved = match podvm_enter::ladder::resolve_payload(root, &name, path_dirs) {
            Ok(p) => p,
            Err(e) => {
                return Err(match scripts.last() {
                    None => (
                        resolve_code(&e.to_string()),
                        format!("{argv0} resolves to nothing runnable: {e}"),
                        false,
                    ),
                    // ⛔ Never a host fallback: the script is found, and its
                    // interpreter is not in the image, so 126.
                    Some((path, line)) => (
                        EXIT_CANNOT_INVOKE,
                        format!(
                            "the payload {path} is a #! script ({line}), and its \
                             interpreter does not resolve inside the image: {e}. \
                             podvm runs the interpreter of a script from the image \
                             only, never from the host (TODO/enter.md T-1413; \
                             TODO/cli.md T-1649)"
                        ),
                        true,
                    ),
                });
            }
        };
        let bytes = match podvm_enter::ladder::payload_head(root, &name, path_dirs) {
            Ok(p) => p.head,
            Err(e) => {
                // ⚠ T-0802's errno split: a directory is found-but-not-
                // invocable (EISDIR is a 126), while an unreadable file is a
                // genuine podvm-side failure (125).
                let msg = e.to_string();
                let code = if msg.contains("Is a directory") {
                    EXIT_CANNOT_INVOKE
                } else {
                    EXIT_RUNTIME_ERROR
                };
                let note = format!("{argv0} resolved but its bytes could not be read: {e}");
                return Err((code, note, routed));
            }
        };
        if !podvm_enter::memfd::has_shebang(&bytes) {
            return Ok(Followed {
                argv,
                resolved,
                bytes,
                scripts,
            });
        }
        let line = podvm_enter::memfd::script_refusal(&resolved, &bytes).shebang;
        let refuse = |why: &str| {
            Err((
                EXIT_CANNOT_INVOKE,
                format!("the payload {resolved} is a #! script ({line}), and {why} (TODO/cli.md T-1649)"),
                true,
            ))
        };
        if scripts.len() == DEPTH {
            return refuse(&format!(
                "it is the interpreter of a chain of more than {DEPTH} #! scripts, \
                 which the kernel also refuses"
            ));
        }
        let Some((interp, arg)) = podvm_enter::memfd::shebang_parts(&bytes) else {
            return refuse("its #! line names no interpreter that the kernel would run");
        };
        if !interp.starts_with('/') {
            return refuse(&format!(
                "its interpreter {interp} is a relative path, which resolves \
                 against the working directory of the kernel and not inside the \
                 image"
            ));
        }
        let script = invoked_path(root, &argv0, path_dirs);
        argv = kernel_argv(&interp, arg.as_deref(), &script, &argv[1..]);
        scripts.push((resolved, line));
    }
}

/// TODO/cli.md T-1649. The remedy that a refusal names where the refused
/// script is the Entrypoint of the image and the caller gave a command.
///
/// docker's argv rule stays (`Plan::argv_for`): the script of the image is
/// the program, and the words of the caller are its arguments. podvm does
/// not drop the Entrypoint on its own, so the refusal names the flag that
/// does.
pub(super) fn entrypoint_retry(first_word: &str) -> String {
    format!(
        "This script is the Entrypoint of the image, and by docker's rule your \
         command became its arguments. To run your command without the script, \
         retry with `--entrypoint {first_word}` and put the remainder of your \
         command after the image (TODO/cli.md T-1649)"
    )
}

/// The refusal sentence of the no-chroot entry, with the remedy of
/// [`entrypoint_retry`] where `retry` names the first word of the caller
/// and `routed` tells that a `#!` script was followed.
pub(super) fn no_chroot_message(
    verb: &str,
    note: &str,
    routed: bool,
    retry: Option<&str>,
) -> String {
    let mut s = format!(
        "podvm {verb}: chroot(2) is denied on this machine, and no no-chroot \
         family runs this payload: {note}. The probe's chroot leg reports the \
         denial (see `podvm probe`) (TODO/enter.md T-1317)"
    );
    if let Some(word) = retry.filter(|_| routed) {
        s.push_str(". ");
        s.push_str(&entrypoint_retry(word));
    }
    s
}

/// The banner clause of a followed script, empty where none was followed.
pub(super) fn banner_note(f: &Followed) -> String {
    match f.scripts.first() {
        None => String::new(),
        Some((path, line)) => format!(
            "; the payload {path} is a #! script ({line}), so podvm runs its \
             interpreter {} from the image with the script path as its \
             argument, as the kernel would (TODO/cli.md T-1649)",
            f.argv.first().map(String::as_str).unwrap_or("")
        ),
    }
}
