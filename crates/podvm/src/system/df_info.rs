use super::*;

/// `podvm system df`.
///
/// ⭐ [`TODO/cli.md`](../../../TODO/cli.md) T-1337's answer to `docker
/// system df`, as data rather than prose. One row per image with its
/// stored blob bytes beside its extracted rootfs bytes, the container
/// count, and what `image prune` would reclaim.
///
/// ⛔ Three properties the drive pins. First, the reclaimable number is
/// the number `image prune` would free on a quiet store: the same gate
/// chain (`prune_candidates(false)`, the `referencing` record-gate,
/// the `in_use` hold check) with `reclaimable_bytes` standing in for
/// `delete`. Second, accounting never fails the verb: an unreadable
/// file counts nothing. Third, only a gate failure (an unopenable
/// store, a lock that will not answer) exits non-zero; the bytes
/// themselves never do.
pub fn df(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, SYSTEM_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, SYSTEM_USAGE) {
        return c;
    }
    // ⭐ The `logout` shape (TODO/cli.md T-0209): flags collect and the
    // verb decides after the loop, so `--help` wins over positionals
    // and every arm need not diverge.
    let mut help = false;
    for a in args {
        match a.as_str() {
            "-h" | "--help" => help = true,
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, SYSTEM_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other => {
                eprintln!("podvm system df: unknown option {other:?}");
                eprint!("{SYSTEM_USAGE}");
                return EXIT_FLAG_ERROR;
            }
        }
    }
    if help {
        print!("{SYSTEM_USAGE}");
        return 0;
    }
    let store = match podvm_image::open_store() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("podvm system df: {e}");
            return e.exit_code();
        }
    };
    let images = match store.list() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("podvm system df: {e}");
            return e.exit_code();
        }
    };
    let containers = match podvm_supervise::list(&store, true) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("podvm system df: {e}");
            return EXIT_RUNTIME_ERROR;
        }
    };
    // ⭐ TODO/image.md T-1322's chain, read-only: what `image prune`
    // would consider, minus what a container references or holds, with
    // the sizing standing in for the unlinking.
    let mut rest: Vec<podvm_image::Record> = Vec::new();
    let mut kept: Vec<String> = Vec::new();
    let candidates = match store.prune_candidates(false) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("podvm system df: {e}");
            return e.exit_code();
        }
    };
    for record in candidates {
        match podvm_supervise::referencing(&store, &record.manifest_digest) {
            Ok(found) if !found.is_empty() => kept.push(record.name()),
            Ok(_) => match store.in_use(&record) {
                Ok(true) => kept.push(record.name()),
                Ok(false) => rest.push(record),
                Err(e) => {
                    eprintln!("podvm system df: {e}");
                    return e.exit_code();
                }
            },
            Err(e) => {
                eprintln!("podvm system df: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        }
    }
    // TODO/image.md T-1787: `image prune` also removes each extraction that
    // no record names.
    let orphans: u64 = crate::images::orphan_extractions(&store)
        .iter()
        .map(|(_, b)| b)
        .sum();
    let reclaimable = match store.reclaimable_bytes(&rest) {
        Ok(b) => b.saturating_add(orphans),
        Err(e) => {
            eprintln!("podvm system df: {e}");
            return e.exit_code();
        }
    };
    // Per-image rows bill what the image needs: a blob two tags share
    // is billed under both, the way docker bills shared layers under
    // every image. The totals below count each blob once, which is the
    // store's own accounting and what the drive asserts.
    let mut rows: Vec<(String, String, String, u64, u64, bool)> = Vec::new();
    for r in &images {
        let mut stored = 0u64;
        for b in r.blobs() {
            stored = stored.saturating_add(store.blob_bytes(b));
        }
        let (rootfs, _) = podvm_extract::paths(&store, &r.manifest_digest);
        let rootfs_bytes = podvm_image::space::dir_bytes(&rootfs);
        let extracted = rootfs.is_dir();
        rows.push((
            r.display_repository(),
            r.tag.clone().unwrap_or_else(|| "<none>".to_string()),
            short_id(&r.digest).to_string(),
            stored,
            rootfs_bytes,
            extracted,
        ));
    }
    rows.sort();
    let repo_w = rows
        .iter()
        .map(|r| r.0.len())
        .max()
        .unwrap_or(10)
        .max("REPOSITORY".len());
    println!(
        "{:<repo_w$}  {:<16}  {:<12}  {:>12}  {:>12}",
        "REPOSITORY", "TAG", "IMAGE ID", "STORED", "ROOTFS"
    );
    let mut rootfs_total = 0u64;
    let mut extracted_count = 0usize;
    for (repo, tag, id, stored, rootfs_bytes, extracted) in &rows {
        println!(
            "{repo:<repo_w$}  {tag:<16}  {id:<12}  {:>12}  {:>12}",
            podvm_image::space::mib(*stored),
            podvm_image::space::mib(*rootfs_bytes)
        );
        rootfs_total = rootfs_total.saturating_add(*rootfs_bytes);
        if *extracted {
            extracted_count += 1;
        }
    }
    // The totals deduplicate: one blob on disk is billed once no matter
    // how many tags need it. A totals line above the rows would disagree
    // with them by exactly the shared bytes; below, it is the store's
    // own accounting.
    let mut unique_stored = 0u64;
    let mut counted: Vec<String> = Vec::new();
    for r in &images {
        for b in r.blobs() {
            if counted.iter().any(|c| c == b) {
                continue;
            }
            counted.push(b.to_string());
            unique_stored = unique_stored.saturating_add(store.blob_bytes(b));
        }
    }
    let running = containers
        .iter()
        .filter(|c| c.state == podvm_supervise::table::State::Running)
        .count();
    println!();
    println!("Images: {} ({} extracted)", images.len(), extracted_count);
    println!("Containers: {} ({} running)", containers.len(), running);
    println!("Stored blobs: {}", podvm_image::space::mib(unique_stored));
    println!(
        "Extracted rootfs: {}",
        podvm_image::space::mib(rootfs_total)
    );
    // TODO/image.md T-1787 (PodUtils/podvm#329): each byte under the store
    // has a line: the trees of the containers (and the trash of T-1781),
    // the spare trees, and the extractions that no image names.
    let spares: u64 = images
        .iter()
        .map(|r| {
            podvm_image::space::dir_bytes(&podvm_extract::tree::spare_dir(
                &store,
                &r.manifest_digest,
            ))
        })
        .sum();
    println!(
        "Container trees: {}",
        podvm_image::space::mib(podvm_image::space::dir_bytes(&podvm_extract::tree::trees(
            &store
        )))
    );
    println!(
        "Spare trees: {} (T-1781; `PODVM_SPARE_TREE=0` makes none)",
        podvm_image::space::mib(spares)
    );
    println!(
        "Extractions no image names: {}",
        podvm_image::space::mib(orphans)
    );
    println!(
        "Reclaimable: {} (what `image prune` would free)",
        podvm_image::space::mib(reclaimable)
    );
    if !kept.is_empty() {
        kept.sort();
        println!("Kept: {} (referenced or in use)", kept.join(", "));
    }
    0
}

/// The first twelve hex digits, docker's IMAGE ID width. A digest that
/// does not parse is not a gate failure: the row still prints.
pub(super) fn short_id(digest: &str) -> &str {
    let hex = digest.split(':').next_back().unwrap_or(digest);
    &hex[..hex.len().min(12)]
}

pub fn info(verb: &str, args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded: Vec<String> = match crate::parity::expand(verb, args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member(verb, &member, SYSTEM_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(verb, args, SYSTEM_USAGE) {
        return c;
    }
    let mut template: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{SYSTEM_USAGE}");
                return 0;
            }
            "-f" | "--format" => match it.next() {
                Some(t) => template = Some(t.clone()),
                None => {
                    eprintln!("podvm system info: --format needs a template");
                    return EXIT_FLAG_ERROR;
                }
            },
            other if other.starts_with("--format=") => {
                template = Some(other["--format=".len()..].to_string())
            }
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(verb, other, SYSTEM_USAGE) {
                    return c;
                }
                return crate::parity::no_arm(verb, other);
            }
            other => {
                eprintln!("podvm system info: unknown option {other:?}");
                eprint!("{SYSTEM_USAGE}");
                return EXIT_FLAG_ERROR;
            }
        }
    }
    // ⛔ The caller's own input is checked before anything about the machine is
    // measured, so a typo is not paid for with a probe.
    if let Some(t) = &template
        && let Err(bad) = format::check_with_documents(t, FIELDS, DOCUMENTS)
    {
        eprintln!("podvm system info: {bad}");
        return EXIT_CLI_ERROR;
    }

    let fields = collect();
    match template {
        Some(t) => match format::render_with_documents(&t, &fields, DOCUMENTS) {
            Ok(line) => {
                println!("{line}");
                0
            }
            Err(bad) => {
                eprintln!("podvm system info: {bad}");
                EXIT_CLI_ERROR
            }
        },
        None => {
            print!("{}", human(&fields));
            0
        }
    }
}

