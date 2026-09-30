//! Which of the HOST's library directories a bundle may reach, and in what
//! order.
//!
//! ⛔ **This is the second of T-100's four measured improvements.** The tool
//! this replaces appends a fixed list of host directories to every search
//! path it builds, with no way to say otherwise. `internal/bundle/hostpolicy.go`
//! therefore MODELS that list rather than controlling it, and "no host
//! directories at all" is a thing this project cannot currently express -
//! which is exactly the state a bundle wants when it is being measured for
//! whether it opened a host object.
//!
//! ⭐ Here the list is the bundle's, and the built-in is only what it falls
//! back to. `PRUN_HOST_LIBRARY_PATH` set to the empty string means no host
//! directory at any priority; set to a list means that list and nothing else.
//!
//! ⚠ Nothing is lost by the change. With the variable unset the order is the
//! one the artefact has today, row for row, and `defaults()` is the only place
//! it is written down.
//!
//! SPDX-License-Identifier: 0BSD

use std::path::Path;

use crate::envx;

/// The variable a bundle sets to take control of the host tail. ⭐ PRESENCE is
/// what switches, not truthiness: the empty value is the useful one and a
/// policy that could not be spelled "none" would not be a policy.
pub const HOST_PATH: &str = "PRUN_HOST_LIBRARY_PATH";

/// Where the host's proprietary graphics driver puts the libraries that must
/// come from the host.
///
/// ⭐ It is not an opt-in and it has no switch of its own, for an ABI reason
/// rather than a preference: the proprietary driver is built against a glibc
/// old enough that any bundled glibc satisfies it, and it is the counterpart
/// of a kernel module the user installed, so a bundled copy would be a stale
/// copy. `internal/bundle/hostpolicy.go` carries the argument.
const DRIVER_DIRS: &[&str] = &["/etc/libnvidiacurrent"];

/// The ordinary host directories, in the order the artefact uses today.
const GENERIC: &[&str] = &["/usr/local/lib", "/usr/lib", "/lib"];

/// The 64-bit and 32-bit tails.
const WIDE64: &[&str] = &["/usr/local/lib64", "/usr/lib64", "/lib64"];
const WIDE32: &[&str] = &["/usr/local/lib32", "/usr/lib32", "/lib32"];

/// Where a relocatable distribution and a driver overlay put theirs.
///
/// ⚠ `/run/opengl-driver/lib` is not decoration. A tracker this project reads
/// carries a NixOS failure fixed by scanning exactly that directory, and a
/// search order missing this row fails on one distribution only, which is the
/// worst kind of missing row.
const OVERLAY: &[&str] = &["/run/opengl-driver/lib", "/run/current-system/sw/lib"];

/// The multiarch directory for the architecture this launcher was built for.
const fn triple_dirs(elf32: bool) -> &'static [&'static str] {
    #[cfg(target_arch = "x86_64")]
    {
        if elf32 {
            &["/usr/lib/i386-linux-gnu"]
        } else {
            &["/usr/lib/x86_64-linux-gnu"]
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        let _ = elf32;
        &["/usr/lib/aarch64-linux-gnu"]
    }
    #[cfg(target_arch = "riscv64")]
    {
        let _ = elf32;
        &["/usr/lib/riscv64-linux-gnu"]
    }
    #[cfg(target_arch = "loongarch64")]
    {
        let _ = elf32;
        &["/usr/lib/loongarch64-linux-gnu"]
    }
    #[cfg(all(target_arch = "powerpc64", target_endian = "big"))]
    {
        let _ = elf32;
        &["/usr/lib/powerpc64-linux-gnu"]
    }
    #[cfg(all(target_arch = "powerpc64", target_endian = "little"))]
    {
        let _ = elf32;
        &["/usr/lib/powerpc64le-linux-gnu"]
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "loongarch64",
        target_arch = "powerpc64"
    )))]
    {
        let _ = elf32;
        &[]
    }
}

/// What the bundle decided about the host.
#[derive(Debug, PartialEq, Eq)]
pub enum Policy {
    /// The bundle said nothing, so the artefact's own order applies.
    Default,
    /// The bundle named the host directories it wants, possibly none.
    Stated(Vec<String>),
}

impl Policy {
    /// Read the policy. ⛔ The variable is REMOVED once read: it is the
    /// bundle's instruction to its own launcher, and leaving it in the
    /// environment would hand it to every child the payload starts.
    pub fn read() -> Policy {
        match std::env::var(HOST_PATH) {
            Ok(v) => {
                envx::unset(HOST_PATH);
                Policy::Stated(v.split(':').map(|s| s.to_string()).collect())
            }
            Err(_) => Policy::Default,
        }
    }

