//! `TOOL.md` section 6.5: the chroot entry sequence, and the other entries.
//!
//! `pod-enter` (PodUtils/podcrates) does the entry: the chroot sequence,
//! the namespace rung, the userland entry, the launch ladder, the FUSE
//! server, the device hand-over and the ELF reader. This crate gives
//! podvm's names to it ([`NAMES`]: the variables, the cache marker, the
//! memfd label), keeps docker's 125 from `podvm-probe`'s `exit`, and keeps
//! each function with podvm's signature where the crate takes the names.
//!
//! A runtime error of the entry is a `pod_core::Failure` inside
//! [`Error::Runtime`]; [`ExitCode::exit_code`] maps each error to docker's
//! code, and [`runtime`] makes one from podvm's text.

pub use pod_enter::{
    abi, binfmt, check_workdir, chroot_landed, entered_rung, entered_word, exit_status, fuse,
    ns_setup_step, ns_tmp_target, plan, run, run_ladder, run_namespace, run_selected, run_userland,
    spawn, spawn_ladder, spawn_namespace, spawn_selected, spawn_userland, status, Bounded, Child,
    Error, Fds, Names, NsMount, Plan, Program, Result, RootDir, ENTERED_RUNG,
};

// docker's codes, from the one file that holds them (TODO/cli.md T-0802).
pub use podvm_probe::exit::{EXIT_CANNOT_INVOKE, EXIT_NOT_FOUND, EXIT_RUNTIME_ERROR};

/// podvm's names: the variables that the payload and the interposer read,
/// the cache marker on disk, and the memfd label.
pub const NAMES: Names = Names {
    product: "podvm",
    mode_request: "PODVM_MODE",
    mode_active: "PODVM_ACTIVE_MODE",
    cache: "PODVM_CACHE",
    maps: "PODVM_MAPS",
    lib_path: "PODVM_LIB_PATH",
    device_spec: "PODVM_DEVICE_SPEC",
    device_fds: "PODVM_DEVICE_FDS",
    cache_marker: ".podvm-cache-complete",
    memfd_label: "podvm-payload",
    // The guest prefixes that `podvm-interpose` never rewrites
    // (`map::longest`): `Maps::new` refuses a volume under one by name.
    reserved: &["/proc", "/.podvm"],
};

/// A runtime error with podvm's text. Exits 125.
pub fn runtime(text: impl Into<String>) -> Error {
    Error::runtime(pod_core::Class::Unavailable, text)
}

/// docker's exit code of an entry error: 126 or 127 for a payload that
/// did not start, 125 for everything podvm could not do.
pub trait ExitCode {
    fn exit_code(&self) -> i32;
}

impl ExitCode for Error {
    fn exit_code(&self) -> i32 {
        self.code_or(EXIT_RUNTIME_ERROR)
    }
}

/// The namespace fallback line, named `podvm:`.
pub fn ns_fallback_line(cause: &str) -> String {
    pod_enter::ns_fallback_line(&NAMES, cause)
}

/// The launch ladder with podvm's names.
pub mod ladder {
    pub use pod_enter::ladder::*;

    use super::NAMES;
    use crate::memfd::MemfdRefusal;

    /// The environment name carrying the payload's resolved guest path,
    /// which the interposer answers `/proc/self/exe` with
    /// (`podvm-interpose`'s `procfs.rs` names it by value).
    pub const GUEST_EXE_VAR: &str = "PODVM_GUEST_EXE";

    /// Parse a `PODVM_MODE` request.
    pub fn parse_mode(s: &str) -> crate::Result<Mode> {
        Mode::parse(&NAMES, s)
    }

    /// Choose the rung, naming `PODVM_MODE` and `PODVM_CACHE`.
    pub fn choose(
        forced: Option<Mode>,
        avail: &Availability,
        memfd_why: Option<&MemfdRefusal>,
    ) -> crate::Result<Mode> {
        pod_enter::ladder::choose(&NAMES, forced, avail, memfd_why)
    }
}

/// The memfd launch with podvm's label.
pub mod memfd {
    pub use pod_enter::memfd::*;

    /// Write `bytes` to a new memfd labelled `podvm-payload`.
    pub fn stage(bytes: &[u8]) -> crate::Result<i64> {
        pod_enter::memfd::stage(&super::NAMES, bytes)
    }

    /// Copy the payload `file` into a new memfd labelled `podvm-payload`,
    /// in chunks of [`STAGE_CHUNK`]: no step holds the whole file.
    pub fn stage_file(file: &std::fs::File) -> crate::Result<i64> {
        pod_enter::memfd::stage_file(&super::NAMES, file)
    }

    /// The memfd probe, worded for podvm.
    pub mod probe {
        pub use pod_enter::memfd::probe::*;

        /// The user text and the fix line for one measurement.
        pub fn answer(m: &MemfdExec) -> Answer {
            pod_enter::memfd::probe::answer(&crate::NAMES, m)
        }
    }
}

/// `--device` with podvm's variables.
pub mod device {
    pub use pod_enter::device::*;

    use super::NAMES;

