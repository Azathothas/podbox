//! The M1 verbs: `pull`, `images`, `rmi`, `tag`, `image prune`, `inspect`,
//! and `login` (TODO/image.md T-0209).
//!
//! `TODO/image.md` T-0201 to T-0204 and `TOOL.md` section 6.8. The argument
//! surface lives here and the work lives in `podbox-image`, so a flag is parsed
//! in one place and a registry is spoken to in one place.
//!
//! ⚠ **`{{.Size}}` is what podbox HOLDS, not what docker reports.** docker's
//! `SIZE` column is the sum of the uncompressed layers, and M1 does not extract,
//! so podbox does not know that number. Printing docker's column heading over a
//! different quantity would be a number that was not measured
//! (`AGENTS.md` absolute 3), so the column says `SIZE (STORED)` and the
//! field is documented as the compressed bytes in `blobs/`. It becomes the
//! extracted size when M2 lands, in `TODO/extract.md`.

use std::io::Write;

use podbox_image::error::{Error, EXIT_CLI_ERROR, EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};
use podbox_image::{clock, credentials, pull, space, Record, Store};

use crate::format;

pub const PULL_USAGE: &str = "\
usage: podbox pull [--platform os/arch[/variant]] <image>

  --tls-verify=B   verify the registry's certificate. Default true. false
                   applies to every registry THIS invocation touches, which is
                   podman's meaning, and it does NOT permit plain HTTP.
  --insecure-registry HOST
                   for HOST only: do not verify its certificate, and fall back
                   to plain HTTP if HTTPS cannot connect. Repeatable. This is
                   docker's flag and it means both things, as docker's does.
                   Also $PODBOX_INSECURE_REGISTRIES (comma separated) and one
                   host per line in $PODBOX_CONFIG, else
                   $XDG_CONFIG_HOME/podbox/registries.conf.

  ⛔ Every use of either is printed on stderr, naming the registry. podbox
    never decides on its own to stop verifying or to speak HTTP: a downgrade
    a caller did not ask for is the one thing an automated caller cannot
    notice.

  --platform P     which manifest to take out of a multi-platform index.
                   Defaults to $PODBOX_DEFAULT_PLATFORM, then to docker's
                   $DOCKER_DEFAULT_PLATFORM, then to the platform this podbox
                   was built for. A bare word is an ARCHITECTURE, as docker
                   reads it: --platform arm64 means linux/arm64.

  ⚠ Pulling a platform this machine cannot execute is allowed and is not a
    warning here: it is what a caller building for another machine wants. What
    refuses is `podbox run`, and only where nothing can execute it.

  Fetch an image and everything it needs into the content-addressed store.
  HTTPS only: a registry that offers only http:// is a named refusal, never a
  downgrade, because tcp/80 egress hangs rather than failing on the runtime
  podbox targets.

  The store is $PODBOX_STORE, else $XDG_DATA_HOME/podbox, else
  $HOME/.local/share/podbox. Blocks AND inodes are checked at it before the
  first layer is fetched, and the refusal names the destination, the free
  amount, the required amount and the unit.
";

pub const EXTRACT_USAGE: &str = "\
usage: podbox extract [--force] <image>

  Unpack a pulled image's layers into a rootfs in the store, and write the
  ownership sidecar beside it. Prints the rootfs path.

  ⛔ Ownership is NEVER restored. chown to an id this machine's user namespace
  does not map returns EINVAL, and that is where five other tools stop. What
  the image intended is recorded in .meta.jsonl beside the rootfs, keyed by
  path; it changes no kernel permission check and is not presented as if it
  does.

  An entry that resolves outside the destination, including through a symlink
  an earlier entry of the same layer created, is refused and the extraction
  fails. A repaired layer cannot be told from a clean one, so podbox does not
  repair one.

  --force          extract again over an existing rootfs
  --platform P     which platform, where the store holds more than one

  ⛔ A reference naming more than one image is REFUSED rather than resolved by
    position. The store holds one record per platform, and picking the most
    recently pulled would unpack an architecture nobody asked for.
";

pub const IMAGES_USAGE: &str = "\
usage: podbox images [options] [image]

  -a, --all        accepted for docker parity; podbox stores no intermediate
                   images, so every image is already listed
  -q, --quiet      print image IDs only, the same as --format '{{.ID}}'
      --digests    show the DIGEST column
      --no-trunc   print full IDs and digests
      --format T   a Go-template-shaped string of {{.Field}} placeholders

  Fields: .Repository .Tag .ID .Digest .CreatedSince .CreatedAt .Size
          .Platform .Store

  ⚠ .Size is the COMPRESSED bytes podbox holds in blobs/, not docker's
    uncompressed total. M1 acquires and does not extract, so the uncompressed
    size is not a number podbox has measured.
";

pub const RMI_USAGE: &str = "\
usage: podbox rmi <image> [image...]
       podbox image rm <image> [image...]

  Remove images and every blob no remaining image reaches. Refuses an image
  any container references or a running container holds, and names it.
";

pub const TAG_USAGE: &str = "\
usage: podbox tag <source> <target>

  Point a second name at the same manifest digest. Nothing is fetched and no
  blob is copied.
";

pub const PRUNE_USAGE: &str = "\
usage: podbox image prune [-a|--all] [-f|--force]

  -a, --all    remove every image no container references or holds, not
               only untagged ones
  -f, --force  accepted for docker parity. podbox never prompts, so there is
               no confirmation for this to suppress (TODO/cli.md T-0806)

  Skips anything a running container holds or any container references,
  and says which.
";

pub const INSPECT_USAGE: &str = "\
usage: podbox inspect [--format T] <image> [image...]

  Fields: .Id .Digest .RepoTags .RepoDigests .Architecture .Os .Created
          .Platform .Size .Store .Layers .RootfsPath .Extracted
          .Exec.Mode .Exec.Shares

  ⚠ .RootfsPath is where the rootfs WOULD be. .Extracted says whether it is
    there; `podbox extract` is what puts it there.

  ⛔ .Exec.* is what `podbox exec` against this image WOULD be, not a property
    of the image. It is a fresh chroot re-entry sharing only the filesystem
    (TODO/enter.md T-0505), and a caller reads it rather than assuming
    docker's namespace entry.

  Without --format, one JSON array, as docker prints.
";

/// `podbox pull`.
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
                    eprintln!("podbox pull: --tls-verify takes true or false, not {v:?}");
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
                eprintln!("podbox pull: {other:?}: pull takes one image");
                return EXIT_CLI_ERROR;
            }
        }
    }
    if let Some(flag) = expecting {
        eprintln!("podbox pull: {flag} needs a value");
        return EXIT_FLAG_ERROR;
    }
    let Some(want) = want else {
        eprint!("{PULL_USAGE}");
        return EXIT_CLI_ERROR;
    };
    // ⛔ Resolved before the store is opened, so a malformed --platform is a
    // usage error and not a runtime one. TODO/probe.md T-0110's contract.
    let platform = match podbox_image::platform::Platform::wanted(platform_flag.as_deref()) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    // ⛔ Also before the store is opened: a bad host in the config file or the
    // environment is invalid input and exits 2, not 125.
    let policy = match podbox_image::transport::Policy::resolve(&insecure, tls_verify) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };

    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let mut out = std::io::stdout().lock();
    match pull::pull(&store, want, &platform, &policy, &mut out) {
        Ok(done) => {
            // ⛔ The provenance of the probe answer is on stderr, never implied.
            // A cached rung and a measured one are different sentences.
            if let podbox_image::probe_cache::Source::Measured(why) = &done.probe_source {
                eprintln!("podbox: probed this machine rather than using the cache: {why}");
            }
            0
        }
        Err(e) => fail(e),
    }
}