/// Everything `info` can report, measured once.
///
/// ⚠ The measuring is HERE and the assembling is in [`fields_from`], so a test
/// can assert the field names without running 50 probe children of whatever
/// binary it happens to be. `podvm_probe::run` re-execs argv[0], and under
/// `cargo test` argv[0] is the test harness.
fn collect() -> Vec<(&'static str, String)> {
    let findings = podvm_probe::run();
    let selection = podvm_probe::select::Selection::choose(&findings);
    let store = podvm_image::open_store()
        .map(|s| s.root().display().to_string())
        .unwrap_or_else(|e| format!("unavailable: {e}"));
    fields_from(
        selection.rung,
        podvm_probe::probes::chroot_usable(&findings),
        selection.exit_code(podvm_probe::select::Strictness::Refuse) == 0,
        store,
    )
}

/// The rung that `podvm run` enters on a machine where the probe selects
/// `selection` and `chroot(2)` is usable or not.
///
/// ⛔ TODO/cli.md T-1713, T-0804 rule 4. `run` asks `chroot_usable` before
/// the selection (`lifecycle::entry::decide_entry`): where chroot is denied
/// it enters a no-chroot family or refuses, whatever the probe selected
/// (`supervise` where ptrace works, `namespace` where user namespaces
/// work). So the field reads `userland` there; elsewhere it is the rung of
/// the selection that `pod-enter` enters.
pub(super) fn entered_rung(selection: Rung, chroot_usable: bool) -> Rung {
    if chroot_usable {
        podvm_enter::entered_rung(selection)
    } else {
        Rung::Userland
    }
}

