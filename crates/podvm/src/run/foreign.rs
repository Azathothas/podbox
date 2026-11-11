//! The platform of the image against this machine's: the binfmt
//! interpreter of a foreign image, or a refusal by name. Split out of
//! `prepare` (TODO/gate.md T-1674).

use super::*;

/// Whether the image runs natively here, after the notes and the copy-in
/// of a foreign interpreter; Err is a refusal. Under the emulate tier
/// (TODO/enter.md T-1651) qemu serves the image, not binfmt.
pub(super) fn support(
    verb: &str,
    image: &str,
    record: &podvm_image::Record,
    rootfs: &str,
    emulate: bool,
) -> Result<bool, i32> {
    let host = Platform::host();
    let support = if emulate {
        binfmt::Support::Native
    } else {
        binfmt::support_for(&record.architecture, &host.arch)
    };
    let mut err = std::io::stderr().lock();
    match &support {
        binfmt::Support::Native => {}
        binfmt::Support::Interpreter(r) => {
            // ⭐ The F flag: the kernel holds the interpreter open, so it works
            // inside the chroot with nothing copied in.
            let _ = writeln!(
                err,
                "podvm: this image is {}/{} and this machine is {}. Running it \
                 through the registered interpreter {} (binfmt flag F, so it is \
                 reachable inside the chroot)",
                record.os, record.architecture, host, r.interpreter
            );
        }
        binfmt::Support::InterpreterNeedsCopyIn(r) => {
            // ⛔ podvm is about to WRITE A FILE INTO SOMEBODY ELSE'S IMAGE.
            // T-0506: the honesty rules have no exception for a helpful edit,
            // and the payload can see this file.
            match binfmt::copy_interpreter_in(rootfs, r) {
                Ok(dest) => {
                    // ⛔ podvm has written a file into somebody else's image.
                    // T-0506: the honesty rules have no exception for a helpful
                    // edit, and the payload can see that file.
                    let _ = writeln!(
                        err,
                        "podvm: this image is {}/{} and this machine is {}. Its \
                         interpreter {} is registered WITHOUT the binfmt F flag, so \
                         the kernel opens it by path at exec time and that path is \
                         inside the container. refused: podvm has COPIED it to {} inside \
                         the image's own rootfs; the payload can see that file.",
                        record.os,
                        record.architecture,
                        host,
                        r.interpreter,
                        dest.display()
                    );
                }
                Err(e) => {
                    let _ = writeln!(
                        err,
                        "podvm {verb}: this image is {}/{} and this machine is {}. Its \
                         interpreter {} lacks the binfmt F flag and could not be \
                         copied into the rootfs: {e}",
                        record.os, record.architecture, host, r.interpreter
                    );
                    return Err(podvm_enter::EXIT_RUNTIME_ERROR);
                }
            }
        }
        binfmt::Support::None { why } => {
            // ⛔ Refused with everything a caller needs, never an `Exec format
            // error` from the kernel with nothing attached to it.
            let _ = writeln!(
                err,
                "podvm {verb}: {image} is {}/{} and this machine is {}. podvm \
                 cannot execute it: {why}",
                record.os, record.architecture, host
            );
            return Err(podvm_enter::EXIT_RUNTIME_ERROR);
        }
    }
    Ok(matches!(support, binfmt::Support::Native))
}