    /// The device spec, owned by the CLI and the container record.
    pub const SPEC_VAR: &str = NAMES.device_spec;
    /// The serve table, built beside the descriptors by the entering process.
    pub const SERVE_VAR: &str = NAMES.device_fds;

    pub fn specs_of(env: &[String]) -> crate::Result<Vec<Mapping>> {
        pod_enter::device::specs_of(&NAMES, env)
    }

    pub fn push_spec(env: &mut Vec<String>, specs: &[Mapping]) {
        pod_enter::device::push_spec(&NAMES, env, specs)
    }

    pub fn push_serve(env: &mut Vec<String>, opened: &Opened) {
        pod_enter::device::push_serve(&NAMES, env, opened)
    }

    pub fn banner_line(m: &Mapping) -> String {
        pod_enter::device::banner_line(&NAMES, m)
    }
}

/// The userland entry with podvm's table variable.
pub mod userland {
    pub use pod_enter::userland::*;

    /// The variable carrying the root-anchoring table, TODO/enter.md
    /// T-1407: the contract with `podvm-interpose`'s `map::MAPS_VAR`.
    pub const MAPS_VAR: &str = super::NAMES.maps;

    /// The variable carrying the image's library directories to the
    /// interposer, TODO/interpose.md T-1646: the contract with
    /// `podvm-interpose`'s `exec::launch::LIB_PATH_VAR`.
    pub const LIB_PATH_VAR: &str = super::NAMES.lib_path;

    /// The environment of a payload that runs through the image's loader,
    /// with podvm's variable for the library path of the interposer.
    pub fn host_env(
        rootfs: &str,
        guest_preload: &str,
        env: &[String],
        lib_dirs: &[String],
    ) -> (Vec<String>, Option<String>) {
        pod_enter::userland::host_env(&super::NAMES, rootfs, guest_preload, env, lib_dirs)
    }

    /// The volume pairs of the table that `env` carries: each pair but the
    /// root pair, TODO/enter.md T-1653. A container record carries the
    /// table of its entry, and `exec` takes the volumes of the container
    /// from it, as docker's `exec` sees the mounts of the container.
    pub fn volumes_of(env: &[String]) -> Vec<Volume> {
        let Some(table) = env
            .iter()
            .find_map(|e| e.strip_prefix(MAPS_VAR).and_then(|r| r.strip_prefix('=')))
        else {
            return Vec::new();
        };
        table
            .split(',')
            .filter_map(|p| p.split_once(':'))
            .filter(|(g, _)| *g != "/")
            .map(|(g, h)| Volume {
                guest: g.to_string(),
                host: h.to_string(),
            })
            .collect()
    }
}

/// The directory rungs' staging with podvm's cache marker.
pub mod stage {
    pub use pod_enter::stage::*;