/// TODO/cli.md T-1766: `probe` names the rung that `run` enters, as
/// `system info` does, so a machine where chroot is denied reads
/// `userland` and `probe --strict` gates on it. The selection of the
/// probes goes to stderr where it differs.
pub fn to_entered(f: &podvm_probe::Findings, s: &mut podvm_probe::select::Selection) {
    let entered = entered_rung(s.rung, podvm_probe::probes::chroot_usable(f));
    if entered != s.rung {
        eprintln!(
            "podvm probe: the probes select `{}`, and `run` enters `{}` on this machine \
             (TODO/cli.md T-1713, T-1766)",
            s.rung.word(),
            entered.word()
        );
        s.rung = entered;
    }
}

pub(super) fn fields_from(
    selection: Rung,
    chroot_usable: bool,
    strict_ok: bool,
    store: String,
) -> Vec<(&'static str, String)> {
    let count = |s: parity::Status| {
        parity::TABLE
            .iter()
            .filter(|r| r.flag.is_none() && r.status == s)
            .count()
            .to_string()
    };
    vec![
        ("Parity", parity::json()),
        ("ParityRows", parity::TABLE.len().to_string()),
        ("ExitCodes", podvm_probe::exit::json()),
        ("Version", env!("CARGO_PKG_VERSION").to_string()),
        ("Rung", selection.word().to_string()),
        (
            "EnteredRung",
            // TODO/machine.md T-1722: `tier = incus` in the configuration,
            // with its socket there, is what `run` enters.
            crate::incus::config::entered_word()
                .map(str::to_string)
                .unwrap_or_else(|| entered_rung(selection, chroot_usable).word().to_string()),
        ),
        ("StrictOk", strict_ok.to_string()),
        ("Store", store),
        // ⭐ T-0804 rule 1: the banner is suppressible by CONFIG and never by
        // the command line, and whether a machine has suppressed it is itself
        // reported. A silent banner nobody can tell from an absent one is the
        // `sandlock` shape this rule exists to refuse.
        ("Banner", banner_state()),
        (
            "Platform",
            podvm_image::platform::Platform::host().to_string(),
        ),
        ("Kernel", kernel_release()),
        (
            "Verbs",
            parity::TABLE
                .iter()
                .filter(|r| r.flag.is_none())
                .count()
                .to_string(),
        ),
        ("VerbsNative", count(parity::Status::Native)),
        ("VerbsDegraded", count(parity::Status::Degraded)),
        ("VerbsStub", count(parity::Status::Stub)),
        ("VerbsNone", count(parity::Status::None)),
    ]
}

/// ⚠ Read from `/proc`, and a dash where it cannot be read. AGENTS.md
/// absolute 3: a dash where a value is unknown, never a plausible-looking one.
fn kernel_release() -> String {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "-".into())
}

pub(super) fn pick(fields: &[(&str, String)], key: &str) -> String {
    fields
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

fn human(fields: &[(&str, String)]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "podvm {}\n  rung          {}   (--strict would {})\n  entered with  {}\n  \
         platform      {}\n  kernel        {}\n  store         {}\n\n",
        pick(fields, "Version"),
        pick(fields, "Rung"),
        if pick(fields, "StrictOk") == "true" {
            "pass"
        } else {
            "refuse"
        },
        pick(fields, "EnteredRung"),
        pick(fields, "Platform"),
        pick(fields, "Kernel"),
        pick(fields, "Store"),
    ));
    out.push_str(&format!(
        "parity (TOOL.md section 6.8): {} rows, {} verbs\n  \
         Native {}   Degraded {}   Stub {}   None {}\n\n",
        pick(fields, "ParityRows"),
        pick(fields, "Verbs"),
        pick(fields, "VerbsNative"),
        pick(fields, "VerbsDegraded"),
        pick(fields, "VerbsStub"),
        pick(fields, "VerbsNone"),
    ));
    // ⛔ **WHICH INTERPOSER OBJECTS THIS BINARY CARRIES, and it is printed
    // because a caller cannot otherwise tell.** A podvm built where
    // `scripts/build-interpose.sh` could not run carries none and declines
    // every payload, which is a different tool from one that carries both.
    // `TOOL.md` section 4.1 forbids output that implies a capability podvm
    // does not have, and silence here would imply one.
    // `TODO/interpose.md` T-0702.
    out.push_str(&format!("{}\n\n", crate::interpose::carried()));
    out.push_str(&parity::text());
    out
}
