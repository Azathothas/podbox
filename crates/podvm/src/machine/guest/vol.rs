//! `podvm guest-9p TAG=PATH[:ro]...`: the volumes of a guest, from inside
//! it, TODO/machine.md T-1723.
//!
//! The host gives each `-v HOST:GUEST` to the emulator as
//! `-virtfs local,path=HOST,mount_tag=podvmN,security_model=none`. In the
//! guest this loads the 9p modules of the pinned kernel (out of its
//! modloop, [`super::kernel`]) with `finit_module(2)` and mounts each tag
//! at its path over virtio, so a write in the guest lands in the host
//! directory. `:ro` mounts read-only. podvm writes the files with the
//! host user's ownership (`security_model=none`): the guest's `chown`
//! of a file in a volume does not reach the host.

/// The modules, in the order that their symbols need.
pub const MODULES: [&str; 4] = ["netfs", "9pnet", "9pnet_virtio", "9p"];

/// The path of each module in the modloop, under `modules/KVER/`.
pub const MODLOOP_PATHS: [(&str, &str); 4] = [
    ("netfs", "kernel/fs/netfs/netfs.ko"),
    ("9pnet", "kernel/net/9p/9pnet.ko"),
    ("9pnet_virtio", "kernel/net/9p/9pnet_virtio.ko"),
    ("9p", "kernel/fs/9p/9p.ko"),
];

/// The mount options of each tag.
pub const OPTIONS: &str = "trans=virtio,version=9p2000.L,msize=262144";

const MS_RDONLY: u64 = 1;

unsafe extern "C" {
    fn mount(
        src: *const i8,
        target: *const i8,
        fstype: *const i8,
        flags: u64,
        data: *const u8,
    ) -> i32;
}

/// One volume: the tag of the emulator, the guest path, read-only or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vol {
    pub tag: String,
    pub guest: String,
    pub ro: bool,
}

/// The volumes of a run, each with its tag `podvmN`.
pub fn vols(guests: &[(String, bool)]) -> Vec<Vol> {
    guests
        .iter()
        .enumerate()
        .map(|(i, (g, ro))| Vol {
            tag: format!("podvm{i}"),
            guest: g.clone(),
            ro: *ro,
        })
        .collect()
}

/// The emulator's argument of one volume.
pub fn virtfs(host: &std::path::Path, v: &Vol) -> String {
    format!(
        "local,path={},mount_tag={},security_model=none{}",
        host.display(),
        v.tag,
        if v.ro { ",readonly=on" } else { "" }
    )
}

/// The argument of the subcommand for one volume.
pub fn arg(v: &Vol) -> String {
    format!("{}={}{}", v.tag, v.guest, if v.ro { ":ro" } else { "" })
}

/// Parse one argument of the subcommand.
pub fn parse(a: &str) -> Option<Vol> {
    let (tag, rest) = a.split_once('=')?;
    let (guest, ro) = match rest.strip_suffix(":ro") {
        Some(g) => (g, true),
        None => (rest, false),
    };
    (!tag.is_empty() && guest.starts_with('/')).then(|| Vol {
        tag: tag.to_string(),
        guest: guest.to_string(),
        ro,
    })
}

fn mount_one(v: &Vol) -> Result<(), String> {
    std::fs::create_dir_all(&v.guest).map_err(|e| format!("mkdir {}: {e}", v.guest))?;
    let c = |s: &str| std::ffi::CString::new(s).map_err(|e| e.to_string());
    let (src, target, fs) = (c(&v.tag)?, c(&v.guest)?, c("9p")?);
    let data = c(OPTIONS)?;
    let flags = if v.ro { MS_RDONLY } else { 0 };
    // SAFETY: mount(2) with four NUL-terminated strings of this frame.
    let r = unsafe {
        mount(
            src.as_ptr(),
            target.as_ptr(),
            fs.as_ptr(),
            flags,
            data.as_ptr() as *const u8,
        )
    };
    if r != 0 {
        return Err(format!(
            "mount {} on {}: {}",
            v.tag,
            v.guest,
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

/// The subcommand. Exit 0 where each volume is mounted, 1 otherwise.
pub fn main(args: &[String]) -> i32 {
    let mut failed = Vec::new();
    for m in MODULES {
        if let Err(e) = super::net::load(m) {
            failed.push(e);
        }
    }
    if failed.is_empty() {
        for a in args {
            match parse(a) {
                Some(v) => {
                    if let Err(e) = mount_one(&v) {
                        failed.push(e);
                    }
                }
                None => failed.push(format!("{a:?} is not TAG=/PATH[:ro]")),
            }
        }
    }
    for f in &failed {
        eprintln!("podvm guest-9p: {f}");
    }
    i32::from(!failed.is_empty())
}
