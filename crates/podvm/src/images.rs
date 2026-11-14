//! The M1 verbs: `pull`, `images`, `rmi`, `tag`, `image prune`, `inspect`,
//! and `login` (TODO/image.md T-0209).
//!
//! `TODO/image.md` T-0201 to T-0204 and `TOOL.md` section 6.8. The argument
//! surface lives here and the work lives in `podvm-image`, so a flag is parsed
//! in one place and a registry is spoken to in one place.
//!
//! ⚠ **`{{.Size}}` is what podvm HOLDS, not what docker reports.** docker's
//! `SIZE` column is the sum of the uncompressed layers, and M1 does not extract,
//! so podvm does not know that number. Printing docker's column heading over a
//! different quantity would be a number that was not measured
//! (`AGENTS.md` absolute 3), so the column says `SIZE (STORED)` and the
//! field is documented as the compressed bytes in `blobs/`. It becomes the
//! extracted size when M2 lands, in `TODO/extract.md`.

use std::io::{IsTerminal, Write};

use podvm_image::error::{Error, EXIT_CLI_ERROR, EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};
use podvm_image::{clock, credentials, pull, space, Record, Store};

use crate::format;

pub const PULL_USAGE: &str = "\
usage: podvm pull [--platform os/arch[/variant]] <image>
       podvm pull -a <repository>

  -a, --all-tags   pull every offered tag of the repository, not one
                   reference. Takes a bare repository: a tag on it is
                   refused, because it would silently select one tag out
                   of the all that was asked for.
  -q, --quiet      print nothing per layer; errors still reach stderr.

  --tls-verify=B   verify the registry's certificate. Default true. false
                   applies to every registry THIS invocation touches, which is
                   podman's meaning, and it does NOT permit plain HTTP.
  --insecure-registry HOST
                   for HOST only: do not verify its certificate, and fall back
                   to plain HTTP if HTTPS cannot connect. Repeatable. This is
                   docker's flag and it means both things, as docker's does.
                   Also $PODVM_INSECURE_REGISTRIES (comma separated) and one
                   host per line in $PODVM_CONFIG, else
                   $XDG_CONFIG_HOME/podvm/registries.conf.

  refused: Every use of either is printed on stderr, naming the registry. podvm
    never decides on its own to stop verifying or to speak HTTP: a downgrade
    a caller did not ask for is the one thing an automated caller cannot
    notice.

  --platform P     which manifest to take out of a multi-platform index.
                   Defaults to $PODVM_DEFAULT_PLATFORM, then to docker's
                   $DOCKER_DEFAULT_PLATFORM, then to the platform this podvm
                   was built for. A bare word is an ARCHITECTURE, as docker
                   reads it: --platform arm64 means linux/arm64.

  note: Pulling a platform this machine cannot execute is allowed and is not a
    warning here: it is what a caller building for another machine wants. What
    refuses is `podvm run`, and only where nothing can execute it.

  Fetch an image and everything it needs into the content-addressed store.
  HTTPS only: a registry that offers only http:// is a named refusal, never a
  downgrade, because tcp/80 egress hangs rather than failing on the runtime
  podvm targets.

  The store is $PODVM_STORE, else $XDG_DATA_HOME/podvm, else
  $HOME/.local/share/podvm. Blocks AND inodes are checked at it before the
  first layer is fetched, and the refusal names the destination, the free
  amount, the required amount and the unit.
";

pub const EXTRACT_USAGE: &str = "\
usage: podvm extract [--force] <image>

  Unpack a pulled image's layers into a rootfs in the store, and write the
  ownership sidecar beside it. Prints the rootfs path.

  refused: Ownership is NEVER restored. chown to an id this machine's user namespace
  does not map returns EINVAL, and that is where five other tools stop. What
  the image intended is recorded in .meta.jsonl beside the rootfs, keyed by
  path; it changes no kernel permission check and is not presented as if it
  does.

  An entry that resolves outside the destination, including through a symlink
  an earlier entry of the same layer created, is refused and the extraction
  fails. A repaired layer cannot be told from a clean one, so podvm does not
  repair one.

  --force          extract again over an existing rootfs
  --platform P     which platform, where the store holds more than one

  refused: A reference naming more than one image is REFUSED rather than resolved by
    position. The store holds one record per platform, and picking the most
    recently pulled would unpack an architecture nobody asked for.
