//! The run archive of a guest: `/podvm-init`, the console nodes, the
//! copies of `-v` where no 9p module serves them, and podvm itself with the
//! modules that the guest loads (TODO/machine.md T-1656, T-1723).

use std::path::PathBuf;

use super::cpio;

/// podvm's own bytes and the modules of the network, for a guest
/// ([`net`]).
pub struct Tools {
    pub podvm: Vec<u8>,
    pub modules: Vec<(String, Vec<u8>)>,
    /// The modules of `--podvm-engine`, in their order (T-1780): the init
    /// loads the names of [`ENGINE_LIST`].
    pub engine: Vec<String>,
}

/// The names of the modules of `--podvm-engine`, one on a line.
pub const ENGINE_LIST: &str = "/podvm/engine.list";

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
        if !t.engine.is_empty() {
            let list = t.engine.join("\n") + "\n";
            w.entry(
                ENGINE_LIST.trim_start_matches('/'),
                cpio::REG | 0o644,
                list.as_bytes(),
                (0, 0),
            )?;
        }
    }
    w.entry("podvm-init", cpio::REG | 0o755, script.as_bytes(), (0, 0))?;
    w.finish()
}

/// podvm's own bytes and the module files, for the run archive.
pub(super) fn tools(
    modules: &[(String, PathBuf)],
    engine: Vec<String>,
) -> Result<Option<Tools>, String> {
    if modules.is_empty() {
        return Ok(None);
    }
    let podvm = std::fs::read("/proc/self/exe").map_err(|e| format!("/proc/self/exe: {e}"))?;
    let mut m = Vec::new();
    for (n, p) in modules {
        m.push((
            n.clone(),
            std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?,
        ));
    }
    Ok(Some(Tools {
        podvm,
        modules: m,
        engine,
    }))
}
