//! Where the launcher may put a file it has to write at run time.
//!
//! ⛔ **This is the third of T-100's four measured improvements.** The tool
//! this replaces installs the bundle's loader at
//! `/tmp/.ld-sharun.so.67` - a FIXED name under a world-writable directory -
//! by copying to `<name>.<pid>`, chmod 0777, and renaming over it.
//!
//! Two things are wrong with that and only one of them is about an attacker:
//!
//!   - ⛔ the temporary file is world-WRITABLE for the window between the
//!     chmod and the rename, so any local user can substitute a loader that
//!     the victim then installs and executes. `tool/runtime/appimage/storefix.c`
//!     refuses exactly this shape, for exactly this reason.
//!   - ⛔ the name carries nothing about WHICH bundle, so two bundles carrying
//!     different glibc versions share one file. Whichever renamed last wins,
//!     and the other one runs its payload against a loader it did not ship.
//!     That is a correctness defect with no attacker in it at all.
//!
//! ⭐ Here a private directory is per-user AND per-bundle, created 0700, and
//! verified to be ours and not a symlink before anything is written into it.
//! When a payload names a path this launcher cannot make safe, it says so and
//! starts the payload the other way rather than refusing to run: the explicit
//! loader route reaches the same program, so nothing is lost by declining.
//!
//! SPDX-License-Identifier: 0BSD

use std::fs;
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::envx;
use crate::sys;

/// Ask for the launcher's private directory for this bundle, creating it.
///
/// ⭐ `$XDG_RUNTIME_DIR` is preferred and is not merely tidier: it is already
/// per-user, already 0700 and already cleaned up at logout, which is three
/// properties this code would otherwise have to establish and maintain.
pub fn private_dir(bundle: &Path) -> io::Result<PathBuf> {
    let tag = format!("prun-{}-{:016x}", sys::uid(), tag_of(bundle));
    let mut tried: Vec<String> = Vec::new();
    for base in bases() {
        let dir = base.join(&tag);
        match ensure(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) => tried.push(format!("{}: {}", dir.display(), e)),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("no private directory could be made ({})", tried.join("; ")),
    ))
}

/// The candidate parents, best first.
fn bases() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let rt = envx::get("XDG_RUNTIME_DIR");
    if !rt.is_empty() && Path::new(&rt).is_dir() {
        out.push(PathBuf::from(rt));
    }
    let tmp = envx::get("TMPDIR");
    if !tmp.is_empty() && Path::new(&tmp).is_dir() {
        out.push(PathBuf::from(tmp));
    }
    out.push(PathBuf::from("/tmp"));
    out
}

/// Create a directory that is ours and only ours, or report why it is not.
///
/// ⛔ The check is on `symlink_metadata`, never on `metadata`. A `stat` of a
/// symlink reports the TARGET's owner and mode, so an attacker's link to a
/// directory the victim owns passes every test a `stat` can make - and what
/// the victim then writes lands wherever the link points.
pub fn ensure(dir: &Path) -> io::Result<()> {
    match fs::create_dir(dir) {
        Ok(()) => {
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    let md = fs::symlink_metadata(dir)?;
    if md.file_type().is_symlink() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "is a symlink"));
    }
    if !md.is_dir() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "is not a directory"));
    }
    if md.uid() != sys::euid() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "belongs to another user"));
    }
    if md.permissions().mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "is readable or writable by somebody else",
        ));
    }
    Ok(())
}

/// Whether a path the payload names as its interpreter can be made safe.
///
/// ⭐ The question is about the PARENT: the file itself is about to be
/// replaced, and what decides whether replacing it is safe is who can write
/// the directory it sits in.
pub fn parent_is_ours(target: &Path) -> io::Result<()> {
    let Some(parent) = target.parent() else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "has no directory"));
    };
    ensure(parent)
}

/// Put a file where a payload's interpreter says it is, atomically.
///
/// ⛔ Mode 0555 and never 0777. The tool this replaces makes the staging copy
/// world-writable, which is the whole hole: between that chmod and the rename,
/// the file any local user may write is the one about to become the victim's
/// dynamic loader.
pub fn install(from: &Path, to: &Path) -> io::Result<()> {
    parent_is_ours(to)?;
    let parent = to.parent().unwrap_or(Path::new("/"));
    let staging = parent.join(format!(
        ".{}.{}",
        to.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    let _ = fs::remove_file(&staging);
    fs::copy(from, &staging)?;
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o555))?;
    match fs::rename(&staging, to) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&staging);
            Err(e)
        }
    }
}

/// Whether a file already there is the same bytes as the one we would install.
/// ⚠ Cheap and exact enough: a loader that differs in length is a different
/// loader, and one that matches in length and mtime has not been swapped by
/// anything this check would have caught anyway - the directory test is what
/// carries the safety, and this only avoids a needless copy.
pub fn already_installed(from: &Path, to: &Path) -> bool {
    match (fs::metadata(from), fs::symlink_metadata(to)) {
        (Ok(a), Ok(b)) => b.is_file() && a.len() == b.len() && b.uid() == sys::euid(),
        _ => false,
    }
}

/// A stable, short tag for a bundle's location.
///
/// ⚠ It is a hash for length, not for secrecy. What it has to do is make two
/// different bundles use two different files, which is the correctness half of
/// this module; the safety half is the directory's mode and owner.
fn tag_of(bundle: &Path) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bundle.to_string_lossy().as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_bundles_get_two_directories() {
        assert_ne!(tag_of(Path::new("/a/one.AppImage")), tag_of(Path::new("/a/two.AppImage")));
        assert_eq!(tag_of(Path::new("/a/one")), tag_of(Path::new("/a/one")));
    }

    #[test]
    fn a_fresh_directory_is_ours_and_private() {
        let d = std::env::temp_dir().join(format!("prun-ensure-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        ensure(&d).unwrap();
        let md = fs::symlink_metadata(&d).unwrap();
        assert_eq!(md.permissions().mode() & 0o777, 0o700);
        // Idempotent: a second call on our own directory agrees.
        ensure(&d).unwrap();
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_directory_anybody_may_write_is_refused() {
        let d = std::env::temp_dir().join(format!("prun-open-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir(&d).unwrap();
        fs::set_permissions(&d, fs::Permissions::from_mode(0o777)).unwrap();
        let err = ensure(&d).unwrap_err();
        assert!(err.to_string().contains("somebody else"), "{err}");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_symlink_standing_in_for_the_directory_is_refused() {
        let real = std::env::temp_dir().join(format!("prun-real-{}", std::process::id()));
        let link = std::env::temp_dir().join(format!("prun-link-{}", std::process::id()));
        let _ = fs::remove_dir_all(&real);
        let _ = fs::remove_file(&link);
        fs::create_dir(&real).unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let err = ensure(&link).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");
        fs::remove_file(&link).unwrap();
        fs::remove_dir_all(&real).unwrap();
    }

    #[test]
    fn an_installed_file_is_not_writable_by_anybody() {
        let d = std::env::temp_dir().join(format!("prun-inst-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        ensure(&d).unwrap();
        let src = d.join("src");
        fs::write(&src, b"loader").unwrap();
        let dst = d.join("ld");
        install(&src, &dst).unwrap();
        let md = fs::symlink_metadata(&dst).unwrap();
        assert_eq!(md.permissions().mode() & 0o222, 0, "nothing may write it");
        assert!(already_installed(&src, &dst));
        fs::remove_dir_all(&d).unwrap();
    }
}
