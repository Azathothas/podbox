//! TODO/cli.md T-1706. A payload that dies on `SIGXFSZ` gets a reason.
//!
//! The kernel sends `SIGXFSZ` when a write or an `ftruncate` passes the soft
//! `RLIMIT_FSIZE`. `pod-enter` prints only "the payload died on SIGXFSZ".
//! The .NET runtime maps a memfd of 2 TiB at start when W^X is on (M1 of
//! the podrunner design review), so a .NET payload dies so under each soft
//! limit below 2 TiB. podvm names the limit that the payload had and, for a
//! .NET payload, `DOTNET_EnableWriteXorExecute=0`. It does not raise or
//! drop the limit.

use std::io::Write;

/// `128 + SIGXFSZ`.
pub const SIGXFSZ_EXIT: i32 = 128 + 25;

/// The soft `RLIMIT_FSIZE` of the payload: a `--ulimit fsize` of the plan,
/// else podvm's own, which the child inherits. `None` where it cannot be
/// read.
pub fn fsize_soft(limits: &[podvm_enter::Limit]) -> Option<u64> {
    if let Some(l) = limits
        .iter()
        .find(|l| l.resource == podvm_enter::Resource::Fsize)
    {
        return Some(l.soft);
    }
    podvm_probe::sys::prlimit(podvm_probe::sys::RLIMIT_FSIZE)
        .ok()
        .map(|(cur, _)| cur)
}

/// Is the payload a .NET program: a `DOTNET_` variable, or a `.dll` beside
/// the program in the image (`guest_exe` is its guest path, `rootfs` the
/// host path of the image root)?
pub fn is_dotnet(env: &[String], rootfs: &str, guest_exe: Option<&str>) -> bool {
    env.iter().any(|e| e.starts_with("DOTNET_"))
        || guest_exe.is_some_and(|g| {
            let p = format!("{}{g}.dll", rootfs.trim_end_matches('/'));
            std::path::Path::new(&p).is_file()
        })
}

/// The lines for an exit `code`; nothing for a code that is not
/// `SIGXFSZ`.
pub fn lines(code: i32, soft: Option<u64>, dotnet: bool) -> Vec<String> {
    if code != SIGXFSZ_EXIT {
        return Vec::new();
    }
    let limit = match soft {
        Some(u64::MAX) | None => "the soft RLIMIT_FSIZE".to_string(),
        Some(n) => format!("the soft RLIMIT_FSIZE of {n} bytes"),
    };
    let mut v = vec![format!(
        "podvm run: SIGXFSZ: a write or an ftruncate of the payload passed {limit} \
         (`ulimit -f` of the caller, or --ulimit fsize); podvm does not raise it \
         (TODO/cli.md T-1706)"
    )];
    if dotnet {
        v.push(
            "podvm run: a .NET runtime with W^X on maps a memfd of 2 TiB at start, \
             so it dies so under a smaller limit; -e DOTNET_EnableWriteXorExecute=0 \
             turns that map off (TODO/cli.md T-1706)"
                .to_string(),
        );
    }
    v
}

/// Write the lines for `code` of the run of `plan`.
pub fn note(code: i32, plan: &podvm_enter::Plan, rootfs: &str, err: &mut dyn Write) {
    if code != SIGXFSZ_EXIT {
        return;
    }
    let var = format!("{}=", podvm_enter::ladder::GUEST_EXE_VAR);
    let guest = plan.env.iter().find_map(|e| e.strip_prefix(var.as_str()));
    let dotnet = is_dotnet(&plan.env, rootfs, guest);
    for l in lines(code, fsize_soft(&plan.limits), dotnet) {
        let _ = writeln!(err, "{l}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigxfsz_names_the_limit_and_the_dotnet_switch() {
        assert!(lines(0, Some(1), true).is_empty());
        assert!(lines(137, Some(1), true).is_empty());
        let v = lines(SIGXFSZ_EXIT, Some(1_073_741_824), false);
        assert_eq!(v.len(), 1);
        assert!(v[0].contains("RLIMIT_FSIZE of 1073741824 bytes"), "{v:?}");
        let v = lines(SIGXFSZ_EXIT, None, true);
        assert_eq!(v.len(), 2);
        assert!(v[1].contains("DOTNET_EnableWriteXorExecute=0"));
        let d = std::env::temp_dir().join(format!("podvm-t1706-{}", std::process::id()));
        std::fs::create_dir_all(d.join("bin")).unwrap();
        std::fs::write(d.join("bin/App.dll"), b"").unwrap();
        let root = d.to_string_lossy().to_string();
        assert!(is_dotnet(&[], &root, Some("/bin/App")));
        assert!(!is_dotnet(&[], &root, Some("/bin/Other")));
        assert!(is_dotnet(&["DOTNET_X=1".to_string()], &root, None));
        let _ = std::fs::remove_dir_all(&d);
        let l = podvm_enter::ulimit("fsize=4096").unwrap();
        assert_eq!(fsize_soft(&[l]), Some(4096));
    }
}
