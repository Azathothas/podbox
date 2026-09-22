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
/// ⛔ `None` is a fact and not an error. A podbox built on a machine with no
/// zig carries no musl object, and the caller's job is then to decline with a
/// named reason rather than to preload something that cannot load.
pub fn object(libc: Libc) -> Option<&'static [u8]> {
    let bytes = match libc {
        Libc::Gnu => GNU,
        Libc::Musl => MUSL,
    };
    (!bytes.is_empty()).then_some(bytes)
}

/// One line naming what this binary carries, for `podbox system info`.
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
pub const GUEST_PATH: &str = "/.podbox/interpose.so";

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
    elf: Option<podbox_enter::abi::Elf>,
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
            elf: podbox_enter::abi::Elf::parse(name, b).ok(),
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
fn is_go(elf: &podbox_enter::abi::Elf) -> bool {
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
    use podbox_enter::abi::{resolve_in, ResolveKind};
    let root = std::path::Path::new(rootfs);
    if !root.is_dir() {
        return Err(format!("{rootfs}: not a directory podbox can read"));
    }
    // ⭐ One walker for both readers: `podbox_enter::abi::resolve_in` owns
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
    let elf = match podbox_enter::abi::Elf::read(&path) {
        Ok(e) => e,
        Err(e) => return Reach::Declined(format!("{e}")),
    };
    classify_elf(argv0, &elf, rootfs, &parsed())
}

