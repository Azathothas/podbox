//! Which directories under the bundle's library tree hold shared objects.
//!
//! ⛔ **This is the fourth of T-100's four measured improvements.** The tool
//! this replaces calls a directory a library directory when a file in it ends
//! with `.so` OR CONTAINS `.so.` anywhere, so a directory holding
//! `ld.so.cache` - an index, not an object - is added to the loader's search
//! path. docs/AGENTS.md already carries the rule and what the substring test
//! cost this project elsewhere: require `.so`, or `.so` followed by a version,
//! AT THE END.
//!
//! ⚠ The file format is the bundle's and is not changed here: a `lib.path`
//! beside the library tree, one directory per line, with `+` standing for the
//! tree's own root so the file survives the bundle being mounted somewhere
//! else. That is frozen output (T-117), and a format of ours is a rename's
//! business rather than a port's.
//!
//! SPDX-License-Identifier: 0BSD

use std::path::{Path, PathBuf};

/// Directories that hold objects the LOADER must not search.
///
/// ⚠ A Python extension tree is full of `.so` files that are modules rather
/// than libraries: they are opened by name by the interpreter, and putting
/// their directory on the loader's path makes every failed lookup in the
/// process stat one more directory for nothing.
const NOT_A_LIBRARY_DIR: &[&str] = &["lib-dynload"];

/// Whether a file name is a shared object.
///
/// ⭐ The whole improvement is here. `libc.so.6` and `libz.so` are objects;
/// `ld.so.cache`, `ld.so.conf` and `libfoo.so.old` are not.
pub fn is_shared_object(name: &str) -> bool {
    if name.ends_with(".so") {
        return !name.starts_with('.') || name.len() > 3;
    }
    let Some(at) = name.rfind(".so.") else {
        return false;
    };
    let ver = &name[at + 4..];
    // A soname's version is digits and dots, and it begins with a digit.
    !ver.is_empty()
        && ver.as_bytes()[0].is_ascii_digit()
        && ver.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && !ver.ends_with('.')
}

/// Walk a library tree and return every subdirectory holding a shared object,
/// as paths relative to the tree, sorted, without the tree's own root.
///
/// ⛔ The walk follows no symlink out of the tree and it does not recurse
/// through one. A bundle whose `lib/` contains a link to `/usr/lib` would
/// otherwise enumerate the host's library layout into the bundle's own search
/// path, which is the opposite of what the tree exists for.
pub fn scan(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let Ok(entries) = dir.read_dir() else { continue };
        let mut holds_object = false;
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                if NOT_A_LIBRARY_DIR.contains(&name.as_str()) {
                    continue;
                }
                let child = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
                stack.push((e.path(), child));
                continue;
            }
            // ⚠ A symlink to an object is an object: a bundle's `lib/` is
            // mostly `libfoo.so.6 -> libfoo.so.6.4.0`, and a walk that only
            // believed regular files would call almost nothing a library.
            if is_shared_object(&name) {
                holds_object = true;
            }
        }
        if holds_object && !rel.is_empty() {
            out.push(rel);
        }
    }
    out.sort();
    out
}

/// The body of a `lib.path` file for a tree.
///
/// The first line is `+`, the tree's own root, and every other line is `+`
/// followed by a subdirectory. ⚠ Order is the file's: the root first, so the
/// bundle's own libraries win over its plugin trees.
pub fn render(dirs: &[String]) -> String {
    let mut s = String::from("+\n");
    for d in dirs {
        s.push('+');
        s.push('/');
        s.push_str(d);
        s.push('\n');
    }
    s
}

/// Turn a `lib.path` body into absolute directories under a tree.
///
/// ⛔ A line that is not rooted at `+` is dropped rather than joined. The file
/// is inside the bundle and a line naming an absolute host directory would put
/// the host's libraries ahead of the bundle's, which is the failure the whole
/// rung exists to prevent - and it would do it silently.
pub fn resolve(root: &Path, body: &str) -> (Vec<PathBuf>, Vec<String>) {
    let mut out = Vec::new();
    let mut refused = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "+" {
            out.push(root.to_path_buf());
            continue;
        }
        match line.strip_prefix("+/") {
            Some(rel) if !rel.starts_with('/') && !rel.split('/').any(|c| c == "..") => {
                out.push(root.join(rel))
            }
            _ => refused.push(line.to_string()),
        }
    }
    (out, refused)
}