";

pub const IMAGES_USAGE: &str = "\
usage: podvm images [options] [image]

  -a, --all        accepted for docker parity; podvm stores no intermediate
                   images, so every image is already listed
  -q, --quiet      print image IDs only, the same as --format '{{.ID}}'
      --digests    show the DIGEST column
      --no-trunc   print full IDs and digests
      --format T   a Go-template-shaped string of {{.Field}} placeholders
      --filter key=value
                   name=<substring> matches the repository or tag;
                   label=<key>[=<value>] matches the labels of the image's
                   OCI config (TODO/cli.md T-1695). Repeatable.

  Fields: .Repository .Tag .ID .Digest .CreatedSince .CreatedAt .Size
          .Platform .Store

  note: .Size is the COMPRESSED bytes podvm holds in blobs/, not docker's
    uncompressed total. M1 acquires and does not extract, so the uncompressed
    size is not a number podvm has measured.
";

pub const RMI_USAGE: &str = "\
usage: podvm rmi <image> [image...]
       podvm image rm <image> [image...]

  Remove images and every blob no remaining image reaches. Refuses an image
  any container references or a running container holds, and names it.
";

pub const TAG_USAGE: &str = "\
usage: podvm tag <source> <target>

  Point a second name at the same manifest digest. Nothing is fetched and no
  blob is copied.
";

pub const PRUNE_USAGE: &str = "\
usage: podvm image prune [-a|--all] [-f|--force]

  -a, --all    remove every image no container references or holds, not
               only untagged ones
  -f, --force  accepted for docker parity. podvm never prompts, so there is
               no confirmation for this to suppress (TODO/cli.md T-0806)

  Skips anything a running container holds or any container references,
  and says which.
";

pub const INSPECT_USAGE: &str = "\
usage: podvm inspect [--format T] <image> [image...]

  Fields: .Id .Digest .RepoTags .RepoDigests .Architecture .Os .Created
          .Platform .Size .Store .Layers .RootfsPath .Extracted
          .Exec.Mode .Exec.Shares
          .Config.Env .Config.Cmd .Config.Entrypoint .Config.WorkingDir
          .Config.User

  note: .Config.* is what the image DECLARES, read through the stored
    config digest with docker's key names and null where the image omits
    a field. It sits beside what podvm applied; the two are never
    merged silently (TODO/cli.md T-1417).

  note: .RootfsPath is where the rootfs WOULD be. .Extracted says whether it is
    there; `podvm extract` is what puts it there.

  refused: .Exec.* is what `podvm exec` against this image WOULD be, not a property
    of the image. It is a fresh chroot re-entry sharing only the filesystem
    (TODO/enter.md T-0505), and a caller reads it rather than assuming
    docker's namespace entry.

  Without --format, one JSON array, as docker prints.
";

