//! The incus image of an OCI image, TODO/machine.md T-1718: a unified
//! tarball of podvm's extracted rootfs.
//!
//! podvm extracts with no ownership change and records what the image
//! meant in `.meta.jsonl` beside the rootfs (`podvm extract --help`). The
//! tarball gives each entry those ids and that mode, so that the files in
//! the instance belong to whom the image says (`/etc/shadow` is
//! `root:shadow`). An entry with no record gets the owner 0 and its mode on
//! disk.
//!
//! The tarball also carries podvm itself at `/.podvm/podvm`, the init of
//! the instance (TODO/machine.md T-1722): a run then pushes no binary. The
//! alias names podvm's own digest, so a new podvm imports the image again.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

/// The version of the tarball's form; a change gives a new alias.
pub const IMAGE_FORMAT: u32 = 3;

/// The alias of an image of `digest` with the podvm of digest `me`.
pub fn alias(digest: &str, me: &str) -> String {
    let hex = digest.trim_start_matches("sha256:");
    format!(
        "podvm-{}-{}-v{IMAGE_FORMAT}",
        &hex[..hex.len().min(16)],
        &me[..me.len().min(16)]
    )
}

/// The intended uid, gid and mode for each path of the sidecar; a later
/// line (a later layer) wins.
pub fn intended(sidecar: &str) -> HashMap<String, (u64, u64, u32)> {
    let mut m = HashMap::new();
    for line in sidecar.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let (Some(p), Some(u), Some(g)) = (
            v.get("path").and_then(Value::as_str),
            v.get("uid").and_then(Value::as_u64),
            v.get("gid").and_then(Value::as_u64),
        ) else {
            continue;
        };
        let Some(mode) = v
            .get("mode")
            .and_then(Value::as_str)
            .and_then(|s| u32::from_str_radix(s, 8).ok())
        else {
            continue;
        };
        m.insert(p.trim_start_matches('/').to_string(), (u, g, mode));
    }
    m
}

/// The unified tarball of `rootfs` at `out`: `metadata.yaml` and
/// `rootfs/`, with the owners and modes of `.meta.jsonl` beside `rootfs`,
/// and `podvm` at `rootfs/.podvm/podvm` (owner 0, mode 0755).
pub fn unified_tarball(rootfs: &Path, out: &Path, podvm: &[u8]) -> Result<(), String> {
    let sidecar = rootfs.with_file_name(".meta.jsonl");
    let meant = intended(&std::fs::read_to_string(&sidecar).unwrap_or_default());
    let f = std::fs::File::create(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut b = tar::Builder::new(std::io::BufWriter::new(f));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1);
    // incus refuses a creation_date of 0 ("Missing creation date").
    let meta = format!(
        "architecture: x86_64\ncreation_date: {now}\nproperties:\n  description: an OCI image of podvm\n"
    );
    let mut h = tar::Header::new_gnu();
    h.set_size(meta.len() as u64);
    h.set_mode(0o644);
    h.set_uid(0);
    h.set_gid(0);
    h.set_mtime(now);
    h.set_cksum();
    let err = |e: std::io::Error| format!("the unified tarball of {}: {e}", rootfs.display());
    b.append_data(&mut h, "metadata.yaml", meta.as_bytes())
        .map_err(err)?;
    walk(&mut b, rootfs, "", &meant).map_err(err)?;
    for (path, mode, data) in [
        ("rootfs/.podvm", 0o755, None),
        ("rootfs/.podvm/podvm", 0o755, Some(podvm)),
    ] {
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(if data.is_some() {
            tar::EntryType::Regular
        } else {
            tar::EntryType::Directory
        });
        h.set_size(data.map_or(0, |d| d.len() as u64));
        h.set_mode(mode);
        h.set_uid(0);
        h.set_gid(0);
        h.set_mtime(now);
        h.set_cksum();
        b.append_data(&mut h, path, data.unwrap_or(&[]))
            .map_err(err)?;
    }
    b.into_inner()
        .and_then(|mut w| std::io::Write::flush(&mut w))
        .map_err(err)
}

fn walk<W: std::io::Write>(
    b: &mut tar::Builder<W>,
    dir: &Path,
    rel: &str,
    meant: &HashMap<String, (u64, u64, u32)>,
) -> std::io::Result<()> {
    let mut names: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    names.sort_by_key(|e| e.file_name());
    for e in names {
        let name = e.file_name().to_string_lossy().into_owned();
        let r = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        let p = e.path();
        let md = std::fs::symlink_metadata(&p)?;
        let mut h = tar::Header::new_gnu();
        h.set_metadata_in_mode(&md, tar::HeaderMode::Deterministic);
        let (u, g, mode) = meant.get(&r).copied().unwrap_or((
            0,
            0,
            std::os::unix::fs::PermissionsExt::mode(&md.permissions()),
        ));
        h.set_uid(u);
        h.set_gid(g);
        h.set_mode(mode & 0o7777);
        let at = format!("rootfs/{r}");
        let ft = md.file_type();
        if ft.is_symlink() {
            let target = std::fs::read_link(&p)?;
            b.append_link(&mut h, &at, target)?;
        } else if ft.is_dir() {
            h.set_size(0);
            b.append_data(&mut h, &at, std::io::empty())?;
            walk(b, &p, &r, meant)?;
        } else if ft.is_file() {
            b.append_data(&mut h, &at, std::fs::File::open(&p)?)?;
        }
        // A socket, a fifo or a device: an extracted rootfs holds none.
    }
    Ok(())
}