struct Options {
    quiet: bool,
    digests: bool,
    no_trunc: bool,
    format: Option<String>,
    filter: Option<String>,
}

/// `podbox images`.
pub fn images(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, IMAGES_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, IMAGES_USAGE) {
        return c;
    }
    let mut o = Options {
        quiet: false,
        digests: false,
        no_trunc: false,
        format: None,
        filter: None,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{IMAGES_USAGE}");
                return 0;
            }
            "-a" | "--all" => {}
            "-q" | "--quiet" => o.quiet = true,
            "--digests" => o.digests = true,
            "--no-trunc" => o.no_trunc = true,
            "--format" => match it.next() {
                Some(t) => o.format = Some(t.clone()),
                None => {
                    eprintln!("podbox images: --format needs a template");
                    return EXIT_FLAG_ERROR;
                }
            },
            other if other.starts_with("--format=") => {
                o.format = Some(other["--format=".len()..].to_string())
            }
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, IMAGES_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other if o.filter.is_none() => o.filter = Some(other.to_string()),
            other => {
                eprintln!("podbox images: {other:?}: images takes at most one image");
                return EXIT_CLI_ERROR;
            }
        }
    }
    if o.quiet && o.format.is_none() {
        o.format = Some("{{.ID}}".to_string());
    }
    // ⛔ Validated BEFORE the store is opened, and therefore before the loop
    // over records. An empty store means zero iterations, and a template
    // checked only inside the loop is never checked at all: measured on
    // 2026-09-08, `--format '{{.Nope}}'` against an empty store printed
    // nothing and exited 0, so a caller's typo read as an empty result set.
    if let Some(t) = &o.format {
        if let Err(bad) = format::check(t, IMAGE_FIELDS) {
            eprintln!("podbox images: {bad}");
            return EXIT_CLI_ERROR;
        }
    }

    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let records = match &o.filter {
        Some(f) => match store.find(f) {
            Ok(r) => r,
            Err(e) => return fail(e),
        },
        None => match store.list() {
            Ok(r) => r,
            Err(e) => return fail(e),
        },
    };

    let mut out = std::io::stdout().lock();
    if let Some(template) = &o.format {
        for r in &records {
            match format::render(template, &image_fields(r, &store, o.no_trunc)) {
                Ok(line) => {
                    let _ = writeln!(out, "{line}");
                }
                Err(bad) => {
                    eprintln!("podbox images: {bad}");
                    return EXIT_CLI_ERROR;
                }
            }
        }
        return 0;
    }

    let mut header: Vec<&str> = vec!["REPOSITORY", "TAG"];
    if o.digests {
        header.push("DIGEST");
    }
    header.extend(["IMAGE ID", "CREATED", "SIZE (STORED)"]);
    let mut rows: Vec<Vec<String>> = vec![header.iter().map(|s| s.to_string()).collect()];
    for r in &records {
        let f = image_fields(r, &store, o.no_trunc);
        let get = |k: &str| pick(&f, k);
        let mut row = vec![get("Repository"), get("Tag")];
        if o.digests {
            row.push(get("Digest"));
        }
        row.extend([get("ID"), get("CreatedSince"), get("Size")]);
        rows.push(row);
    }
    let _ = write!(out, "{}", table(&rows));
    0
}

/// `podbox rmi` and `podbox image rm`.
pub fn rmi(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, RMI_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, RMI_USAGE) {
        return c;
    }
    let mut wanted: Vec<&str> = Vec::new();
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{RMI_USAGE}");
                return 0;
            }
            // docker's `-f` forces removal of a tagged image. podbox already
            // removes every name the reference gives; what it refuses is an
            // image IN USE, and no flag overrides that, because the deletion
            // would be under a running payload.
            "-f" | "--force" => {}
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, RMI_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other => wanted.push(other),
        }
    }
    if wanted.is_empty() {
        eprint!("{RMI_USAGE}");
        return EXIT_CLI_ERROR;
    }
    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let mut code = 0;
    for want in wanted {
        // ⭐ TODO/image.md T-1322. Records before holds: a created
        // container takes no hold, so only its record sees the reference,
        // and docker's rule keys on any container, running or stopped.
        // Every record the name resolves to is checked, and
        // `store.remove` below stays the last word on running payloads.
        // Unresolvable here is `remove`'s own error to report below,
        // once, rather than two refusals for one name.
        if let Ok(records) = store.find(want) {
            let mut failed = false;
            let mut referrers: Vec<String> = Vec::new();
            for record in &records {
                match podbox_supervise::referencing(&store, &record.manifest_digest) {
                    Ok(found) => referrers.extend(found.iter().map(|c| c.name.clone())),
                    Err(e) => {
                        eprintln!("podbox rmi: {e}");
                        code = EXIT_RUNTIME_ERROR;
                        failed = true;
                    }
                }
            }
            if failed {
                continue;
            }
            if !referrers.is_empty() {
                referrers.sort();
                referrers.dedup();
                eprintln!(
                    "podbox rmi: {want} is referenced by container {} and was not removed",
                    referrers.join(", ")
                );
                code = EXIT_RUNTIME_ERROR;
                continue;
            }
        }
        match store.remove(want) {
            Ok(done) => {
                for name in &done.untagged {
                    println!("Untagged: {name}");
                }
                for d in &done.deleted {
                    println!("Deleted: {d}");
                }
            }
            Err(e) => {
                eprintln!("podbox rmi: {e}");
                code = e.exit_code();
            }
        }
    }
    code
}

/// `podbox tag`.
pub fn tag(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, TAG_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, TAG_USAGE) {
        return c;
    }
    let mut positional: Vec<&str> = Vec::new();
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{TAG_USAGE}");
                return 0;
            }
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, TAG_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other => positional.push(other),
        }
    }
    if positional.len() != 2 {
        eprint!("{TAG_USAGE}");
        return EXIT_CLI_ERROR;
    }
    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    match store.tag(positional[0], positional[1]) {
        Ok(_) => 0,
        Err(e) => fail(e),
    }
}

pub const LOGIN_USAGE: &str = "\
usage: podbox login [-u|--username USER] [--password-stdin] [SERVER]

  Log in to a registry. The password arrives on stdin, never as an
  argument: an argument lives in the shell history and the process table.
  There is no interactive prompt; an automated caller pipes the password
  in, and a runtime that blocks on input is the hang AGENTS.md forbids.

  Stores in ~/.docker/config.json, the file docker writes, or through the
  configured credential helper where one is named for the server. SERVER
  defaults to https://index.docker.io/v1/, docker's canonical key.

  The stored login is used, never printed: pulls send it, errors name the
  registry, and result files never carry it (TODO/image.md T-0209).
