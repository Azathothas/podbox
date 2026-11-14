//! The confine layer of `run --podvm-confine=SPEC` (TODO/enter.md T-1658,
//! issue 170): Landlock, the seccomp deny filter of `pod-confine` and
//! `no_new_privs`, applied by `pod-enter` in the payload's child as the
//! last step before the exec (`Plan.confine`).
//!
//! SPEC is `read=P,P;write=P;exec=P;connect=PORT,PORT`, each part
//! optional, each path a guest path. `read` gives read access, `write`
//! read and write, `exec` read and execute. `connect` holds a TCP connect
//! to the listed ports (Landlock ABI 4); with no `connect`, TCP is free.
//! Each path that no rule names is denied. A path written `?/P` is
//! optional: it gets no rule where the image lacks it (a profile for many
//! images, as `errand`); a path without `?` that the image lacks is a
//! refusal by name.
//!
//! The spec adds the paths that a program needs to start: read and
//! execute of the system directories of the image that exist (`/bin`,
//! `/sbin`, `/usr`, `/lib`, `/lib64`, `/lib32`, and podvm's own `/.podvm`), read of `/proc`, and
//! the device nodes `/dev/null` (read and write), `/dev/zero`,
//! `/dev/random` and `/dev/urandom` (read), and the files of `/etc` that
//! the dynamic loader reads before `main` (`/etc/ld.so.cache` of glibc,
//! `/etc/ld-musl-ARCH.path` of musl; TODO/enter.md T-1764). Each other file
//! of `/etc` is not among them: a profile names each file that it reads.

pub use pod_confine::{Access, Spec};

/// The system directories that a program of the image runs from.
/// `/.podvm` is podvm's own directory of the rootfs (the probe of
/// `probe-inside`).
pub const SYSTEM_EXEC: &[&str] = &[
    "/bin", "/sbin", "/usr", "/lib", "/lib64", "/lib32", "/.podvm",
];
/// The paths that a program reads at its start.
/// Without `/etc/ld.so.cache`, glibc's loader searches only `/lib` and
/// `/usr/lib`, so python of `/usr/local/lib` did not start (T-1764).
pub const SYSTEM_READ: &[&str] = &[
    "/proc",
    "/dev/zero",
    "/dev/random",
    "/dev/urandom",
    "/etc/ld.so.cache",
    "/etc/ld-musl-x86_64.path",
    "/etc/ld-musl-aarch64.path",
];
/// The paths that a program writes at its start.
pub const SYSTEM_WRITE: &[&str] = &["/dev/null"];

/// What `--podvm-confine` asks for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ask {
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub exec: Vec<String>,
    pub connect: Option<Vec<u16>>,
}

/// Parse SPEC. Err names the part that does not parse.
pub fn parse(spec: &str) -> Result<Ask, String> {
    let mut a = Ask::default();
    for part in spec.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let Some((key, value)) = part.split_once('=') else {
            return Err(format!("{part:?} is not KEY=VALUE"));
        };
        let items: Vec<&str> = value
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .collect();
        match key.trim() {
            "read" | "write" | "exec" => {
                let list = match key.trim() {
                    "read" => &mut a.read,
                    "write" => &mut a.write,
                    _ => &mut a.exec,
                };
                for p in items {
                    if !p.trim_start_matches('?').starts_with('/') {
                        return Err(format!("the path {p:?} of {key} is not absolute"));
                    }
                    list.push(p.to_string());
                }
            }
            "connect" => {
                let mut ports = Vec::new();
                for p in items {
                    match p.parse::<u16>() {
                        Ok(n) if n > 0 => ports.push(n),
                        _ => return Err(format!("the port {p:?} of connect is not 1 to 65535")),
                    }
                }
                a.connect = Some(ports);
            }
            other => {
                return Err(format!("{other:?} is not read, write, exec or connect"));
            }
        }
    }
    Ok(a)
}