/// `podvm pull`.
pub fn pull(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, PULL_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, PULL_USAGE) {
        return c;
    }
    let mut want: Option<&str> = None;
    let mut platform_flag: Option<String> = None;
    let mut insecure: Vec<String> = Vec::new();
    let mut tls_verify: Option<bool> = None;
    let mut all_tags = false;
    let mut quiet = false;
    // ⚠ Which flag is still waiting for its value, so `--platform <image>` is a
    // usage error naming the flag rather than a pull of something odd.
    let mut expecting: Option<&'static str> = None;
    for a in args {
        if let Some(flag) = expecting {
            match flag {
                "--platform" => platform_flag = Some(a.clone()),
                _ => insecure.push(a.clone()),
            }
            expecting = None;
            continue;
        }
        match a.as_str() {
            "-h" | "--help" => {
                print!("{PULL_USAGE}");
                return 0;
            }
            "--platform" => expecting = Some("--platform"),
            "--insecure-registry" => expecting = Some("--insecure-registry"),
            "-a" | "--all-tags" => all_tags = true,
            "-q" | "--quiet" => quiet = true,
            // ⚠ A bare `--tls-verify` is `=true`, as docker and podman read it.
            "--tls-verify" => tls_verify = Some(true),
            other if other.starts_with("--platform=") => {
                platform_flag = Some(other["--platform=".len()..].to_string());
            }
            other if other.starts_with("--insecure-registry=") => {
                insecure.push(other["--insecure-registry=".len()..].to_string());
            }
            other if other.starts_with("--tls-verify=") => match &other["--tls-verify=".len()..] {
                "true" | "1" => tls_verify = Some(true),
                "false" | "0" => tls_verify = Some(false),
                v => {
                    eprintln!("podvm pull: --tls-verify takes true or false, not {v:?}");
                    return EXIT_FLAG_ERROR;
                }
            },
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, PULL_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other if want.is_none() => want = Some(other),
            other => {
                eprintln!("podvm pull: {other:?}: pull takes one image");
                return EXIT_CLI_ERROR;
            }
        }
    }
    if let Some(flag) = expecting {
        eprintln!("podvm pull: {flag} needs a value");
        return EXIT_FLAG_ERROR;
    }
    let Some(want) = want else {
        eprint!("{PULL_USAGE}");
        return EXIT_CLI_ERROR;
    };
    // ⛔ Resolved before the store is opened, so a malformed --platform is a
    // usage error and not a runtime one. TODO/probe.md T-0110's contract.
    let platform = match podvm_image::platform::wanted(platform_flag.as_deref()) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    // ⛔ Also before the store is opened: a bad host in the config file or the
    // environment is invalid input and exits 2, not 125.
    let policy = match podvm_image::transport::resolve(&insecure, tls_verify) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };

    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    if all_tags {
        // ⭐ TODO/cli.md T-1331. `-a` takes a bare repository: a tag on it
        // would silently select one tag out of the all that was asked for.
        // The default `:latest` is applied inside `Reference::parse`, so
        // explicitness is read here, off the raw reference, by the same
        // last-colon rule the parser uses.
        let tail = want.rsplit('/').next().unwrap_or(want);
        if tail.contains(':') || tail.contains('@') {
            eprintln!(
                "podvm {verb}: -a takes a repository, not {want:?}: a tag selects one out of the all it asks for"
            );
            return EXIT_FLAG_ERROR;
        }
        let reference = match podvm_image::reference::Reference::parse(want) {
            Ok(r) => r,
            Err(e) => return fail(e.into()),
        };
        let mut out: Box<dyn std::io::Write> = if quiet {
            Box::new(std::io::sink())
        } else {
            Box::new(std::io::stdout().lock())
        };
        return match podvm_image::pull::pull_all(
            &store,
            reference.endpoint(),
            &reference.repository,
            &platform,
            &policy,
            &mut out,
        ) {
            Ok(pulled) => {
                eprintln!("podvm {verb}: pulled {} tag(s)", pulled.len());
                0
            }
            Err(e) => fail(e),
        };
    }
    let mut out: Box<dyn std::io::Write> = if quiet {
        Box::new(std::io::sink())
    } else {
        Box::new(std::io::stdout().lock())
    };
    match pull::pull(&store, want, &platform, &policy, &mut out) {
        Ok(done) => {
            // ⛔ The provenance of the probe answer is on stderr, never implied.
            // A cached rung and a measured one are different sentences.
            // T-1767: `-q` keeps stderr for errors.
            if !quiet && let podvm_image::probe_cache::Source::Measured(why) = &done.probe_source {
                eprintln!("podvm: probed this machine rather than using the cache: {why}");
            }
            0
        }
        Err(e) => fail(e),
    }
}

