//! The model of a profile: load, `extends`, `-s`, the strict check, and the
//! `run` flags of the effective profile. TODO/cli.md T-1654.

use std::path::{Path, PathBuf};

use toml::{Table, Value};

use super::values::*;

/// The keys of each table. A key outside these is an error that names it.
const TOP: &[&str] = &[
    "extends",
    "description",
    "leaves_out",
    "needs",
    "fs",
    "limits",
    "net",
    "env",
];
const FS: &[&str] = &[
    "image",
    "backend",
    "user",
    "wrap",
    "read_only",
    "workspace",
    "workdir",
    "volumes",
    "tmpfs",
];
const LIMITS: &[&str] = &["file_max", "memory", "cpus", "timeout"];
const NET: &[&str] = &["mode", "ports", "allow"];
const ENV: &[&str] = &["set"];
const BACKENDS: &[&str] = &["auto", "machine", "incus", "emulate"];
/// The deepest chain of `extends`: a longer one is a cycle or a mistake.
const MAX_DEPTH: usize = 8;

/// One profile, merged and checked.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Checked {
    pub description: String,
    /// What the shape does not model (TODO/cli.md T-1659).
    pub leaves_out: Vec<String>,
    /// The programs that the image must have, or the run refuses.
    pub needs: Vec<String>,
    /// The image of the shape.
    pub image: Option<String>,
    pub backend: Option<String>,
    pub user: Option<String>,
    pub wrap: Vec<String>,
    pub read_only: bool,
    pub workspace: Option<String>,
    pub workdir: Option<String>,
    pub volumes: Vec<String>,
    pub tmpfs: Vec<String>,
    pub file_max: Option<u64>,
    pub memory: Option<u64>,
    pub cpus: Option<u32>,
    pub timeout: Option<u64>,
    pub net_mode: Option<String>,
    pub net_ports: Vec<u16>,
    pub net_allow: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// Where a profile came from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    BuiltIn,
    File(PathBuf),
}

impl Source {
    pub fn word(&self) -> String {
        match self {
            Source::BuiltIn => "built-in".to_string(),
            Source::File(p) => p.display().to_string(),
        }
    }
}

/// The directory of the user's profiles: `$XDG_CONFIG_HOME/podvm/profiles`,
/// else `$HOME/.config/podvm/profiles`.
pub fn user_dir() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(x) if !x.is_empty() => PathBuf::from(x),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(base.join("podvm").join("profiles"))
}

/// A name that can be a file name: letters, digits, `-`, `_` and `.`, not
/// starting with `.`.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

/// The raw table of `name`: the user's file first, then the built-in.
pub fn raw(name: &str, dir: Option<&Path>) -> Result<(Table, Source), String> {
    if !valid_name(name) {
        return Err(format!(
            "profile {name:?}: a name takes letters, digits, -, _ and . only"
        ));
    }
    if let Some(dir) = dir {
        let path = dir.join(format!("{name}.toml"));
        if path.is_file() {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("profile {name}: {}: {e}", path.display()))?;
            let table = text.parse::<Table>().map_err(|e| {
                format!(
                    "profile {name}: {}: {}",
                    path.display(),
                    e.to_string().trim()
                )
            })?;
            return Ok((table, Source::File(path)));
        }
    }
    match super::builtin::text(name) {
        Some(text) => text
            .parse::<Table>()
            .map(|t| (t, Source::BuiltIn))
            .map_err(|e| format!("profile {name}: the built-in text: {e}")),
        None => Err(format!(
            "profile {name}: no such profile. `podvm profile list` names each one"
        )),
    }
}

/// The table of `name` with each parent of its `extends` chain merged
/// under it. `extends` itself is gone from the result.
pub fn resolve(name: &str, dir: Option<&Path>) -> Result<(Table, Source), String> {
    let mut chain = vec![name.to_string()];
    let (mut table, source) = raw(name, dir)?;
    let mut layers = Vec::new();
    loop {
        let parent = match table.remove("extends") {
            None => break,
            Some(Value::String(p)) => p,
            Some(other) => {
                return Err(format!(
                    "profile {}: `extends` must be a string, not {}",
                    chain.last().map(String::as_str).unwrap_or(name),
                    article(&other)
                ))
            }
        };
        if chain.contains(&parent) {
            return Err(format!(
                "profile {name}: `extends` makes a cycle: {} -> {parent}",
                chain.join(" -> ")
            ));
        }
        if chain.len() >= MAX_DEPTH {
            return Err(format!(
                "profile {name}: `extends` is deeper than {MAX_DEPTH}: {}",
                chain.join(" -> ")
            ));
        }
        chain.push(parent.clone());
        layers.push(table);
        table = raw(&parent, dir)?.0;
    }
    // `table` is the root of the chain; each child goes over it in turn.
    while let Some(child) = layers.pop() {
        merge(&mut table, child);
    }
    Ok((table, source))
}

