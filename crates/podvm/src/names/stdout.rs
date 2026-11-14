//! TODO/cli.md T-1816: a failed write to stdout ends the verb with exit 125
//! and a line on stderr that names the error, never with SIGABRT and never
//! with exit 0.
//!
//! The print macros panic when a write fails, and the release profile
//! aborts on a panic (`panic = "abort"`, which stays: a panic is a defect,
//! and a full disk is not one). The hook below runs before the abort: a
//! panic of a print macro becomes exit 125. [`flushed`] checks the last
//! flush at the exit of each verb, and [`failed`] is the answer of a verb
//! that writes through its own writer.

use std::io::Write;

/// The exit code of a failed write to stdout: podvm could not give its
/// answer, which is a runtime error of podvm.
pub const EXIT: i32 = podvm_image::error::EXIT_RUNTIME_ERROR;

/// The verb of this process, for the message.
fn verb() -> String {
    std::env::args()
        .nth(1)
        .unwrap_or_else(|| "podvm".to_string())
}

/// Write `line` to stderr, best effort: stderr can be full too.
fn say(line: &str) {
    let _ = std::io::stderr().write_all(line.as_bytes());
}

/// Set once a failed write was reported, so that the flush at the exit
/// does not report the same bytes again.
static REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The answer of a verb whose write to stdout failed with `e`.
pub fn failed(e: &std::io::Error) -> i32 {
    if !REPORTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        say(&format!(
            "podvm {}: stdout: {e} (TODO/cli.md T-1816)\n",
            verb()
        ));
    }
    EXIT
}

/// `code`, or [`EXIT`] where the last flush of stdout fails.
pub fn flushed(code: i32) -> i32 {
    match std::io::stdout().flush() {
        Ok(()) => code,
        Err(e) => failed(&e),
    }
}

/// The text of the panic of a print macro on `stream`, if `msg` is one.
pub fn print_failure<'a>(msg: &'a str, stream: &str) -> Option<&'a str> {
    msg.strip_prefix("failed printing to ")?
        .strip_prefix(stream)?
        .strip_prefix(": ")
}

/// Install the hook. A panic of a print macro on stdout says so and exits
/// 125; on stderr it exits 125 with no line, because stderr takes none. Each
/// other panic keeps the default hook.
pub fn install() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let p = info.payload();
        let msg = p
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        if let Some(why) = print_failure(&msg, "stdout") {
            say(&format!(
                "podvm {}: stdout: {why} (TODO/cli.md T-1816)\n",
                verb()
            ));
            std::process::exit(EXIT);
        }
        if print_failure(&msg, "stderr").is_some() {
            std::process::exit(EXIT);
        }
        default(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdout_full_the_panic_of_a_print_macro_is_read() {
        let m = "failed printing to stdout: No space left on device (os error 28)";
        assert_eq!(
            print_failure(m, "stdout"),
            Some("No space left on device (os error 28)")
        );
        assert_eq!(print_failure(m, "stderr"), None);
        assert_eq!(print_failure("index out of bounds", "stdout"), None);
    }
}
