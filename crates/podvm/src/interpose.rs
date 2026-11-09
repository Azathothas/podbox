//! The two interposer objects as bytes, and which one a payload may load.
//!
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-0702. `build.rs` puts
//! them in `OUT_DIR`, or puts two empty files there when
//! `scripts/build-interpose.sh` has not run.
//!
//! ⛔ **ONE OBJECT PER LIBC, AND THE CHOICE IS NOT A PREFERENCE.** A preloaded
//! object is loaded by the payload's own dynamic loader and resolves its
//! imports against the payload's libc. `experiments/results/interposer-abi.txt`
//! measured both directions: a musl-linked object into a glibc payload dies at
//! `/lib/x86_64-linux-gnu/libc.so: invalid ELF header`, before a symbol is
//! read, because musl's libc declares no SONAME and the glibc path of that name
//! is a linker script. The reverse dies on a missing `__snprintf_chk`.
//!
//! ⚠ **Placement lives here too, since T-0702's second half landed.**
//! [`apply`] classifies the payload, writes the selected object INSIDE the
//! rootfs before the chroot, and sets `LD_PRELOAD` to the path the payload
//! will see. `run`, `exec` and `create` all call that one function, so the
//! three verbs cannot disagree about whether a payload is reachable.

/// Which libc an object is linked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Libc {
    Gnu,
    Musl,
}

impl Libc {
    pub fn word(self) -> &'static str {
        match self {
            Libc::Gnu => "glibc",
            Libc::Musl => "musl",
        }
    }
}

/// The glibc-linked object, empty where it was not built.
pub const GNU: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/interpose-gnu.so"));

/// The musl-linked object, empty where it was not built.
pub const MUSL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/interpose-musl.so"));

/// The object for `libc`, or `None` where this binary carries none.
///
/// ⛔ `None` is a fact and not an error. A podvm built on a machine with no
/// zig carries no musl object, and the caller's job is then to decline with a
/// named reason rather than to preload something that cannot load.
pub fn object(libc: Libc) -> Option<&'static [u8]> {
    let bytes = match libc {
        Libc::Gnu => GNU,
        Libc::Musl => MUSL,
    };
    (!bytes.is_empty()).then_some(bytes)
}

/// One line naming what this binary carries, for `podvm system info`.
pub fn carried() -> String {
    match (object(Libc::Gnu).is_some(), object(Libc::Musl).is_some()) {
        (true, true) => "interposer: glibc and musl objects embedded".into(),
        (true, false) => "interposer: the glibc object only. A musl payload is declined".into(),
        (false, true) => "interposer: the musl object only. A glibc payload is declined".into(),
        (false, false) => "interposer: none embedded, so every payload is declined. \
             ./scripts/build-interpose.sh builds them"
            .into(),
    }
}

/// Where the placed object sits, as the payload sees it.
///
/// ⭐ One path, and T-0702's `Prove` reads it back from inside the payload.
pub const GUEST_PATH: &str = "/.podvm/interpose.so";

