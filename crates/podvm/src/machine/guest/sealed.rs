//! `--podvm-sealed`: the run of a guest whose data never touch a host file
//! or the argv of a host process, TODO/machine.md T-1751 (ask V-7 of
//! podswarm, its shield `guest`).
//!
//! ⭐ The caller gives the run as a tar stream on podvm's stdin, the
//! bundle: the member [`RUN_JSON`] names the command, the environment and
//! the working directory, and each other member is a file of the guest.
//! podvm keeps the bundle in its memory. The initrd on the host holds the
//! image and a generic init only; the guest has no NIC, no 9p volume and
//! no stdin file. In the guest, `podvm guest-channel --sealed` opens the
//! first stream of `pod-link` over the port `podvm.ch`, the host answers
//! it with the bundle, and the guest unpacks it into its root, which is
//! RAM.
//!
//! ⛔ An error names no member and no value of the bundle: each can hold a
//! value of the run.

use std::io::Read;
use std::path::Path;

/// The member of the bundle that names the run.
pub const RUN_JSON: &str = ".podvm-run.json";
/// The shell fragment of the run in the guest, which the init sources.
pub const FRAGMENT: &str = "/podvm-sealed.sh";

/// The run that [`RUN_JSON`] names. An empty `argv` keeps the image's
/// command, and no `workdir` keeps the image's.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Run {
    pub argv: Vec<String>,
    pub env: Vec<String>,
    pub workdir: Option<String>,
}

fn strings(v: &serde_json::Value, key: &str) -> Result<Vec<String>, String> {
    match v.get(key) {
        None | Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(serde_json::Value::Array(a)) => a
            .iter()
            .map(|s| {
                s.as_str().map(str::to_string).ok_or(format!(
                    "{RUN_JSON}: {key} holds a value that is not a string"
                ))
            })
            .collect(),
        Some(_) => Err(format!("{RUN_JSON}: {key} is not a list of strings")),
    }
}

/// Read [`RUN_JSON`].
pub fn parse_run(bytes: &[u8]) -> Result<Run, String> {
    let v: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| format!("{RUN_JSON} is not JSON"))?;
    if !v.is_object() {
        return Err(format!("{RUN_JSON} is not a JSON object"));
    }
    let env = strings(&v, "env")?;
    if env.iter().any(|e| !e.contains('=')) {
        return Err(format!("{RUN_JSON}: each entry of env is NAME=VALUE"));
    }
    let workdir = match v.get("workdir") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(w)) if w.starts_with('/') => Some(w.clone()),
        Some(_) => return Err(format!("{RUN_JSON}: workdir is an absolute path")),
    };
    Ok(Run {
        argv: strings(&v, "argv")?,
        env,
        workdir,
    })
}

/// Check the bundle on the host: a tar stream with one [`RUN_JSON`].
pub fn check(bundle: &[u8]) -> Result<Run, String> {
    let mut run = None;
    let mut a = tar::Archive::new(bundle);
    for e in a
        .entries()
        .map_err(|_| "the bundle is not a tar stream".to_string())?
    {
        let mut e = e.map_err(|_| "the bundle is not a tar stream".to_string())?;
        let path = e
            .path()
            .map_err(|_| "a member of the bundle has no path".to_string())?;
        if path.as_os_str() == RUN_JSON || path.as_os_str() == "./.podvm-run.json" {
            let mut b = Vec::new();
            e.read_to_end(&mut b)
                .map_err(|_| format!("{RUN_JSON} does not read"))?;
            run = Some(parse_run(&b)?);
        }
    }
    run.ok_or(format!("the bundle holds no {RUN_JSON}"))
}

/// Unpack the bundle into `root`, the guest's root, and give the run.
/// A member never lands outside `root` (`tar` refuses `..`). A member
/// belongs to the uid and gid of its header; a header with no number in
/// the field (a `tar::Builder` that set none) gives root's.
pub fn unpack(bundle: &[u8], root: &Path) -> Result<Run, String> {
    let run = check(bundle)?;
    let mut a = tar::Archive::new(bundle);
    a.set_preserve_permissions(true);
    a.set_overwrite(true);
    for e in a.entries().map_err(|e| e.to_string())? {
        let mut e = e.map_err(|e| e.to_string())?;
        let Ok(path) = e.path().map(|p| p.into_owned()) else {
            return Err("a member of the bundle has no path".to_string());
        };
        if path.as_os_str() == RUN_JSON || path.as_os_str() == "./.podvm-run.json" {
            continue;
        }
        let owner = (e.header().uid().unwrap_or(0), e.header().gid().unwrap_or(0));
        let landed = e
            .unpack_in(root)
            .map_err(|_| "a member of the bundle does not unpack".to_string())?;
        if landed && owner != (0, 0) {
            let at = root.join(&path);
            std::os::unix::fs::lchown(&at, Some(owner.0 as u32), Some(owner.1 as u32))
                .map_err(|_| "a member of the bundle takes no owner".to_string())?;
        }
    }
    Ok(run)
}