";

/// `podbox login`.
pub fn login(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, LOGIN_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, LOGIN_USAGE) {
        return c;
    }
    let parsed = match parse_login_args(verb, args) {
        Ok(p) => p,
        Err(c) => {
            eprint!("{LOGIN_USAGE}");
            return c;
        }
    };
    if parsed.help {
        print!("{LOGIN_USAGE}");
        return 0;
    }
    let server = parsed
        .server
        .as_deref()
        .unwrap_or("https://index.docker.io/v1/");
    let mut password = String::new();
    use std::io::Read;
    if std::io::stdin().read_to_string(&mut password).is_err() {
        eprint!("{LOGIN_USAGE}");
        return EXIT_CLI_ERROR;
    }
    while password.ends_with('\n') || password.ends_with('\r') {
        password.pop();
    }
    if password.is_empty() {
        eprint!("{LOGIN_USAGE}");
        return EXIT_CLI_ERROR;
    }
    match credentials::store_login(server, &parsed.username, &password) {
        Ok(()) => {
            println!("Login Succeeded");
            0
        }
        Err(e) => {
            eprintln!("podbox login: {e}");
            EXIT_RUNTIME_ERROR
        }
    }
}

#[derive(Debug)]
struct LoginArgs {
    username: String,
    server: Option<String>,
    help: bool,
}

/// The argument shape, pure so tests own it. The password never appears
/// here: it arrives on stdin after parsing, so no usage string, log line,
/// or test fixture ever carries one. Unknown dash flags are refused by the
/// caller through the parity table, not here: this function sees only what
/// `admit_all` accepted.
fn parse_login_args(verb: &str, args: &[String]) -> Result<LoginArgs, i32> {
    let mut username: Option<String> = None;
    let mut password_stdin = false;
    let mut server: Option<String> = None;
    let mut help = false;
    let mut expecting: Option<&'static str> = None;
    for a in args {
        if let Some(flag) = expecting {
            if flag == "--username" {
                username = Some(a.clone());
            }
            expecting = None;
            continue;
        }
        match a.as_str() {
            "-h" | "--help" => help = true,
            "-u" | "--username" => expecting = Some("--username"),
            "--password-stdin" => password_stdin = true,
            // ⚠ The `=` forms, as `pull`'s `--platform=` is handled: `admit`
            // cuts at `=` before the table lookup, so these already passed
            // `admit_all`, and without an arm here they would reach `no_arm`
            // and read as a defect in podbox rather than a value it accepts.
            other if other.starts_with("--username=") => {
                username = Some(other["--username=".len()..].to_string());
            }
            other if other.starts_with("-u=") => {
                username = Some(other["-u=".len()..].to_string());
            }
            other if other.starts_with('-') => {
                crate::parity::admit(verb, other, LOGIN_USAGE)?;
                return Err(crate::parity::no_arm(verb, other));
            }
            other => {
                if server.is_some() {
                    return Err(EXIT_CLI_ERROR);
                }
                server = Some(other.to_string());
            }
        }
    }
    if help {
        return Ok(LoginArgs {
            username: String::new(),
            server,
            help: true,
        });
    }
    // ⛔ An empty username is a flag-shape refusal, not a stored login: the
    // read path skips entries with no user, so storing one would write a
    // login that never reads back.
    if expecting.is_some() || username.as_deref().unwrap_or("").is_empty() || !password_stdin {
        return Err(EXIT_FLAG_ERROR);
    }
    Ok(LoginArgs {
        username: username.unwrap_or_default(),
        server,
        help,
    })
}

