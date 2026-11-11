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
//!
//! The entry resolves the working directory and the first program through
//! the same table (`pod-enter`'s `abi::GuestRoot`, here a
//! `podvm_enter::userland::Maps`), so `-w` and a first program can be in a
//! volume: each is the file of the volume, as the payload sees it.

use podvm_enter::userland::{Family, Volume};

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
/// `pod-enter`'s `Maps::new` (with `podvm_enter::NAMES`) and its table builders.
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

/// The table of a run: the root pair of `rootfs` and the volumes. A
/// volume that the table cannot hold is refused at 125 with its reason;
/// `parse` of `run` refused each such volume as a flag error before.
pub fn maps(
    verb: &str,
    rootfs: &str,
    volumes: &[Volume],
) -> Result<podvm_enter::userland::Maps, i32> {
    podvm_enter::userland::Maps::new(&podvm_enter::NAMES, rootfs, volumes).map_err(|why| {
        eprintln!("podvm {verb}: -v: {why} (TODO/enter.md T-1653)");
        podvm_image::error::EXIT_RUNTIME_ERROR
    })
}

/// The banner lines of the volumes: one for each volume, and one for each
/// source on a `noexec` mount, from one `statfs(2)` of each source.
///
/// ⭐ `pod-hostprobe`'s `Statfs::flag(ST_NOEXEC)` answers for the mount
/// that holds the path, so podvm reads no mount table (TODO/enter.md
/// T-1653).
pub fn banner(volumes: &[Volume]) -> String {
    banner_with(volumes, noexec)
}

/// True where the mount that holds `path` is `noexec`; `None` where
/// `statfs(2)` fails or the kernel did not fill the mount flags.
pub fn noexec(path: &str) -> Option<bool> {
    use podvm_probe::sys;
    let c = sys::CBuf::new(path)?;
    sys::statfs(&c).ok()?.flag(sys::ST_NOEXEC)
}

/// [`banner`] with a given answer for the `noexec` flag of a source.
pub fn banner_with(volumes: &[Volume], noexec: impl Fn(&str) -> Option<bool>) -> String {
    let mut s = String::new();
    for v in volumes {
        s.push_str(&format!(
            "podvm: userland: -v {}:{} is a path rewrite through PODVM_MAPS, not a \
             mount: a write lands in {}, and a call that does not pass the interposer \
             (a static program, a raw syscall instruction) sees the image's {} \
             (TODO/enter.md T-1653)\n",
            v.host, v.guest, v.host, v.guest
        ));
        match noexec(&v.host) {
            Some(true) => s.push_str(&format!(
                "podvm: userland: the volume source {} is on a noexec mount, so a \
                 program in {} cannot exec from it (TODO/enter.md T-1653)\n",
                v.host, v.guest
            )),
            Some(false) => {}
            None => s.push_str(&format!(
                "podvm: userland: podvm cannot read the mount flags of {} with \
                 statfs(2), so it cannot tell whether a program in {} can exec \
                 (TODO/enter.md T-1653)\n",
                v.host, v.guest
            )),
        }
    }
    s
}

#[cfg(test)]
mod tests;
