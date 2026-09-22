//! `TODO/image.md` T-0209: registry credentials, without one ever entering
//! this tree's logs, errors, or result files.
//!
//! One read path: every credential podbox sends comes through [`lookup`],
//! which reads the two files the audience's machines already have,
//! `~/.docker/config.json` and `$XDG_RUNTIME_DIR/containers/auth.json`.
//! A named `credsStore` or `credHelpers` helper is executed when the config
//! names one, and a helper that fails is a named refusal rather than a
//! silent fall back to anonymous: falling back turns a permission problem
//! into a 404 about a repository that exists.
//!
//! ⛔ Nothing here prints. [`Credential`] exposes no debug view of the
//! secret, and the password never reaches a [`crate::error::Error`]. A test
//! below pins that with a canary.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A registry login. Held in memory, never logged, never put in an error.
#[derive(Clone)]
pub struct Credential {
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for Credential {
    /// ⛔ The username may print; the password never does. Test failures
    /// calling `expect_err` render this, which is exactly where a derived
    /// `Debug` would leak the secret it was written to catch.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Credential {{ username: {:?}, password: <redacted> }}",
            self.username
        )
    }
}

/// Credentials for `host`.
///
/// `Ok(None)` means no file names any: the pull proceeds anonymously and a
/// private repository answers 401, which names the registry. Absent files,
/// malformed files, and unknown hosts all answer `Ok(None)`. `Err` means a
/// named helper failed, and the string names the helper.
pub fn lookup(host: &str) -> Result<Option<Credential>, String> {
    let (docker, containers) = default_paths();
    lookup_in(&docker, &containers, host)
}

/// Credentials for `host` from two explicit files. [`lookup`] resolves the
/// paths; tests point them at scratch. The containers file wins where both
/// name the host: it is the runtime audience's file.
pub fn lookup_in(
    docker_config: &Path,
    containers_auth: &Path,
    host: &str,
) -> Result<Option<Credential>, String> {
    // ⛔ The helper is consulted before the files: a config that names a
    // helper for this host delegates trust explicitly, and files beside it
    // must not silently win over that delegation.
    if let Some(name) = helper_for(docker_config, containers_auth, host) {
        let server = format!("https://{host}/v2/");
        return helper_login(&name, &server).map(Some);
    }
    if let Some(cred) = read_auth_file(containers_auth).get(host).cloned() {
        return Ok(Some(cred));
    }
    Ok(read_auth_file(docker_config).get(host).cloned())
}

/// The `Authorization` value for `cred`: `Basic` over `user:password`.
pub fn basic_header(cred: &Credential) -> String {
    use base64::Engine as _;
    let joined = format!("{}:{}", cred.username, cred.password);
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(joined)
    )
}

/// Parse one `auths` document into host logins. Pure, so tests own it.
/// Entries that do not decode are skipped: a file that names nothing usable
/// for a host is absence for that host, not a failure of the lookup.
pub fn parse_auth_json(text: &str) -> HashMap<String, Credential> {
    let mut out = HashMap::new();
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(text) else {
        return out;
    };
    let Some(auths) = doc.get("auths").and_then(|a| a.as_object()) else {
        return out;
    };
    for (host, entry) in auths {
        if let Some(cred) = parse_entry(entry) {
            out.insert(host.clone(), cred);
        }
    }
    out
}

/// One `auths` entry: the `auth` field first, then the `username` plus
/// `password` pair podman's file uses. Anything else is not a login.
fn parse_entry(entry: &serde_json::Value) -> Option<Credential> {
    use base64::Engine as _;
    if let Some(encoded) = entry.get("auth").and_then(|a| a.as_str()) {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()?;
        let text = String::from_utf8(raw).ok()?;
        let (username, password) = text.split_once(':')?;
        if username.is_empty() {
            return None;
        }
        return Some(Credential {
            username: username.into(),
            password: password.into(),
        });
    }
    let username = entry.get("username").and_then(|u| u.as_str())?;
    let password = entry.get("password").and_then(|p| p.as_str())?;
    if username.is_empty() {
        return None;
    }
    Some(Credential {
        username: username.into(),
        password: password.into(),
    })
}

/// Read one `auths` file into host logins. Absent or unreadable is empty.
fn read_auth_file(path: &Path) -> HashMap<String, Credential> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    parse_auth_json(&text)
}