/// `podbox image prune`.
pub fn prune(verb: &str, args: &[String]) -> i32 {
    let mut all = false;
    // ⭐ TODO/cli.md T-1330. Bundled shorts expand by the one shared rule
    // (`parity::expand`) before the table sees them; the local `a`/`f`
    // splitter this replaces knew only those two members.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, PRUNE_USAGE),
    };
    if let Some(c) = crate::parity::admit_all(verb, &expanded, PRUNE_USAGE) {
        return c;
    }
    for a in &expanded {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{PRUNE_USAGE}");
                return 0;
            }
            "-a" | "--all" => all = true,
            "-f" | "--force" => {}
            // docker takes `-af` as one cluster, expanded above by the
            // shared rule (TODO/cli.md T-1330).
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, PRUNE_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other => return unknown("image prune", other, PRUNE_USAGE),
        }
    }
    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    // ⭐ TODO/image.md T-1322. The candidate set is the store's own
    // (`prune_candidates`, the same filter `prune` uses), and records gate
    // before holds: referenced candidates are skipped and named, and only
    // the unreferenced remainder reaches `delete`, which stays the last
    // word on running payloads.
    let candidates = match store.prune_candidates(all) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let mut rest: Vec<podbox_image::Record> = Vec::new();
    let mut referenced: Vec<(String, Vec<String>)> = Vec::new();
    for record in candidates {
        match podbox_supervise::referencing(&store, &record.manifest_digest) {
            Ok(found) if !found.is_empty() => {
                let mut names: Vec<String> = found.iter().map(|c| c.name.clone()).collect();
                names.sort();
                referenced.push((record.name(), names));
            }
            Ok(_) => rest.push(record),
            Err(e) => {
                eprintln!("podbox image prune: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        }
    }
    match store.delete(&rest, podbox_image::store::Held::Skip) {
        Ok(done) => {
            if !done.deleted.is_empty() {
                println!("Deleted Images:");
                for name in &done.untagged {
                    println!("untagged: {name}");
                }
                for d in &done.deleted {
                    println!("deleted: {d}");
                }
            }
            // ⛔ T-0204: prune skips anything locked AND SAYS WHICH. A silent
            // skip is a prune that reports success having done nothing it was
            // asked to do. T-1322's record-skips name their containers the
            // same way.
            for name in &done.skipped {
                println!("skipped: {name} is in use by a running container");
            }
            for (name, containers) in &referenced {
                println!(
                    "skipped: {name} is referenced by container {}",
                    containers.join(", ")
                );
            }
            println!("\nTotal reclaimed space: {}", space::mib(done.freed_bytes));
            0
        }
        Err(e) => fail(e),
    }
}

/// `podbox inspect`, for images. Containers are M3.
/// `podbox extract <image>`: `TODO/extract.md` T-0301 to T-0307, milestone M2.
///
/// ⛔ **This is the verb M2 can drive.** [`TODO/milestones.md`](../../../TODO/milestones.md)
/// T-1103's acceptance runs `podbox run --rm`, which is M3, so extraction would
/// otherwise be implemented with nothing able to exercise it until another
/// milestone lands. That is how a component ships untested.
pub fn extract(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, EXTRACT_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, EXTRACT_USAGE) {
        return c;
    }
    let mut want: Option<&str> = None;
    let mut force = false;
    let mut platform_flag: Option<String> = None;
    let mut expect_platform = false;
    for a in args {
        if expect_platform {
            platform_flag = Some(a.clone());
            expect_platform = false;
            continue;
        }
        match a.as_str() {
            "-h" | "--help" => {
                print!("{EXTRACT_USAGE}");
                return 0;
            }
            "--force" => force = true,
            "--platform" => expect_platform = true,
            other if other.starts_with("--platform=") => {
                platform_flag = Some(other["--platform=".len()..].to_string());
            }
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, EXTRACT_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other if want.is_none() => want = Some(other),
            other => {
                eprintln!("podbox extract: {other:?}: extract takes one image");
                return EXIT_CLI_ERROR;
            }
        }
    }
    if expect_platform {
        eprintln!("podbox extract: --platform needs a value, for example linux/arm64");
        return EXIT_FLAG_ERROR;
    }
    let Some(want) = want else {
        eprint!("{EXTRACT_USAGE}");
        return EXIT_CLI_ERROR;
    };
    // ⚠ `None` where no flag was given, so `find_one_for` prefers the host's
    // platform and refuses only where that leaves more than one.
    let platform = match platform_flag.as_deref() {
        Some(p) => match podbox_image::platform::Platform::parse(p) {
            Ok(p) => Some(p),
            Err(e) => return fail(e),
        },
        None => None,
    };

    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let record = match store.find_one_for(want, platform.as_ref()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("podbox extract: {e}");
            return e.exit_code();
        }
    };

    if podbox_extract::is_extracted(&store, &record.manifest_digest) && !force {
        // ⚠ Already done is not an error, and the path is still printed: a
        // caller pipes this into `run`, and a second invocation must answer the
        // same thing as the first.
        let (rootfs, _) = podbox_extract::paths(&store, &record.manifest_digest);
        println!("{}", rootfs.display());
        return 0;
    }
    if force {
        if let Err(e) = podbox_extract::remove_extracted(&store, &record.manifest_digest) {
            eprintln!("podbox extract: {e}");
            return podbox_image::error::EXIT_RUNTIME_ERROR;
        }
    }

    let manifest_digest = match podbox_image::digest::Digest::parse(&record.manifest_digest) {
        Ok(d) => d,
        Err(e) => return fail(e),
    };
    let bytes = match store.read_blob(&manifest_digest) {
        Ok(b) => b,
        Err(e) => return fail(e),
    };
    let manifest: podbox_image::oci::Manifest = match serde_json::from_slice(&bytes) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "podbox extract: the stored manifest for {want} does not parse: {e}. \
                 Re-pull the image; the blob is present and is not a manifest."
            );
            return podbox_image::error::EXIT_RUNTIME_ERROR;
        }
    };

    // ⛔ A GC must not delete the blobs out from under an extraction in flight,
    // and T-0204's lock is what stops it.
    let _held = match store.hold(&record) {
        Ok(l) => Some(l),
        Err(e) => {
            eprintln!("podbox extract: {e}");
            return podbox_image::error::EXIT_RUNTIME_ERROR;
        }
    };

    let mut out = std::io::stderr().lock();
    match podbox_extract::extract(&store, &manifest, &record.manifest_digest, &mut out) {
        Ok(done) => {
            // ⚠ The estimate flag travels with the number, every time.
            let sized = if done.uncompressed_estimated {
                format!("{} (estimated)", space::mib(done.uncompressed_bytes))
            } else {
                space::mib(done.uncompressed_bytes)
            };
            let _ = writeln!(
                out,
                "podbox: {} layers, {} entries, {} removed by whiteouts, {sized} uncompressed",
                done.layers, done.entries, done.removed
            );
            if done.ownership_dropped > 0 {
                crate::diagnose::report_dropped(&mut out, &done, &podbox_probe::identity::read());
            }
            if done.skipped > 0 {
                let _ = writeln!(
                    out,
                    "podbox: {} entries were not materialised ({}). podbox cannot \
                     mknod on this class of runtime, which is the premise of the \
                     whole tool rather than a defect here",
                    done.skipped,
                    done.skipped_kinds.join(", ")
                );
            }
            // ⛔ stdout carries the answer and nothing else, so this verb
            // composes. T-0110 settled that channel contract.
            println!("{}", done.rootfs.display());
            0
        }
        Err(e) => {
            eprintln!("podbox extract: {e}");
            podbox_image::error::EXIT_RUNTIME_ERROR
        }
    }
}

pub const SAVE_USAGE: &str = "\
usage: podbox save [--output FILE] <image>

  -o, --output F  write the tarball to F instead of stdout.

  Write one image as an OCI-layout tarball: oci-layout, index.json and
  the blobs the record names, each verified on the way out. The tarball
  goes to stdout (or F); human chatter to stderr, so the stream composes.
  `podbox load` reads it back with every blob verified on the way in
  (TODO/image.md T-1320).
";

pub const LOAD_USAGE: &str = "\
usage: podbox load [--input FILE]

  -i, --input F   read the tarball from F instead of stdin.

  Read an OCI-layout tarball `podbox save` wrote: every blob is hashed
  against its descriptor before it is committed, and the record is
  registered under the tarball's own ref-name annotation. A tarball whose
  bytes changed in transit is refused the way a bad pull is
  (TODO/image.md T-1320).
";

pub const IMPORT_USAGE: &str = "\
usage: podbox import <rootfs.tar> [REPOSITORY[:TAG]]

  Build a runnable record from a plain rootfs tar: the tar becomes the
  image's only layer, and the config and manifest are synthesized around
  its digest. A compressed file is refused by name, because the record's
  media type promises a plain tar. Unnamed imports are recorded as
  `imported` with no tag (TODO/image.md T-1320).
";

/// What `save` was asked for.
struct SaveArgs {
    output: Option<String>,
    want: String,
}

fn parse_save(args: &[String]) -> std::result::Result<SaveArgs, i32> {
    let mut output = None;
    let mut want: Option<String> = None;
    let mut expect_output = false;
    for a in args {
        if expect_output {
            output = Some(a.clone());
            expect_output = false;
            continue;
        }
        match a.as_str() {
            "-h" | "--help" => {
                print!("{SAVE_USAGE}");
                return Err(0);
            }
            "-o" | "--output" => expect_output = true,
            other if other.starts_with("--output=") => {
                output = Some(other["--output=".len()..].to_string())
            }
            other if other.starts_with('-') => {
                crate::parity::admit("save", other, SAVE_USAGE)?;
                return Err(crate::parity::no_arm("save", other));
            }
            other if want.is_none() => want = Some(other.to_string()),
            other => {
                eprintln!("podbox save: {other:?}: save takes one image");
                return Err(EXIT_CLI_ERROR);
            }
        }
    }
    if expect_output {
        eprintln!("podbox save: --output needs a file");
        return Err(EXIT_FLAG_ERROR);
    }
    let Some(want) = want else {
        print!("{SAVE_USAGE}");
        return Err(EXIT_CLI_ERROR);
    };
    Ok(SaveArgs { output, want })
}

