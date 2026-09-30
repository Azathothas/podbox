//! TODO/enter.md T-1317: entering a payload without `chroot(2)`.
//!
//! Where the probe's chroot leg is denied, the chroot sequence cannot run,
//! but the payload file is still readable from the host side. This module
//! holds the pieces of entering it as it is, with the host's root:
//!
//! * a dynamic payload runs through the image's own loader, executed by
//!   host path, with the image's library directories on `LD_LIBRARY_PATH`
//!   and the interposer preloaded by host path;
//! * a static payload runs from a staged memfd, exactly as the ladder's
//!   memfd rung stages it, minus the chroot beneath.
//!
//! What this rung is not: isolation of any kind. The payload sees the
//! host's filesystem with only its working directory inside the image,
//! and absolute paths resolve on the host except where the interposer
//! rewrites them. The banner names all of it on every userland run, and
//! `PODBOX_ACTIVE_MODE` reads `userland`.
//!
//! Scripts (`#!`) are refused: their interpreter path resolves on the
//! host root, so the image's script would run under the host's
//! interpreter. Foreign architectures are refused: `binfmt_misc`
//! resolution without a chroot is unproven ground this rung does not
//! break.

use crate::memfd;

/// Which no-chroot family a payload takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Through the image's own loader, with the interposer preloaded.
    Loader,
    /// From a staged memfd, for static payloads.
    Memfd,
}

impl Family {
    pub fn name(self) -> &'static str {
        match self {
            Family::Loader => "loader",
            Family::Memfd => "memfd",
        }
    }
}

/// A dynamic payload's loader invocation, all paths host-absolute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderPlan {
    /// The image's loader, by host path.
    pub loader_host: String,
    /// The payload, by host path.
    pub payload_host: String,
    /// The image's library directories, by host path, that exist.
    pub lib_dirs: Vec<String>,
}

/// The image's library directories that exist, by host path.
///
/// The loader's own directory first (it is a library directory wherever
/// the image keeps its loader), then the standard ones. An empty answer
/// is a refusal upstream: a dynamic payload with nowhere to load from
/// fails inside the loader with nothing attached to it.
pub fn lib_dirs(rootfs: &str, interp_guest: &str) -> Vec<String> {
    let base = rootfs.trim_end_matches('/');
    let mut out = Vec::new();
    let mut push = |dir: String| {
        if !out.contains(&dir) && std::path::Path::new(&dir).is_dir() {
            out.push(dir);
        }
    };
    if let Some((d, _)) = interp_guest.rsplit_once('/') {
        if !d.is_empty() {
            push(format!("{base}{d}"));
        }
    }
    for d in [
        "/lib",
        "/lib64",
        "/usr/lib",
        "/usr/lib64",
        "/lib/x86_64-linux-gnu",
        "/lib/aarch64-linux-gnu",
        "/lib/riscv64-linux-gnu",
        "/lib/s390x-linux-gnu",
        "/lib/ppc64le-linux-gnu",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
        "/usr/lib/riscv64-linux-gnu",
        "/usr/lib/s390x-linux-gnu",
        "/usr/lib/ppc64le-linux-gnu",
    ] {
        push(format!("{base}{d}"));
    }
    out
}

/// The program path the loader opens, keeping the invoked name.
///
/// The resolver answers the file; but the kernel's loader opens by path
/// and the payload dispatches on argv[0]'s basename. So the loader opens
/// the UNRESOLVED spelling, the guest path as invoked mapped to the host,
/// where it names something openable. Falls back to the resolved file
/// where it does not: a name that only resolved through the search path
/// is still the file the guest kernel would run, and a link whose target
/// is absolute (`/bin/sh` pointing at `/bin/busybox` on alpine) dangles
/// on the host side, so the resolved file is what opens. The invoked
/// name a multi-call binary dispatches on is carried separately by
/// [`loader_argv_for`].
pub fn invocation_path(
    rootfs: &str,
    argv0: &str,
    path_dirs: &[String],
    resolved_host: &str,
) -> String {
    let base = rootfs.trim_end_matches('/');
    if argv0.contains('/') {
        let host = format!("{base}/{}", argv0.trim_start_matches('/'));
        if std::path::Path::new(&host).exists() {
            return host;
        }
    } else {
        for d in path_dirs {
            let host = format!("{base}/{}/{argv0}", d.trim_matches('/'));
            if std::path::Path::new(&host).exists() {
                return host;
            }
        }
    }
    resolved_host.to_string()
}