fn classify_elf(name: &str, elf: &podbox_enter::abi::Elf, rootfs: &str, objs: &Parsed) -> Reach {
    use podbox_enter::abi::Flavour;
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
                 podbox recognises"
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
            "this podbox binary carries no {} interposer object, so the tier \
             is declined; ./scripts/build-interpose.sh builds them",
            libc.word()
        ));
    };
    let Some(obj) = &slot.elf else {
        return Reach::Declined(format!(
            "the embedded {} object does not parse, so podbox refuses to \
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
    let Some(libc_path) = podbox_enter::abi::libc_beside(rootfs, interp, elf.machine) else {
        return Reach::Declined(format!(
            "{interp} names no C library podbox can find in the image"
        ));
    };
    let libc_elf = match podbox_enter::abi::Elf::read(&libc_path) {
        Ok(l) => l,
        Err(e) => return Reach::Declined(format!("{e}")),
    };
    // ⭐ T-0709's assertion, through T-0706's channel: select and assert,
    // never try. A version predicate the pair fails is a decline that names
    // the version, not a relocation error inside the payload. The check runs
    // against the LINK SET, not the libc alone: the loader resolves every
    // NEEDED library, and an import `libgcc_s.so.1` satisfies is not the
    // libc's to define.
    let mut rest = Vec::new();
    if let Some(dir) = libc_path.rsplit_once('/').map(|(d, _)| d) {
        let sib = format!("{dir}/libgcc_s.so.1");
        if sib != libc_path {
            if let Ok(e) = podbox_enter::abi::Elf::read(&sib) {
                rest.push(e);
            }
        }
    }
    let interp_path = format!(
        "{}/{rel}",
        rootfs.trim_end_matches('/'),
        rel = interp.trim_start_matches('/')
    );
    if interp_path != libc_path {
        if let Ok(e) = podbox_enter::abi::Elf::read(&interp_path) {
            rest.push(e);
        }
    }
    let set = podbox_enter::abi::union(&libc_elf, &rest);
    match podbox_enter::abi::admits(obj, &set) {
        podbox_enter::abi::Verdict::Admitted => Reach::Preload {
            libc,
            object: bytes,
        },
        podbox_enter::abi::Verdict::Refused(why) => Reach::Declined(why),
    }
}

/// Write the object inside the rootfs, where the payload's loader sees it.
///
/// ⭐ Sibling plus rename, so a concurrent reader never sees a half-written
/// object. An absolute path from outside the chroot does not resolve inside
/// it, which is why the write happens here rather than at run time.
pub fn place(rootfs: &str, object_bytes: &[u8]) -> Result<(), String> {
    let dir = format!("{}/.podbox", rootfs.trim_end_matches('/'));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{dir}: {e}"))?;
    let tmp = format!("{dir}/.interpose.so.tmp");
    std::fs::write(&tmp, object_bytes).map_err(|e| format!("{tmp}: {e}"))?;
    std::fs::rename(&tmp, format!("{dir}/interpose.so"))
        .map_err(|e| format!("{dir}/interpose.so: {e}"))?;
    Ok(())
}

/// The variable that carries the requested identity into the payload.
///
/// ⚠ The name is shared with `podbox-interpose`, which reads the same bytes
/// out of its own environ: that crate cannot depend on this one, so the two
/// spellings are one fact in two homes, and
/// `experiments/106-interpose-identity.sh` asserts they agree by driving the
/// behaviour end to end.
pub const IDENTITY_VAR: &str = "PODBOX_IDENTITY";

/// The variable carrying the memo descriptor number into the payload.
///
/// ⭐ One path: `podbox-supervise::table` owns the file, the number and the
/// grammar. This re-exports it so `run` and `exec` read one name; the
/// interposer's own spelling lives in `podbox-interpose` and is asserted by
/// `experiments/105-interpose-ownership.sh`.
pub use podbox_supervise::table::{
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
pub fn ephemeral_memo_path(store: &podbox_image::Store) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    store
        .root()
        .join("staging")
        .join(format!("memo-{}-{nanos}.tmp", std::process::id()))
}

/// The `LD_PRELOAD` value with podbox's object first.
///
/// A caller-supplied value keeps working behind it. Podbox first means the
/// ownership wall stays cleared whatever else the payload preloads.
fn merge_preload(env: &[String]) -> String {
    match env.iter().rev().find_map(|e| e.strip_prefix("LD_PRELOAD=")) {
        Some(cur) if !cur.is_empty() => format!("{GUEST_PATH} {cur}"),
        _ => GUEST_PATH.to_string(),
    }
}

fn decline(note: &mut String, verb: &str, argv0: &str, why: String) {
    note.push_str(&format!(
        "podbox {verb}: interpose: declined for {argv0:?}: {why}\n"
    ));
}

/// Classify the payload, place the object, and set the environment.
///
/// ⭐ ONE CALL for `run`, `exec` and `create`, so the three verbs cannot
/// disagree about whether a payload is reachable.
///
/// ⛔ T-0710: where the tier loads, the memo descriptor must already be handed
/// in `env` (`PODBOX_MEMO_FD`), put there by the caller that opened the host
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
    let path_dirs = podbox_enter::Plan::path_from(env);
    match classify(rootfs, first, &path_dirs) {
        Reach::Preload { libc, object } => {
            // ⛔ The memo before the preload: without it the tier would start
            // with no ownership record. Refuse, do not decline.
            if memo_fd_of(env).is_none() {
                return Err(format!(
                    "podbox {verb}: interpose: the payload is reachable but no \
                     ownership memo descriptor ({MEMO_FD_VAR}) was handed to it. \
                     podbox refuses rather than starting a second process with \
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
                    // podbox wrote a file into somebody's image.
                    note.push_str(&format!(
                        "podbox: interpose: {GUEST_PATH} ({} object) is preloaded \
                         for this payload. ⛔ podbox WROTE that file into the \
                         image's own rootfs and the payload can see it\n",
                        libc.word()
                    ));
                    if let Some(id) = &identity {
                        note.push_str(&format!(
                            "podbox: interpose: identity faked to {id} (--user): \
                             `getuid` and friends answer the record, not the \
                             kernel, and the run is degraded \
                             (TODO/interpose.md T-0711)\n"
                        ));
                    }
                    // T-0708: the tier emulates four operations it cannot run,
                    // and says so on every load rather than only where one
                    // fires. The live counts ride `inspect` under
                    // `Interpose.Emulated`; this line states the capability.
                    note.push_str(
                        "podbox: interpose: emulating mknod, mount, unshare \
                         and clone flag-strip for this payload: each is \
                         counted and `inspect` carries the tally under \
                         Interpose.Emulated (TODO/interpose.md T-0708)\n",
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
mod tests {
    use super::*;

    /// ⭐ **The assertion that would have caught two glibc objects**, which is
    /// the failure `scripts/build-interpose.sh` records as its own reason for
    /// existing: before it asserted `DT_NEEDED`, the build exited 0 having
    /// produced the wrong object twice.
    ///
    /// ⚠ It reads the bytes rather than running `readelf`, so it holds inside
    /// `cargo test` with no external tool. The dynamic string table carries the
    /// `DT_NEEDED` names verbatim, and the two spellings are the discriminator:
    /// musl's libc declares no SONAME, so its object names `libc.so`, and a
    /// glibc one names `libc.so.6`.
    ///
    /// ⛔ **It SKIPS rather than fails where nothing is embedded.** A machine
    /// with no zig builds podbox and must still pass its tests; T-0706's
    /// channel is what reports the absence to a caller at run time.
    #[test]
    fn each_embedded_object_names_its_own_libc() {
        fn has(bytes: &[u8], needle: &str) -> bool {
            bytes.windows(needle.len()).any(|w| w == needle.as_bytes())
        }
        if let Some(gnu) = object(Libc::Gnu) {
            assert!(
                has(gnu, "libc.so.6"),
                "the glibc object does not name libc.so.6, so it is not glibc-linked"
            );
        }
        if let Some(musl) = object(Libc::Musl) {
            assert!(has(musl, "libc.so"), "the musl object names no libc at all");
            // ⚠ An exact spelling, because `libc.so.6` CONTAINS `libc.so`. The
            // same trap `scripts/build-interpose.sh` names, one layer up.
            assert!(
                !has(musl, "libc.so.6"),
                "the musl object names libc.so.6, so it is a SECOND GLIBC \
                 OBJECT and the whole one-object-per-libc requirement is \
                 inverted (TODO/interpose.md T-0702)"
            );
        }
    }

    /// ⚠ The two objects are different artefacts. Embedding the same bytes
    /// twice would pass the check above for the glibc half and read as success.
    #[test]
    fn the_two_embedded_objects_are_not_the_same_bytes() {
        if let (Some(gnu), Some(musl)) = (object(Libc::Gnu), object(Libc::Musl)) {
            assert_ne!(gnu, musl, "one object is embedded under both names");
        }
    }

    /// ⛔ The line a caller reads has to say which state this binary is in,
    /// including the state where it carries nothing.
    #[test]
    fn the_carried_line_names_the_state_it_is_in() {
        let line = carried();
        assert!(line.starts_with("interposer: "), "{line}");
        match (object(Libc::Gnu).is_some(), object(Libc::Musl).is_some()) {
            (true, true) => assert!(line.contains("glibc and musl"), "{line}"),
            (false, false) => assert!(line.contains("declined"), "{line}"),
            _ => assert!(line.contains("declined"), "{line}"),
        }
    }

    // ------------------------------------------------- T-0706 classification

    use podbox_enter::abi::Elf;

    const X86_64: u16 = 62;
    const AARCH64: u16 = 183;

    fn elf(interp: Option<&str>, sections: &[&str], machine: u16) -> Elf {
        Elf {
            path: "/payload".to_string(),
            machine,
            interp: interp.map(str::to_string),
            needed: Vec::new(),
            soname: None,
            sections: sections.iter().map(|s| (*s).to_string()).collect(),
            defined: Vec::new(),
            imported: Vec::new(),
            declares: Vec::new(),
        }
    }

    fn no_objects() -> Parsed {
        let empty = || Slot {
            bytes: None,
            elf: None,
        };
        Parsed {
            gnu: empty(),
            musl: empty(),
        }
    }

    fn declined_name(r: Reach) -> String {
        let Reach::Declined(why) = r else {
            panic!("a decline was expected and the payload was admitted");
        };
        why
    }

    /// ⭐ Row 2 of T-0706's table: no `PT_INTERP` means no loader, and nothing
    /// reads the variable. The absence of an object cannot matter here, so no
    /// embedded bytes are needed and none are read.
    #[test]
    fn a_static_payload_is_declined_by_name() {
        let why = declined_name(classify_elf(
            "/podbox",
            &elf(None, &[], X86_64),
            "/rootfs",
            &no_objects(),
        ));
        assert!(why.contains("statically linked"), "{why}");
        assert!(why.contains("PT_INTERP"), "{why}");
    }

    /// ⭐ Row 3: Go markers outrank a present interpreter, because a Go
    /// payload is unreachable regardless of linkage. T-1209 refuses an
    /// invented image reference, so this arm is proved here and not by a run.
    #[test]
    fn go_markers_outrank_a_present_interpreter() {
        for marker in [
            ".note.go.buildid",
            ".gosymtab",
            ".gopclntab",
            ".go.buildinfo",
        ] {
            let why = declined_name(classify_elf(
                "/server",
                &elf(Some("/lib64/ld-linux-x86-64.so.2"), &[marker], X86_64),
                "/rootfs",
                &no_objects(),
            ));
            assert!(why.contains("Go"), "{marker}: {why}");
        }
    }

    /// An interpreter podbox does not recognise gets a decline, never a
    /// guess at an object.
    #[test]
    fn an_unknown_interpreter_is_declined() {
        let why = declined_name(classify_elf(
            "/p",
            &elf(Some("/opt/weird/loader"), &[], X86_64),
            "/rootfs",
            &no_objects(),
        ));
        assert!(why.contains("no libc"), "{why}");
    }

    /// A binary built without the objects declines naming what is missing,
    /// rather than preloading nothing.
    #[test]
    fn a_binary_without_objects_declines_naming_the_libc() {
        let why = declined_name(classify_elf(
            "/bin/sh",
            &elf(Some("/lib64/ld-linux-x86-64.so.2"), &[], X86_64),
            "/rootfs",
            &no_objects(),
        ));
        assert!(why.contains("glibc"), "{why}");
        assert!(why.contains("build-interpose.sh"), "{why}");
    }

    /// ⛔ A foreign payload under binfmt gets a decline, never an object its
    /// loader cannot read.
    #[test]
    fn another_architecture_is_declined() {
        let objs = Parsed {
            gnu: Slot {
                bytes: Some(b"present but unparsed"),
                elf: Some(elf(None, &[], AARCH64)),
            },
            musl: Slot {
                bytes: None,
                elf: None,
            },
        };
        let why = declined_name(classify_elf(
            "/bin/sh",
            &elf(Some("/lib64/ld-linux-x86-64.so.2"), &[], X86_64),
            "/rootfs",
            &objs,
        ));
        assert!(why.contains("0x"), "{why}");
    }

    /// A present object that does not parse is refused with that reason,
    /// rather than preloaded unchecked or reported as absent.
    #[test]
    fn an_unparsable_object_is_refused_as_unparsable() {
        let objs = Parsed {
            gnu: Slot {
                bytes: Some(b"present but unparsed"),
                elf: None,
            },
            musl: Slot {
                bytes: None,
                elf: None,
            },
        };
        let why = declined_name(classify_elf(
            "/bin/sh",
            &elf(Some("/lib64/ld-linux-x86-64.so.2"), &[], X86_64),
            "/rootfs",
            &objs,
        ));
        assert!(why.contains("does not parse"), "{why}");
    }

    /// A payload that is not an ELF file is declined with the reader's
    /// reason. The tier selects on `PT_INTERP`, which only ELF carries, and
    /// declining is the safe direction: a wrong preload fails the payload.
    #[test]
    fn a_script_is_declined_with_the_readers_reason() {
        let d = std::env::temp_dir().join(format!("podbox-cls-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("bin")).unwrap();
        std::fs::write(d.join("bin/tool.sh"), b"#!/bin/sh\necho hi\n").unwrap();
        let root = d.to_string_lossy().to_string();
        let why = declined_name(classify(&root, "tool.sh", &["/bin".to_string()]));
        assert!(why.contains("not an ELF file"), "{why}");
        // ⛔ And the decline wrote nothing into the image.
        assert!(!d.join(".podbox").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A real ELF file is resolved along `path_dirs` exactly as production
    /// passes them: with leading slashes, several entries, the hit not
    /// first. This mirrors a `run` payload of a bare name.
    #[test]
    fn resolve_finds_a_real_elf_along_path_dirs() {
        let me = std::env::current_exe().unwrap();
        let d = std::env::temp_dir().join(format!("podbox-rsv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("bin")).unwrap();
        std::fs::copy(&me, d.join("bin/prog")).unwrap();
        let root = d.to_string_lossy().to_string();
        let dirs = ["/usr/local/sbin", "/usr/bin", "/bin"]
            .iter()
            .map(|s| (*s).to_string())
            .collect::<Vec<_>>();
        let got = resolve(&root, "prog", &dirs).unwrap();
        assert!(got.ends_with("bin/prog"), "{got}");
        // ⚠ And the absolute branch against the same tree.
        let got = resolve(&root, "/bin/prog", &dirs).unwrap();
        assert!(got.ends_with("bin/prog"), "{got}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A payload the image does not hold is declined, and the later `execve`
    /// still reports it as not found: the decline does not swallow the 127.
    #[test]
    fn a_missing_payload_is_declined() {
        let d = std::env::temp_dir().join(format!("podbox-clm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let root = d.to_string_lossy().to_string();
        let why = declined_name(classify(&root, "absent", &["/bin".to_string()]));
        assert!(why.contains("names no file"), "{why}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// ⭐ The alpine shape, and the defect that declined every dynamic
    /// payload: `/bin/sh` points at the absolute `/bin/busybox`, which names
    /// the host's file from outside and the image's from inside. The link
    /// resolves under the rootfs, to the file the guest kernel would run.
    #[test]
    fn an_absolute_link_resolves_under_the_rootfs() {
        let me = std::env::current_exe().unwrap();
        let d = std::env::temp_dir().join(format!("podbox-cla-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("bin")).unwrap();
        std::fs::copy(&me, d.join("bin/busybox")).unwrap();
        std::os::unix::fs::symlink("/bin/busybox", d.join("bin/sh")).unwrap();
        let root = d.to_string_lossy().to_string();
        // ⚠ The answer is the target, not the link: it is the file the guest
        // kernel would run, and reading it answers about that file.
        let got = resolve(&root, "sh", &["/sbin".to_string(), "/bin".to_string()]).unwrap();
        assert!(got.ends_with("bin/busybox"), "{got}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// ⛔ `..` that escapes the root is refused rather than followed: the
    /// path it names is the host's, and classifying it answers about the
    /// wrong machine.
    #[test]
    fn a_payload_escaping_the_image_is_refused() {
        let d = std::env::temp_dir().join(format!("podbox-clx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("bin")).unwrap();
        std::os::unix::fs::symlink("../../outside", d.join("bin/evil")).unwrap();
        let root = d.to_string_lossy().to_string();
        let why = declined_name(classify(&root, "evil", &["/bin".to_string()]));
        assert!(why.contains("escapes the image"), "{why}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A link loop is refused with that reason, rather than followed until
    /// the process runs out of anything.
    #[test]
    fn a_link_loop_is_refused() {
        let d = std::env::temp_dir().join(format!("podbox-cll-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("bin")).unwrap();
        std::os::unix::fs::symlink("b", d.join("bin/a")).unwrap();
        std::os::unix::fs::symlink("a", d.join("bin/b")).unwrap();
        let root = d.to_string_lossy().to_string();
        let why = declined_name(classify(&root, "a", &["/bin".to_string()]));
        assert!(why.contains("too many levels of symlinks"), "{why}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// ⭐ The channel T-0706's `Prove` greps for. A decline that does not
    /// carry this line passes a tree where nothing is declined.
    #[test]
    fn the_decline_line_carries_the_channel() {
        let mut note = String::new();
        decline(&mut note, "run", "/podbox", "static".to_string());
        assert!(note.contains("interpose: declined"), "{note}");
        assert!(note.contains("/podbox"), "{note}");
    }

    // ---------------------------------------------------- T-0702 placement

    /// The object lands at the guest path, whole, and the staging file is
    /// gone. A second placement overwrites rather than stacking.
    #[test]
    fn placement_writes_the_object_where_the_payload_sees_it() {
        let d = std::env::temp_dir().join(format!("podbox-plc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let root = d.to_string_lossy().to_string();
        place(&root, b"first").unwrap();
        assert_eq!(
            std::fs::read(d.join(".podbox/interpose.so")).unwrap(),
            b"first"
        );
        assert!(!d.join(".podbox/.interpose.so.tmp").exists());
        place(&root, b"second").unwrap();
        assert_eq!(
            std::fs::read(d.join(".podbox/interpose.so")).unwrap(),
            b"second"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Podbox first, so the ownership wall stays cleared whatever else the
    /// payload preloads. An absent caller value leaves the path bare.
    #[test]
    fn a_caller_preload_survives_behind_podboxs() {
        assert_eq!(merge_preload(&[]), "/.podbox/interpose.so");
        assert_eq!(
            merge_preload(&["LD_PRELOAD=/mine.so".to_string()]),
            "/.podbox/interpose.so /mine.so"
        );
        assert_eq!(
            merge_preload(&["LD_PRELOAD=".to_string()]),
            "/.podbox/interpose.so"
        );
    }

    /// ⭐ T-0710: the descriptor grammar the host and the interposer share.
    /// Decimal digits only; anything else is no descriptor rather than a guess.
    #[test]
    fn the_memo_descriptor_parses_as_decimal_or_not_at_all() {
        assert_eq!(memo_fd_of(&["PODBOX_MEMO_FD=17".to_string()]), Some(17));
        assert_eq!(memo_fd_of(&[]), None);
        assert_eq!(memo_fd_of(&["PODBOX_MEMO_FD=".to_string()]), None);
        assert_eq!(memo_fd_of(&["PODBOX_MEMO_FD=nobody".to_string()]), None);
        assert_eq!(memo_fd_of(&["PODBOX_MEMO_FD=-1".to_string()]), None);
        // ⚠ The last wins, as every other `*_for` in this tree resolves it:
        // the caller wins over the image.
        assert_eq!(
            memo_fd_of(&[
                "PODBOX_MEMO_FD=17".to_string(),
                "PODBOX_MEMO_FD=18".to_string()
            ]),
            Some(18)
        );
    }

    /// ⛔ An entry that would load the tier without a memo refuses rather than
    /// starting with no record. A declined tier needs none.
    #[test]
    fn a_reachable_payload_without_a_memo_is_refused_not_declined() {
        // A missing payload is declined (no memo needed); the refusal arm
        // needs a real rootfs with a real ELF, which this unit does not build.
        // What this pins is the grammar above, which `apply` reads: without it
        // the refusal cannot fire. The end-to-end refusal is asserted by
        // `experiments/105-interpose-ownership.sh` check G's harness, which
        // hands fd 17 where the tier loads.
        assert!(memo_fd_of(&[]).is_none());
        assert_eq!(memo_fd_env(), "PODBOX_MEMO_FD=17");
        assert_eq!(MEMO_CHILD_FD, 17);
    }
}