/// The SPEC of `ask`, the inverse of [`parse`].
pub fn render(ask: &Ask) -> String {
    let mut parts = Vec::new();
    for (k, v) in [
        ("read", &ask.read),
        ("write", &ask.write),
        ("exec", &ask.exec),
    ] {
        if !v.is_empty() {
            parts.push(format!("{k}={}", v.join(",")));
        }
    }
    if let Some(c) = &ask.connect {
        let ports: Vec<String> = c.iter().map(u16::to_string).collect();
        parts.push(format!("connect={}", ports.join(",")));
    }
    parts.join(";")
}

/// The `pod-confine` spec of `ask`, with guest paths. `exists` tells
/// whether a guest path exists in the image: a system path that the image
/// lacks gets no rule, and a path of the ask always gets one (a missing
/// one is a refusal of `pod-enter` by name).
pub fn spec(ask: &Ask, exists: &dyn Fn(&str) -> bool) -> Spec {
    let mut s = Spec::new();
    let rx = Access::READ | Access::EXECUTE;
    let rw = Access::READ | Access::WRITE;
    for p in SYSTEM_EXEC.iter().filter(|p| exists(p)) {
        s = s.allow(*p, rx);
    }
    for p in SYSTEM_READ.iter().filter(|p| exists(p)) {
        s = s.allow(*p, Access::READ);
    }
    for p in SYSTEM_WRITE.iter().filter(|p| exists(p)) {
        s = s.allow(*p, rw);
    }
    // `?/P` is optional: no rule where the image lacks P.
    let wanted = |p: &String| match p.strip_prefix('?') {
        Some(o) => exists(o).then(|| o.to_string()),
        None => Some(p.clone()),
    };
    for (list, access) in [(&ask.read, Access::READ), (&ask.write, rw), (&ask.exec, rx)] {
        for p in list.iter().filter_map(wanted) {
            s = s.allow(p, access);
        }
    }
    s.connect = ask.connect.clone();
    s
}

/// The Landlock ABI of this kernel, or None where it has none.
pub fn abi() -> Option<u32> {
    pod_confine::probe()
}

/// The lowest ABI of the layer, and the lowest of a TCP rule.
pub const MIN_ABI: u32 = pod_confine::MIN_ABI;
pub const NET_ABI: u32 = pod_confine::NET_ABI;

/// [`spec`] for the image tree at `rootfs` (a host path): a system path
/// gets its rule where the tree has it.
pub fn spec_in(ask: &Ask, rootfs: &std::path::Path) -> Spec {
    let exists =
        |guest: &str| std::fs::symlink_metadata(rootfs.join(guest.trim_start_matches('/'))).is_ok();
    spec(ask, &exists)
}

/// The host paths that a payload of the loader family (no chroot) reads
/// beside its image: the host's `/proc` and device nodes, which it sees
/// as they are.
pub const HOST_READ: &[&str] = &["/proc", "/dev/zero", "/dev/random", "/dev/urandom"];
pub const HOST_WRITE: &[&str] = &["/dev/null"];

/// The host path of the guest path `guest`: under the volume that serves
/// it, else under the rootfs (the table of the interposer, read
/// lexically).
pub fn host_of(maps: &crate::userland::Maps, guest: &str) -> String {
    match maps.volume_at(guest) {
        Some(v) => {
            let rest = guest
                .trim_start_matches('/')
                .split('/')
                .skip(v.guest.split('/').filter(|c| !c.is_empty()).count())
                .collect::<Vec<_>>()
                .join("/");
            if rest.is_empty() {
                v.host.clone()
            } else {
                format!("{}/{rest}", v.host.trim_end_matches('/'))
            }
        }
        None => format!("{}{guest}", maps.rootfs().trim_end_matches('/')),
    }
}

/// How many links one guest path may name before the path is refused: the
/// limit of the kernel's own resolution.
const MAX_LINKS: u32 = 40;

