//! The bundle's environment file, and the expansion it promises.
//!
//! ⛔ **This is the first of T-100's four measured improvements.** The file's
//! syntax is a shell's - `export `, single quotes that suppress expansion,
//! double quotes that do not - and the launcher this replaces expands an UNSET
//! variable to the LITERAL `${NAME}`. Every line of the shape
//!
//!     XDG_DATA_DIRS=${SHARUN_DIR}/share:${XDG_DATA_DIRS}:/usr/share
//!
//! then puts the eleven characters `${XDG_DATA_DIRS}` into a search path as a
//! RELATIVE component on any host where the variable is unset, which resolves
//! against the process's working directory - the user's, never the bundle's.
//! `scripts/common/sharun-expand.sh` is the reproduction, in three arms
//! against the artefact that ships.
//!
//! ⭐ Expanding to nothing is only half the fix, and saying so is the other
//! half: it turns a relative component into an EMPTY one, and an empty field
//! in a colon list means the working directory to a POSIX shell and to more
//! than one loader. So this module also implements `${NAME:+word}`, which is
//! how a writer says "and the separator too, only if it is set":
//!
//!     XDG_DATA_DIRS=${PRUN_DIR}/share${XDG_DATA_DIRS:+:${XDG_DATA_DIRS}}:/usr/share
//!
//! ⚠ The launcher cleans what IT composes (see `envx`). A value this file
//! assigns is the file's own statement and is set as written.
//!
//! SPDX-License-Identifier: 0BSD

use std::collections::BTreeMap;

/// One `.env` file, read.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DotEnv {
    /// Assignments in file order. ⚠ Order matters: a later line may reference
    /// a name an earlier line set, which is what a shell does.
    pub set: Vec<(String, String)>,
    /// Names an `unset` line asked for. They are applied AFTER everything
    /// else, so a file can set a variable for its own expansions and still
    /// keep it out of the payload's environment.
    pub unset: Vec<String>,
}

/// How a name is looked up during expansion. Passing this in rather than
/// reading the process environment is what makes the expander testable
/// without a process to mutate.
pub trait Lookup {
    fn get(&self, name: &str) -> Option<String>;
}

/// The real environment.
pub struct Process;

impl Lookup for Process {
    fn get(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

impl Lookup for BTreeMap<String, String> {
    fn get(&self, name: &str) -> Option<String> {
        BTreeMap::get(self, name).cloned()
    }
}

/// Parse a `.env` body. ⛔ A malformed line is skipped and reported by the
/// caller rather than taken as a reason to stop: a bundle that will not start
/// because one line has no `=` is worse than one that starts without it, and
/// the launcher this replaces calls `exit(1)` on a read error.
pub fn parse<L: Lookup>(body: &str, env: &L) -> (DotEnv, Vec<String>) {
    let mut out = DotEnv::default();
    let mut complaints = Vec::new();
    // ⭐ A file's own earlier assignments are visible to its later lines, and
    // they win over the process environment, which is what a shell sourcing
    // the file would do.
    let mut local: BTreeMap<String, String> = BTreeMap::new();

    for (n, raw) in body.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("unset ") {
            for name in rest.split_whitespace() {
                out.unset.push(name.to_string());
            }
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, val)) = line.split_once('=') else {
            complaints.push(format!("line {}: no '=', ignored: {}", n + 1, line));
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            complaints.push(format!("line {}: empty name, ignored", n + 1));
            continue;
        }
        let val = val.trim();
        let expanded = if let Some(inner) = strip_pair(val, '\'') {
            // Single quotes suppress expansion, in this file as in a shell.
            inner.to_string()
        } else {
            let inner = strip_pair(val, '"').unwrap_or(val);
            expand(inner, &Layered { local: &local, outer: env })
        };
        local.insert(key.to_string(), expanded.clone());
        out.set.push((key.to_string(), expanded));
    }
    (out, complaints)
}