/// The shell fragment of the run: the exports, the `cd` and the `set --`.
pub fn fragment(run: &Run) -> String {
    let q = super::init::sh_quote;
    let mut s = String::new();
    for e in &run.env {
        if let Some((k, v)) = e.split_once('=')
            && !k.is_empty()
            && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            s.push_str(&format!("export {k}={}\n", q(v)));
        }
    }
    if let Some(w) = &run.workdir {
        s.push_str(&format!("cd {} || exit 125\n", q(w)));
    }
    if !run.argv.is_empty() {
        let words: Vec<String> = run.argv.iter().map(|a| q(a)).collect();
        s.push_str(&format!("set -- {}\n", words.join(" ")));
    }
    s
}

/// `-overcommit mem-lock=on` where `RLIMIT_MEMLOCK` holds the guest's
/// memory, else `None` and the reason.
pub fn mem_lock(mem: u64) -> Result<Vec<String>, String> {
    let limits = std::fs::read_to_string("/proc/self/limits").map_err(|e| e.to_string())?;
    let line = limits
        .lines()
        .find(|l| l.starts_with("Max locked memory"))
        .ok_or("no line of the locked memory in /proc/self/limits")?;
    let soft = line.split_whitespace().nth(3).unwrap_or("");
    let fits = soft == "unlimited" || soft.parse::<u64>().is_ok_and(|n| n >= mem + (64 << 20));
    if fits {
        Ok(vec!["-overcommit".into(), "mem-lock=on".into()])
    } else {
        Err(format!(
            "RLIMIT_MEMLOCK is {soft} bytes, less than the guest's memory and 64 MiB"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(run: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        let mut add = |p: &str, d: &[u8]| {
            let mut h = tar::Header::new_gnu();
            h.set_size(d.len() as u64);
            h.set_mode(0o600);
            h.set_cksum();
            b.append_data(&mut h, p, d).unwrap();
        };
        add(RUN_JSON, run.as_bytes());
        for (p, d) in files {
            add(p, d);
        }
        b.into_inner().unwrap()
    }

    #[test]
    fn the_bundle_names_the_run_and_its_files() {
        let b = bundle(
            r#"{"argv":["sh","-c","echo $X"],"env":["X=can'ary"],"workdir":"/w"}"#,
            &[("w/canary-name", b"canary-body")],
        );
        let run = check(&b).unwrap();
        assert_eq!(run.argv, ["sh", "-c", "echo $X"]);
        let f = fragment(&run);
        assert!(f.contains("export X='can'\\''ary'\n"), "{f}");
        assert!(f.contains("cd '/w' || exit 125\n"));
        assert!(f.ends_with("set -- 'sh' '-c' 'echo $X'\n"));
        let root = std::env::temp_dir().join(format!("podvm-sealed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        unpack(&b, &root).unwrap();
        assert_eq!(
            std::fs::read(root.join("w/canary-name")).unwrap(),
            b"canary-body"
        );
        assert!(!root.join(RUN_JSON).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// ⛔ An error of the bundle holds no value of the run.
    #[test]
    fn a_bad_bundle_refuses_and_names_no_value() {
        assert!(check(b"canary-not-a-tar").unwrap_err().contains("tar"));
        let b = bundle(r#"{"env":["canary-no-equals"]}"#, &[]);
        let e = check(&b).unwrap_err();
        assert!(!e.contains("canary"), "{e}");
        let b = bundle(r#"{"workdir":"canary-relative"}"#, &[]);
        assert!(!check(&b).unwrap_err().contains("canary"));
        let mut t = tar::Builder::new(Vec::new());
        let mut h = tar::Header::new_gnu();
        h.set_size(1);
        h.set_cksum();
        t.append_data(&mut h, "canary-file", &b"x"[..]).unwrap();
        let e = check(&t.into_inner().unwrap()).unwrap_err();
        assert!(e.contains("holds no") && !e.contains("canary"), "{e}");
    }
}
