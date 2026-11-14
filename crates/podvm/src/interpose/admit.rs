//! One interposer object against one payload (TODO/interpose.md T-0709,
//! T-1644, T-1880): the object is present, it parses, it is of the
//! payload's machine, and the link set that the loader resolves inside the
//! image admits it.

use super::{Libc, Reach, Slot};

/// Check one object against the payload `elf`: present, parsed, of its
/// machine, and admitted by the link set read inside the image.
pub(super) fn admit_slot(
    name: &str,
    elf: &podvm_enter::abi::Elf,
    rootfs: &str,
    interp: &str,
    libc: Libc,
    slot: &Slot,
) -> Reach {
    let Some(bytes) = slot.bytes else {
        return Reach::Declined(format!(
            "this podvm binary carries no {} interposer object, so the tier \
             is declined; ./scripts/build-interpose.sh builds them",
            libc.word()
        ));
    };
    let Some(obj) = &slot.elf else {
        return Reach::Declined(format!(
            "the embedded {} object does not parse, so podvm refuses to \
             preload what it cannot check",
            libc.word()
        ));
    };
    // ⛔ The payload's architecture against the object's, before anything
    // else about the pair: a foreign payload under binfmt would otherwise get
    // an object its loader cannot read.
    if obj.machine != elf.machine {
        return Reach::Declined(format!(
            "{name} is ELF machine 0x{:x} and the {} object is 0x{:x}, so the \
             tier cannot serve it",
            elf.machine,
            libc.word(),
            obj.machine
        ));
    }
    let Some(libc_path) = podvm_enter::abi::libc_beside(rootfs, interp, elf.machine) else {
        return Reach::Declined(format!(
            "{interp} names no C library podvm can find in the image"
        ));
    };
    // ⭐ T-0709's assertion, through T-0706's channel: select and assert,
    // never try. A version predicate the set fails is a decline that names
    // the version, not a relocation error inside the payload. T-1644: the
    // check runs against the LINK SET the loader resolves, read inside the
    // rootfs: the interpreter and the object's whole DT_NEEDED closure. The
    // libc's own directory joins the search, for a layout the standard list
    // does not name.
    let root = std::path::Path::new(rootfs);
    let mut dirs = podvm_enter::abi::guest_lib_dirs(rootfs, interp);
    if let Some(dir) = guest_dir_of(rootfs, &libc_path)
        && !dirs.contains(&dir)
    {
        dirs.push(dir);
    }
    let read = |p: &str| podvm_enter::abi::Elf::read(p);
    let set = match podvm_enter::abi::link_set(root, obj, interp, &dirs, &read) {
        Ok(s) => s,
        Err(why) => return Reach::Declined(why),
    };
    match podvm_enter::abi::admits_set(obj, &set) {
        podvm_enter::abi::Verdict::Admitted => Reach::Preload {
            libc,
            object: bytes,
        },
        podvm_enter::abi::Verdict::Refused(why) => Reach::Declined(why),
    }
}

/// The guest directory of a host path under `rootfs`, T-1644.
///
/// The libc search can answer a canonical path, so the canonical root is
/// tried after the given one. `None` where the path lies under neither.
pub(super) fn guest_dir_of(rootfs: &str, host: &str) -> Option<String> {
    let dir = host.rsplit_once('/').map(|(d, _)| d)?;
    let base = rootfs.trim_end_matches('/');
    let rel = match dir.strip_prefix(base) {
        Some(r) => r.to_string(),
        None => {
            let canon = std::fs::canonicalize(base).ok()?;
            dir.strip_prefix(canon.to_str()?)?.to_string()
        }
    };
    (rel.is_empty() || rel.starts_with('/')).then(|| format!("/{}", rel.trim_start_matches('/')))
}