/// What `load` was asked for.
struct LoadArgs {
    input: Option<String>,
}

fn parse_load(args: &[String]) -> std::result::Result<LoadArgs, i32> {
    let mut input = None;
    let mut expect_input = false;
    for a in args {
        if expect_input {
            input = Some(a.clone());
            expect_input = false;
            continue;
        }
        match a.as_str() {
            "-h" | "--help" => {
                print!("{LOAD_USAGE}");
                return Err(0);
            }
            "-i" | "--input" => expect_input = true,
            other if other.starts_with("--input=") => {
                input = Some(other["--input=".len()..].to_string())
            }
            other if other.starts_with('-') => {
                crate::parity::admit("load", other, LOAD_USAGE)?;
                return Err(crate::parity::no_arm("load", other));
            }
            other => {
                eprintln!("podbox load: {other:?}: load takes no image argument, only --input");
                return Err(EXIT_CLI_ERROR);
            }
        }
    }
    if expect_input {
        eprintln!("podbox load: --input needs a file");
        return Err(EXIT_FLAG_ERROR);
    }
    Ok(LoadArgs { input })
}

/// What `import` was asked for.
struct ImportArgs {
    tar: String,
    reference: Option<String>,
}

fn parse_import(args: &[String]) -> std::result::Result<ImportArgs, i32> {
    let mut positionals: Vec<String> = Vec::new();
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{IMPORT_USAGE}");
                return Err(0);
            }
            other if other.starts_with('-') => {
                crate::parity::admit("import", other, IMPORT_USAGE)?;
                return Err(crate::parity::no_arm("import", other));
            }
            other => positionals.push(other.to_string()),
        }
    }
    if positionals.len() > 2 {
        eprintln!("podbox import: import takes a tarball and at most one name");
        return Err(EXIT_CLI_ERROR);
    }
    let mut it = positionals.into_iter();
    let Some(tar) = it.next() else {
        print!("{IMPORT_USAGE}");
        return Err(EXIT_CLI_ERROR);
    };
    let reference = it.next();
    Ok(ImportArgs { tar, reference })
}

pub fn save(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, SAVE_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, SAVE_USAGE) {
        return c;
    }
    let o = match parse_save(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    // ⛔ stdout carries the tarball and nothing else where no file was
    // given, so the stream composes. T-0110 settled that channel contract.
    if let Some(path) = o.output {
        let mut file = match std::fs::File::create(&path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("podbox save: {path}: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        };
        match podbox_image::layout::save(&store, &o.want, &mut file) {
            Ok(done) => {
                eprintln!(
                    "podbox save: saved {} as {} blobs ({} bytes), digest {}",
                    o.want,
                    done.blobs,
                    podbox_image::space::mib(done.bytes),
                    done.record.manifest_digest
                );
                println!("{path}");
                0
            }
            Err(e) => {
                eprintln!("podbox save: {e}");
                // ⚠ A half-written tarball is a trap for the next load, so a
                // failed save removes what it started.
                let _ = std::fs::remove_file(&path);
                EXIT_RUNTIME_ERROR
            }
        }
    } else {
        let mut out = std::io::stdout().lock();
        match podbox_image::layout::save(&store, &o.want, &mut out) {
            Ok(done) => {
                eprintln!(
                    "podbox save: saved {} as {} blobs ({} bytes), digest {}",
                    o.want,
                    done.blobs,
                    podbox_image::space::mib(done.bytes),
                    done.record.manifest_digest
                );
                0
            }
            Err(e) => {
                eprintln!("podbox save: {e}");
                EXIT_RUNTIME_ERROR
            }
        }
    }
}

pub fn load(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, LOAD_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, LOAD_USAGE) {
        return c;
    }
    let o = match parse_load(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    // ⚠ stdin spills to a file first: `load` needs a path, and a tarball
    // is streamed, never held fully in memory for the copy.
    let file_arg: Option<std::path::PathBuf> = o.input.map(std::path::PathBuf::from);
    let temp: Option<std::path::PathBuf>;
    let src: &std::path::Path = match &file_arg {
        Some(p) => {
            temp = None;
            p
        }
        None => {
            let path = std::env::temp_dir().join(format!("podbox-load-in-{}", std::process::id()));
            let mut stdin = std::io::stdin().lock();
            let mut file = match std::fs::File::create(&path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("podbox load: {path:?}: {e}");
                    return EXIT_RUNTIME_ERROR;
                }
            };
            if let Err(e) = std::io::copy(&mut stdin, &mut file) {
                eprintln!("podbox load: stdin does not read: {e}");
                let _ = std::fs::remove_file(&path);
                return EXIT_RUNTIME_ERROR;
            }
            temp = Some(path);
            temp.as_ref().expect("just set")
        }
    };
    let code = match podbox_image::layout::load(&store, src) {
        Ok(record) => {
            println!("Loaded image: {}", loaded_name(&record));
            0
        }
        Err(e) => {
            eprintln!("podbox load: {e}");
            EXIT_RUNTIME_ERROR
        }
    };
    if let Some(path) = temp {
        let _ = std::fs::remove_file(&path);
    }
    code
}

pub fn import(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, IMPORT_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, IMPORT_USAGE) {
        return c;
    }
    let o = match parse_import(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    match podbox_image::layout::import(&store, std::path::Path::new(&o.tar), o.reference.as_deref())
    {
        Ok(record) => {
            println!("Loaded image: {}", loaded_name(&record));
            0
        }
        Err(e) => {
            eprintln!("podbox import: {e}");
            EXIT_RUNTIME_ERROR
        }
    }
}

/// The `Loaded image:` line both verbs print: the ref-name where one was
/// recorded, the manifest digest where none was.
fn loaded_name(record: &podbox_image::Record) -> String {
    match &record.tag {
        Some(tag) => format!("{}:{tag}", record.repository),
        None => record.manifest_digest.clone(),
    }
}

pub const VERIFY_USAGE: &str = "\
usage: podbox verify [image|all]

  No argument (or `all`): hash every indexed blob of every record against
  its digest. One image: print its provenance first, then sweep its blobs.

  One line per mismatch plus a summary, and a non-zero exit where any
  blob fails. Verify reports; it never refetches (TODO/image.md T-1321).
";

/// What `verify` was asked for.
struct VerifyArgs {
    want: Option<String>,
}

fn parse_verify(args: &[String]) -> std::result::Result<VerifyArgs, i32> {
    let mut positionals: Vec<String> = Vec::new();
    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{VERIFY_USAGE}");
                return Err(0);
            }
            other if other.starts_with('-') => {
                crate::parity::admit("verify", other, VERIFY_USAGE)?;
                return Err(crate::parity::no_arm("verify", other));
            }
            other => positionals.push(other.to_string()),
        }
    }
    if positionals.len() > 1 {
        eprintln!("podbox verify: verify takes one image or `all`");
        return Err(EXIT_CLI_ERROR);
    }
    Ok(VerifyArgs {
        want: positionals.into_iter().next(),
    })
}