/// The basename of a host path: the final `/`-separated element, or the
/// whole string where it carries none.
fn basename_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The argv the loader execs for a dynamic payload.
///
/// Usually the loader, the payload by host path, then the payload's own
/// arguments. Where the payload resolved to a busybox multi-call binary
/// through another name (`sh` through `/bin/sh`, whose absolute link the
/// host cannot open, so the resolved file is what opens), the invoked
/// name is passed as the first payload argument: busybox dispatches on
/// argv[0]'s basename, which the resolved file renamed to `busybox`, so
/// the applet name rides beside it exactly as `busybox sh ...` spells
/// it on a command line. Under a chroot the kernel would exec the link
/// with the caller's argv and busybox would select the same applet, so
/// this reproduces that selection without it.
///
/// Returns the argv and whether the applet name was inserted: the banner
/// names the insert, because the payload's argv[0] is not the path that
/// was opened.
pub fn loader_argv_for(
    loader_host: &str,
    payload_host: &str,
    argv0: &str,
    rest: &[String],
) -> (Vec<String>, bool) {
    let invoked = basename_of(argv0);
    let resolved = basename_of(payload_host);
    if resolved == "busybox" && !invoked.is_empty() && invoked != "busybox" {
        let mut argv = Vec::with_capacity(rest.len() + 3);
        argv.push(loader_host.to_string());
        argv.push(payload_host.to_string());
        argv.push(invoked.to_string());
        argv.extend(rest.iter().cloned());
        return (argv, true);
    }
    let mut argv = Vec::with_capacity(rest.len() + 2);
    argv.push(loader_host.to_string());
    argv.push(payload_host.to_string());
    argv.extend(rest.iter().cloned());
    (argv, false)
}

/// Resolve the loader invocation for a dynamic payload.
///
/// `interp_guest` is the `PT_INTERP` path as the image sees it,
/// `payload_host` the payload by host path. The loader resolves inside the
/// rootfs first (TODO/enter.md T-1408): Debian's `/lib64/ld-linux-*.so.2`
/// is an absolute symlink whose target exists only in the image, so the
/// naive host join dangles and `is_executable` on it refuses a payload the
/// guest kernel runs. The resolved file is what must exist and be
/// executable by host path, and at least one library directory must exist,
/// or the refusal names the guest path rather than failing inside the
/// loader later.
pub fn loader_plan(
    rootfs: &str,
    interp_guest: &str,
    payload_host: &str,
) -> Result<LoaderPlan, String> {
    // ⭐ T-1408. The rootfs is the resolution root for every guest path on
    // this rung; the host root is never a fallback. Where the walk fails
    // the naive join is what the refusal names beside the guest path, so a
    // missing loader still reads as missing rather than as unresolvable.
    let loader_host = match crate::abi::resolve_in(std::path::Path::new(rootfs), interp_guest) {
        Ok(p) => p.display().to_string(),
        Err(_) => format!("{}{interp_guest}", rootfs.trim_end_matches('/')),
    };
    if !memfd::is_executable(std::path::Path::new(&loader_host)) {
        return Err(format!(
            "{interp_guest} names the image's loader, but {loader_host} is not \
             an executable file on the host side"
        ));
    }
    let dirs = lib_dirs(rootfs, interp_guest);
    if dirs.is_empty() {
        return Err(format!(
            "the image keeps no library directory beside {interp_guest}, so its \
             loader would fail on the first library it cannot find"
        ));
    }
    Ok(LoaderPlan {
        loader_host,
        payload_host: payload_host.to_string(),
        lib_dirs: dirs,
    })
}

