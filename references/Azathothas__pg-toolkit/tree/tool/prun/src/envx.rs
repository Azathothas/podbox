//! Reading and writing the environment, and the one rule about search paths.
//!
//! ⭐ The library path this launcher hands the loader never carries an empty or
//! a relative component. Both mean the process's working directory - the
//! user's, never the bundle's - and a search path that reaches the working
//! directory is how a bundle loads whatever happens to be sitting in the
//! directory the user double-clicked from.
//!
//! ⛔ The rule binds `join`, which is the launcher's own product, and it stops
//! there. A caller's `PATH` is the CALLER's list: `prepend` checks what it
//! adds and leaves every field already in it exactly as it found it, because
//! a launcher that quietly rewrote a user's search path would be doing
//! something nobody asked for and nobody could see.
//!
//! ⚠ And a value a bundle's own `.env` assigns is that file's statement, set
//! as written; `dotenv` gives it `${NAME:+word}` so it can say what it means.
//!
//! SPDX-License-Identifier: 0BSD

use std::ffi::OsStr;

/// Read a variable, absent and empty alike answering with the empty string.
/// ⚠ The distinction between the two matters in `.env` expansion and nowhere
/// else in this launcher, and `dotenv` is where it is kept.
pub fn get(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

/// Whether a variable is set at all.
pub fn present(name: &str) -> bool {
    std::env::var_os(name).is_some()
}

/// Set a variable to exactly this value.
pub fn set<K: AsRef<OsStr>, V: AsRef<OsStr>>(name: K, val: V) {
    std::env::set_var(name, val);
}

/// Remove a variable.
pub fn unset<K: AsRef<OsStr>>(name: K) {
    std::env::remove_var(name);
}

/// Put a directory at the FRONT of a colon list, once.
///
/// ⚠ An entry already in the list is not moved. Re-ordering a path a caller
/// set deliberately is a change nobody asked for, and the launcher's own
/// entries all arrive through here, so a repeat means the caller had it too.
pub fn prepend<K: AsRef<OsStr>, V: AsRef<OsStr>>(name: K, val: V) {
    let name = name.as_ref();
    let val = val.as_ref().to_string_lossy().to_string();
    if !usable(&val) {
        return;
    }
    let old = std::env::var(name).unwrap_or_default();
    if old.is_empty() {
        std::env::set_var(name, &val);
        return;
    }
    if old.split(':').any(|f| f == val) {
        return;
    }
    std::env::set_var(name, format!("{val}:{old}"));
}

/// Put a directory at the END of a colon list, once.
pub fn append<K: AsRef<OsStr>, V: AsRef<OsStr>>(name: K, val: V) {
    let name = name.as_ref();
    let val = val.as_ref().to_string_lossy().to_string();
    if !usable(&val) {
        return;
    }
    let old = std::env::var(name).unwrap_or_default();
    if old.is_empty() {
        std::env::set_var(name, &val);
        return;
    }
    if old.split(':').any(|f| f == val) {
        return;
    }
    std::env::set_var(name, format!("{old}:{val}"));
}

/// Join components into a search path, dropping what must not be in one and
/// keeping the first of any repeat.
///
/// ⛔ De-duplication is not cosmetic here. The library path this launcher
/// builds is handed to the loader, which stats every directory in it for every
/// object it opens; a list with the same directory four times is four times
/// the work on every lookup that misses.
pub fn join<I, S>(parts: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out: Vec<String> = Vec::new();
    for p in parts {
        for field in p.as_ref().split(':') {
            if usable(field) && !out.iter().any(|o| o == field) {
                out.push(field.to_string());
            }
        }
    }
    out.join(":")
}

/// Whether a component belongs in the library path the launcher composed.
///
/// ⛔ Empty is the case that matters, and it is the one a naive fix creates:
/// expanding an unset `${VAR}` to nothing turns `a:${VAR}:b` into `a::b`, and
/// an empty field is the working directory to a POSIX shell and to more than
/// one loader. A relative component is the same hazard spelled out.
pub fn usable(field: &str) -> bool {
    !field.is_empty() && field.starts_with('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_component_is_not_usable() {
        assert!(!usable(""));
        assert!(!usable("."));
        assert!(!usable("relative/path"));
        assert!(!usable("${XDG_DATA_DIRS}"));
        assert!(usable("/usr/lib"));
    }

    #[test]
    fn joining_drops_the_empty_and_the_relative() {
        assert_eq!(join(["/a::/b"]), "/a:/b");
        assert_eq!(join(["/a:.:/b"]), "/a:/b");
        assert_eq!(join(["/a", "", "/b"]), "/a:/b");
        assert_eq!(join(["/a:${NOPE}:/b"]), "/a:/b");
    }

    #[test]
    fn joining_keeps_the_first_of_a_repeat() {
        assert_eq!(join(["/a:/b", "/b:/c", "/a"]), "/a:/b:/c");
    }

    #[test]
    fn order_is_the_order_given() {
        assert_eq!(join(["/first", "/second", "/third"]), "/first:/second:/third");
    }
}