    /// The host directories, in order, for a payload of this class.
    ///
    /// ⚠ A directory that is not there is dropped. The loader stats every
    /// entry of the path on every lookup that misses, and a list that names
    /// six directories no host has is six wasted stats per miss - but the
    /// stronger reason is that a stated policy should read as what it got.
    pub fn dirs(&self, elf32: bool, cache: &Path) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut push = |d: &str| {
            if envx::usable(d) && Path::new(d).is_dir() {
                out.push(d.to_string());
            }
        };
        match self {
            Policy::Stated(list) => {
                for d in list {
                    push(d);
                }
            }
            Policy::Default => {
                for d in DRIVER_DIRS {
                    push(d);
                }
                for d in GENERIC {
                    push(d);
                }
                for d in if elf32 { WIDE32 } else { WIDE64 } {
                    push(d);
                }
                for d in triple_dirs(elf32) {
                    push(d);
                }
                for d in cache_dirs(cache) {
                    push(&d);
                }
                for d in OVERLAY {
                    push(d);
                }
            }
        }
        out
    }
}

/// The directories named in the host's loader cache.
///
/// ⭐ Reading the cache is how a bundle reaches a host layout nobody listed:
/// a distribution that puts its libraries somewhere none of the constants
/// above name still has them in the cache, because that is what the cache is.
///
/// ⚠ The scan is over printable runs rather than over the cache's own format,
/// deliberately: there are three formats in the field - the old one, the
/// current glibc one and musl's absence of one - and a parser for the current
/// one answers nothing on the other two while a run scan answers all three.
/// The cost is a false directory now and then, and every candidate is stat'd
/// before it is used.
pub fn cache_dirs(cache: &Path) -> Vec<String> {
    let Ok(data) = std::fs::read(cache) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        while i < data.len() && !printable(data[i]) {
            i += 1;
        }
        let start = i;
        while i < data.len() && printable(data[i]) {
            i += 1;
        }
        if start >= data.len() || data[start] != b'/' {
            continue;
        }
        let run = &data[start..i];
        let Some(cut) = run.iter().rposition(|&c| c == b'/') else { continue };
        if cut == 0 {
            continue;
        }
        let Ok(dir) = std::str::from_utf8(&run[..cut]) else { continue };
        if !out.iter().any(|o| o == dir) {
            out.push(dir.to_string());
        }
    }
    // ⚠ Sorted so the order does not depend on where in the file a name
    // happened to land, which would make one host's search order differ from
    // another's for no reason anybody could see.
    out.sort();
    out
}

fn printable(b: u8) -> bool {
    (0x20..=0x7e).contains(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stated_empty_policy_reaches_no_host_directory() {
        let p = Policy::Stated(vec![String::new()]);
        assert!(p.dirs(false, Path::new("/nonexistent")).is_empty());
    }

    #[test]
    fn a_stated_policy_keeps_only_what_is_there() {
        let p = Policy::Stated(vec!["/".into(), "/definitely-not-a-directory-here".into()]);
        assert_eq!(p.dirs(false, Path::new("/nonexistent")), vec!["/".to_string()]);
    }

    #[test]
    fn a_relative_component_never_reaches_the_path() {
        let p = Policy::Stated(vec![".".into(), "lib".into(), String::new()]);
        assert!(p.dirs(false, Path::new("/nonexistent")).is_empty());
    }

    #[test]
    fn the_cache_scan_takes_directories_and_not_files() {
    // ⚠ The pid is in the name, and it is not decoration. `fs.protected_regular`
    // is 1 on an ordinary kernel, which refuses an O_CREAT open of an existing
    // file in a sticky world-writable directory when the opener does not own it.
    // A fixed name under /tmp therefore works until somebody runs this suite as a
    // second uid, and then fails as PermissionDenied for a reason that reads like
    // a broken checkout. Measured: five of these failed exactly that way after an
    // earlier run had left the files behind owned by another account.
        let f = std::env::temp_dir().join(format!("prun-cache-test-{}", std::process::id()));
        let mut blob = Vec::new();
        blob.extend_from_slice(b"ld.so.cache-1.1\0");
        blob.extend_from_slice(b"libc.so.6\0/usr/lib/x86_64-linux-gnu/libc.so.6\0");
        blob.extend_from_slice(b"/opt/vendor/lib/libthing.so.1\0");
        std::fs::write(&f, &blob).unwrap();
        let got = cache_dirs(&f);
        assert_eq!(
            got,
            vec!["/opt/vendor/lib".to_string(), "/usr/lib/x86_64-linux-gnu".to_string()]
        );
    }

    #[test]
    fn a_missing_cache_is_no_directories_rather_than_an_error() {
        assert!(cache_dirs(Path::new("/no/such/cache")).is_empty());
    }
}
