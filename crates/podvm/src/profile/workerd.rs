//! The config of the profile `workerd-test` (TODO/cli.md T-1661).
//!
//! workerd has no OCI image of its party; its pin is
//! [`super::tools::WORKERD`]. podvm writes this config beside the program,
//! so the run sees both at the tool's guest directory.

/// The config: the worker `/workspace/worker.js`, and no outbound service,
/// so each `fetch` of the worker fails. The date leaves `process`
/// undefined. An embed is relative to the config, so the worker is
/// `../../workspace/worker.js` from `/opt/podvm-workerd`.
pub const CONFIG: &str = r#"using Workerd = import "/workerd/workerd.capnp";
const config :Workerd.Config = (
  services = [
    (name = "main", worker = .worker),
    (name = "none", network = (allow = [])),
  ],
);
const worker :Workerd.Worker = (
  modules = [ (name = "worker.js", esModule = embed "../../workspace/worker.js") ],
  compatibilityDate = "2025-09-15",
  globalOutbound = "none",
);
"#;

/// The command prefix: `workerd test` of the config.
pub fn prefix() -> Vec<String> {
    let t = &super::tools::WORKERD;
    vec![
        t.guest_program(),
        "test".into(),
        format!("{}/config.capnp", t.guest_dir),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_has_no_outbound_service() {
        assert!(CONFIG.contains("globalOutbound = \"none\""));
        assert!(CONFIG.contains("network = (allow = [])"));
        assert!(!CONFIG.contains("public"));
        assert_eq!(prefix()[1], "test");
        assert!(CONFIG.contains(&format!("../..{}/worker.js", "/workspace")));
    }
}