/// A lookup that answers from the file's own assignments first.
struct Layered<'a, L: Lookup> {
    local: &'a BTreeMap<String, String>,
    outer: &'a L,
}

impl<L: Lookup> Lookup for Layered<'_, L> {
    fn get(&self, name: &str) -> Option<String> {
        match self.local.get(name) {
            Some(v) => Some(v.clone()),
            None => self.outer.get(name),
        }
    }
}

fn strip_pair(s: &str, q: char) -> Option<&str> {
    let b = s.as_bytes();
    if b.len() >= 2 && b[0] == q as u8 && b[b.len() - 1] == q as u8 {
        Some(&s[1..s.len() - 1])
    } else {
        None
    }
}

/// Expand a value.
///
/// ⭐ The forms are the shell's, and each one exists because a bundle needs
/// it: `${NAME}` and `$NAME` to interpolate, `${NAME:-word}` for a default,
/// `${NAME:+word}` to make a SEPARATOR conditional, and the `-`/`+` pair
/// without the colon for the "set but empty" distinction that
/// TODO/RULES.md records a check having got wrong.
pub fn expand<L: Lookup>(val: &str, env: &L) -> String {
    let mut out = String::with_capacity(val.len());
    let b = val.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() && b[i + 1] == b'$' {
            // A backslash before a dollar is how a value says a literal one.
            out.push('$');
            i += 2;
            continue;
        }
        if b[i] != b'$' {
            out.push(b[i] as char);
            i += 1;
            continue;
        }
        i += 1;
        if i >= b.len() {
            out.push('$');
            break;
        }
        if b[i] == b'{' {
            let Some(end) = matching_brace(b, i) else {
                // ⚠ An unbalanced brace is left alone rather than guessed at.
                out.push('$');
                continue;
            };
            let body = &val[i + 1..end];
            out.push_str(&braced(body, env));
            i = end + 1;
            continue;
        }
        let start = i;
        while i < b.len() && (b[i] == b'_' || (b[i] as char).is_ascii_alphanumeric()) {
            i += 1;
        }
        if i == start {
            out.push('$');
            continue;
        }
        // ⛔ Unset expands to NOTHING. The launcher this replaces puts the
        // reference back, which is the defect this whole module is about.
        out.push_str(&env.get(&val[start..i]).unwrap_or_default());
    }
    out
}

