//! A GitHub release through `gh`, with the public URL as the fallback
//! (TODO/cli.md T-1667, issue 188).
//!
//! The rule: podvm fetches a release asset with `gh release download`, so
//! a host with a `gh` login gets its own rate limit. Where that call fails
//! (no `gh`, a rejected token, an answer over the limit), podvm takes the
//! public URL. podvm does not ask `gh auth status` first: a host can be
//! logged in and still be refused, and a host with no login can still
//! download a public asset. The SHA-256 of the pin checks the bytes of each
//! path the same way.

use std::process::Command;

/// One asset of a release.
#[derive(Debug)]
pub struct Release {
    pub repo: &'static str,
    pub tag: String,
    pub asset: String,
}

/// How the bytes came.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    Gh,
    PublicUrl,
}

/// The bytes of `r` through the program `gh`, at most `max`, else through
/// `fallback`. Each failure of `gh` is named on stderr before the fallback.
pub fn fetch(
    r: &Release,
    gh: &str,
    max: u64,
    fallback: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<(Vec<u8>, Via), String> {
    let out = Command::new(gh)
        .args(["release", "download", &r.tag, "-R", r.repo])
        .args(["-p", &r.asset, "-O", "-"])
        .output();
    let why = match out {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => Some(format!("{gh} did not start: {e}")),
        Ok(o) if !o.status.success() => {
            let err = String::from_utf8_lossy(&o.stderr);
            Some(format!(
                "{gh} release download exited {}: {}",
                o.status.code().unwrap_or(-1),
                err.lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .chars()
                    .take(160)
                    .collect::<String>()
            ))
        }
        Ok(o) if o.stdout.len() as u64 > max => Some(format!(
            "{gh} gave {} bytes, over the limit {max}",
            o.stdout.len()
        )),
        Ok(o) => return Ok((o.stdout, Via::Gh)),
    };
    if let Some(why) = why {
        eprintln!(
            "podvm: {}: {why}; podvm takes the public URL (TODO/cli.md T-1667)",
            r.asset
        );
    }
    fallback().map(|b| (b, Via::PublicUrl))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r() -> Release {
        Release {
            repo: "owner/repo",
            tag: "v1".into(),
            asset: "a.tar.xz".into(),
        }
    }

    /// A planted `gh` in a directory of its own.
    fn planted(tag: &str, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let d = std::env::temp_dir().join(format!("podvm-gh-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("gh");
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[test]
    fn a_rejected_gh_token_falls_back_to_the_public_url() {
        let gh = planted("rejected", "echo 'HTTP 401: Bad credentials' >&2; exit 1");
        let (b, via) = fetch(&r(), gh.to_str().unwrap(), 1 << 20, || {
            Ok(b"public".to_vec())
        })
        .unwrap();
        assert_eq!((b.as_slice(), via), (&b"public"[..], Via::PublicUrl));
        let _ = std::fs::remove_dir_all(gh.parent().unwrap());
    }

    #[test]
    fn gh_gives_the_bytes_where_its_call_works() {
        // The planted gh prints its arguments: the call names the tag, the
        // repository and the asset, and writes to standard output.
        let gh = planted("works", "printf '%s ' \"$@\"");
        let (b, via) = fetch(&r(), gh.to_str().unwrap(), 1 << 20, || {
            panic!("no fallback where gh works")
        })
        .unwrap();
        assert_eq!(via, Via::Gh);
        assert_eq!(
            String::from_utf8(b).unwrap(),
            "release download v1 -R owner/repo -p a.tar.xz -O - "
        );
        let (_, via) = fetch(&r(), gh.to_str().unwrap(), 4, || Ok(Vec::new())).unwrap();
        assert_eq!(via, Via::PublicUrl, "an answer over the limit falls back");
        let _ = std::fs::remove_dir_all(gh.parent().unwrap());
    }

    #[test]
    fn no_gh_falls_back_to_the_public_url() {
        let (_, via) = fetch(&r(), "/nonexistent/podvm-gh", 1, || Ok(Vec::new())).unwrap();
        assert_eq!(via, Via::PublicUrl);
    }
}
