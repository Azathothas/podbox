//! The confine layer of `run --podvm-confine=SPEC` (TODO/enter.md T-1658,
//! issue 170): Landlock, the seccomp deny filter of `pod-confine` and
//! `no_new_privs`, applied by `pod-enter` in the payload's child as the
//! last step before the exec (`Plan.confine`).
//!
//! SPEC is `read=P,P;write=P;exec=P;connect=PORT,PORT`, each part
//! optional, each path a guest path. `read` gives read access, `write`
//! read and write, `exec` read and execute. `connect` holds a TCP connect
//! to the listed ports (Landlock ABI 4); with no `connect`, TCP is free.
//! Each path that no rule names is denied.
//!
//! The spec adds the paths that a program needs to start: read and
//! execute of the system directories of the image that exist (`/bin`,
//! `/sbin`, `/usr`, `/lib`, `/lib64`, `/lib32`, and podvm's own `/.podvm`), read of `/proc`, and
//! the device nodes `/dev/null` (read and write), `/dev/zero`,
//! `/dev/random` and `/dev/urandom` (read). `/etc` is not among them: a
//! profile names each file of `/etc` that it reads.

use pod_confine::Access;
pub use pod_confine::Spec;

/// The system directories that a program of the image runs from.
/// `/.podvm` is podvm's own directory of the rootfs (the probe of
/// `probe-inside`).
pub const SYSTEM_EXEC: &[&str] = &[
    "/bin", "/sbin", "/usr", "/lib", "/lib64", "/lib32", "/.podvm",
];
/// The paths that a program reads at its start.
pub const SYSTEM_READ: &[&str] = &["/proc", "/dev/zero", "/dev/random", "/dev/urandom"];
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
                    if !p.starts_with('/') {
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
    for p in &ask.read {
        s = s.allow(p.clone(), Access::READ);
    }
    for p in &ask.write {
        s = s.allow(p.clone(), rw);
    }
    for p in &ask.exec {
        s = s.allow(p.clone(), rx);
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
            !paths.iter().any(|p| p.starts_with("/etc")),
            "/etc is not given"
        );
        let ws = s.paths.iter().find(|r| r.path == "/workspace").unwrap();
        assert!(ws.access.contains(Access::READ) && !ws.access.contains(Access::WRITE));
        let tool = s.paths.iter().find(|r| r.path == "/opt/tool").unwrap();
        assert!(tool.access.contains(Access::EXECUTE));
        assert_eq!(s.connect, Some(vec![443]));
        assert!(!s.deny.is_empty(), "the default deny list of pod-confine");
    }

    #[test]
    fn only_a_rung_with_a_chroot_carries_the_layer() {
        use podvm_probe::select::Rung;
        assert!(rung_carries(Rung::Namespace) && rung_carries(Rung::Chroot));
        assert!(!rung_carries(Rung::Userland) && !rung_carries(Rung::Supervise));
    }
}