/// Rewrite the environment for a no-chroot entry.
///
/// * `LD_PRELOAD`'s guest-absolute object becomes the host path, so the
///   loader finds what `place` wrote. Other preloaded objects ride
///   unchanged.
/// * `LD_LIBRARY_PATH` becomes the image's library directories, so the
///   loader resolves the payload's `DT_NEEDED` from the image and not
///   the host. Later wins, like every other `*_for` here: the caller's
///   own value, if any, is dropped in favour of the image's.
///
/// `guest_preload` is the path the tier was placed under as the payload
/// sees it (`LD_PRELOAD`'s first element where the tier loaded). Where
/// the tier declined and nothing carries it, only the library path is
/// set: a dynamic payload without the tier still needs its libraries.
///
/// Returns the environment and the caller preload that was dropped, if
/// any: an `LD_PRELOAD` naming guest paths cannot resolve without a
/// chroot, so it is dropped rather than left to fail the load, and the
/// caller names the drop on the banner.
pub fn host_env(
    rootfs: &str,
    guest_preload: &str,
    env: &[String],
    lib_dirs: &[String],
) -> (Vec<String>, Option<String>) {
    let base = rootfs.trim_end_matches('/');
    let mut out = Vec::with_capacity(env.len() + 1);
    let mut dropped: Option<String> = None;
    for e in env {
        let Some((k, v)) = e.split_once('=') else {
            continue;
        };
        if k == "LD_PRELOAD" {
            if !v.split(' ').any(|p| p == guest_preload) {
                dropped = Some(v.to_string());
                continue;
            }
            let mut joined = String::new();
            for p in v.split(' ') {
                if !joined.is_empty() {
                    joined.push(' ');
                }
                if p == guest_preload {
                    joined.push_str(base);
                }
                joined.push_str(p);
            }
            out.push(format!("LD_PRELOAD={joined}"));
        } else if k == "LD_LIBRARY_PATH" {
            continue;
        } else {
            out.push(e.clone());
        }
    }
    out.push(format!("LD_LIBRARY_PATH={}", lib_dirs.join(":")));
    (out, dropped)
}

/// The variable carrying the root-anchoring table, TODO/enter.md T-1407.
///
/// Duplicate by value of the interposer's `map::MAPS_VAR`: this crate takes
/// no dependency on that object, and the spelling is the contract between
/// the two.
pub const MAPS_VAR: &str = "PODBOX_MAPS";

/// How long one side of a table pair may be, mirroring the interposer's
/// `map::MAX_SIDE`: a longer side is malformed and its pair is skipped
/// there, which on this rung would silently stop rewriting and leave host
/// paths bare. The builder below refuses instead.
const MAX_MAP_SIDE: usize = 4096;