/// The subdirectory names one level under the tree, from a resolved list. The
/// directory-class rules key on these.
pub fn classes(root: &Path, dirs: &[PathBuf]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for d in dirs {
        let Ok(rel) = d.strip_prefix(root) else { continue };
        let Some(first) = rel.components().next() else { continue };
        let name = first.as_os_str().to_string_lossy().into_owned();
        if !name.is_empty() && !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ⛔ The defect this module exists for.
    #[test]
    fn an_index_is_not_a_shared_object() {
        assert!(!is_shared_object("ld.so.cache"));
        assert!(!is_shared_object("ld.so.conf"));
        assert!(!is_shared_object("ld.so.preload"));
        assert!(!is_shared_object("libfoo.so.old"));
        assert!(!is_shared_object("libfoo.so.6.bak"));
        assert!(!is_shared_object("notes.txt"));
        assert!(!is_shared_object("libfoo.so."));
    }

    #[test]
    fn an_object_is_one() {
        assert!(is_shared_object("libz.so"));
        assert!(is_shared_object("libc.so.6"));
        assert!(is_shared_object("libcrypto.so.3"));
        assert!(is_shared_object("libfoo.so.6.4.0"));
        assert!(is_shared_object("_ssl.cpython-311-x86_64-linux-gnu.so"));
        assert!(is_shared_object("ld-linux-x86-64.so.2"));
    }

    #[test]
    fn a_lib_path_body_round_trips() {
        let root = Path::new("/opt/app/lib");
        let body = render(&["gtk-3.0".into(), "gio/modules".into()]);
        assert_eq!(body, "+\n+/gtk-3.0\n+/gio/modules\n");
        let (dirs, refused) = resolve(root, &body);
        assert!(refused.is_empty());
        assert_eq!(
            dirs,
            vec![
                PathBuf::from("/opt/app/lib"),
                PathBuf::from("/opt/app/lib/gtk-3.0"),
                PathBuf::from("/opt/app/lib/gio/modules"),
            ]
        );
        assert_eq!(classes(root, &dirs), vec!["gtk-3.0".to_string(), "gio".to_string()]);
    }

    // ⛔ A host directory in the bundle's own file is refused, not joined.
    #[test]
    fn a_line_that_escapes_the_tree_is_refused() {
        let root = Path::new("/opt/app/lib");
        let (dirs, refused) = resolve(root, "+\n/usr/lib\n+/../../etc\n+/ok\n");
        assert_eq!(dirs, vec![PathBuf::from("/opt/app/lib"), PathBuf::from("/opt/app/lib/ok")]);
        assert_eq!(refused, vec!["/usr/lib".to_string(), "+/../../etc".to_string()]);
    }

    #[test]
    fn the_scan_finds_only_directories_that_hold_objects() {
    // ⚠ The pid is in the name, and it is not decoration. `fs.protected_regular`
    // is 1 on an ordinary kernel, which refuses an O_CREAT open of an existing
    // file in a sticky world-writable directory when the opener does not own it.
    // A fixed name under /tmp therefore works until somebody runs this suite as a
    // second uid, and then fails as PermissionDenied for a reason that reads like
    // a broken checkout. Measured: five of these failed exactly that way after an
    // earlier run had left the files behind owned by another account.
        let base = std::env::temp_dir().join(format!("prun-scan-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        for d in ["", "gtk-3.0", "gio/modules", "papers", "lib-dynload"] {
            std::fs::create_dir_all(base.join(d)).unwrap();
        }
        std::fs::write(base.join("libc.so.6"), b"x").unwrap();
        std::fs::write(base.join("gtk-3.0/libgtk.so"), b"x").unwrap();
        std::fs::write(base.join("gio/modules/libgiofam.so"), b"x").unwrap();
        std::fs::write(base.join("papers/ld.so.cache"), b"x").unwrap();
        std::fs::write(base.join("lib-dynload/_ssl.so"), b"x").unwrap();
        let got = scan(&base);
        assert_eq!(got, vec!["gio/modules".to_string(), "gtk-3.0".to_string()]);
    }
}
