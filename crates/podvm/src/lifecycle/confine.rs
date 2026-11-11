//! TODO/enter.md T-1658: the confine layer of `run --podvm-confine=SPEC`.
//!
//! The layer runs on a rung with a chroot (the namespace rung and the
//! chroot rung), where the paths of the spec are guest paths and
//! `pod-enter` opens each under the root before the fork. The record keeps
//! the SPEC, so the launcher of `run -d` and each `exec` apply it too. Each other case
//! refuses by name, before anything runs: a payload that asked for the
//! layer never runs without it.

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
) -> Result<Option<Spec>, i32> {
    let Some(ask) = ask else {
        return Ok(None);
    };
    if let Err(why) = check(ask, entered, confine::abi()) {
        return Err(refuse(verb, why));
    }
    Ok(Some(confine::spec_in(ask, std::path::Path::new(rootfs))))
}

/// The refusals of the layer, as a pure function of its inputs.
pub fn check(ask: &Ask, entered: Rung, abi: Option<u32>) -> Result<(), String> {
    if !confine::rung_carries(entered) {
        return Err(format!(
            "the layer needs a rung with a chroot (namespace or chroot), and this run \
             enters the {} rung, so podvm refuses: a payload that asks for the layer \
             never runs without it",
            entered.word()
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
        assert!(check(&ask, Rung::Namespace, Some(7)).is_ok());
        let e = check(&ask, Rung::Userland, Some(7)).unwrap_err();
        assert!(e.contains("userland rung"), "{e}");
        let e = check(&ask, Rung::Namespace, None).unwrap_err();
        assert!(e.contains("no Landlock"), "{e}");
        let e = check(&ask, Rung::Chroot, Some(3)).unwrap_err();
        assert!(e.contains("ABI 4"), "{e}");
        let no_tcp = confine::parse("read=/workspace").unwrap();
        assert!(check(&no_tcp, Rung::Chroot, Some(3)).is_ok());
        assert!(check(&no_tcp, Rung::Chroot, Some(2)).is_err());
    }
}