/// Build the root-anchoring `PODBOX_MAPS` table for a userland run.
///
/// TODO/enter.md T-1407. Guest `/` maps to the rootfs host path, so an
/// absolute guest path resolves inside the image instead of landing on the
/// host tree. The never-rewrite paths are NOT in the table: `/.podbox`
/// (the guard), `/proc` (the builtin) and `PODBOX_EXCLUDE_PATH` (the user
/// list) are enforced interposer-side (`map.rs` longest/longest_to), and a
/// caller table that tried to move them would be skipped there anyway. No
/// caller edits the built table by hand.
///
/// A rootfs the table cannot spell refuses rather than rewriting nothing:
/// relative, NUL-carrying, `:`/`,`-carrying, empty-after-trim, or overlong.
/// An empty table rewrites nothing, which on this rung is a silent host
/// write, so a malformed root is a refusal, never a table.
///
/// SEAM for the CLI agent (run prepare, `--unsafe-host-paths` flag, parity
/// row): on a userland run set `PODBOX_MAPS` to the returned string before
/// the payload starts (scrubbing any caller-supplied value first, the way
/// `PODBOX_GUEST_EXE` is scrubbed), unless `--unsafe-host-paths` opts into
/// the pass-through; name the table or the opt-in on the banner, because
/// the substitution must be banner-visible and never silent. Returns the
/// table string; `MAPS_VAR` is the variable that carries it.
pub fn userland_maps_table(rootfs: &str) -> Result<String, String> {
    if rootfs.contains('\0') {
        return Err("the rootfs path contains a NUL byte".to_string());
    }
    let base = rootfs.trim_end_matches('/');
    if base.is_empty() {
        return Err("the rootfs path is empty".to_string());
    }
    if !base.starts_with('/') {
        return Err(format!("{rootfs:?} is not an absolute rootfs path"));
    }
    if base.contains(':') || base.contains(',') {
        return Err(format!(
            "{rootfs:?} cannot be spelled in a {MAPS_VAR} table: `:` and `,` \
             separate pairs, so a table carrying it would parse as the wrong \
             mapping (TODO/enter.md T-1407)"
        ));
    }
    if base.len() > MAX_MAP_SIDE {
        return Err(format!(
            "the rootfs path is {} bytes, over the {MAX_MAP_SIDE}-byte table \
             side the interposer reads (TODO/enter.md T-1407)",
            base.len()
        ));
    }
    Ok(format!("/:{base}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("podbox-userland-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("lib")).unwrap();
        std::fs::create_dir_all(d.join("usr/lib")).unwrap();
        d
    }

    /// The loader's directory leads, the standard ones follow, and only
    /// directories that exist are named.
    #[test]
    fn lib_dirs_lead_with_the_loader_and_skip_the_absent() {
        let d = tree("libs");
        let root = d.to_string_lossy().to_string();
        let dirs = lib_dirs(&root, "/lib64/ld-linux-x86-64.so.2");
        assert_eq!(
            dirs,
            vec![format!("{root}/lib"), format!("{root}/usr/lib"),],
            "{dirs:?}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A loader that is not executable by host path refuses naming it,
    /// rather than failing inside the loader later.
    #[test]
    fn a_missing_loader_is_a_sentence() {
        let d = tree("noloader");
        let root = d.to_string_lossy().to_string();
        let e =
            loader_plan(&root, "/lib/ld-musl-x86_64.so.1", &format!("{root}/bin/sh")).unwrap_err();
        assert!(e.contains("not an executable file"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// An executable loader with library directories resolves whole.
    #[test]
    fn an_executable_loader_resolves_whole() {
        let d = tree("loader");
        std::fs::write(d.join("lib/ld.so"), b"\x7fELF").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(d.join("lib/ld.so"))
                .unwrap()
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(d.join("lib/ld.so"), perms).unwrap();
        }
        let root = d.to_string_lossy().to_string();
        let plan = loader_plan(&root, "/lib/ld.so", &format!("{root}/bin/sh")).unwrap();
        assert_eq!(plan.loader_host, format!("{root}/lib/ld.so"));
        assert!(plan.lib_dirs.contains(&format!("{root}/lib")), "{plan:?}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// TODO/enter.md T-1408: Debian's `/lib64/ld-linux-*.so.2` is an
    /// absolute symlink whose target exists only in the image. The naive
    /// host join dangles there, so the loader must resolve inside the
    /// rootfs first: the resolved file is what opens, and the refusal where
    /// it still fails names the guest path.
    #[test]
    #[cfg(unix)]
    fn an_absolute_link_loader_resolves_inside_the_rootfs() {
        let d = tree("absloader");
        std::fs::write(d.join("lib/real-ld.so"), b"\x7fELF").unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(d.join("lib/real-ld.so"))
                .unwrap()
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(d.join("lib/real-ld.so"), perms).unwrap();
        }
        std::fs::create_dir_all(d.join("lib64")).unwrap();
        // Absolute as the guest kernel reads it: under the root it names the
        // real loader, on the host it dangles.
        std::os::unix::fs::symlink("/lib/real-ld.so", d.join("lib64/ld-linux-x86-64.so.2"))
            .unwrap();
        let root = d.to_string_lossy().to_string();
        let plan = loader_plan(
            &root,
            "/lib64/ld-linux-x86-64.so.2",
            &format!("{root}/bin/sh"),
        )
        .unwrap();
        assert_eq!(
            plan.loader_host,
            format!("{root}/lib/real-ld.so"),
            "{plan:?}"
        );
        // And the refusal where nothing resolves still names the guest path.
        let e = loader_plan(&root, "/lib64/absent-ld.so.2", &format!("{root}/bin/sh")).unwrap_err();
        assert!(e.contains("/lib64/absent-ld.so.2"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// The guest preload path becomes the host path; the library path is
    /// set; a caller value loses to the image's.
    #[test]
    fn host_env_rewrites_preload_and_sets_lib_path() {
        let env = [
            "LD_PRELOAD=/.podbox/interpose.so /mine.so".to_string(),
            "LD_LIBRARY_PATH=/caller".to_string(),
            "PATH=/bin".to_string(),
        ];
        let (got, dropped) = host_env(
            "/store/rootfs",
            "/.podbox/interpose.so",
            &env,
            &["/store/rootfs/lib".to_string()],
        );
        assert!(
            got.contains(&"LD_PRELOAD=/store/rootfs/.podbox/interpose.so /mine.so".to_string()),
            "{got:?}"
        );
        assert!(
            got.contains(&"LD_LIBRARY_PATH=/store/rootfs/lib".to_string()),
            "{got:?}"
        );
        assert!(
            !got.iter().any(|e| e == "LD_LIBRARY_PATH=/caller"),
            "{got:?}"
        );
        assert!(got.contains(&"PATH=/bin".to_string()), "{got:?}");
        assert_eq!(dropped, None, "{dropped:?}");
    }

    /// Where the tier declined, no preload is rewritten and the library
    /// path is still set.
    #[test]
    fn host_env_without_a_tier_sets_only_the_library_path() {
        let env = ["PATH=/bin".to_string()];
        let (got, dropped) = host_env(
            "/store/rootfs",
            "/.podbox/interpose.so",
            &env,
            &["/store/rootfs/lib".to_string()],
        );
        assert!(!got.iter().any(|e| e.starts_with("LD_PRELOAD=")), "{got:?}");
        assert!(
            got.contains(&"LD_LIBRARY_PATH=/store/rootfs/lib".to_string()),
            "{got:?}"
        );
        assert_eq!(dropped, None, "{dropped:?}");
    }

    /// A caller-only preload is dropped and reported, because guest paths
    /// cannot resolve without a chroot.
    #[test]
    fn host_env_drops_a_caller_only_preload_and_reports_it() {
        let env = ["LD_PRELOAD=/mine.so".to_string()];
        let (got, dropped) = host_env(
            "/store/rootfs",
            "/.podbox/interpose.so",
            &env,
            &["/store/rootfs/lib".to_string()],
        );
        assert!(!got.iter().any(|e| e.starts_with("LD_PRELOAD=")), "{got:?}");
        assert_eq!(dropped, Some("/mine.so".to_string()), "{dropped:?}");
    }

    /// The invocation keeps the invoked name: `sh` through the search path
    /// stays `sh`, not the resolved `busybox` file, so applet dispatch
    /// survives the loader invocation.
    #[test]
    fn invocation_keeps_the_invoked_name() {
        let d = tree("invocation");
        let root = d.to_string_lossy().to_string();
        std::fs::create_dir_all(d.join("bin")).unwrap();
        std::fs::write(d.join("bin/busybox"), b"\x7fELF").unwrap();
        std::fs::hard_link(d.join("bin/busybox"), d.join("bin/sh")).unwrap_or_else(|_| {
            std::fs::write(d.join("bin/sh"), b"\x7fELF").unwrap();
        });
        let resolved = format!("{root}/bin/busybox");
        let got = invocation_path(&root, "sh", &["/bin".to_string()], &resolved);
        assert_eq!(got, format!("{root}/bin/sh"), "{got}");
        let slash = invocation_path(&root, "/bin/sh", &[], &resolved);
        assert_eq!(slash, format!("{root}/bin/sh"), "{slash}");
        let missing = invocation_path(&root, "absent-tool", &["/bin".to_string()], &resolved);
        assert_eq!(missing, resolved, "{missing}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// An absolute link to nowhere on the host dangles there (the alpine
    /// shape the lane measured: `/bin/sh` pointing at the absolute
    /// `/bin/busybox`, absent from the host root): the resolved file is
    /// what opens, and the invoked name is carried by
    /// [`loader_argv_for`], not by the path.
    #[test]
    #[cfg(unix)]
    fn invocation_falls_back_where_an_absolute_link_dangles() {
        let d = tree("abslink");
        let root = d.to_string_lossy().to_string();
        std::fs::create_dir_all(d.join("bin")).unwrap();
        std::fs::write(d.join("bin/busybox"), b"\x7fELF").unwrap();
        std::os::unix::fs::symlink("/nonexistent-podbox-target/busybox", d.join("bin/sh")).unwrap();
        let resolved = format!("{root}/bin/busybox");
        let got = invocation_path(&root, "sh", &["/bin".to_string()], &resolved);
        assert_eq!(got, resolved, "{got}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// `sh` through a busybox file runs as `busybox sh`: the applet name
    /// rides beside the opened path.
    #[test]
    fn loader_argv_inserts_the_applet_name_for_busybox() {
        let rest = ["-c".to_string(), "echo hi".to_string()];
        let (argv, inserted) = loader_argv_for("/root/lib/ld.so", "/root/bin/busybox", "sh", &rest);
        assert!(inserted);
        assert_eq!(
            argv,
            vec![
                "/root/lib/ld.so",
                "/root/bin/busybox",
                "sh",
                "-c",
                "echo hi",
            ],
            "{argv:?}"
        );
    }

    /// An absolute invoked spelling inserts the same way: the link the
    /// kernel would exec is what selects the applet.
    #[test]
    fn loader_argv_inserts_the_applet_name_for_an_absolute_spelling() {
        let (argv, inserted) =
            loader_argv_for("/root/lib/ld.so", "/root/bin/busybox", "/bin/sh", &[]);
        assert!(inserted);
        assert_eq!(
            argv,
            vec!["/root/lib/ld.so", "/root/bin/busybox", "sh"],
            "{argv:?}"
        );
    }

    /// `busybox` named as itself passes through: the caller's own
    /// arguments already select the applet.
    #[test]
    fn loader_argv_leaves_explicit_busybox_alone() {
        let rest = ["sh".to_string()];
        let (argv, inserted) =
            loader_argv_for("/root/lib/ld.so", "/root/bin/busybox", "busybox", &rest);
        assert!(!inserted);
        assert_eq!(
            argv,
            vec!["/root/lib/ld.so", "/root/bin/busybox", "sh"],
            "{argv:?}"
        );
    }

    /// An ordinary payload passes through untouched.
    #[test]
    fn loader_argv_leaves_ordinary_payloads_alone() {
        let rest = ["-c".to_string()];
        let (argv, inserted) = loader_argv_for("/root/lib/ld.so", "/root/bin/sh", "sh", &rest);
        assert!(!inserted);
        assert_eq!(
            argv,
            vec!["/root/lib/ld.so", "/root/bin/sh", "-c"],
            "{argv:?}"
        );
    }

    /// TODO/enter.md T-1407: guest `/` anchors on the rootfs host path, and
    /// the variable that carries it is `PODBOX_MAPS`.
    #[test]
    fn the_userland_table_anchors_root_on_the_rootfs() {
        assert_eq!(MAPS_VAR, "PODBOX_MAPS");
        assert_eq!(
            userland_maps_table("/store/rootfs").as_deref(),
            Ok("/:/store/rootfs")
        );
        // A trailing slash trims rather than doubling the separator.
        assert_eq!(
            userland_maps_table("/store/rootfs/").as_deref(),
            Ok("/:/store/rootfs")
        );
    }

    /// TODO/enter.md T-1407: a rootfs the table cannot spell refuses rather
    /// than rewriting nothing, which on this rung would be a silent host
    /// write. The exclusions themselves live interposer-side and never in
    /// the built value: no `/.podbox`, `/proc` or exclude list rides here.
    #[test]
    fn an_unspellable_rootfs_is_a_refusal_not_an_empty_table() {
        for bad in ["", "/", "relative/root", "/tmp/a:b", "/tmp/a,b", "/tmp/\0x"] {
            assert!(userland_maps_table(bad).is_err(), "{bad:?} was accepted");
        }
        let long = format!("/{}", "a".repeat(MAX_MAP_SIDE));
        assert!(
            userland_maps_table(&long).is_err(),
            "an overlong root was accepted"
        );
        let table = userland_maps_table("/store/rootfs").unwrap();
        assert!(!table.contains(".podbox"), "{table}");
        assert!(!table.contains("/proc"), "{table}");
    }
}
