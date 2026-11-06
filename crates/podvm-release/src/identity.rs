//! The release identity a tag was published under.
//!
//! ⛔ The project was podbox, in `Azathothas/podbox`, up to and including
//! `v0.1.0-beta.13`. Those releases keep their names and their signatures:
//! the assets are `podbox-*`, and each bundle is signed by that repository's
//! nightly workflow. Every later tag is podvm's: `podvm-*` assets, signed by
//! the nightly workflow of the repository the release is read from.
//!
//! Shared by `podvm-verify`, `release-notes` and `podvm-publish`, so the
//! three cannot disagree about where the old identity ends.

#![allow(dead_code)]

/// The last beta published under the name podbox.
pub const LAST_PODBOX_BETA: u32 = 13;

/// True where `tag` was released under the old name and identity.
pub fn released_as_podbox(tag: &str) -> bool {
    match tag.strip_prefix("v0.1.0-beta.") {
        Some(n) => n.parse::<u32>().is_ok_and(|n| n <= LAST_PODBOX_BETA),
        None => tag.starts_with("v0.0."),
    }
}

/// The asset name prefix of the release `tag`: `podbox` or `podvm`.
pub fn asset_prefix(tag: &str) -> &'static str {
    if released_as_podbox(tag) {
        "podbox"
    } else {
        "podvm"
    }
}

/// The certificate identity a bundle for `tag` must carry. `repo` is the
/// repository the release is read from; the old releases ignore it.
pub fn signing_identity(repo: &str, tag: &str) -> String {
    let signer = if released_as_podbox(tag) {
        "Azathothas/podbox"
    } else {
        repo
    };
    format!("https://github.com/{signer}/.github/workflows/nightly.yml@refs/tags/{tag}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_up_to_beta_13_keeps_the_podbox_identity() {
        for tag in ["v0.1.0-beta.1", "v0.1.0-beta.12", "v0.1.0-beta.13"] {
            assert!(released_as_podbox(tag), "{tag}");
            assert_eq!(asset_prefix(tag), "podbox");
            assert_eq!(
                signing_identity("PodUtils/podvm", tag),
                format!(
                    "https://github.com/Azathothas/podbox/.github/workflows/nightly.yml@refs/tags/{tag}"
                )
            );
        }
    }

    #[test]
    fn a_later_tag_takes_the_podvm_identity() {
        for tag in ["v0.1.0-beta.14", "v0.1.0", "v0.2.0-beta.1"] {
            assert!(!released_as_podbox(tag), "{tag}");
            assert_eq!(asset_prefix(tag), "podvm");
            assert_eq!(
                signing_identity("PodUtils/podvm", tag),
                format!(
                    "https://github.com/PodUtils/podvm/.github/workflows/nightly.yml@refs/tags/{tag}"
                )
            );
        }
    }
}
