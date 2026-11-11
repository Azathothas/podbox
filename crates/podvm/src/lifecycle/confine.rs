//! TODO/enter.md T-1658: the confine layer of `run --podvm-confine=SPEC`.
//!
//! The layer runs on a rung with a chroot (the namespace rung and the
//! chroot rung), where the paths of the spec are guest paths and
//! `pod-enter` opens each under the root before the fork, and on the loader
//! family of the userland rung, where each guest path becomes its host path
//! through the table of the interposer (the rootfs and each `-v`). The
//! record keeps the SPEC, so the launcher of `run -d` and each `exec` apply
//! it too. Each other case refuses by name, before anything runs: a payload
//! that asked for the layer never runs without it.

use podvm_enter::confine::{self, Ask, Spec};
use podvm_probe::select::Rung;

fn refuse(verb: &str, why: String) -> i32 {
    eprintln!("podvm {verb}: --podvm-confine: {why} (TODO/enter.md T-1658)");
    podvm_image::error::EXIT_RUNTIME_ERROR
}

/// The spec of the run, or None without `--podvm-confine`.
pub fn decide(
    verb: &str,
    ask: Option<&Ask>,
    entered: Rung,
    rootfs: &str,
    userland: Option<&super::Userland>,
) -> Result<Option<Spec>, i32> {
    let Some(ask) = ask else {
        return Ok(None);
    };
    // The loader family: the image's loader runs the payload, with no
    // dispatcher of podvm and no emulator in its process.
    let loader =
        userland.filter(|u| u.dispatch_exe.is_none() && u.emulator.is_none() && !u.syscall);
    if let Err(why) = check(ask, entered, loader.is_some(), confine::abi()) {
        return Err(refuse(verb, why));
    }
    Ok(Some(match loader {
        Some(u) => confine::host_spec(ask, &u.maps),
        None => confine::spec_in(ask, std::path::Path::new(rootfs)),
    }))
}

/// The refusals of the layer, as a pure function of its inputs.
pub fn check(ask: &Ask, entered: Rung, loader: bool, abi: Option<u32>) -> Result<(), String> {
    let carried = confine::rung_carries(entered) || (entered == Rung::Userland && loader);
    if !carried {
        let which = if entered == Rung::Userland {
            "the userland rung with podvm's dispatcher or an emulator (the native-static \
             rung, the syscall or the emulate tier)"
                .to_string()
        } else {
            format!("the {} rung", entered.word())
        };
        return Err(format!(
            "the layer runs on a rung with a chroot (namespace or chroot) and on the \
             loader family of the userland rung, and this run enters {which}, so podvm \
             refuses: a payload that asks for the layer never runs without it"
        ));
    }
    match abi {
        None => Err(
            "this kernel has no Landlock, so podvm refuses: a payload that asks for \
                     the layer never runs without it"
                .to_string(),
        ),
        Some(a) if a < confine::MIN_ABI => Err(format!(
            "this kernel has Landlock ABI {a}, and the layer needs ABI {} or later",
            confine::MIN_ABI
        )),
        Some(a) if ask.connect.is_some() && a < confine::NET_ABI => Err(format!(
            "`connect` needs Landlock ABI {} (TCP rules), and this kernel has ABI {a}",
            confine::NET_ABI
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_case_with_no_mechanism_refuses_by_name() {
        let ask = confine::parse("read=/workspace;connect=443").unwrap();
        assert!(check(&ask, Rung::Namespace, false, Some(7)).is_ok());
        assert!(check(&ask, Rung::Userland, true, Some(7)).is_ok());
        let e = check(&ask, Rung::Userland, false, Some(7)).unwrap_err();
        assert!(e.contains("dispatcher or an emulator"), "{e}");
        let e = check(&ask, Rung::Supervise, false, Some(7)).unwrap_err();
        assert!(e.contains("supervise rung"), "{e}");
        let e = check(&ask, Rung::Namespace, false, None).unwrap_err();
        assert!(e.contains("no Landlock"), "{e}");
        let e = check(&ask, Rung::Chroot, false, Some(3)).unwrap_err();
        assert!(e.contains("ABI 4"), "{e}");
        let no_tcp = confine::parse("read=/workspace").unwrap();
        assert!(check(&no_tcp, Rung::Chroot, false, Some(3)).is_ok());
        assert!(check(&no_tcp, Rung::Chroot, false, Some(2)).is_err());
    }
}