/// What the tier does with one payload.
///
/// ⛔ `Declined` still runs the payload. It declines the TIER, never the run:
/// T-0706's decision is that a mode that cannot reach the payload says so by
/// name rather than failing later and deeper.
pub enum Reach {
    /// Preload the object for this libc.
    Preload { libc: Libc, object: &'static [u8] },
    /// Run without the tier, and say why.
    Declined(String),
}

/// The embedded objects, parsed for the checks the loader would run.
///
/// ⭐ Bytes and parse travel together, so a test can fake either half: an
/// absent object, a present one that does not parse, and a parsed one of
/// another architecture are three different arms and each is driven without
/// depending on what this binary embedded.
struct Slot {
    bytes: Option<&'static [u8]>,
    elf: Option<podvm_enter::abi::Elf>,
}

struct Parsed {
    gnu: Slot,
    musl: Slot,
}

fn slot(name: &str, libc: Libc) -> Slot {
    match object(libc) {
        None => Slot {
            bytes: None,
            elf: None,
        },
        Some(b) => Slot {
            bytes: Some(b),
            elf: podvm_enter::abi::Elf::parse(name, b).ok(),
        },
    }
}

fn parsed() -> Parsed {
    Parsed {
        gnu: slot("the embedded glibc object", Libc::Gnu),
        musl: slot("the embedded musl object", Libc::Musl),
    }
}

/// Go build markers. `.note.go.buildid` is in every Go binary; the other
/// three where it was not stripped.
fn is_go(elf: &podvm_enter::abi::Elf) -> bool {
    elf.sections.iter().any(|s| {
        matches!(
            s.as_str(),
            ".note.go.buildid" | ".gosymtab" | ".gopclntab" | ".go.buildinfo"
        )
    })
}

/// The payload file, read from outside the rootfs.
///
/// An argument with a `/` in it names a path under the rootfs. A bare name is
/// looked for along `path_dirs` and the first file wins, which is the order
/// the child tries after the chroot.
fn resolve(rootfs: &str, argv0: &str, path_dirs: &[String]) -> Result<String, String> {
    use podvm_enter::abi::{resolve_in, ResolveKind};
    let root = std::path::Path::new(rootfs);
    if !root.is_dir() {
        return Err(format!("{rootfs}: not a directory podvm can read"));
    }
    // ⭐ One walker for both readers: `podvm_enter::abi::resolve_in` owns
    // the guest-kernel symlink walk, and this maps its kinds to sentences.
    // A second copy here would be the copy that diverges.
    let say = |guest: &str, kind: ResolveKind| match kind {
        ResolveKind::Absent => format!("{argv0} names no file in the image"),
        ResolveKind::Escapes => format!("{guest} escapes the image"),
        ResolveKind::Loop => format!("{guest} has too many levels of symlinks"),
    };
    if argv0.contains('/') {
        return resolve_in(root, argv0)
            .map(|p| p.display().to_string())
            .map_err(|kind| say(argv0, kind));
    }
    let mut refused: Option<String> = None;
    for d in path_dirs {
        let guest = format!("{}/{argv0}", d.trim_end_matches('/'));
        match resolve_in(root, &guest) {
            Ok(p) => return Ok(p.display().to_string()),
            Err(ResolveKind::Absent) => continue,
            // ⚠ The first refusal wins, so a loop reads as a loop rather
            // than as a missing file once the other directories miss.
            Err(kind) => {
                if refused.is_none() {
                    refused = Some(say(&guest, kind));
                }
            }
        }
    }
    Err(refused.unwrap_or_else(|| format!("{argv0} names no file in the image")))
}

/// Read the payload ELF once and say whether the tier reaches it.
///
/// ⭐ Advisory, and the safe direction on every doubt. A wrong decline costs
/// one stderr line; a wrong preload hands the payload's loader an object it
/// cannot load, which fails the payload for the tier's sake.
pub fn classify(rootfs: &str, argv0: &str, path_dirs: &[String]) -> Reach {
    let path = match resolve(rootfs, argv0, path_dirs) {
        Ok(p) => p,
        Err(why) => return Reach::Declined(why),
    };
    let elf = match podvm_enter::abi::Elf::read(&path) {
        Ok(e) => e,
        Err(e) => return Reach::Declined(format!("{e}")),
    };
    classify_elf(argv0, &elf, rootfs, &parsed())
}

fn classify_elf(name: &str, elf: &podvm_enter::abi::Elf, rootfs: &str, objs: &Parsed) -> Reach {
    use podvm_enter::abi::Flavour;
    // ⭐ Row 3 of T-0706's table first: a Go payload is unreachable REGARDLESS
    // of linkage, so its markers outrank a present `PT_INTERP`.
    if is_go(elf) {
        return Reach::Declined(format!(
            "{name} carries Go build markers, and Go issues syscalls directly \
             rather than through libc, so the tier cannot reach it"
        ));
    }
    let Some(interp) = &elf.interp else {
        return Reach::Declined(format!(
            "{name} is statically linked (no PT_INTERP), so no loader reads \
             LD_PRELOAD for it"
        ));
    };
    let libc = match Flavour::of_libc_name(interp) {
        Flavour::Unknown => {
            return Reach::Declined(format!(
                "{name} names {interp} as its interpreter, which names no libc \
                 podvm recognises"
            ));
        }
        Flavour::Glibc => Libc::Gnu,
        Flavour::Musl => Libc::Musl,
    };
    let slot = match libc {
        Libc::Gnu => &objs.gnu,
        Libc::Musl => &objs.musl,
    };
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
fn guest_dir_of(rootfs: &str, host: &str) -> Option<String> {
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

/// Write the object inside the rootfs, where the payload's loader sees it.
///
/// ⭐ Sibling plus rename, so a concurrent reader never sees a half-written
/// object. An absolute path from outside the chroot does not resolve inside
/// it, which is why the write happens here rather than at run time.
pub fn place(rootfs: &str, object_bytes: &[u8]) -> Result<(), String> {
    let dir = format!("{}/.podvm", rootfs.trim_end_matches('/'));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{dir}: {e}"))?;
    let tmp = format!("{dir}/.interpose.so.tmp");
    std::fs::write(&tmp, object_bytes).map_err(|e| format!("{tmp}: {e}"))?;
    std::fs::rename(&tmp, format!("{dir}/interpose.so"))
        .map_err(|e| format!("{dir}/interpose.so: {e}"))?;
    Ok(())
}

/// The variable that carries the requested identity into the payload.
///
/// ⚠ The name is shared with `podvm-interpose`, which reads the same bytes
/// out of its own environ: that crate cannot depend on this one, so the two
/// spellings are one fact in two homes, and
/// `experiments/106-interpose-identity.sh` asserts they agree by driving the
/// behaviour end to end.
pub const IDENTITY_VAR: &str = "PODVM_IDENTITY";

/// The variable carrying the memo descriptor number into the payload.
///
/// ⭐ One path: `podvm-supervise::table` owns the file, the number and the
/// grammar. This re-exports it so `run` and `exec` read one name; the
/// interposer's own spelling lives in `podvm-interpose` and is asserted by
/// `experiments/105-interpose-ownership.sh`.
pub use podvm_supervise::table::{
    ensure_memo_file, memo_fd_env, memo_fd_of, open_memo, MEMO_CHILD_FD, MEMO_FD_VAR,
};

/// An ephemeral host memo for a run with no container record.
///
/// Foreground `run` and image-path `exec` enter a rootfs with no container
/// around it, so there is no `containers/<id>/` to sit beside. The file still
/// lives on the host under the store's staging directory, where the payload
/// cannot reach it, and the caller removes it when the payload exits. Per-run,
/// never shared: two foreground runs sharing one memo would answer each
/// other's `stat` with the wrong intent.
pub fn ephemeral_memo_path(store: &podvm_image::Store) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    store
        .root()
        .join("staging")
        .join(format!("memo-{}-{nanos}.tmp", std::process::id()))
}

/// The `LD_PRELOAD` value with podvm's object first.
///
/// A caller-supplied value keeps working behind it. Podvm first means the
/// ownership wall stays cleared whatever else the payload preloads.
fn merge_preload(env: &[String]) -> String {
    match env.iter().rev().find_map(|e| e.strip_prefix("LD_PRELOAD=")) {
        Some(cur) if !cur.is_empty() => format!("{GUEST_PATH} {cur}"),
        _ => GUEST_PATH.to_string(),
    }
}

fn decline(note: &mut String, verb: &str, argv0: &str, why: String) {
    note.push_str(&format!(
        "podvm {verb}: interpose: declined for {argv0:?}: {why}\n"
    ));
}

/// Classify the payload, place the object, and set the environment.
///
/// ⭐ ONE CALL for `run`, `exec` and `create`, so the three verbs cannot
/// disagree about whether a payload is reachable.
///
/// ⛔ T-0710: where the tier loads, the memo descriptor must already be handed
/// in `env` (`PODVM_MEMO_FD`), put there by the caller that opened the host
/// file beside the container record. An entry that would load the tier without
/// one is refused (`Err`) rather than started with no memo: a second process
/// that silently has no ownership record answers a `stat` with the real uid
/// and contradicts the first. A declined tier carries no memo and needs none.
/// Nothing on the host reads the memo to decide anything; it is the payload's
/// own view, not evidence.
pub fn apply(
    verb: &str,
    rootfs: &str,
    argv: &[String],
    env: &mut Vec<String>,
    note: &mut String,
) -> Result<(), String> {
    let Some(first) = argv.first() else {
        return Ok(());
    };
    // ⭐ T-0711: the requested identity, if any. Named in the banner wherever
    // the tier loads, because a `getuid` that answers a record must never
    // read as a kernel answer.
    let identity = env.iter().rev().find_map(|e| {
        let (k, v) = e.split_once('=')?;
        (k == IDENTITY_VAR && !v.is_empty()).then(|| v.to_string())
    });
    let path_dirs = podvm_enter::Plan::path_from(env);
    match classify(rootfs, first, &path_dirs) {
        Reach::Preload { libc, object } => {
            // ⛔ The memo before the preload: without it the tier would start
            // with no ownership record. Refuse, do not decline.
            if memo_fd_of(env).is_none() {
                return Err(format!(
                    "podvm {verb}: interpose: the payload is reachable but no \
                     ownership memo descriptor ({MEMO_FD_VAR}) was handed to it. \
                     podvm refuses rather than starting a second process with \
                     no record that would contradict the first \
                     (TODO/interpose.md T-0710)"
                ));
            }
            match place(rootfs, object) {
                Ok(()) => {
                    let merged = merge_preload(env);
                    // ⭐ Later wins and the earlier is removed, which is
                    // `Plan::env_for`'s own rule: a duplicate name must not reach
                    // `execve`, where glibc and musl resolve it differently.
                    env.retain(|e| e.split('=').next().unwrap_or("") != "LD_PRELOAD");
                    env.push(format!("LD_PRELOAD={merged}"));
                    // ⛔ Said, because T-0506 has no exception for a helpful edit:
                    // podvm wrote a file into somebody's image.
                    note.push_str(&format!(
                        "podvm: interpose: {GUEST_PATH} ({} object) is preloaded \
                         for this payload. refused: podvm WROTE that file into the \
                         image's own rootfs and the payload can see it\n",
                        libc.word()
                    ));
                    if let Some(id) = &identity {
                        note.push_str(&format!(
                            "podvm: interpose: identity faked to {id} (--user): \
                             `getuid` and friends answer the record, not the \
                             kernel, and the run is degraded \
                             (TODO/interpose.md T-0711)\n"
                        ));
                    }
                    // T-0708: the tier emulates four operations it cannot run,
                    // and says so on every load rather than only where one
                    // fires. The live counts ride `inspect` under
                    // `Interpose.Emulated`; this line states the capability.
                    // T-0413: `/proc/self` answers ride the same tally under
                    // `Interpose.Emulated.procfs`, exact or refused, with the
                    // mount-table fixture generated from live topology.
                    note.push_str(
                        "podvm: interpose: emulating mknod, mount, unshare \
                         and clone flag-strip for this payload: each is \
                         counted and `inspect` carries the tally under \
                         Interpose.Emulated (TODO/interpose.md T-0708). \
                         /proc/self/fd, /proc/self/exe and the mount-table \
                         files are emulated where exactly answerable and \
                         refused elsewhere, counted under \
                         Interpose.Emulated.procfs (TODO/complete.md T-0413)\n",
                    );
                }
                Err(e) => decline(
                    note,
                    verb,
                    first,
                    format!("the object could not be placed: {e}"),
                ),
            }
        }
        Reach::Declined(why) => {
            // ⚠ A declined tier carries no memo, so `--user` changes nothing
            // here: saying so keeps the flag from reading as honoured.
            let why = match &identity {
                Some(id) => format!("{why}; --user {id} has no interposed payload to act through"),
                None => why,
            };
            decline(note, verb, first, why);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
