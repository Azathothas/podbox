//! TODO/enter.md T-1652: the choice of the native-static rung.

use super::fields_and_copy::static_pie;
use super::*;
use podvm_enter::userland::Maps;

/// TODO/enter.md T-1652 and T-1648. Where chroot is denied a static payload
/// takes the native-static rung where Syscall User Dispatch arms; where it
/// does not, it refuses at 125, naming the static reason, the host paths
/// and every route. With `--unsafe-host-paths` it enters the memfd family
/// where a memfd executes.
#[test]
fn a_static_payload_takes_the_native_static_rung_or_refuses() {
    let denied = podvm_probe::Findings {
        rows: vec![(
            "chroot(/tmp)",
            podvm_probe::verdict::Outcome::denied(podvm_probe::sys::Errno(1)),
        )],
        identity: podvm_probe::identity::Identity::default(),
        writable: Vec::new(),
        self_exe: String::new(),
    };
    let d = std::env::temp_dir().join(format!("podvm-static-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    std::fs::write(d.join("bin/prog"), static_pie()).unwrap();
    let m = Maps::of_root(&d.to_string_lossy());
    let (argv, env) = (["prog".to_string()], ["PATH=/bin".to_string()]);
    let plain = decide_entry("run", &m, &argv, "/", &env, true, &denied, false, None);
    let opted = decide_entry("run", &m, &argv, "/", &env, true, &denied, true, None);
    for want in [
        "statically linked",
        "resolve on the host",
        "--unsafe-host-paths",
        "chroot(2) is permitted",
        "T-1651",
        "T-1652",
    ] {
        assert!(
            STATIC_HOST_PATHS.contains(want),
            "{want}: {STATIC_HOST_PATHS}"
        );
    }
    if podvm_dispatch::available().is_ok() {
        let u = plain.unwrap().expect("a rung");
        assert_eq!(u.dispatch_exe.as_deref(), Some("/bin/prog"));
        assert!(u.banner.contains("native-static rung"), "{}", u.banner);
    } else {
        assert_eq!(plain.unwrap_err(), podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    {
        if podvm_enter::memfd::memfd_exec().runs() {
            let u = opted.unwrap().expect("the opt-in decides a family");
            assert_eq!(u.family, podvm_enter::userland::Family::Memfd);
            assert!(u.banner.contains("--unsafe-host-paths"), "{}", u.banner);
        } else {
            assert_eq!(opted.unwrap_err(), podvm_image::error::EXIT_RUNTIME_ERROR);
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}
