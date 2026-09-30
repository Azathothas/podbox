//! Starting a bundle the way a desktop starts one.
//!
//! ⭐ A desktop does not run the payload. It runs one file at the top of the
//! bundle, by a name the format fixes, and everything about which program that
//! means is the bundle's own business. So this mode's whole job is: tell the
//! payload where the HOST's directories were before the bundle rearranged
//! them, then work out which program the bundle is.
//!
//! ⛔ The host values are not a convenience. A bundle that redirects the
//! payload's home so its configuration does not collide with the host's leaves
//! the payload no way to reach the user's real files - a file chooser opening
//! in a directory the user has never seen. `HOST_*` is how the payload asks.
//!
//! SPDX-License-Identifier: 0BSD

use std::path::{Path, PathBuf};

use crate::envx;
use crate::names;
use crate::report::Report;
use crate::sys;

/// Apply the mode, and return the program to run and how to run it.
pub fn prepare(root: &Path, bin: &Path, r: &mut Report) -> Option<Program> {
    let home = first_of(&["REAL_HOME", "HOME"], "");
    envx::set("HOST_HOME", &home);
    envx::set(
        "HOST_XDG_CONFIG_HOME",
        first_of(&["REAL_XDG_CONFIG_HOME", "XDG_CONFIG_HOME"], &format!("{home}/.config")),
    );
    envx::set(
        "HOST_XDG_DATA_HOME",
        first_of(&["REAL_XDG_DATA_HOME", "XDG_DATA_HOME"], &format!("{home}/.local/share")),
    );
    envx::set(
        "HOST_XDG_CACHE_HOME",
        first_of(&["REAL_XDG_CACHE_HOME", "XDG_CACHE_HOME"], &format!("{home}/.cache")),
    );
    envx::set(
        "HOST_XDG_STATE_HOME",
        first_of(&["REAL_XDG_STATE_HOME", "XDG_STATE_HOME"], &format!("{home}/.local/state")),
    );

    envx::set("APPIMAGE_ARCH", std::env::consts::ARCH);
    envx::set("APPIMAGE_UID", sys::uid().to_string());
    // ⚠ Before PATH is touched, or this records the bundle's own answer.
    envx::set("HOSTPATH", envx::get("PATH"));
    envx::prepend("PATH", bin);
    envx::set("APPDIR", root);
    names::export_dir(&root.to_string_lossy());
    if !envx::present("ARGV0") {
        // ⭐ Carried from the launcher this project read beside the fork: a
        // payload that re-executes itself needs the name it was invoked under,
        // and the fork dropped this on the way past.
        if let Some(a0) = std::env::args().next() {
            envx::set("ARGV0", a0);
        }
    }
    r.say("APPDIR", &root.to_string_lossy());
    r.say("HOSTPATH", &envx::get("HOSTPATH"));

    // A bundle may carry its own opening script, and then it decides.
    let script = root.join("AppRun.sh");
    if script.exists() {
        return Some(Program::Script(script));
    }
    match name_of(root) {
        Some(name) => {
            r.note(&format!("bundle names {name} as its program"));
            Some(Program::Named(bin.join(name)))
        }
        None => {
            r.refused(&format!(
                "{} carries no .desktop entry with an Exec line and no .app file, so nothing says which program it is",
                root.display()
            ));
            None
        }
    }
}

/// What the bundle turned out to be.
pub enum Program {
    /// The bundle's own opening script.
    Script(PathBuf),
    /// A program in the bundle's `bin/`.
    Named(PathBuf),
}

/// Which program a bundle is.
///
/// ⭐ Two sources, and the second is carried from the launcher upstream of the
/// fork: a one-line `.app` file. The fork reads only the desktop entry, so a
/// bundle with no desktop integration - a command-line tool - has no way to
/// say what it is. That is a capability, and TODO/port.md rule 2 keeps it.
pub fn name_of(root: &Path) -> Option<String> {
    if let Some(n) = from_desktop(root) {
        return Some(n);
    }
    let app = root.join(".app");
    let body = std::fs::read_to_string(app).ok()?;
    let first = body.lines().next()?.trim();
    clean(first)
}

fn from_desktop(root: &Path) -> Option<String> {
    let entries = root.read_dir().ok()?;
    // ⚠ Sorted, so a bundle carrying two desktop entries picks the same one on
    // every host. A directory walk's order is the filesystem's.
    let mut names: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        // ⚠ The whole suffix, not the extension. A file called `.desktop`
        // with nothing before it has no extension at all by the standard
        // library's reckoning, and one called `app.Desktop` has the wrong one;
        // the format's own rule is the ending.
        .filter(|p| p.is_file() && p.to_string_lossy().ends_with(".desktop"))
        .collect();
    names.sort();
    for p in names {
        let Ok(body) = std::fs::read_to_string(&p) else { continue };
        for line in body.lines() {
            let Some(rest) = line.strip_prefix("Exec=") else { continue };
            let Some(first) = rest.split_whitespace().next() else { continue };
            if let Some(n) = clean(first) {
                return Some(n);
            }
        }
    }
    None
}

/// A desktop entry's `Exec` may be quoted and may be a path.
fn clean(s: &str) -> Option<String> {
    let s = s.trim().trim_matches(|c| c == '\'' || c == '"');
    let base = Path::new(s).file_name()?.to_string_lossy().into_owned();
    if base.is_empty() {
        None
    } else {
        Some(base)
    }
}

fn first_of(names: &[&str], fallback: &str) -> String {
    for n in names {
        let v = envx::get(n);
        if !v.is_empty() {
            return v;
        }
    }
    fallback.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("prun-apprun-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_desktop_entry_names_the_program() {
        let d = bundle("desktop");
        std::fs::write(d.join("app.desktop"), "[Desktop Entry]\nExec=/usr/bin/thing %U\n").unwrap();
        assert_eq!(name_of(&d).as_deref(), Some("thing"));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_quoted_exec_line_is_unwrapped() {
        let d = bundle("quoted");
        std::fs::write(d.join("app.desktop"), "Exec='my-thing'\n").unwrap();
        assert_eq!(name_of(&d).as_deref(), Some("my-thing"));
        std::fs::remove_dir_all(&d).unwrap();
    }

    // ⭐ The capability the fork dropped and this one keeps.
    #[test]
    fn a_bundle_with_no_desktop_entry_can_still_say_what_it_is() {
        let d = bundle("appfile");
        std::fs::write(d.join(".app"), "toolname\n").unwrap();
        assert_eq!(name_of(&d).as_deref(), Some("toolname"));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn the_desktop_entry_wins_over_the_app_file() {
        let d = bundle("both");
        std::fs::write(d.join("a.desktop"), "Exec=from-desktop\n").unwrap();
        std::fs::write(d.join(".app"), "from-app-file\n").unwrap();
        assert_eq!(name_of(&d).as_deref(), Some("from-desktop"));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_bundle_that_says_nothing_answers_nothing() {
        let d = bundle("silent");
        assert_eq!(name_of(&d), None);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