pub fn verify(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, VERIFY_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, VERIFY_USAGE) {
        return c;
    }
    let o = match parse_verify(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let records: Vec<podbox_image::Record> = match o.want.as_deref() {
        None | Some("all") => match store.list() {
            Ok(r) => r,
            Err(e) => return fail(e),
        },
        Some(want) => match store.find_one(want) {
            Ok(r) => vec![r],
            Err(e) => {
                eprintln!("podbox verify: {e}");
                return e.exit_code();
            }
        },
    };
    let single = o.want.as_deref().is_some_and(|w| w != "all");
    let mut checked = 0usize;
    let mut mismatched = 0usize;
    let mut code = 0;
    for record in &records {
        if single {
            print_provenance(&store, record);
        }
        let name = match &record.tag {
            Some(tag) => format!("{}:{tag}", record.repository),
            None => record.repository.clone(),
        };
        let digests: Vec<String> = record.blobs().iter().map(|s| s.to_string()).collect();
        match store.verify_blobs(&digests) {
            Ok(hits) => {
                checked += digests.len();
                mismatched += hits.len();
                for hit in &hits {
                    println!("MISMATCH {name} {}: computed {}", hit.want, hit.got);
                }
                if hits.is_empty() {
                    println!("OK {name} ({} blobs)", digests.len());
                } else {
                    code = EXIT_RUNTIME_ERROR;
                }
            }
            Err(e) => {
                eprintln!("podbox verify: {e}");
                code = EXIT_RUNTIME_ERROR;
            }
        }
    }
    println!("verify: {checked} blobs checked, {mismatched} mismatched");
    code
}

/// The provenance half of `verify <image>`: how this record's bytes got
/// here, or the honest absence where no pull recorded one.
fn print_provenance(store: &Store, record: &podbox_image::Record) {
    match store.read_provenance() {
        Ok(lines) => {
            let mut shown = 0;
            for p in lines
                .iter()
                .filter(|p| p.manifest_digest == record.manifest_digest)
            {
                println!("registry: {}", p.registry);
                println!("repository: {}", p.repository);
                println!("tag: {}", p.tag.as_deref().unwrap_or("(untagged)"));
                println!("manifest: {}", p.manifest_digest);
                println!("pulled-at: {}", p.pulled_at);
                println!("podbox-version: {}", p.podbox_version);
                shown += 1;
            }
            if shown == 0 {
                println!(
                    "no provenance line for {} (recorded before T-1321, imported, or loaded)",
                    record.manifest_digest
                );
            }
        }
        Err(e) => eprintln!("podbox verify: {e}"),
    }
}

pub fn inspect(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, INSPECT_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, INSPECT_USAGE) {
        return c;
    }
    let mut template: Option<String> = None;
    let mut wanted: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{INSPECT_USAGE}");
                return 0;
            }
            "-f" | "--format" => match it.next() {
                Some(t) => template = Some(t.clone()),
                None => {
                    eprintln!("podbox inspect: --format needs a template");
                    return EXIT_FLAG_ERROR;
                }
            },
            other if other.starts_with("--format=") => {
                template = Some(other["--format=".len()..].to_string())
            }
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, INSPECT_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other => wanted.push(other),
        }
    }
    if wanted.is_empty() {
        eprint!("{INSPECT_USAGE}");
        return EXIT_CLI_ERROR;
    }
    // ⛔ Before the store is even opened. Every reference may fail to resolve,
    // and a template checked only inside the loop over what DID resolve is
    // never checked at all. It is also the caller's own input, so it is
    // reported before anything about the machine is.
    // ⚠ Checked against the IMAGE fields here and against the container's in
    // `lifecycle::inspect_container`, because the two kinds have different
    // fields and a union would accept a name neither of them has.
    if let Some(t) = &template {
        if format::check(t, INSPECT_FIELDS).is_err()
            && format::check(t, crate::lifecycle::CONTAINER_INSPECT_FIELDS).is_err()
        {
            let bad = format::check(t, INSPECT_FIELDS).unwrap_err();
            eprintln!("podbox inspect: {bad}");
            return EXIT_CLI_ERROR;
        }
    }
    let store = match podbox_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };

    let mut records = Vec::new();
    let mut code = 0;
    // ⭐ TODO/cli.md T-1319. A reference that resolved to a container already
    // printed its one document inside `inspect_container`, so the image array
    // below is skipped on that path: printing it as well is a second document
    // no JSON parser can consume beside the first.
    let mut saw_container = false;
    for want in &wanted {
        match store.find_one(want) {
            Ok(r) => records.push(r),
            Err(e) => {
                // ⭐ M4. An image FIRST, then a container, which is docker's own
                // order. ⚠ The two have different fields, so a template is
                // checked against whichever kind the reference resolved to
                // rather than against a union neither has.
                match crate::lifecycle::inspect_container(want, template.as_deref()) {
                    Some(0) => saw_container = true,
                    Some(c) => {
                        saw_container = true;
                        code = c;
                    }
                    None => {
                        eprintln!("podbox inspect: {e}");
                        code = e.exit_code();
                    }
                }
            }
        }
    }
    if let Some(t) = &template {
        for r in &records {
            match format::render(t, &inspect_fields(r, &store)) {
                Ok(line) => println!("{line}"),
                Err(bad) => {
                    eprintln!("podbox inspect: {bad}");
                    return EXIT_CLI_ERROR;
                }
            }
        }
        return code;
    }
    let docs: Vec<String> = records.iter().map(|r| inspect_json(r, &store)).collect();
    // ⭐ TODO/cli.md T-1319. Where every reference resolved to a container,
    // its document is already on stdout and the empty image array is not
    // printed after it. Where an image resolved, the array below is byte for
    // byte what it has always been.
    if records.is_empty() && saw_container {
        return code;
    }
    println!("[{}]", docs.join(","));
    code
}

// ------------------------------------------------------------------- fields

/// The field names `podbox images --format` answers to.
///
/// ⚠ These names exist so a template can be validated with NO record in hand,
/// which is what an empty store leaves the caller with. The test
/// `the_field_names_are_the_names_the_builder_produces` is what stops this list
/// and the builder below drifting: `docs/conventions/forbidden-patterns.md`
/// forbids a value in two places with no check that they agree.
pub const IMAGE_FIELDS: &[&str] = &[
    "Repository",
    "Tag",
    "ID",
    "Digest",
    "CreatedSince",
    "CreatedAt",
    "Size",
    "Platform",
    "Store",
];

/// The same, for `podbox inspect --format`.
pub const INSPECT_FIELDS: &[&str] = &[
    "Id",
    "Digest",
    "RepoTags",
    "RepoDigests",
    "Architecture",
    "Os",
    "Created",
    "Platform",
    "Size",
    "Store",
    "Layers",
    // ⭐ M2. `TODO/extract.md` T-0302's Prove reads the sidecar at
    // `<RootfsPath>/../.meta.jsonl`, so this field is what makes that command
    // writable at all.
    "RootfsPath",
    "Extracted",
    // ⛔ TODO/enter.md T-0505. What `podbox exec` against this image WOULD be,
    // and not a property of the image. It is constant because podbox has one
    // exec mechanism; when M4 gives a container its own record the same two
    // names sit on that record and answer from the container that was entered.
    "Exec.Mode",
    "Exec.Shares",
];