/// The server of `podvm login` and `podvm logout` with no SERVER: docker's
/// canonical key, from `pod-services` (`docker-hub-registry`, fact
/// `credential-key`, TODO/deps.md T-1679).
pub(crate) fn default_login_server() -> &'static str {
    pod_services::get("docker-hub-registry")
        .and_then(|s| s.text("credential-key"))
        .expect("pod-services holds docker-hub-registry credential-key (TODO/deps.md T-1679)")
}

mod list;
pub use list::*;

mod login_extract;
pub use login_extract::*;

mod save_load;
pub use save_load::*;

mod inspect;
pub use inspect::*;

#[cfg(test)]
mod exit_tests;
#[cfg(test)]
mod tests; // TODO/image.md T-1842

/// docker's `image` sub-command group.
pub fn group(args: &[String]) -> i32 {
    let rest = if args.len() > 1 { &args[1..] } else { &[] };
    match args.first().map(String::as_str) {
        Some(s) if s == "ls" || s == "list" => images(&format!("image {s}"), rest),
        Some(s) if s == "rm" || s == "remove" => rmi(&format!("image {s}"), rest),
        Some("prune") => prune("image prune", rest),
        Some("tag") => tag("image tag", rest),
        Some("inspect") => inspect("image inspect", rest),
        Some("pull") => pull("image pull", rest),
        Some("extract") => extract("image extract", rest),
        Some("-h") | Some("--help") | None => {
            println!("usage: podvm image ls | rm | prune | tag | inspect | pull");
            0
        }
        Some(first) if first.starts_with('-') => {
            // ⛔ TODO/cli.md T-0801. The table decides: `image` takes
            // subverbs, never flags, so a flag has no row and is refused
            // rather than reaching the subverb match below.
            if let Err(c) = crate::parity::admit(
                "image",
                first,
                "usage: podvm image ls | rm | prune | tag | inspect | pull",
            ) {
                return c;
            }
            crate::parity::no_arm("image", first)
        }
        Some(other) => {
            eprintln!("podvm image: {other:?}: not implemented yet");
            EXIT_RUNTIME_ERROR
        }
    }
}

/// TODO/image.md T-1781: remove the extraction of each manifest digest of
/// `before` that no record of the store names after a removal. The delete
/// of the store unlinks the blobs only, so the extracted root of a removed
/// image stayed (8.2 MiB for alpine, 811 MiB for golang), and with it the
/// spare tree. A digest that another tag still names keeps its extraction.
pub(crate) fn drop_extractions(store: &podvm_image::Store, before: &[String]) {
    let Ok(now) = store.list() else {
        return;
    };
    for d in before {
        if !now.iter().any(|r| &r.manifest_digest == d) {
            let _ = podvm_extract::remove_extracted(store, d);
        }
    }
}

/// TODO/image.md T-1787 (PodUtils/podvm#329): each extraction of the store
/// that no record names, with its bytes: what an `rmi` of an older podvm
/// left. An extraction starts from a record (`extract`, `run`), and a
/// removal skips an image that a run holds, so an extraction with no
/// record is left over, never in work.
pub(crate) fn orphan_extractions(store: &podvm_image::Store) -> Vec<(String, u64)> {
    let records = store.list().unwrap_or_default();
    let Ok(rd) = std::fs::read_dir(store.root().join("rootfs")) else {
        return Vec::new();
    };
    rd.flatten()
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            let digest = name.replacen('_', ":", 1);
            let named = records.iter().any(|r| r.manifest_digest == digest);
            (!named && e.path().is_dir())
                .then(|| (digest, podvm_image::space::dir_bytes(&e.path())))
        })
        .collect()
}