/// Merge `over` into `base`: a table merges key by key, each other value
/// replaces.
pub fn merge(base: &mut Table, over: Table) {
    for (k, v) in over {
        match (base.get_mut(&k), v) {
            (Some(Value::Table(b)), Value::Table(o)) => merge(b, o),
            (_, v) => {
                base.insert(k, v);
            }
        }
    }
}

/// Apply one `-s KEY=VALUE`. The value parses as TOML and falls back to a
/// string. Each table of the path is made where it is absent.
pub fn set(table: &mut Table, spec: &str) -> Result<(), String> {
    let Some((path, text)) = spec.split_once('=') else {
        return Err(format!(
            "-s {spec:?}: the form is KEY=VALUE, as fs.user=1000"
        ));
    };
    let keys: Vec<&str> = path.trim().split('.').collect();
    if keys.iter().any(|k| k.is_empty()) {
        return Err(format!("-s {spec:?}: the key {path:?} has an empty part"));
    }
    let value = match format!("v = {text}").parse::<Table>() {
        Ok(mut t) => t
            .remove("v")
            .unwrap_or_else(|| Value::String(text.to_string())),
        Err(_) => Value::String(text.to_string()),
    };
    let mut at = table;
    for k in &keys[..keys.len() - 1] {
        let entry = at
            .entry(k.to_string())
            .or_insert_with(|| Value::Table(Table::new()));
        at = match entry {
            Value::Table(t) => t,
            other => {
                return Err(format!(
                    "-s {spec:?}: `{k}` is {}, not a table",
                    article(other)
                ))
            }
        };
    }
    at.insert(keys[keys.len() - 1].to_string(), value);
    Ok(())
}

/// The strict check of a merged table.
pub fn check(t: &Table) -> Result<Checked, String> {
    only(t, "", TOP)?;
    let mut c = Checked {
        description: match t.get("description") {
            None => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => {
                return Err(format!(
                    "`description` must be a string, not {}",
                    article(other)
                ))
            }
        },
        leaves_out: strings(t, "", "leaves_out")?,
        needs: strings(t, "", "needs")?,
        ..Checked::default()
    };
    if let Some(fs) = want_table(t, "fs")? {
        only(fs, "fs", FS)?;
        c.image = string(fs, "fs", "image")?;
        c.backend = string(fs, "fs", "backend")?;
        if let Some(b) = &c.backend
            && !BACKENDS.contains(&b.as_str())
        {
            return Err(format!(
                "`fs.backend` takes {}, not {b:?}",
                BACKENDS.join(", ")
            ));
        }
        // A uid is an integer where `-s fs.user=1000` parsed it as TOML.
        c.user = match fs.get("user") {
            Some(Value::Integer(n)) if *n >= 0 => Some(n.to_string()),
            _ => string(fs, "fs", "user")?,
        };
        c.wrap = strings(fs, "fs", "wrap")?;
        c.read_only = boolean(fs, "fs", "read_only")?.unwrap_or(false);
        c.workspace = string(fs, "fs", "workspace")?;
        c.workdir = string(fs, "fs", "workdir")?;
        c.volumes = strings(fs, "fs", "volumes")?;
        c.tmpfs = strings(fs, "fs", "tmpfs")?;
    }
    if let Some(l) = want_table(t, "limits")? {
        only(l, "limits", LIMITS)?;
        c.file_max = size(l, "limits", "file_max")?;
        c.memory = size(l, "limits", "memory")?;
        c.cpus = integer(l, "limits", "cpus", 1, 64)?.map(|n| n as u32);
        c.timeout = integer(l, "limits", "timeout", 1, i64::MAX)?.map(|n| n as u64);
    }
    if let Some(n) = want_table(t, "net")? {
        only(n, "net", NET)?;
        c.net_mode = string(n, "net", "mode")?;
        c.net_allow = strings(n, "net", "allow")?;
        c.net_ports = match n.get("ports") {
            None => Vec::new(),
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| match v {
                    Value::Integer(p) if (1..=65535).contains(p) => Ok(*p as u16),
                    other => Err(format!(
                        "`net.ports` holds ports from 1 to 65535, not {other}"
                    )),
                })
                .collect::<Result<_, _>>()?,
            Some(other) => {
                return Err(format!(
                    "`net.ports` must be an array of ports, not {}",
                    article(other)
                ))
            }
        };
    }
    if let Some(e) = want_table(t, "env")? {
        only(e, "env", ENV)?;
        let set = match e.get("set") {
            None => None,
            Some(Value::Table(x)) => Some(x),
            Some(other) => {
                return Err(format!("`env.set` must be a table, not {}", article(other)))
            }
        };
        if let Some(set) = set {
            for (k, v) in set {
                match v {
                    Value::String(s) => c.env.push((k.clone(), s.clone())),
                    other => {
                        return Err(format!(
                            "`env.set.{k}` must be a string, not {}",
                            article(other)
                        ))
                    }
                }
            }
        }
    }
    Ok(c)
}

