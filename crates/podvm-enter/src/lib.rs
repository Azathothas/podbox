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

    /// The `PODVM_MAPS` table for a userland run.
    pub fn userland_maps_table(rootfs: &str) -> Result<String, String> {
        pod_enter::userland::userland_maps_table(&super::NAMES, rootfs)
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