    /// Stage a persistent per-image root under the store's cache base,
    /// marked `.podvm-cache-complete`.
    pub fn stage_cache(
        store_root: &std::path::Path,
        digest: &str,
        rootfs: &std::path::Path,
    ) -> Result<std::path::PathBuf, Refusal> {
        pod_enter::stage::stage_cache(&super::NAMES, store_root, digest, rootfs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// podvm's names reach the payload unchanged: the request is scrubbed
    /// and the entered rung is reported under `PODVM_ACTIVE_MODE`.
    #[test]
    fn podvm_names_reach_the_payload() {
        let mut env = vec!["PODVM_MODE=memfd".to_string()];
        Plan::set_active_mode(&NAMES, &mut env, "chroot");
        assert_eq!(env, vec!["PODVM_ACTIVE_MODE=chroot".to_string()]);
        assert_eq!(userland::MAPS_VAR, "PODVM_MAPS");
        assert_eq!(userland::LIB_PATH_VAR, "PODVM_LIB_PATH");
        assert_eq!(device::SPEC_VAR, "PODVM_DEVICE_SPEC");
    }

    /// `host_env` gives the image's library directories to the interposer
    /// under podvm's variable, and drops the caller's value of it.
    #[test]
    fn host_env_sets_podvms_library_path() {
        let env = ["PODVM_LIB_PATH=/caller".to_string()];
        let dirs = ["/store/rootfs/lib".to_string()];
        let (got, _) = userland::host_env("/store/rootfs", "/.podvm/interpose.so", &env, &dirs);
        assert!(
            got.contains(&"PODVM_LIB_PATH=/store/rootfs/lib".to_string()),
            "{got:?}"
        );
        assert!(
            !got.contains(&"PODVM_LIB_PATH=/caller".to_string()),
            "{got:?}"
        );
    }

    fn vol(host: &str, guest: &str) -> userland::Volume {
        userland::Volume {
            guest: guest.to_string(),
            host: host.to_string(),
        }
    }

    /// The table of `maps` as `PODVM_MAPS` spells it.
    fn table(maps: &userland::Maps) -> String {
        userland::userland_maps_table(&NAMES, maps).unwrap()
    }

    /// TODO/enter.md T-1653. A volume is one `GUEST:HOST` pair after the
    /// root pair, and `volumes_of` reads the same pairs back from the
    /// environment that carries the table, without the root pair.
    #[test]
    fn a_volume_is_a_pair_beside_the_root_pair() {
        let vs = [vol("/host/dir", "/data/"), vol("/", "/hostroot")];
        let maps = userland::Maps::new(&NAMES, "/store/rootfs/", &vs).unwrap();
        let t = table(&maps);
        assert_eq!(t, "/:/store/rootfs,/data:/host/dir,/hostroot:/");
        let env = [format!("PODVM_MAPS={t}"), "PATH=/bin".to_string()];
        assert_eq!(
            userland::volumes_of(&env),
            vec![vol("/host/dir", "/data"), vol("/", "/hostroot")]
        );
        // --unsafe-host-paths: no root pair, the volumes stay.
        let maps = userland::Maps::new(&NAMES, "/store/rootfs", &vs[..1]).unwrap();
        let t = userland::userland_volume_table(&NAMES, &maps).unwrap();
        assert_eq!(t, "/data:/host/dir");
        assert!(userland::volumes_of(&["PATH=/bin".to_string()]).is_empty());
    }

    /// TODO/enter.md T-1653. Each spelling that the table cannot carry, or
    /// that the interposer would never match, is refused by name.
    #[test]
    fn a_volume_the_table_cannot_spell_is_refused() {
        let refused = |host: &str, guest: &str, want: &str| {
            let e = userland::Maps::new(&NAMES, "/r", &[vol(host, guest)])
                .and_then(|m| userland::userland_maps_table(&NAMES, &m))
                .unwrap_err();
            assert!(e.contains(want), "{host}:{guest}: {e}");
        };
        refused("/a,b", "/data", "the host side");
        refused("/a", "/d,e", "the guest side");
        refused("/a", "data", "must be an absolute path");
        refused("/a", "/", "cannot replace the guest root");
        refused("/a", "/x/../etc", "normal path");
        refused("/a", "/x//y", "normal path");
        refused("/a", "/proc/self", "never rewrites a path under /proc");
        refused("/a", "/.podvm", "never rewrites a path under /.podvm");
        let two = [vol("/a", "/data"), vol("/b", "/data/")];
        let e = userland::Maps::new(&NAMES, "/r", &two).unwrap_err();
        assert!(e.contains("a second volume at the guest path /data"), "{e}");
        let many: Vec<_> = (0..userland::MAX_MAP_PAIRS)
            .map(|i| vol("/a", &format!("/v{i}")))
            .collect();
        let e = userland::Maps::new(&NAMES, "/r", &many).unwrap_err();
        assert!(e.contains("over the 31"), "{e}");
        let maps = userland::Maps::new(&NAMES, "/r", &many[1..]).unwrap();
        assert!(userland::userland_maps_table(&NAMES, &maps).is_ok());
    }

    /// TODO/enter.md T-1653. A working directory and a first program in a
    /// volume resolve to the files of the volume, not to the files of the
    /// image at the same guest path.
    #[test]
    fn a_guest_path_in_a_volume_resolves_in_the_volume() {
        use std::os::unix::fs::PermissionsExt;
        let d = std::env::temp_dir().join(format!("podvm-enter-vol-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (root, host) = (d.join("rootfs"), d.join("host"));
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::create_dir_all(host.join("sub")).unwrap();
        let prog = host.join("prog");
        std::fs::write(&prog, b"\x7fELF").unwrap();
        std::fs::set_permissions(&prog, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (r, h) = (root.to_str().unwrap(), host.to_str().unwrap());
        let maps = userland::Maps::new(&NAMES, r, &[vol(h, "/data")]).unwrap();
        // The image has no /data/sub; the volume has it.
        assert!(check_workdir(r, "/data/sub", "img").is_err());
        check_workdir(&maps, "/data/sub", "img").unwrap();
        check_workdir(&maps, "/data", "img").unwrap();
        // TODO/enter.md T-1716: a missing directory inside a volume names
        // the volume, not the image.
        let e = check_workdir(&maps, "/data/none", "img")
            .unwrap_err()
            .to_string();
        assert!(
            e.contains(&format!(
                "in the volume {h}:/data is not a directory in the volume"
            )),
            "{e}"
        );
        let e = check_workdir(&maps, "/none", "img")
            .unwrap_err()
            .to_string();
        assert!(e.contains("is not a directory in the image"), "{e}");
        let path = ["/bin".to_string()];
        assert!(ladder::resolve_payload(r, "/data/prog", &path).is_err());
        let got = ladder::resolve_payload(&maps, "/data/prog", &path).unwrap();
        assert_eq!(std::path::Path::new(&got), prog.as_path());
        assert_eq!(
            ladder::guest_exe(&maps, "/data/prog", &path).as_deref(),
            Some("/data/prog")
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A runtime error exits 125; a payload that did not start 126 or 127.
    #[test]
    fn errors_map_to_dockers_codes() {
        assert_eq!(runtime("x").exit_code(), EXIT_RUNTIME_ERROR);
        assert_eq!(
            Error::CannotInvoke("x".into()).exit_code(),
            EXIT_CANNOT_INVOKE
        );
        assert_eq!(Error::NotFound("x".into()).exit_code(), EXIT_NOT_FOUND);
    }
}
