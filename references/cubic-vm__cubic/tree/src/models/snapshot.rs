use serde::Deserialize;

/// A qcow2 internal snapshot, addressed by name
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Snapshot {
    pub name: String,
}
