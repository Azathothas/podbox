//! `TODO/image.md` T-0209: registry credentials, through
//! `pod_oci::credentials`, at podvm's paths.
//!
//! One read path: every credential podvm sends comes through [`lookup`],
//! which reads `~/.docker/config.json` and
//! `$XDG_RUNTIME_DIR/containers/auth.json` ([`default_paths`]).

use std::path::PathBuf;

pub use pod_oci::credentials::*;

/// The two files podvm reads: docker's, then the containers file, from
/// `$HOME` and `$XDG_RUNTIME_DIR`.
pub fn files() -> Files {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    Files::new(home.as_deref(), runtime.as_deref())
}

/// Where [`lookup`] reads: docker's file, then the containers file.
pub fn default_paths() -> (PathBuf, PathBuf) {
    let f = files();
    (f.docker, f.containers)
}

/// Credentials for `host`. `Ok(None)`: no file names any. `Err` names the
/// helper that failed.
pub fn lookup(host: &str) -> Result<Option<Credential>, String> {
    files().lookup(host).map_err(|e| e.to_string())
}

/// Store a login in podvm's files or through their helper.
pub fn store_login(server: &str, username: &str, password: &str) -> Result<(), String> {
    files()
        .store_login(server, username, password)
        .map_err(|e| e.to_string())
}

/// Remove the login for `server`. True where an entry went away.
pub fn remove_login(server: &str) -> Result<bool, String> {
    files().remove_login(server).map_err(|e| e.to_string())
}