/// `podbox exec`'s mode, in one word each, for `inspect` and for the banner.
///
/// ⛔ TODO/enter.md T-0505. `docker exec` enters the container's namespaces.
/// podbox has none to enter, so its `exec` re-runs the section 6.5 sequence
/// against the same rootfs: it shares the filesystem tree and NOTHING else, not
/// the process table, not `/proc`, not signals, not the original's environment.
/// A caller reads these rather than assuming.
pub const EXEC_MODE: &str = "fresh-chroot";
pub const EXEC_SHARES: &str = "filesystem";

fn image_fields(r: &Record, store: &Store, no_trunc: bool) -> Vec<(&'static str, String)> {
    let id = r.config_digest.clone();
    let short_id = id
        .strip_prefix("sha256:")
        .map(|h| h[..12.min(h.len())].to_string())
        .unwrap_or_else(|| id.clone());
    vec![
        ("Repository", r.display_repository()),
        ("Tag", r.tag.clone().unwrap_or_else(|| "<none>".into())),
        ("ID", if no_trunc { id } else { short_id }),
        ("Digest", r.digest.clone()),
        (
            "CreatedSince",
            r.created
                .as_deref()
                .and_then(clock::since)
                // ⛔ A dash where the value is unknown, never a fabricated one.
                .unwrap_or_else(|| "-".into()),
        ),
        ("CreatedAt", r.created.clone().unwrap_or_else(|| "-".into())),
        ("Size", space::mib(r.stored_bytes)),
        ("Platform", r.platform.clone()),
        ("Store", store.root().display().to_string()),
    ]
}

fn inspect_fields(r: &Record, store: &Store) -> Vec<(&'static str, String)> {
    vec![
        ("Id", r.config_digest.clone()),
        ("Digest", r.digest.clone()),
        (
            "RepoTags",
            r.tag.as_ref().map(|_| r.name()).unwrap_or_default(),
        ),
        (
            "RepoDigests",
            format!("{}@{}", r.display_repository(), r.digest),
        ),
        ("Architecture", r.architecture.clone()),
        ("Os", r.os.clone()),
        ("Created", r.created.clone().unwrap_or_else(|| "-".into())),
        ("Platform", r.platform.clone()),
        ("Size", r.stored_bytes.to_string()),
        ("Store", store.root().display().to_string()),
        ("Layers", r.layers.join(" ")),
        ("RootfsPath", {
            let (rootfs, _) = podbox_extract::paths(store, &r.manifest_digest);
            rootfs.display().to_string()
        }),
        // ⚠ Whether the rootfs is THERE, which is a different question from
        // where it would be. `inspect` on a pulled-but-unextracted image must
        // not imply a tree that does not exist.
        (
            "Extracted",
            podbox_extract::is_extracted(store, &r.manifest_digest).to_string(),
        ),
        ("Exec.Mode", EXEC_MODE.to_string()),
        ("Exec.Shares", EXEC_SHARES.to_string()),
    ]
}

fn inspect_json(r: &Record, store: &Store) -> String {
    // ⚠ Built through serde_json rather than by concatenation, so a repository
    // or a tag carrying a quote cannot break the document.
    serde_json::json!({
        "Id": r.config_digest,
        "Digest": r.digest,
        "RepoTags": r.tag.as_ref().map(|_| vec![r.name()]).unwrap_or_default(),
        "RepoDigests": [format!("{}@{}", r.display_repository(), r.digest)],
        "Architecture": r.architecture,
        "Os": r.os,
        "Created": r.created,
        "Platform": r.platform,
        "Size": r.stored_bytes,
        "Store": store.root().display().to_string(),
        "Layers": r.layers,
        "ManifestDigest": r.manifest_digest,
        "PulledAt": r.pulled_at,
        // ⛔ T-0505, and nested here because it is nested in --format too. A
        // caller that reads one and not the other must not find two shapes.
        "Exec": { "Mode": EXEC_MODE, "Shares": EXEC_SHARES },
    })
    .to_string()
}

fn pick(fields: &[(&str, String)], key: &str) -> String {
    fields
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// docker's two-space-padded columns.
pub(crate) fn table(rows: &[Vec<String>]) -> String {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0usize; columns];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let mut out = String::new();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i + 1 == row.len() {
                out.push_str(cell);
            } else {
                out.push_str(cell);
                out.push_str(&" ".repeat(widths[i] - cell.chars().count() + 3));
            }
        }
        out.push('\n');
    }
    out
}

fn unknown(verb: &str, flag: &str, usage: &str) -> i32 {
    eprintln!("podbox {verb}: unknown option {flag:?}");
    eprint!("{usage}");
    EXIT_FLAG_ERROR
}