/// The helper the configs name for `host`, if any: per-host `credHelpers`
/// first, then the global `credsStore`. Docker's two spellings, both read.
fn helper_for(docker_config: &Path, containers_auth: &Path, host: &str) -> Option<String> {
    for path in [docker_config, containers_auth] {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if let Some(name) = doc
            .get("credHelpers")
            .and_then(|h| h.get(host))
            .and_then(|h| h.as_str())
        {
            return Some(name.into());
        }
        if let Some(name) = doc.get("credsStore").and_then(|h| h.as_str()) {
            return Some(name.into());
        }
    }
    None
}

/// The login from the helper the config names for `server_url`.
///
/// # Errors
///
/// Returns an error naming the helper where it is missing, fails, or prints
/// nothing parseable. A failed helper is never a silent fall back.
pub fn helper_login(helper: &str, server_url: &str) -> Result<Credential, String> {
    use std::io::Write;
    use std::process::Stdio;
    let bin = format!("docker-credential-{helper}");
    let mut child = std::process::Command::new(&bin)
        .arg("get")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("credential helper {bin} would not start: {e}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| format!("credential helper {bin} takes no stdin"))?
        .write_all(server_url.as_bytes())
        .map_err(|e| format!("credential helper {bin} would not take the server URL: {e}"))?;
    let out = child
        .wait_with_output()
        .map_err(|e| format!("credential helper {bin} has no exit to read: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "credential helper {bin} refused {server_url}: exit {}",
            out.status.code().map_or("signal".into(), |c| c.to_string())
        ));
    }
    let reply: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|_| {
        format!("credential helper {bin} printed nothing parseable for {server_url}")
    })?;
    let username = reply
        .get("Username")
        .and_then(|u| u.as_str())
        .filter(|u| !u.is_empty())
        .ok_or_else(|| format!("credential helper {bin} named no user for {server_url}"))?;
    let password = reply.get("Secret").and_then(|s| s.as_str()).unwrap_or("");
    Ok(Credential {
        username: username.into(),
        password: password.into(),
    })
}

/// Store a login through the named helper: `docker-credential-<helper>`
/// `store` with `{"ServerURL","Username","Secret"}` on stdin.
///
/// # Errors
///
/// Returns an error naming the helper where it would not start or where
/// storing fails. Never falls back to the file: the config named the
/// helper, so the file beside it must not silently win.
pub fn helper_store(
    helper: &str,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<(), String> {
    use std::io::Write;
    use std::process::Stdio;
    let bin = format!("docker-credential-{helper}");
    let mut child = std::process::Command::new(&bin)
        .arg("store")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("credential helper {bin} would not start: {e}"))?;
    let body = serde_json::json!({
        "ServerURL": server_url,
        "Username": username,
        "Secret": password,
    });
    let text = serde_json::to_string(&body)
        .map_err(|e| format!("credential helper {bin} takes no JSON: {e}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| format!("credential helper {bin} takes no stdin"))?
        .write_all(text.as_bytes())
        .map_err(|e| format!("credential helper {bin} would not take the login: {e}"))?;
    let status = child
        .wait()
        .map_err(|e| format!("credential helper {bin} has no exit to read: {e}"))?;
    if !status.success() {
        return Err(format!(
            "credential helper {bin} refused to store the login for {server_url}: exit {}",
            status.code().map_or("signal".into(), |c| c.to_string())
        ));
    }
    Ok(())
}

/// Store a login: through the named helper where one is configured for the
/// host, else in docker's file (or the containers file where that file
/// already holds this host's login, so one host never splits across two
/// files). [`store_login`] resolves the paths; tests point them at scratch.
pub fn store_login(server: &str, username: &str, password: &str) -> Result<(), String> {
    let (docker, containers) = default_paths();
    store_login_in(&docker, &containers, server, username, password)
}

/// [`store_login`] against explicit files.
pub fn store_login_in(
    docker_config: &Path,
    containers_auth: &Path,
    server: &str,
    username: &str,
    password: &str,
) -> Result<(), String> {
    let host = server_host(server);
    // ⛔ The helper first, like the read path: a config that names a helper
    // for this host must not gain a file entry beside it. Nothing is written
    // unless the helper answers.
    if let Some(name) = helper_for(docker_config, containers_auth, host) {
        let server_url = format!("https://{host}/v2/");
        return helper_store(&name, &server_url, username, password);
    }
    // One home: the file that already holds this host wins, else docker's.
    let target = if read_auth_file(containers_auth).contains_key(host) {
        containers_auth
    } else {
        docker_config
    };
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("would not create {}: {e}", parent.display()))?;
        }
    }
    let before = std::fs::read_to_string(target).unwrap_or_default();
    let after = upsert_auth_json(&before, host, username, password);
    write_owner_only(target, &after)
}

