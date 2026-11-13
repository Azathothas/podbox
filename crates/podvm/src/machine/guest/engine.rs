//! `--podvm-engine`: the modules of a container engine in a guest of the
//! machine tier, TODO/machine.md T-1780 (PodUtils/podrunner#92).
//!
//! The pinned kernel (Alpine `linux-virt` 6.12) has the bridge, `veth`,
//! netfilter and overlayfs as modules of its modloop, which podvm pins by
//! its SHA-256. With the flag, podvm takes [`TOP`] and each module that
//! `modules.dep` of the modloop names for them, in the order of their
//! symbols, and the guest loads them before the command
//! (`podvm guest-net --load`). Each module out of the pinned modloop is
//! pinned with it. The list of the modules and the files stay in the
//! store, beside the kernel, after the first use.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The modules that a container engine needs: a network of bridges and
/// `veth` pairs, the netfilter of `iptables-nft` and of `iptables-legacy`
/// for its DNS and its port mapping, and the storage driver `overlay2`.
///
/// `crc32c_generic` comes first: `libcrc32c`, which the netfilter modules
/// need, has a soft dependency on the algorithm `crc32c` that
/// `modules.dep` does not name (`modules.softdep`), and its load fails with
/// ENOENT before it. The generic form needs no SSE4.2, which a CPU of TCG
/// may not have.
pub const TOP: &[&str] = &[
    "crc32c_generic",
    "bridge",
    "br_netfilter",
    "veth",
    "overlay",
    "nf_tables",
    "nft_chain_nat",
    "nft_masq",
    "nft_compat",
    "nft_reject",
    "nf_reject_ipv4",
    "xt_MASQUERADE",
    "xt_addrtype",
    "xt_conntrack",
    "xt_nat",
    "xt_tcpudp",
    "xt_comment",
    "xt_mark",
    "ip_tables",
    "iptable_nat",
    "iptable_filter",
    "ip6_tables",
    "ip6table_nat",
    "ip6table_filter",
];

/// The name of a module of a path of `modules.dep`.
fn name_of(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.trim_end_matches(".ko").replace('-', "_")
}

/// The modules of `top` and of their dependencies, in an order where each
/// module comes after the modules that it needs: `(name, path)`, the path
/// under `modules/KVER/`. A name that `modules.dep` does not hold is an
/// error: the kernel would have it built in, or not at all.
pub fn closure(dep: &str, top: &[&str]) -> Result<Vec<(String, String)>, String> {
    let mut map: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
    for line in dep.lines() {
        let Some((path, deps)) = line.split_once(':') else {
            continue;
        };
        let deps = deps.split_whitespace().map(name_of).collect();
        map.insert(name_of(path), (path.to_string(), deps));
    }
    let mut out: Vec<(String, String)> = Vec::new();
    fn visit(
        n: &str,
        map: &BTreeMap<String, (String, Vec<String>)>,
        out: &mut Vec<(String, String)>,
        depth: usize,
    ) -> Result<(), String> {
        if out.iter().any(|(m, _)| m == n) {
            return Ok(());
        }
        if depth > 64 {
            return Err(format!("modules.dep: the dependencies of {n} do not end"));
        }
        let (path, deps) = map
            .get(n)
            .ok_or(format!("modules.dep names no module {n}"))?;
        for d in deps {
            visit(d, map, out, depth + 1)?;
        }
        if !out.iter().any(|(m, _)| m == n) {
            out.push((n.to_string(), path.clone()));
        }
        Ok(())
    }
    for t in top {
        visit(t, &map, &mut out, 0)?;
    }
    Ok(out)
}

/// The list file and the module files of the engine in `dir`.
fn list_path(dir: &Path) -> PathBuf {
    dir.join(format!(
        "engine-{}.list",
        &super::kernel::modloop_sha256()[..16]
    ))
}

fn module_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!(
        "{name}-{}.ko",
        &super::kernel::modloop_sha256()[..16]
    ))
}

/// The modules of the engine in `store_root/machine/`, in their order,
/// taken out of the pinned modloop where they are not there yet.
pub fn ensure(store_root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    if std::env::var_os("PODVM_MACHINE_KERNEL").is_some() {
        return Err(
            "--podvm-engine needs the modules of the pinned kernel, and PODVM_MACHINE_KERNEL \
             names another kernel (TODO/machine.md T-1780)"
                .to_string(),
        );
    }
    let dir = store_root.join("machine");
    let listed = |dir: &Path| -> Option<Vec<(String, PathBuf)>> {
        let text = std::fs::read_to_string(list_path(dir)).ok()?;
        let v: Vec<_> = text
            .lines()
            .filter(|l| !l.is_empty())
            .map(|n| (n.to_string(), module_path(dir, n)))
            .collect();
        v.iter().all(|(_, p)| p.is_file()).then_some(v)
    };
    if let Some(v) = listed(&dir) {
        return Ok(v);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let modloop = super::kernel::fetch_modloop(&dir)?;
    let img = super::squashfs::Image::new(&modloop).map_err(|e| format!("the modloop: {e}"))?;
    let kver = super::kernel::modloop_kver(&img)?;
    let dep = img
        .read(&format!("modules/{kver}/modules.dep"))
        .map_err(|e| format!("the modloop: {e}"))?;
    let mods = closure(&String::from_utf8_lossy(&dep), TOP)?;
    for (n, path) in &mods {
        let bytes = img
            .read(&format!("modules/{kver}/{path}"))
            .map_err(|e| format!("the modloop: {e}"))?;
        let p = module_path(&dir, n);
        let tmp = p.with_extension("partial");
        std::fs::write(&tmp, &bytes)
            .and_then(|()| std::fs::rename(&tmp, &p))
            .map_err(|e| format!("{}: {e}", p.display()))?;
    }
    let names: Vec<&str> = mods.iter().map(|(n, _)| n.as_str()).collect();
    std::fs::write(list_path(&dir), names.join("\n") + "\n")
        .map_err(|e| format!("{}: {e}", list_path(&dir).display()))?;
    listed(&dir).ok_or_else(|| format!("{}: the modules did not stay", dir.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_closure_puts_each_dependency_first_once() {
        let dep = "kernel/net/bridge/bridge.ko: kernel/net/802/stp.ko kernel/net/llc/llc.ko\n\
                   kernel/net/802/stp.ko: kernel/net/llc/llc.ko\n\
                   kernel/net/llc/llc.ko:\n\
                   kernel/net/bridge/br_netfilter.ko: kernel/net/bridge/bridge.ko kernel/net/802/stp.ko kernel/net/llc/llc.ko\n\
                   kernel/drivers/net/veth.ko:\n";
        let got = closure(dep, &["br_netfilter", "veth", "bridge"]).unwrap();
        let names: Vec<_> = got.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["llc", "stp", "bridge", "br_netfilter", "veth"]);
        assert_eq!(got[2].1, "kernel/net/bridge/bridge.ko");
        assert!(closure(dep, &["overlay"]).unwrap_err().contains("overlay"));
    }
}
