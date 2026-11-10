//! `-v HOST:GUEST` on a host with no `/dev/fuse` and no `mount(2)`.
//!
//! [`TODO/enter.md`](../../../../TODO/enter.md) T-1653. A volume here is a
//! path rewrite, not a mount and not a copy: each `-v HOST:GUEST` becomes
//! one `GUEST:HOST` pair in the `PODVM_MAPS` table beside the root pair,
//! and the interposer takes the longest matching prefix, so the guest
//! directory wins over the root pair. A write lands in the host
//! directory, as with a docker bind mount.
//!
//! Only the loader family of the userland rung has the interposer that
//! reads the table. Each other entry refuses a volume by name: the chroot
//! rung cannot reach a host path after the chroot, the memfd family has
//! no interposer (T-1652), and the machine tier boots a guest that no host
//! path reaches.

use podvm_enter::userland::{volume_at, Family, Volume};

use crate::lifecycle::Userland;

/// The refusal of `:ro` until T-1655 gives a read-only layer.
pub const RO_REFUSAL: &str = "`:ro` is refused: podvm cannot enforce a read-only \
     volume yet, and a volume that reads as read-only and takes writes would be a \
     lie. The read-only layer of TODO/enter.md T-1655 serves it with EROFS; until \
     then, drop `:ro` (TODO/enter.md T-1653)";

/// Parse one `-v` value: `HOST:GUEST[:OPTIONS]`.
///
/// The host side is an absolute path, or a relative one that starts with
/// `.`, and it must exist: docker creates a missing host directory, and
/// podvm refuses rather than creating one. It becomes its canonical path,
/// because the kernel reports a working directory with each link
/// resolved, and the reverse map of the interposer matches that form
/// (T-1643). A bare name is a named volume and a single path is an
/// anonymous volume: both need a volume store, and podvm keeps none.
/// `rw` is the default and is accepted; `ro` and each other option are
/// refused by name. The table checks of the pair run in
/// `podvm_enter::userland::userland_maps_table`.
pub fn parse(spec: &str) -> Result<Volume, String> {
    let parts: Vec<&str> = spec.split(':').collect();
    let (host, guest, opts) = match parts.as_slice() {
        [_] => {
            return Err(
                "an anonymous volume needs a volume store, and podvm keeps none: \
                 name a host path, as HOST:GUEST"
                    .to_string(),
            )
        }
        [h, g] => (*h, *g, ""),
        [h, g, o] => (*h, *g, *o),
        _ => {
            return Err(
                "takes HOST:GUEST[:OPTIONS], and a path with `:` cannot be spelled \
                 in the PODVM_MAPS table"
                    .to_string(),
            )
        }
    };
    for o in opts.split(',') {
        match o {
            "" | "rw" => {}
            "ro" => return Err(RO_REFUSAL.to_string()),
            other => {
                return Err(format!(
                    "the option {other:?} has nothing to act on: a volume here is a \
                     path rewrite, not a mount. podvm takes rw, and refuses ro until \
                     TODO/enter.md T-1655"
                ))
            }
        }
    }
    if host.is_empty() || !(host.starts_with('/') || host.starts_with('.')) {
        return Err(format!(
            "{host:?} is a named volume, which needs a volume store, and podvm keeps \
             none: name a host path that starts with `/` or `.`"
        ));
    }
    if !guest.starts_with('/') {
        return Err(format!("the guest path {guest:?} is not absolute"));
    }
    let canon = std::fs::canonicalize(host).map_err(|e| {
        format!(
            "the host path {host:?}: {e}. docker creates a missing host path; podvm \
             refuses rather than creating one"
        )
    })?;
    let Some(host) = canon.to_str() else {
        return Err(format!("the host path {canon:?} is not UTF-8"));
    };
    Ok(Volume {
        guest: guest.to_string(),
        host: host.to_string(),
    })
}

/// Refuse a working directory or a first program inside a volume.
///
/// Both resolve inside the rootfs, in `pod-enter`, and not through the
/// table: a path in a volume there would run the image's file or start
/// in the image's directory, so each is refused by name. This runs before
/// the entry decision, which would otherwise answer that the image has
/// no such file.
pub fn refuse_paths(
    verb: &str,
    volumes: &[Volume],
    argv: &[String],
    workdir: &str,
) -> Result<(), i32> {
    let argv0 = argv.first().map(String::as_str).unwrap_or("");
    let why = if let Some(v) = volume_at(volumes, workdir) {
        format!(
            "the working directory {workdir} is inside the volume at {}, and the \
             entry starts it in the image's rootfs, not through the table: use \
             `cd {workdir}` in the command",
            v.guest
        )
    } else if let Some(v) = argv0
        .starts_with('/')
        .then(|| volume_at(volumes, argv0))
        .flatten()
    {
        format!(
            "the program {argv0} is inside the volume at {}, and the entry resolves \
             the first program in the image's rootfs, not through the table: start \
             it from a shell of the image, as `sh -c {argv0}`",
            v.guest
        )
    } else {
        return Ok(());
    };
    refuse(verb, volumes, &why)
}