/// Write `text` to `path` with owner-only permissions.
///
/// Creates the file where missing; an existing file keeps its ownership and
/// only its mode is narrowed. A credential file a umask left readable is
/// the defect this exists for.
fn write_owner_only(path: &Path, text: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, text).map_err(|e| format!("would not write {}: {e}", path.display()))?;
    let perm = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, perm)
        .map_err(|e| format!("would not narrow {}: {e}", path.display()))?;
    Ok(())
}

/// Where [`lookup`] reads: docker's file, then the containers file.
///
/// The containers path follows the runtime first: `$XDG_RUNTIME_DIR` where
/// set, else `~/.config/containers/auth.json`.
pub fn default_paths() -> (PathBuf, PathBuf) {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let docker = home
        .as_ref()
        .map(|h| h.join(".docker/config.json"))
        .unwrap_or_default();
    let containers = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .map(|d| d.join("containers/auth.json"))
        .or_else(|| {
            home.as_ref()
                .map(|h| h.join(".config/containers/auth.json"))
        })
        .unwrap_or_default();
    (docker, containers)
}

/// Insert or replace the `auth` entry for `server`, keeping every other
/// key and host. `podbox login` writes through here; an empty or broken
/// document starts fresh rather than failing the login.
pub fn upsert_auth_json(text: &str, server: &str, username: &str, password: &str) -> String {
    use base64::Engine as _;
    let joined = format!("{username}:{password}");
    let encoded = base64::engine::general_purpose::STANDARD.encode(joined);
    let mut doc: serde_json::Value =
        serde_json::from_str(text).unwrap_or_else(|_| serde_json::json!({}));
    if !doc.is_object() {
        doc = serde_json::json!({});
    }
    doc["auths"][server]["auth"] = serde_json::Value::String(encoded);
    serde_json::to_string(&doc).unwrap_or_default()
}