/// Find the `}` that closes the `{` at `i`, allowing one level of nesting so
/// `${A:+:${A}}` - the exact shape the fix asks a writer to use - parses.
fn matching_brace(b: &[u8], i: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut j = i;
    while j < b.len() {
        match b[j] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

fn braced<L: Lookup>(body: &str, env: &L) -> String {
    for (marker, colon) in [(":-", true), (":+", true), ("-", false), ("+", false)] {
        let Some(at) = find_op(body, marker) else {
            continue;
        };
        let name = &body[..at];
        let word = &body[at + marker.len()..];
        let cur = env.get(name);
        let present = match (colon, &cur) {
            (true, Some(v)) => !v.is_empty(),
            (false, Some(_)) => true,
            (_, None) => false,
        };
        let plus = marker.ends_with('+');
        return if present == plus {
            // `+` yields the word when present; `-` yields it when absent.
            expand(word, env)
        } else if plus {
            String::new()
        } else {
            cur.unwrap_or_default()
        };
    }
    env.get(body).unwrap_or_default()
}

/// Find an operator at the top level of a braced body. ⚠ The scan stops at the
/// first `$`, so an operator inside a nested reference is not mistaken for
/// this one's.
fn find_op(body: &str, marker: &str) -> Option<usize> {
    let b = body.as_bytes();
    let m = marker.as_bytes();
    let mut i = 0;
    while i + m.len() <= b.len() {
        if b[i] == b'$' {
            return None;
        }
        if &b[i..i + m.len()] == m && i > 0 {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    // ⛔ The defect this module exists for, stated as its own case.
    #[test]
    fn an_unset_reference_expands_to_nothing() {
        let e = env(&[]);
        assert_eq!(expand("${ABSENT}", &e), "");
        assert_eq!(expand("$ABSENT", &e), "");
        assert_eq!(expand("/a:${ABSENT}:/b", &e), "/a::/b");
    }

    // ⭐ And the half that makes the fix complete: the separator goes too.
    #[test]
    fn the_conditional_form_removes_the_separator_as_well() {
        let e = env(&[]);
        assert_eq!(expand("/a${ABSENT:+:${ABSENT}}:/b", &e), "/a:/b");
        let e = env(&[("SET", "/x")]);
        assert_eq!(expand("/a${SET:+:${SET}}:/b", &e), "/a:/x:/b");
    }

    #[test]
    fn a_set_reference_interpolates() {
        let e = env(&[("D", "/opt/app")]);
        assert_eq!(expand("${D}/share", &e), "/opt/app/share");
        assert_eq!(expand("$D/share", &e), "/opt/app/share");
    }

    #[test]
    fn the_colon_forms_tell_empty_from_unset() {
        let e = env(&[("EMPTY", "")]);
        assert_eq!(expand("${EMPTY:-fallback}", &e), "fallback");
        assert_eq!(expand("${EMPTY-fallback}", &e), "");
        assert_eq!(expand("${EMPTY:+word}", &e), "");
        assert_eq!(expand("${EMPTY+word}", &e), "word");
        assert_eq!(expand("${GONE-fallback}", &e), "fallback");
        assert_eq!(expand("${GONE+word}", &e), "");
    }

    #[test]
    fn a_literal_dollar_survives() {
        let e = env(&[]);
        assert_eq!(expand("100\\$", &e), "100$");
        assert_eq!(expand("a $ b", &e), "a $ b");
        assert_eq!(expand("${unbalanced", &e), "${unbalanced");
    }

    // ⛔ `$LIB` is the LOADER's expansion, not this one's, and the rule here is
    // uniform: an unset name expands to nothing, `$LIB` included. A bundle
    // that needs the loader to see the token writes it with the backslash, and
    // this case is what says so. ⚠ Upstream's own tracker carries a report of
    // `$LIB` expanding to the wrong value on a host whose layout differs from
    // the build's, which is a reason to be able to pass it through exactly.
    #[test]
    fn the_loader_s_own_token_needs_the_backslash_form() {
        let e = env(&[]);
        assert_eq!(expand("/usr/$LIB/vkbasalt", &e), "/usr//vkbasalt");
        assert_eq!(expand("/usr/\\$LIB/vkbasalt", &e), "/usr/$LIB/vkbasalt");
    }

    #[test]
    fn parsing_takes_the_shell_shapes() {
        let e = env(&[("HOME", "/home/u")]);
        let (got, bad) = parse(
            "# a comment\n\nexport A=1\nB='${HOME}'\nC=\"${HOME}/c\"\nD=$HOME/d\nunset E F\n",
            &e,
        );
        assert!(bad.is_empty());
        assert_eq!(
            got.set,
            vec![
                ("A".to_string(), "1".to_string()),
                ("B".to_string(), "${HOME}".to_string()),
                ("C".to_string(), "/home/u/c".to_string()),
                ("D".to_string(), "/home/u/d".to_string()),
            ]
        );
        assert_eq!(got.unset, vec!["E".to_string(), "F".to_string()]);
    }

    #[test]
    fn a_later_line_sees_an_earlier_one() {
        let e = env(&[]);
        let (got, _) = parse("A=/opt\nB=${A}/bin\n", &e);
        assert_eq!(got.set[1].1, "/opt/bin");
    }

    #[test]
    fn a_line_with_no_equals_is_reported_and_skipped() {
        let e = env(&[]);
        let (got, bad) = parse("GOOD=1\nnonsense\n", &e);
        assert_eq!(got.set.len(), 1);
        assert_eq!(bad.len(), 1);
        assert!(bad[0].contains("no '='"));
    }
}
