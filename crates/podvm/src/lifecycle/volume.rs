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

/// Parse one `-v` value: `HOST:GUEST[:OPTIONS]`.
///
/// The host side is an absolute path, or a relative one that starts with
/// `.`, and it must exist: docker creates a missing host directory, and
/// podvm refuses rather than creating one. It becomes its canonical path,
/// because the kernel reports a working directory with each link
/// resolved, and the reverse map of the interposer matches that form
/// (T-1643). A bare name is a named volume and a single path is an
/// anonymous volume: both need a volume store, and podvm keeps none.
/// `rw` is the default; `ro` makes the volume read-only through the layer
/// of the interposer (TODO/cli.md T-1655); each other option is refused by
/// name. The table checks of the pair run in `pod-enter`'s `Maps::new`
/// (with `podvm_enter::NAMES`) and its table builders.
///
/// The answer is the pair and true where the options hold `ro`.
pub fn parse(spec: &str) -> Result<(Volume, bool), String> {
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
    let mut ro = false;
    for o in opts.split(',') {
        match o {
            "" | "rw" => {}
            "ro" => ro = true,
            other => {
                return Err(format!(
                    "the option {other:?} has nothing to act on: a volume here is a \
                     path rewrite, not a mount. podvm takes rw and ro (TODO/cli.md T-1655)"
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
    Ok((
        Volume {
            guest: guest.to_string(),
            host: host.to_string(),
        },
        ro,
    ))
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
    // ⭐ TODO/enter.md T-1724: the native-static rung maps each volume to a
    // host directory descriptor of its handler.
    if forced.is_none() && userland.is_some_and(|u| u.dispatch_exe.is_some()) {
        return Ok(());
    }
    let why = match (forced, userland.map(|u| u.family)) {
        (None, Some(Family::Loader)) => return Ok(()),
        (Some(mode), _) => format!(
            "PODVM_MODE={} forces a rung with no path table, and only the userland \
             loader family serves a volume",
            mode.name()
        ),
        (None, None) => "only the userland loader family serves a volume, through its path \
             table, and it cannot take this payload (the line above says why), so podvm \
             enters the chroot rung, where a host path is out of reach and podvm cannot \
             mount(2) one in"
            .to_string(),
        (None, Some(Family::Memfd)) => "the memfd family runs a static payload with no \
             interposer, so no path rewrite reaches it; the native-static rung \
             (TODO/enter.md T-1652, T-1724) serves a volume where the kernel takes \
             prctl(PR_SET_SYSCALL_USER_DISPATCH) and --unsafe-host-paths is absent"
            .to_string(),
    };
    refuse(verb, volumes, &why)
}

/// TODO/cli.md T-1772: make the working directory in the rootfs where it
/// is absent, as docker does, before `check_workdir`. Each component is
/// made in turn and none is followed: a link or a file of the image on the
/// way stops the walk, and the check then refuses as before. A directory
/// in a volume is left to the volume.
pub fn make_workdir(rootfs: &str, workdir: &str, volumes: &[Volume]) {
    let in_volume = volumes.iter().any(|v| {
        let g = v.guest.trim_end_matches('/');
        workdir == g || workdir.starts_with(&format!("{g}/"))
    });
    let parts: Vec<&str> = workdir.split('/').filter(|p| !p.is_empty()).collect();
    if in_volume || !workdir.starts_with('/') || parts.iter().any(|p| *p == "." || *p == "..") {
        return;
    }
    let mut at = std::path::PathBuf::from(rootfs);
    for part in parts {
        at.push(part);
        match std::fs::symlink_metadata(&at) {
            Ok(m) if m.file_type().is_dir() => continue,
            Ok(_) => return,
            Err(_) => {
                if std::fs::create_dir(&at).is_err() {
                    return;
                }
            }
        }
    }
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
    let mut all = volumes.to_vec();
    all.extend(device_pairs(rootfs, volumes));
    podvm_enter::userland::Maps::new(&podvm_enter::NAMES, rootfs, &all).map_err(|why| {
        eprintln!("podvm {verb}: -v: {why} (TODO/enter.md T-1653)");
        podvm_image::error::EXIT_RUNTIME_ERROR
    })
}

/// TODO/image.md T-1774: where the rootfs holds a regular file in place of a
/// device (the shim of T-0401, where mknod(2) is refused), the table maps
/// the path to the host's device, so a read of `/dev/urandom` does not end
/// at 1 MiB. Only where the host's node is a character device, and not
/// where a volume of the caller covers the path.
pub fn device_pairs(rootfs: &str, volumes: &[Volume]) -> Vec<Volume> {
    use std::os::unix::fs::FileTypeExt;
    let chr =
        |p: &std::path::Path| std::fs::metadata(p).is_ok_and(|m| m.file_type().is_char_device());
    ["/dev/null", "/dev/zero", "/dev/random", "/dev/urandom"]
        .into_iter()
        .filter(|d| !volumes.iter().any(|v| v.guest.trim_end_matches('/') == *d))
        .filter(|d| {
            let inner = std::path::Path::new(rootfs).join(d.trim_start_matches('/'));
            // The table is made before the completion writes its shim, so
            // a node that is not there yet takes the host's device too.
            chr(std::path::Path::new(d)) && !chr(&inner)
        })
        .map(|d| Volume {
            guest: d.to_string(),
            host: d.to_string(),
        })
        .collect()
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

/// The read-only table of a run (`PODVM_RO`, read by the interposer): `ro:/`
/// under `--read-only`, with `rw:` for each writable volume, and `ro:` for
/// each `:ro` volume. ⛔ TODO/cli.md T-1655: only the loader family of the
/// userland rung has the interposer that reads it, so each other entry
/// refuses by name, as a volume does. A caller's own `PODVM_RO` is dropped
/// first.
pub fn read_only_env(
    verb: &str,
    read_only: bool,
    ro_guests: &[String],
    volumes: &[Volume],
    userland: Option<&Userland>,
    env: &mut Vec<String>,
) -> Result<(), i32> {
    env.retain(|e| !e.starts_with("PODVM_RO="));
    if !read_only && ro_guests.is_empty() {
        return Ok(());
    }
    if !matches!(userland.map(|u| u.family), Some(Family::Loader)) {
        eprintln!(
            "podvm {verb}: --read-only and :ro need the read-only layer of the \
             interposer, which only the userland loader family has; this entry \
             cannot make the rootfs read-only without mount(2) (TODO/cli.md T-1655)"
        );
        return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    env.push(format!(
        "PODVM_RO={}",
        read_only_table(read_only, ro_guests, volumes)
    ));
    Ok(())
}

/// TODO/cli.md T-1726. The read-only table of a container's run, from its
/// record, into the environment of an `exec`: as [`read_only_env`] does for
/// `run`, a caller's own value goes first, and an entry with no read-only
/// layer refuses by name.
pub fn container_read_only(
    verb: &str,
    record_env: &[String],
    userland: Option<&Userland>,
    env: &mut Vec<String>,
) -> Result<(), i32> {
    env.retain(|e| !e.starts_with("PODVM_RO="));
    let Some(table) = record_env.iter().find(|e| e.starts_with("PODVM_RO=")) else {
        return Ok(());
    };
    if !matches!(userland.map(|u| u.family), Some(Family::Loader))
        || userland.is_some_and(|u| u.dispatch_exe.is_some())
    {
        eprintln!(
            "podvm {verb}: the container has a :ro volume or --read-only, which needs \
             the read-only layer of the interposer, and this entry has none \
             (TODO/cli.md T-1655, T-1726)"
        );
        return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    env.push(table.clone());
    Ok(())
}

/// The value of `PODVM_RO`.
pub fn read_only_table(read_only: bool, ro_guests: &[String], volumes: &[Volume]) -> String {
    let mut entries = Vec::new();
    if read_only {
        // `/dev` stays writable, as docker's tmpfs on `/dev` is: a shell
        // writes to `/dev/null`, which is a file of the rootfs here (T-0401).
        entries.push("ro:/".to_string());
        entries.push("rw:/dev".to_string());
        for v in volumes {
            if !ro_guests.contains(&v.guest) {
                entries.push(format!("rw:{}", v.guest));
            }
        }
    }
    entries.extend(ro_guests.iter().map(|g| format!("ro:{g}")));
    entries.join(",")
}