/// The host path of the guest path `guest`, with each link resolved inside
/// the tree that serves it. A link target that starts with `/` resolves
/// from the root of the image (or of the volume), as the interposer
/// resolves it, and `..` stops at that root: the host's own `/bin/sh` is
/// not the `/bin/sh` of the image. `None` where the image lacks the path.
fn host_resolved(maps: &crate::userland::Maps, guest: &str) -> Option<String> {
    let (base, mut todo): (String, std::collections::VecDeque<String>) = match maps.volume_at(guest)
    {
        Some(v) => {
            let parts = v.guest.split('/').filter(|c| !c.is_empty()).count();
            let rest = guest
                .trim_start_matches('/')
                .split('/')
                .skip(parts)
                .map(str::to_string)
                .collect();
            (v.host.trim_end_matches('/').to_string(), rest)
        }
        None => {
            let rest = guest
                .trim_start_matches('/')
                .split('/')
                .map(str::to_string)
                .collect();
            (maps.rootfs().trim_end_matches('/').to_string(), rest)
        }
    };
    let mut done: Vec<String> = Vec::new();
    let mut links = 0u32;
    while let Some(c) = todo.pop_front() {
        match c.as_str() {
            "" | "." => continue,
            ".." => {
                done.pop();
                continue;
            }
            _ => done.push(c),
        }
        let host = format!("{}/{}", base, done.join("/"));
        let md = std::fs::symlink_metadata(&host).ok()?;
        if !md.file_type().is_symlink() {
            continue;
        }
        links += 1;
        if links > MAX_LINKS {
            return None;
        }
        let target = std::fs::read_link(&host).ok()?;
        let text = target.to_string_lossy().into_owned();
        done.pop();
        if text.starts_with('/') {
            done.clear();
        }
        let mut next: std::collections::VecDeque<String> =
            text.split('/').map(str::to_string).collect();
        while let Some(c) = todo.pop_front() {
            next.push_back(c);
        }
        todo = next;
    }
    // The root of the tree where the path named no component: the host
    // side of a volume pair can carry a trailing `/`.
    let host = if done.is_empty() {
        base.trim_end_matches('/').to_string()
    } else {
        format!("{}/{}", base.trim_end_matches('/'), done.join("/"))
    };
    std::fs::symlink_metadata(&host).ok()?;
    Some(host)
}

/// TODO/enter.md T-1887 (PodUtils/podvm#364, the patch of Nemo-010).
///
/// The spec of `ask` with host paths, for the loader family: each guest
/// path through `maps` with its links resolved inside the tree, and the
/// host's `/proc` and device nodes.
///
/// TODO/enter.md T-1888: a required path that does not resolve inside the
/// tree that serves it is refused with the guest path that the caller
/// wrote. A link of the image to a path that only the host has took a
/// rule on the host's inode before.
pub fn host_spec(ask: &Ask, maps: &crate::userland::Maps) -> Result<Spec, String> {
    let exists = |guest: &str| host_resolved(maps, guest).is_some();
    let mut s = spec(ask, &exists);
    for r in &mut s.paths {
        r.path = host_resolved(maps, &r.path).ok_or_else(|| {
            let lexical = host_of(maps, &r.path);
            if std::fs::symlink_metadata(&lexical).is_ok() {
                format!(
                    "the path {:?} of the spec is a link of the image that leaves the tree \
                     that serves it, so the image does not hold its target (TODO/enter.md T-1888)",
                    r.path
                )
            } else {
                format!("the path {:?} of the spec is not in the image", r.path)
            }
        })?;
    }
    for p in HOST_READ
        .iter()
        .filter(|p| std::path::Path::new(p).exists())
    {
        s = s.allow(*p, Access::READ);
    }
    for p in HOST_WRITE
        .iter()
        .filter(|p| std::path::Path::new(p).exists())
    {
        s = s.allow(*p, Access::READ | Access::WRITE);
    }
    Ok(s)
}

