//! The values of the flags of the hardening line. TODO/cli.md T-1655.

use super::*;

/// The seconds of `--timeout`: a whole number over 0. TODO/cli.md T-1655.
pub(super) fn timeout(verb: &str, a: &str) -> Result<u64, i32> {
    match a.parse::<u64>() {
        Ok(n) if n > 0 && n <= u64::MAX / 1000 => Ok(n),
        _ => {
            eprintln!("podvm {verb}: --timeout takes whole seconds over 0, not {a:?}");
            Err(EXIT_FLAG_ERROR)
        }
    }
}

/// One `--ulimit`, checked now so that a bad spec is a flag error before
/// anything is fetched. TODO/cli.md T-1655.
pub(super) fn ulimit(verb: &str, a: &str) -> Result<String, i32> {
    match podvm_enter::ulimit(a) {
        Ok(_) => Ok(a.to_string()),
        Err(why) => {
            eprintln!("podvm {verb}: --ulimit {why}");
            Err(EXIT_FLAG_ERROR)
        }
    }
}

/// One `--security-opt`: `no-new-privileges` (also `:true`, `=true`) sets
/// the bit in the child; each other option is refused by name, because no
/// security backend of podvm takes it. TODO/cli.md T-1655.
pub(super) fn security_opt(verb: &str, a: &str) -> Result<bool, i32> {
    match a {
        "no-new-privileges" | "no-new-privileges:true" | "no-new-privileges=true" => Ok(true),
        "no-new-privileges:false" | "no-new-privileges=false" => Ok(false),
        other => {
            eprintln!(
                "podvm {verb}: --security-opt {other:?} is refused: podvm takes \
                 no-new-privileges only; seccomp, apparmor, label and the rest have \
                 no backend here (TODO/cli.md T-1655)"
            );
            Err(EXIT_FLAG_ERROR)
        }
    }
}

/// One `--cap-drop`: `ALL` sets no-new-privileges in the child, which is
/// what podvm can enforce without a privilege; a named capability is
/// refused by name, because podvm does not change the capability sets.
/// TODO/cli.md T-1655.
pub(super) fn cap_drop(verb: &str, a: &str) -> Result<bool, i32> {
    if a.eq_ignore_ascii_case("all") {
        return Ok(true);
    }
    eprintln!(
        "podvm {verb}: --cap-drop {a:?} is refused: podvm takes ALL only, as \
         no-new-privileges in the payload; it does not change the capability \
         sets (TODO/cli.md T-1655)"
    );
    Err(EXIT_FLAG_ERROR)
}
