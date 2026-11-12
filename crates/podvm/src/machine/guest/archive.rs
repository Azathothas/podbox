//! The run archive of a guest: `/podvm-init`, the console nodes, the
//! copies of `-v` where no 9p module serves them, and podvm itself with the
//! modules that the guest loads (TODO/machine.md T-1656, T-1723).

use std::path::PathBuf;

use super::cpio;

/// podvm's own bytes and the modules of the network, for a guest
/// ([`net`]).
pub struct Tools {
    pub podvm: Vec<u8>,
    pub modules: Vec<(&'static str, Vec<u8>)>,
}

/// The archive of one run: `/podvm-init`, the console nodes, and, where
/// `tools` gives them, podvm itself and the modules of the network.
pub fn run_archive(
    script: &str,
    tools: Option<&Tools>,
    copies: &[(PathBuf, String)],
    stdin: Option<&[u8]>,
) -> std::io::Result<Vec<u8>> {
    let mut w = cpio::Newc::new(Vec::new());
    for (host, guest) in copies {
        w.tree_at(host, guest)?;
    }
    if let Some(bytes) = stdin {
        w.entry("podvm", cpio::DIR | 0o755, &[], (0, 0))?;
        w.entry("podvm/stdin", cpio::REG | 0o600, bytes, (0, 0))?;
    }
    w.entry("dev", cpio::DIR | 0o755, &[], (0, 0))?;
    w.entry("dev/console", cpio::CHR | 0o600, &[], (5, 1))?;
    w.entry("dev/ttyS0", cpio::CHR | 0o660, &[], (4, 64))?;
    if let Some(t) = tools {
        w.entry("podvm", cpio::DIR | 0o755, &[], (0, 0))?;
        w.entry("podvm/podvm", cpio::REG | 0o755, &t.podvm, (0, 0))?;
        w.entry("podvm/modules", cpio::DIR | 0o755, &[], (0, 0))?;
        for (n, bytes) in &t.modules {
            w.entry(
                &format!("podvm/modules/{n}.ko"),
                cpio::REG | 0o644,
                bytes,
                (0, 0),
            )?;
        }
    }
    w.entry("podvm-init", cpio::REG | 0o755, script.as_bytes(), (0, 0))?;
    w.finish()
}

/// podvm's own bytes and the module files, for the run archive.
pub(super) fn tools(modules: &[(&'static str, PathBuf)]) -> Result<Option<Tools>, String> {
    if modules.is_empty() {
        return Ok(None);
    }
    let podvm = std::fs::read("/proc/self/exe").map_err(|e| format!("/proc/self/exe: {e}"))?;
    let mut m = Vec::new();
    for (n, p) in modules {
        m.push((
            *n,
            std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?,
        ));
    }
    Ok(Some(Tools { podvm, modules: m }))
}