/// The `run` flags of a checked profile. `cwd` resolves a relative
/// `fs.workspace`. A key that this host or tier has no mechanism for
/// refuses here, naming the key.
pub fn run_args(c: &Checked, cwd: &Path) -> Result<Vec<String>, String> {
    let machine = c.backend.as_deref() == Some("machine");
    let mut a = Vec::new();
    match c.backend.as_deref() {
        None | Some("auto") => {}
        Some(b) => a.push(format!("--podvm-tier={b}")),
    }
    if let Some(u) = &c.user {
        a.push(format!("--user={u}"));
    }
    if c.read_only {
        a.push("--read-only".into());
    }
    if let Some(w) = &c.workspace {
        let host = cwd.join(w);
        let host = std::fs::canonicalize(&host)
            .map_err(|e| format!("`fs.workspace` {w:?}: {}: {e}", host.display()))?;
        a.push("-v".into());
        a.push(format!("{}:/workspace", host.display()));
    }
    if let Some(w) = &c.workdir {
        a.push(format!("--workdir={w}"));
    }
    for v in &c.volumes {
        a.push("-v".into());
        a.push(v.clone());
    }
    for t in &c.tmpfs {
        a.push(format!("--tmpfs={t}"));
    }
    if let Some(n) = c.file_max {
        a.push(format!("--ulimit=fsize={n}"));
    }
    if let Some(m) = c.memory {
        if !machine {
            return Err(format!(
                "`limits.memory` needs fs.backend = \"machine\": a rootfs tier has no \
                 cgroup that bounds memory, so the limit would not hold (TODO/cli.md \
                 T-0804). The machine tier gives it as guest RAM ({m} bytes)"
            ));
        }
        a.push(format!("--podvm-mem={m}"));
    }
    if let Some(n) = c.cpus {
        if !machine {
            return Err(
                "`limits.cpus` needs fs.backend = \"machine\": a rootfs tier has no \
                 cgroup that bounds CPUs, so the limit would not hold (TODO/cli.md \
                 T-0804). The machine tier gives it as vCPUs"
                    .to_string(),
            );
        }
        a.push(format!("--podvm-cpus={n}"));
    }
    if let Some(t) = c.timeout {
        a.push(format!("--timeout={t}"));
    }
    match c.net_mode.as_deref() {
        None | Some("host") => {}
        // TODO/net.md T-1665: the egress broker; the run refuses it where
        // no mechanism holds the payload to it.
        Some("broker") => {
            a.push("--network=broker".into());
            if !c.net_ports.is_empty() {
                let p: Vec<String> = c.net_ports.iter().map(u16::to_string).collect();
                a.push(format!("--ports={}", p.join(",")));
            }
            for h in &c.net_allow {
                a.push(format!("--allow={h}"));
            }
        }
        Some(m) => {
            return Err(format!(
                "`net.mode` {m:?} has no mechanism yet: `net.mode` takes \"host\" (the \
                 host's network) or \"broker\" (TODO/net.md T-1665). The network entries \
                 T-1662 to T-1664 add the other modes"
            ))
        }
    }
    for (k, v) in &c.env {
        a.push("-e".into());
        a.push(format!("{k}={v}"));
    }
    Ok(a)
}

/// The effective profile as TOML text, for `profile show`.
pub fn render(t: &Table) -> String {
    let mut out = String::new();
    let mut tables = Vec::new();
    for (k, v) in t {
        match v {
            Value::Table(sub) => tables.push((k.clone(), sub)),
            other => out.push_str(&format!("{k} = {}\n", other.to_string().trim())),
        }
    }
    for (k, sub) in tables {
        render_table(&mut out, &k, sub);
    }
    out
}

fn render_table(out: &mut String, path: &str, t: &Table) {
    out.push_str(&format!("\n[{path}]\n"));
    let mut subs = Vec::new();
    for (k, v) in t {
        match v {
            Value::Table(sub) => subs.push((format!("{path}.{k}"), sub)),
            other => out.push_str(&format!("{k} = {}\n", other.to_string().trim())),
        }
    }
    for (p, sub) in subs {
        render_table(out, &p, sub);
    }
}