/// Refuse the volumes on each entry that cannot serve them: only the
/// loader family reads the table. `forced` is the `PODVM_MODE` rung,
/// where one is forced.
pub fn refuse_unserved(
    verb: &str,
    volumes: &[Volume],
    userland: Option<&Userland>,
    forced: Option<podvm_enter::ladder::Mode>,
) -> Result<(), i32> {
    if volumes.is_empty() {
        return Ok(());
    }
    let why = match (forced, userland.map(|u| u.family)) {
        (None, Some(Family::Loader)) => return Ok(()),
        (Some(mode), _) => format!(
            "PODVM_MODE={} forces a rung with no path table, and only the userland \
             loader family serves a volume",
            mode.name()
        ),
        (None, None) => "this host permits chroot(2), so podvm enters the chroot rung, \
             where a host path is out of reach and podvm cannot mount(2) one in. Only \
             the userland loader family serves a volume, where chroot is denied"
            .to_string(),
        (None, Some(Family::Memfd)) => "the memfd family runs a static payload with no \
             interposer, so no path rewrite reaches it, until TODO/enter.md T-1652 maps \
             a volume to a host directory descriptor"
            .to_string(),
    };
    refuse(verb, volumes, &why)
}

/// One refusal line at 125, naming the first volume.
fn refuse(verb: &str, volumes: &[Volume], why: &str) -> Result<(), i32> {
    if let Some(v) = volumes.first() {
        eprintln!(
            "podvm {verb}: -v {}:{}: {why} (TODO/enter.md T-1653)",
            v.host, v.guest
        );
    }
    Err(podvm_image::error::EXIT_RUNTIME_ERROR)
}

/// The banner lines of the volumes: one for each volume, and one for each
/// source on a `noexec` mount, read from this process's mount table.
pub fn banner(volumes: &[Volume]) -> String {
    if volumes.is_empty() {
        return String::new();
    }
    let table = std::fs::read_to_string("/proc/self/mountinfo").ok();
    banner_with(volumes, table.as_deref())
}

/// [`banner`] over a given mount table, `None` where it cannot be read.
pub fn banner_with(volumes: &[Volume], mountinfo: Option<&str>) -> String {
    let mut s = String::new();
    for v in volumes {
        s.push_str(&format!(
            "podvm: userland: -v {}:{} is a path rewrite through PODVM_MAPS, not a \
             mount: a write lands in {}, and a call that does not pass the interposer \
             (a static program, a raw syscall instruction) sees the image's {} \
             (TODO/enter.md T-1653)\n",
            v.host, v.guest, v.host, v.guest
        ));
        match mountinfo.and_then(|m| mount_options(m, &v.host)) {
            Some(o) if o.split(',').any(|f| f == "noexec") => s.push_str(&format!(
                "podvm: userland: the volume source {} is on a noexec mount, so a \
                 program in {} cannot exec from it (TODO/enter.md T-1653)\n",
                v.host, v.guest
            )),
            Some(_) => {}
            None => s.push_str(&format!(
                "podvm: userland: podvm cannot read the mount of {} in \
                 /proc/self/mountinfo, so it cannot tell whether a program in {} can \
                 exec (TODO/enter.md T-1653)\n",
                v.host, v.guest
            )),
        }
    }
    s
}

/// The options of the mount that holds the canonical path `path`: the
/// longest mount point over it, and the last line of equal length, which
/// is the mount on top. Fields 5 and 6 of a `mountinfo` line are the mount
/// point (with `\ooo` escapes) and the options of the mount.
pub fn mount_options(mountinfo: &str, path: &str) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for line in mountinfo.lines() {
        let mut f = line.split(' ');
        let (Some(point), Some(opts)) = (f.nth(4), f.next()) else {
            continue;
        };
        let point = unescape(point);
        let holds = point == "/"
            || path == point
            || path
                .strip_prefix(point.as_str())
                .is_some_and(|r| r.starts_with('/'));
        if holds && best.is_none_or(|(n, _)| point.len() >= n) {
            best = Some((point.len(), opts));
        }
    }
    best.map(|(_, o)| o.to_string())
}

/// Undo the `\ooo` octal escapes of a `mountinfo` field.
fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\'
            && i + 3 < b.len()
            && (b'0'..=b'3').contains(&b[i + 1])
            && b[i + 2..i + 4].iter().all(|c| (b'0'..=b'7').contains(c))
        {
            out.push((b[i + 1] - b'0') * 64 + (b[i + 2] - b'0') * 8 + (b[i + 3] - b'0'));
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests;
