//! The options of the image's loader on the first exec of the loader
//! family: `--inhibit-cache` (TODO/enter.md T-1719) and `--argv0`
//! (TODO/cli.md T-1660).
//!
//! Without `--argv0` the payload's `argv[0]` is the host path of the file
//! that the loader opened. Python derives `sys.executable` and its
//! virtual environment from `argv[0]`: the `#!/opt/venv/bin/python3` of
//! a venv script gave `sys.prefix` the host path of `/usr/local`, and the
//! venv's modules were not found (measured with ghcr.io/marimo-team/marimo).
//! The interposer's exec route of a child passes `--argv0` already
//! (`crates/podvm-interpose/src/exec/launch.rs`); this is the same rule for
//! the first exec.

/// Whether the loader file `name` (its resolved file name) is a glibc
/// loader older than 2.33, which does not take `--argv0`. glibc named its
/// loader `ld-2.NN.so` up to 2.33; the musl loader takes the option. The
/// rule of `loader_lacks_argv0` of the interposer.
pub fn lacks_argv0(name: &str) -> bool {
    let Some(minor) = name
        .strip_prefix("ld-2.")
        .and_then(|r| r.strip_suffix(".so"))
    else {
        return false;
    };
    !minor.is_empty()
        && minor.len() <= 3
        && minor.bytes().all(|b| b.is_ascii_digit())
        && minor.parse::<u32>().is_ok_and(|m| m < 33)
}

/// Insert the options after the loader (`argv[0]` of `loader_argv`):
/// `--inhibit-cache` where the loader takes it, and `--argv0 INVOKED`
/// where the loader takes it and no busybox applet rides in its place.
pub fn options(
    loader_argv: &mut Vec<String>,
    loader_host: &str,
    inhibit_cache: bool,
    invoked: &str,
    applet: bool,
) {
    let name = loader_host.rsplit('/').next().unwrap_or(loader_host);
    let mut at = 1;
    if inhibit_cache {
        loader_argv.insert(at, "--inhibit-cache".to_string());
        at += 1;
    }
    if !applet && !invoked.is_empty() && !lacks_argv0(name) && loader_argv.len() >= at {
        loader_argv.insert(at, "--argv0".to_string());
        loader_argv.insert(at + 1, invoked.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn the_payload_gets_its_invoked_name_where_the_loader_takes_it() {
        let mut a = v(&[
            "/r/lib64/ld-linux-x86-64.so.2",
            "/r/usr/local/bin/python3.13",
            "m.py",
        ]);
        options(
            &mut a,
            "/r/lib64/ld-linux-x86-64.so.2",
            true,
            "/opt/venv/bin/python3",
            false,
        );
        assert_eq!(
            a,
            v(&[
                "/r/lib64/ld-linux-x86-64.so.2",
                "--inhibit-cache",
                "--argv0",
                "/opt/venv/bin/python3",
                "/r/usr/local/bin/python3.13",
                "m.py"
            ])
        );
        // glibc 2.31 lacks the option; a busybox applet takes its place.
        let mut a = v(&["/r/lib/ld-2.31.so", "/r/bin/x"]);
        options(&mut a, "/r/lib/ld-2.31.so", false, "x", false);
        assert_eq!(a, v(&["/r/lib/ld-2.31.so", "/r/bin/x"]));
        let mut a = v(&["/r/lib/ld-musl-x86_64.so.1", "/r/bin/busybox", "sh"]);
        options(&mut a, "/r/lib/ld-musl-x86_64.so.1", false, "sh", true);
        assert_eq!(
            a,
            v(&["/r/lib/ld-musl-x86_64.so.1", "/r/bin/busybox", "sh"])
        );
        assert!(lacks_argv0("ld-2.17.so"));
        assert!(!lacks_argv0("ld-2.33.so"));
        assert!(!lacks_argv0("ld-musl-x86_64.so.1"));
    }
}