/// Whether a rung can carry the layer: a rung with a chroot, where the
/// paths of the spec are guest paths. Elsewhere `run` refuses by name and
/// never runs unconfined (the Decision of T-1658).
pub fn rung_carries(rung: podvm_probe::select::Rung) -> bool {
    matches!(
        rung,
        podvm_probe::select::Rung::Namespace | podvm_probe::select::Rung::Chroot
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spec_parses_and_renders_back() {
        let s = "read=/workspace,/etc/hostname;write=/workspace/out;connect=443";
        let a = parse(s).unwrap();
        assert_eq!(a.read, ["/workspace", "/etc/hostname"]);
        assert_eq!(a.write, ["/workspace/out"]);
        assert_eq!(a.connect, Some(vec![443]));
        assert_eq!(render(&a), s);
        assert_eq!(parse("").unwrap(), Ask::default());
    }

    #[test]
    fn a_bad_part_is_named() {
        assert!(parse("read=etc").unwrap_err().contains("not absolute"));
        assert!(parse("connect=0").unwrap_err().contains("1 to 65535"));
        assert!(parse("listen=80").unwrap_err().contains("listen"));
        assert!(parse("read").unwrap_err().contains("KEY=VALUE"));
    }

    #[test]
    fn the_ruleset_names_the_system_paths_that_exist_and_each_asked_path() {
        let a = parse("read=/workspace;exec=/opt/tool;connect=443").unwrap();
        let s = spec(&a, &|p| p != "/lib32");
        let paths: Vec<&str> = s.paths.iter().map(|r| r.path.as_str()).collect();
        assert!(paths.contains(&"/usr") && paths.contains(&"/proc"));
        assert!(
            !paths.contains(&"/lib32"),
            "a path the image lacks has no rule"
        );
        assert!(
            !paths
                .iter()
                .any(|p| p.starts_with("/etc") && !SYSTEM_READ.contains(p)),
            "/etc is not given, beyond the files of the loader (T-1764)"
        );
        let ws = s.paths.iter().find(|r| r.path == "/workspace").unwrap();
        assert!(ws.access.contains(Access::READ) && !ws.access.contains(Access::WRITE));
        let tool = s.paths.iter().find(|r| r.path == "/opt/tool").unwrap();
        assert!(tool.access.contains(Access::EXECUTE));
        assert_eq!(s.connect, Some(vec![443]));
        assert!(!s.deny.is_empty(), "the default deny list of pod-confine");
    }

    #[test]
    fn the_loader_reads_its_cache_under_each_spec() {
        // TODO/enter.md T-1764: python of /usr/local/lib needs ld.so.cache.
        let ask = parse("read=/usr;write=/tmp").unwrap();
        let s = spec(&ask, &|p| p == "/etc/ld.so.cache" || p == "/usr");
        let has = |p: &str| {
            s.paths
                .iter()
                .any(|r| r.path == p && r.access == Access::READ)
        };
        assert!(has("/etc/ld.so.cache"), "{:?}", s.paths);
        assert!(
            !has("/etc/ld-musl-x86_64.path"),
            "a file that the image lacks gets no rule"
        );
    }

    #[test]
    fn an_optional_path_gets_a_rule_only_where_the_image_has_it() {
        let a = parse("read=?/etc/ssl,?/etc/pki,/etc/passwd").unwrap();
        let s = spec(&a, &|p| p != "/etc/pki");
        let paths: Vec<&str> = s.paths.iter().map(|r| r.path.as_str()).collect();
        assert!(paths.contains(&"/etc/ssl") && paths.contains(&"/etc/passwd"));
        assert!(!paths
            .iter()
            .any(|p| p.contains("pki") || p.starts_with('?')));
        assert!(parse("read=?etc").is_err());
    }

    #[test]
    fn a_guest_path_maps_through_the_volume_or_the_rootfs() {
        let names = crate::NAMES;
        let v = crate::userland::Volume {
            guest: "/workspace".into(),
            host: "/h/proj".into(),
        };
        let maps = crate::userland::Maps::new(&names, "/s/root", &[v]).unwrap();
        assert_eq!(host_of(&maps, "/workspace"), "/h/proj");
        assert_eq!(host_of(&maps, "/workspace/out/a"), "/h/proj/out/a");
        assert_eq!(host_of(&maps, "/etc/passwd"), "/s/root/etc/passwd");
        assert_eq!(host_of(&maps, "/workspacex"), "/s/root/workspacex");
    }

    #[test]
    fn only_a_rung_with_a_chroot_carries_the_layer() {
        use podvm_probe::select::Rung;
        assert!(rung_carries(Rung::Namespace) && rung_carries(Rung::Chroot));
        assert!(!rung_carries(Rung::Userland) && !rung_carries(Rung::Supervise));
    }

    /// A tree of the test: a fresh directory under the temporary directory.
    fn scratch(tag: &str) -> std::path::PathBuf {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("podvm-enter-{}-{n}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_link_of_the_image_that_starts_with_a_slash_resolves_inside_it() {
        let root = scratch("abs-link-in");
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::write(root.join("bin/busybox"), b"x").unwrap();
        std::os::unix::fs::symlink("/bin/busybox", root.join("bin/sh")).unwrap();
        let maps = crate::userland::Maps::of_root(root.to_str().unwrap());
        let s = host_spec(&parse("read=/bin/sh").unwrap(), &maps).unwrap();
        let want = format!("{}/bin/busybox", root.display());
        let got: Vec<&String> = s.paths.iter().map(|r| &r.path).collect();
        assert!(s.paths.iter().any(|r| r.path == want), "{got:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_link_that_names_a_host_path_is_a_path_the_image_lacks() {
        let root = scratch("abs-link-out");
        std::fs::create_dir_all(root.join("etc")).unwrap();
        std::os::unix::fs::symlink("/bin/sh", root.join("etc/host-only")).unwrap();
        let maps = crate::userland::Maps::of_root(root.to_str().unwrap());
        let s = host_spec(&parse("read=?/etc/host-only").unwrap(), &maps).unwrap();
        let got: Vec<&String> = s.paths.iter().map(|r| &r.path).collect();
        assert!(
            !s.paths.iter().any(|r| r.path.contains("host-only")),
            "{got:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_volume_pair_of_one_device_node_maps_to_the_host_node() {
        // The device nodes of the host are volume pairs whose guest is the
        // node itself. A guest path that names the volume root leaves an
        // empty remainder, which used to come out as `<node>/` and was
        // then refused by the kernel with ENOTDIR.
        let root = scratch("vol-root");
        let vol = scratch("vol-host");
        let node = vol.join("zero");
        std::fs::write(&node, b"x").unwrap();
        let v = crate::userland::Volume {
            guest: "/dev/zero".into(),
            host: node.to_str().unwrap().to_string(),
        };
        let maps = crate::userland::Maps::new(&crate::NAMES, root.to_str().unwrap(), &[v]).unwrap();
        let want = node.to_str().unwrap().to_string();
        assert_eq!(host_resolved(&maps, "/dev/zero"), Some(want.clone()));
        // The optional form asks the resolver, not the fallback of a
        // required path, so a rule here means the resolution succeeded.
        let s = host_spec(&parse("read=?/dev/zero").unwrap(), &maps).unwrap();
        let got: Vec<&String> = s.paths.iter().map(|r| &r.path).collect();
        assert!(s.paths.iter().any(|r| r.path == want), "{got:?}");
        assert!(!s.paths.iter().any(|r| r.path.ends_with('/')), "{got:?}");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&vol);
    }

    /// TODO/enter.md T-1888: a required rule on a link to a path that only
    /// the host has refuses, and names the guest path.
    #[test]
    fn confine_required_link_out_of_the_tree_refuses_by_guest_path() {
        let root = scratch("abs-link-req");
        std::fs::create_dir_all(root.join("etc")).unwrap();
        std::os::unix::fs::symlink("/bin/sh", root.join("etc/host-only")).unwrap();
        let maps = crate::userland::Maps::of_root(root.to_str().unwrap());
        let e = host_spec(&parse("read=/etc/host-only").unwrap(), &maps).unwrap_err();
        assert!(e.contains("\"/etc/host-only\""), "{e}");
        assert!(e.contains("T-1888"), "{e}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
