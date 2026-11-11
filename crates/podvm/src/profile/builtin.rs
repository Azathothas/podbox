//! The built-in profiles. TODO/cli.md T-1654.
//!
//! Each is TOML, checked by the same code as a user's file. A user file of
//! the same name under the profile directory takes the place of one.

/// The names, in the order of `profile list`.
pub const NAMES: &[&str] = &["default", "workspace", "readonly", "machine", "incus"];

/// The text of a built-in profile.
pub fn text(name: &str) -> Option<&'static str> {
    Some(match name {
        "default" => {
            r#"description = "the ladder selects the rung; no table changes a flag"
[fs]
backend = "auto"
"#
        }
        "workspace" => {
            r#"description = "the current directory at /workspace, the working directory there"
extends = "default"
[fs]
workspace = "."
workdir = "/workspace"
"#
        }
        "readonly" => {
            r#"description = "workspace, with a read-only root and an empty /tmp"
extends = "workspace"
[fs]
read_only = true
tmpfs = ["/tmp"]
"#
        }
        "machine" => {
            r#"description = "workspace, in a guest of the machine tier with 1 GiB and 2 vCPUs"
extends = "workspace"
[fs]
backend = "machine"
[limits]
memory = "1g"
cpus = 2
"#
        }
        "incus" => {
            r#"description = "a disposable instance of an incusd (TODO/machine.md T-1718)"
extends = "default"
[fs]
backend = "incus"
"#
        }
        _ => return None,
    })
}