/// The lookup key for a server as the user names it: scheme and path
/// stripped, so `https://index.docker.io/v1/` and `index.docker.io` name
/// one host.
pub fn server_host(server: &str) -> &str {
    let bare = server
        .strip_prefix("https://")
        .or_else(|| server.strip_prefix("http://"))
        .unwrap_or(server);
    bare.split('/').next().unwrap_or(bare)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const CANARY_USER: &str = "canary-user-09f2";
    const CANARY_PASS: &str = "canary-pass-7b3e";
    /// `printf %s 'canary-user-09f2:canary-pass-7b3e' | base64 -w0`, taken
    /// outside this crate: the oracle is independent of the implementation.
    const CANARY_HEADER: &str = "Basic Y2FuYXJ5LXVzZXItMDlmMjpjYW5hcnktcGFzcy03YjNl";
    const HOST: &str = "127.0.0.1:5443";

    struct ScratchDir(PathBuf);
    impl ScratchDir {
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn scratch_dir() -> ScratchDir {
        let d = std::env::temp_dir().join(format!(
            "podbox-cred-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .subsec_nanos()
        ));
        std::fs::create_dir_all(&d).expect("scratch dir");
        ScratchDir(d)
    }

    /// An `auths` document with one base64 `auth` entry, the docker spelling.
    fn scratch_auth(user: &str, pass: &str, host: &str) -> (ScratchDir, PathBuf) {
        let dir = scratch_dir();
        let p = dir.path().join("auth.json");
        let joined = format!("{user}:{pass}");
        let enc = base64_encode_for_test(joined.as_bytes());
        let mut f = std::fs::File::create(&p).expect("scratch file");
        write!(f, "{{\"auths\":{{\"{host}\":{{\"auth\":\"{enc}\"}}}}}}").expect("scratch write");
        (dir, p)
    }

    /// Test-only encoder, so the fixture never depends on the module's path.
    fn base64_encode_for_test(bytes: &[u8]) -> String {
        const ALPH: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut enc = String::new();
        let mut i = 0;
        while i < bytes.len() {
            let b0 = bytes[i] as u32;
            let b1 = if i + 1 < bytes.len() {
                bytes[i + 1] as u32
            } else {
                0
            };
            let b2 = if i + 2 < bytes.len() {
                bytes[i + 2] as u32
            } else {
                0
            };
            let n = (b0 << 16) | (b1 << 8) | b2;
            enc.push(ALPH[((n >> 18) & 63) as usize] as char);
            enc.push(ALPH[((n >> 12) & 63) as usize] as char);
            enc.push(if i + 1 < bytes.len() {
                ALPH[((n >> 6) & 63) as usize] as char
            } else {
                '='
            });
            enc.push(if i + 2 < bytes.len() {
                ALPH[(n & 63) as usize] as char
            } else {
                '='
            });
            i += 3;
        }
        enc
    }

    #[test]
    fn an_auth_entry_decodes_to_its_user_and_password() {
        let (_dir, p) = scratch_auth(CANARY_USER, CANARY_PASS, HOST);
        let got = lookup_in(Path::new("/nonexistent"), &p, HOST)
            .expect("lookup runs")
            .expect("the seeded host has a login");
        assert_eq!(got.username, CANARY_USER);
        assert_eq!(got.password, CANARY_PASS);
    }

    #[test]
    fn an_unknown_host_is_anonymous_not_an_error() {
        let (_dir, p) = scratch_auth(CANARY_USER, CANARY_PASS, HOST);
        let got = lookup_in(Path::new("/nonexistent"), &p, "example.com").expect("lookup runs");
        assert!(got.is_none());
    }

    #[test]
    fn a_malformed_file_is_absence_not_a_panic() {
        let dir = scratch_dir();
        let p = dir.path().join("auth.json");
        std::fs::write(&p, "not json at all").expect("scratch write");
        let got = lookup_in(Path::new("/nonexistent"), &p, HOST).expect("lookup runs");
        assert!(got.is_none());
    }

    #[test]
    fn the_basic_header_is_the_precomputed_value() {
        let cred = Credential {
            username: CANARY_USER.into(),
            password: CANARY_PASS.into(),
        };
        assert_eq!(basic_header(&cred), CANARY_HEADER);
    }

    #[test]
    fn a_missing_helper_is_a_named_refusal() {
        let err = helper_login("podbox-no-such-helper-xyz", "https://x/v2/")
            .expect_err("a missing helper fails");
        assert!(err.contains("podbox-no-such-helper-xyz"), "names it: {err}");
    }

    #[test]
    fn a_named_but_failing_helper_is_a_named_refusal() {
        let dir = scratch_dir();
        let cfg = dir.path().join("config.json");
        std::fs::write(
            &cfg,
            r#"{"auths":{},"credHelpers":{"127.0.0.1:5443":"podbox-no-such-helper-xyz"}}"#,
        )
        .expect("scratch write");
        let err = lookup_in(&cfg, Path::new("/nonexistent"), HOST).expect_err("helper fails");
        assert!(err.contains("podbox-no-such-helper-xyz"), "names it: {err}");
    }

    #[test]
    fn the_password_reaches_no_header_or_error_string() {
        let cred = Credential {
            username: CANARY_USER.into(),
            password: CANARY_PASS.into(),
        };
        assert!(
            !basic_header(&cred).contains(CANARY_PASS),
            "canary in header"
        );
        let err = helper_login("podbox-no-such-helper-xyz", "https://x/v2/")
            .expect_err("a missing helper fails");
        assert!(!err.contains(CANARY_PASS), "canary in refusal");
    }

    #[test]
    fn upsert_creates_auths_where_none_exist() {
        let out = upsert_auth_json("{}", HOST, CANARY_USER, CANARY_PASS);
        let back = parse_auth_json(&out);
        let got = back.get(HOST).expect("written entry reads back");
        assert_eq!(got.username, CANARY_USER);
        assert_eq!(got.password, CANARY_PASS);
    }

    #[test]
    fn upsert_replaces_one_host_and_keeps_the_other() {
        let (_dir, p) = scratch_auth("other-user", "other-pass", "example.com:5000");
        let text = std::fs::read_to_string(&p).expect("scratch read");
        let out = upsert_auth_json(&text, HOST, CANARY_USER, CANARY_PASS);
        let back = parse_auth_json(&out);
        assert_eq!(back.get(HOST).expect("new host").username, CANARY_USER);
        assert_eq!(
            back.get("example.com:5000").expect("old host").username,
            "other-user"
        );
    }

    #[test]
    fn upsert_keeps_non_auth_keys() {
        let out = upsert_auth_json(
            r#"{"credsStore":"desktop","auths":{}}"#,
            HOST,
            CANARY_USER,
            CANARY_PASS,
        );
        let doc: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(doc["credsStore"], "desktop");
        assert!(doc["auths"][HOST]["auth"].is_string());
    }

    #[test]
    fn upsert_starts_fresh_on_a_broken_document() {
        let out = upsert_auth_json("not json", HOST, CANARY_USER, CANARY_PASS);
        let back = parse_auth_json(&out);
        assert!(back.contains_key(HOST));
    }

    #[test]
    fn a_store_through_a_missing_helper_is_a_named_refusal() {
        let err = helper_store("podbox-no-such-helper-xyz", "https://x/v2/", "u", "p")
            .expect_err("a missing helper fails the store");
        assert!(err.contains("podbox-no-such-helper-xyz"), "names it: {err}");
    }

    #[test]
    fn server_names_reduce_to_their_host() {
        assert_eq!(
            server_host("https://index.docker.io/v1/"),
            "index.docker.io"
        );
        assert_eq!(server_host("https://127.0.0.1:5443/v2/"), "127.0.0.1:5443");
        assert_eq!(server_host("127.0.0.1:5443"), "127.0.0.1:5443");
        assert_eq!(server_host("example.com"), "example.com");
    }

    #[test]
    fn store_writes_a_readable_entry() {
        let dir = scratch_dir();
        let docker = dir.path().join("config.json");
        let containers = dir.path().join("auth.json");
        store_login_in(&docker, &containers, HOST, CANARY_USER, CANARY_PASS).expect("store runs");
        let text = std::fs::read_to_string(&docker).expect("file written");
        let back = parse_auth_json(&text);
        let got = back.get(HOST).expect("entry reads back");
        assert_eq!(got.username, CANARY_USER);
        assert_eq!(got.password, CANARY_PASS);
    }

    #[test]
    fn store_updates_the_file_that_holds_the_host() {
        let (_dir, containers) = scratch_auth("old-user", "old-pass", HOST);
        let dir = scratch_dir();
        let docker = dir.path().join("config.json");
        store_login_in(&docker, &containers, HOST, CANARY_USER, CANARY_PASS).expect("store runs");
        assert!(!docker.exists(), "one home: the containers file wins");
        let text = std::fs::read_to_string(&containers).expect("file kept");
        let back = parse_auth_json(&text);
        assert_eq!(back.get(HOST).expect("entry").username, CANARY_USER);
    }

    #[test]
    fn store_sets_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch_dir();
        let docker = dir.path().join("config.json");
        let containers = dir.path().join("auth.json");
        store_login_in(&docker, &containers, HOST, CANARY_USER, CANARY_PASS).expect("store runs");
        let mode = std::fs::metadata(&docker)
            .expect("file written")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "no group or other access");
    }

    #[test]
    fn store_through_a_missing_helper_is_a_named_refusal() {
        let dir = scratch_dir();
        let docker = dir.path().join("config.json");
        std::fs::write(
            &docker,
            r#"{"auths":{},"credHelpers":{"127.0.0.1:5443":"podbox-no-such-helper-xyz"}}"#,
        )
        .expect("scratch write");
        let containers = dir.path().join("auth.json");
        let err = store_login_in(&docker, &containers, HOST, CANARY_USER, CANARY_PASS)
            .expect_err("helper fails");
        assert!(err.contains("podbox-no-such-helper-xyz"), "names it: {err}");
        let written = std::fs::read_to_string(&docker).expect("read");
        assert!(
            !written.contains(CANARY_USER),
            "nothing stored beside a failed helper"
        );
    }
}