fn fail(e: Error) -> i32 {
    eprintln!("podbox: {e}");
    e.exit_code()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TODO/image.md T-1320. save/load/import parse their own surface and
    /// nothing else's.
    #[test]
    fn save_load_import_parse_their_flags() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let o = parse_save(&v(&["-o", "f.tar", "img:tag"])).unwrap();
        assert_eq!(o.output.as_deref(), Some("f.tar"));
        assert_eq!(o.want, "img:tag");
        let o = parse_save(&v(&["--output=f.tar", "img"])).unwrap();
        assert_eq!(o.output.as_deref(), Some("f.tar"));
        assert!(parse_save(&v(&[])).is_err());
        assert!(parse_save(&v(&["a", "b"])).is_err());
        let o = parse_load(&v(&["-i", "f.tar"])).unwrap();
        assert_eq!(o.input.as_deref(), Some("f.tar"));
        let o = parse_load(&v(&[])).unwrap();
        assert_eq!(o.input, None);
        let o = parse_import(&v(&["root.tar", "me:v1"])).unwrap();
        assert_eq!(o.tar, "root.tar");
        assert_eq!(o.reference.as_deref(), Some("me:v1"));
        let o = parse_import(&v(&["root.tar"])).unwrap();
        assert_eq!(o.reference, None);
        assert!(parse_import(&v(&[])).is_err());
    }

    /// TODO/image.md T-1321. `verify` takes one image, `all`, or nothing.
    #[test]
    fn verify_parses_one_image_all_or_nothing() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let o = parse_verify(&v(&["img:tag"])).unwrap();
        assert_eq!(o.want.as_deref(), Some("img:tag"));
        let o = parse_verify(&v(&["all"])).unwrap();
        assert_eq!(o.want.as_deref(), Some("all"));
        let o = parse_verify(&v(&[])).unwrap();
        assert_eq!(o.want, None);
        assert!(parse_verify(&v(&["a", "b"])).is_err());
    }

    #[test]
    fn the_table_pads_every_column_to_its_widest_cell() {
        let rows = vec![
            vec!["REPOSITORY".to_string(), "TAG".to_string()],
            vec!["alpine".to_string(), "latest".to_string()],
        ];
        let got = table(&rows);
        let lines: Vec<&str> = got.lines().collect();
        assert!(lines[0].starts_with("REPOSITORY   TAG"), "{got}");
        assert!(lines[1].starts_with("alpine       latest"), "{got}");
        // ⚠ No trailing padding on the last column: a trailing run of spaces
        // is invisible in review and breaks an exact-match acceptance.
        assert!(!lines[1].ends_with(' '), "{got:?}");
    }

    #[test]
    fn the_field_names_are_the_names_the_builder_produces() {
        // ⛔ The remedy docs/conventions/forbidden-patterns.md names for a value
        // in two places: a check that they agree. The list is what validates a
        // template with no record in hand; the builder is what fills one in.
        let store =
            Store::open(std::env::temp_dir().join(format!("podbox-names-{}", std::process::id())))
                .unwrap();
        let r = a_record();
        let built: Vec<&str> = image_fields(&r, &store, false)
            .iter()
            .map(|(k, _)| *k)
            .collect();
        assert_eq!(built, IMAGE_FIELDS);
        let built: Vec<&str> = inspect_fields(&r, &store).iter().map(|(k, _)| *k).collect();
        assert_eq!(built, INSPECT_FIELDS);
        let _ = std::fs::remove_dir_all(store.root());
    }

    #[test]
    fn a_template_naming_a_field_no_verb_has_is_refused_without_any_record() {
        // ⭐ The defect this pass found: with the check inside the loop, an
        // empty store made `--format '{{.Nope}}'` print nothing and exit 0.
        assert!(format::check("{{.Nope}}", IMAGE_FIELDS).is_err());
        assert!(format::check("{{.Tag", IMAGE_FIELDS).is_err());
        assert!(format::check("table {{.Tag}}", IMAGE_FIELDS).is_err());
        assert!(format::check("{{.Digest}}", IMAGE_FIELDS).is_ok());
        // ⚠ The two verbs do not have the same fields, and each is checked
        // against its own list.
        assert!(format::check("{{.RepoTags}}", IMAGE_FIELDS).is_err());
        assert!(format::check("{{.RepoTags}}", INSPECT_FIELDS).is_ok());
    }

    fn a_record() -> Record {
        Record {
            repository: "docker.io/library/alpine".into(),
            tag: Some("latest".into()),
            digest: format!("sha256:{}", "1".repeat(64)),
            digest_media_type: podbox_image::oci::MEDIA_OCI_INDEX.into(),
            manifest_digest: format!("sha256:{}", "2".repeat(64)),
            config_digest: format!("sha256:{}", "3".repeat(64)),
            platform: "linux/amd64".into(),
            layers: vec![],
            stored_bytes: 0,
            architecture: "amd64".into(),
            os: "linux".into(),
            created: None,
            pulled_at: clock::now(),
        }
    }

    #[test]
    fn an_image_with_no_creation_timestamp_prints_a_dash_and_not_a_guess() {
        // ⛔ AGENTS.md absolute 3.
        let store =
            Store::open(std::env::temp_dir().join(format!("podbox-fields-{}", std::process::id())))
                .unwrap();
        let f = image_fields(&a_record(), &store, false);
        assert_eq!(pick(&f, "CreatedSince"), "-");
        assert_eq!(pick(&f, "CreatedAt"), "-");
        assert_eq!(pick(&f, "ID"), "333333333333");
        assert_eq!(pick(&f, "Repository"), "alpine");
        let _ = std::fs::remove_dir_all(store.root());
    }

    fn login_args(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn login_takes_a_user_a_stdin_flag_and_a_server() {
        let got = parse_login_args(
            "login",
            &login_args(&["-u", "bob", "--password-stdin", "example.com:5000"]),
        )
        .expect("parses");
        assert_eq!(got.username, "bob");
        assert_eq!(got.server.as_deref(), Some("example.com:5000"));
        assert!(!got.help);
    }

    #[test]
    fn login_names_no_server_where_none_is_given() {
        // The caller defaults it to docker's canonical key.
        let got = parse_login_args(
            "login",
            &login_args(&["--username", "bob", "--password-stdin"]),
        )
        .expect("parses");
        assert!(got.server.is_none());
    }

    #[test]
    fn login_accepts_the_equals_forms() {
        // ⛔ `admit` cuts at `=` before the table lookup, so these already
        // passed `admit_all`: without an arm they would read as a defect in
        // podbox rather than a value it accepts.
        let got = parse_login_args(
            "login",
            &login_args(&["--username=bob", "--password-stdin"]),
        )
        .expect("parses");
        assert_eq!(got.username, "bob");
        let got = parse_login_args("login", &login_args(&["-u=bob", "--password-stdin"]))
            .expect("parses");
        assert_eq!(got.username, "bob");
    }

    #[test]
    fn login_help_needs_nothing_else() {
        let got = parse_login_args("login", &login_args(&["--help"])).expect("parses");
        assert!(got.help);
    }

    #[test]
    fn login_without_a_user_is_a_flag_refusal() {
        // TODO/cli.md T-0802: a flag-shape refusal is 125.
        assert_eq!(
            parse_login_args("login", &login_args(&["--password-stdin"])).expect_err("no user"),
            EXIT_FLAG_ERROR
        );
    }

    #[test]
    fn login_without_password_stdin_is_a_flag_refusal() {
        // Podbox never prompts, so the flag is required, not defaulted.
        assert_eq!(
            parse_login_args("login", &login_args(&["-u", "bob"])).expect_err("no stdin flag"),
            EXIT_FLAG_ERROR
        );
    }

    #[test]
    fn login_with_a_dangling_user_flag_is_a_flag_refusal() {
        assert_eq!(
            parse_login_args("login", &login_args(&["-u"])).expect_err("dangling"),
            EXIT_FLAG_ERROR
        );
    }

    #[test]
    fn login_with_an_empty_user_is_a_flag_refusal() {
        // The read path skips entries with no user, so storing one would
        // write a login that never reads back.
        assert_eq!(
            parse_login_args("login", &login_args(&["-u", "", "--password-stdin"]))
                .expect_err("empty user"),
            EXIT_FLAG_ERROR
        );
        assert_eq!(
            parse_login_args("login", &login_args(&["--username=", "--password-stdin"]))
                .expect_err("empty equals user"),
            EXIT_FLAG_ERROR
        );
    }

    #[test]
    fn login_with_two_servers_is_a_verb_refusal() {
        // TODO/cli.md T-0802: the verb refusing afterwards is 1.
        assert_eq!(
            parse_login_args(
                "login",
                &login_args(&["-u", "bob", "--password-stdin", "a.example", "b.example"])
            )
            .expect_err("two servers"),
            EXIT_CLI_ERROR
        );
    }

    #[test]
    fn login_with_an_unlisted_flag_is_a_flag_refusal() {
        assert_eq!(
            parse_login_args(
                "login",
                &login_args(&["-u", "bob", "--password-stdin", "--quiet"])
            )
            .expect_err("unlisted"),
            EXIT_FLAG_ERROR
        );
    }
}
