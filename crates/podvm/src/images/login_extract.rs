use super::*;

#[derive(Debug)]
pub(super) struct LoginArgs {
    pub(super) username: String,
    pub(super) server: Option<String>,
    pub(super) help: bool,
}

/// The argument shape, pure so tests own it. The password never appears
/// here: it arrives on stdin after parsing, so no usage string, log line,
/// or test fixture ever carries one. Unknown dash flags are refused by the
/// caller through the parity table, not here: this function sees only what
/// `admit_all` accepted.
pub(super) fn parse_login_args(verb: &str, args: &[String]) -> Result<LoginArgs, i32> {
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
            // and read as a defect in podvm rather than a value it accepts.
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

/// `podvm logout`.
pub fn logout(verb: &str, args: &[String]) -> i32 {
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, LOGOUT_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, LOGOUT_USAGE) {
        return c;
    }
    let parsed = match parse_logout_args(verb, args) {
        Ok(p) => p,
        Err(c) => {
            eprint!("{LOGOUT_USAGE}");
            return c;
        }
    };
    if parsed.help {
        print!("{LOGOUT_USAGE}");
        return 0;
    }
    let server = parsed
        .server
        .as_deref()
        .unwrap_or_else(|| crate::images::default_login_server());
    match credentials::remove_login(server) {
        Ok(true) => {
            println!(
                "Removing login credentials for {}",
                credentials::server_host(server)
            );
            0
        }
        // Still logged out where nothing was stored, and says so: a
        // typo'd server reads as a no-op rather than a success, so the
        // refusal (not the runtime error: nothing failed to run).
        Ok(false) => {
            eprintln!(
                "podvm {verb}: not logged in to {}",
                credentials::server_host(server)
            );
            EXIT_CLI_ERROR
        }
        Err(e) => {
            eprintln!("podvm {verb}: {e}");
            EXIT_RUNTIME_ERROR
        }
    }
}

#[derive(Debug)]
pub(super) struct LogoutArgs {
    pub(super) server: Option<String>,
    pub(super) help: bool,
}

/// The argument shape, pure so tests own it: one optional server, help,
/// and nothing else. Unknown dash flags are refused by the caller
/// through the parity table, not here.
pub(super) fn parse_logout_args(verb: &str, args: &[String]) -> Result<LogoutArgs, i32> {
    let mut server: Option<String> = None;
    let mut help = false;
    for a in args {
        match a.as_str() {
            "-h" | "--help" => help = true,
            other if other.starts_with('-') => {
                crate::parity::admit(verb, other, LOGOUT_USAGE)?;
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
        return Ok(LogoutArgs {
            server: None,
            help: true,
        });
    }
    Ok(LogoutArgs { server, help })
}

/// `podvm image prune`.
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
    let store = match podvm_image::open_store() {
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
    let mut rest: Vec<podvm_image::Record> = Vec::new();
    let mut referenced: Vec<(String, Vec<String>)> = Vec::new();
    for record in candidates {
        match podvm_supervise::referencing(&store, &record.manifest_digest) {
            Ok(found) if !found.is_empty() => {
                let mut names: Vec<String> = found.iter().map(|c| c.name.clone()).collect();
                names.sort();
                referenced.push((record.name(), names));
            }
            Ok(_) => rest.push(record),
            Err(e) => {
                eprintln!("podvm image prune: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        }
    }
    match store.delete(&rest, podvm_image::store::Held::Skip) {
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

/// `podvm inspect`, for images. Containers are M3.
/// `podvm extract <image>`: `TODO/extract.md` T-0301 to T-0307, milestone M2.
///
/// ⛔ **This is the verb M2 can drive.** [`TODO/milestones.md`](../../../TODO/milestones.md)
/// T-1103's acceptance runs `podvm run --rm`, which is M3, so extraction would
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
                eprintln!("podvm extract: {other:?}: extract takes one image");
                return EXIT_CLI_ERROR;
            }
        }
    }
    if expect_platform {
        eprintln!("podvm extract: --platform needs a value, for example linux/arm64");
        return EXIT_FLAG_ERROR;
    }
    let Some(want) = want else {
        eprint!("{EXTRACT_USAGE}");
        return EXIT_CLI_ERROR;
    };
    // ⚠ `None` where no flag was given, so `find_one_for` prefers the host's
    // platform and refuses only where that leaves more than one.
    let platform = match platform_flag.as_deref() {
        Some(p) => match podvm_image::platform::Platform::parse(p) {
            Ok(p) => Some(p),
            Err(e) => return fail(e.into()),
        },
        None => None,
    };

    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let record = match store.find_one_for(want, platform.as_ref()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("podvm extract: {e}");
            return e.exit_code();
        }
    };

    if podvm_extract::is_extracted(&store, &record.manifest_digest) && !force {
        // ⚠ Already done is not an error, and the path is still printed: a
        // caller pipes this into `run`, and a second invocation must answer the
        // same thing as the first.
        let (rootfs, _) = podvm_extract::paths(&store, &record.manifest_digest);
        println!("{}", rootfs.display());
        return 0;
    }
    if force && let Err(e) = podvm_extract::remove_extracted(&store, &record.manifest_digest) {
        eprintln!("podvm extract: {e}");
        return podvm_image::error::EXIT_RUNTIME_ERROR;
    }

    let manifest_digest = match podvm_image::digest::Digest::parse(&record.manifest_digest) {
        Ok(d) => d,
        Err(e) => return fail(e.into()),
    };
    let bytes = match store.read_blob(&manifest_digest) {
        Ok(b) => b,
        Err(e) => return fail(e),
    };
    let manifest: podvm_image::oci::Manifest = match serde_json::from_slice(&bytes) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "podvm extract: the stored manifest for {want} does not parse: {e}. \
                 Re-pull the image; the blob is present and is not a manifest."
            );
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };

    // ⛔ A GC must not delete the blobs out from under an extraction in flight,
    // and T-0204's lock is what stops it.
    let _held = match store.hold(&record) {
        Ok(l) => Some(l),
        Err(e) => {
            eprintln!("podvm extract: {e}");
            return podvm_image::error::EXIT_RUNTIME_ERROR;
        }
    };

    let mut out = std::io::stderr().lock();
    match podvm_extract::extract(&store, &manifest, &record.manifest_digest, &mut out) {
        Ok(done) => {
            // ⚠ The estimate flag travels with the number, every time.
            let sized = if done.uncompressed_estimated {
                format!("{} (estimated)", space::mib(done.uncompressed_bytes))
            } else {
                space::mib(done.uncompressed_bytes)
            };
            let _ = writeln!(
                out,
                "podvm: {} layers, {} entries, {} removed by whiteouts, {sized} uncompressed",
                done.layers, done.entries, done.removed
            );
            if done.ownership_dropped > 0 {
                crate::diagnose::report_dropped(
                    &mut out,
                    &done,
                    &podvm_probe::identity::read(),
                    true,
                );
            }
            if done.skipped > 0 {
                let _ = writeln!(
                    out,
                    "podvm: {} entries were not materialised ({}). podvm cannot \
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
            eprintln!("podvm extract: {e}");
            podvm_image::error::EXIT_RUNTIME_ERROR
        }
    }
}
