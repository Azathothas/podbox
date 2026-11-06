use super::*;

/// The provenance half of `verify <image>`: how this record's bytes got
/// here, or the honest absence where no pull recorded one.
pub(super) fn print_provenance(store: &Store, record: &podvm_image::Record) {
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
                println!("podvm-version: {}", p.podvm_version);
                shown += 1;
            }
            if shown == 0 {
                println!(
                    "no provenance line for {} (recorded before T-1321, imported, or loaded)",
                    record.manifest_digest
                );
            }
        }
        Err(e) => eprintln!("podvm verify: {e}"),
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
                    eprintln!("podvm inspect: --format needs a template");
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
    if let Some(t) = &template
        && format::check(t, INSPECT_FIELDS).is_err()
        && format::check(t, crate::lifecycle::CONTAINER_INSPECT_FIELDS).is_err()
    {
        let bad = format::check(t, INSPECT_FIELDS).unwrap_err();
        eprintln!("podvm inspect: {bad}");
        return EXIT_CLI_ERROR;
    }
    let store = match podvm_image::open_store() {
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
                        eprintln!("podvm inspect: {e}");
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
                    eprintln!("podvm inspect: {bad}");
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

/// The field names `podvm images --format` answers to.
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

/// The same, for `podvm inspect --format`.
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
    // ⛔ TODO/enter.md T-0505. What `podvm exec` against this image WOULD be,
    // and not a property of the image. It is constant because podvm has one
    // exec mechanism; when M4 gives a container its own record the same two
    // names sit on that record and answer from the container that was entered.
    "Exec.Mode",
    "Exec.Shares",
    // ⭐ TODO/cli.md T-1417. What the image declares, beside what podvm
    // applied: dotted names are single registered names (T-0505), so an
    // unregistered one is still refused rather than rendering half.
    "Config.Env",
    "Config.Cmd",
    "Config.Entrypoint",
    "Config.WorkingDir",
    "Config.User",
];

/// `podvm exec`'s mode, in one word each, for `inspect` and for the banner.
///
/// ⛔ TODO/enter.md T-0505. `docker exec` enters the container's namespaces.
/// podvm has none to enter, so its `exec` re-runs the section 6.5 sequence
/// against the same rootfs: it shares the filesystem tree and NOTHING else, not
/// the process table, not `/proc`, not signals, not the original's environment.
/// A caller reads these rather than assuming.
pub const EXEC_MODE: &str = "fresh-chroot";
pub const EXEC_SHARES: &str = "filesystem";

pub(super) fn image_fields(
    r: &Record,
    store: &Store,
    no_trunc: bool,
) -> Vec<(&'static str, String)> {
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

pub(super) fn inspect_fields(r: &Record, store: &Store) -> Vec<(&'static str, String)> {
    let cfg = run_config_of(store, r);
    let flat = config_flat(&cfg);
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
            let (rootfs, _) = podvm_extract::paths(store, &r.manifest_digest);
            rootfs.display().to_string()
        }),
        // ⚠ Whether the rootfs is THERE, which is a different question from
        // where it would be. `inspect` on a pulled-but-unextracted image must
        // not imply a tree that does not exist.
        (
            "Extracted",
            podvm_extract::is_extracted(store, &r.manifest_digest).to_string(),
        ),
        ("Exec.Mode", EXEC_MODE.to_string()),
        ("Exec.Shares", EXEC_SHARES.to_string()),
        ("Config.Env", flat.0),
        ("Config.Cmd", flat.1),
        ("Config.Entrypoint", flat.2),
        ("Config.WorkingDir", flat.3),
        ("Config.User", flat.4),
    ]
}

/// TODO/cli.md T-1417. The image's declared `Config`, read through the
/// stored config digest.
///
/// `None` where the blob is missing or does not parse: `inspect` is a
/// reader, and a broken store is `verify`'s to report (T-1321), so the
/// Config half renders null rather than failing the whole document.
pub(super) fn run_config_of(store: &Store, r: &Record) -> Option<podvm_image::oci::RunConfig> {
    let d = podvm_image::digest::Digest::parse(&r.config_digest).ok()?;
    let bytes = store.read_blob(&d).ok()?;
    let cfg: podvm_image::oci::Config = serde_json::from_slice(&bytes).ok()?;
    Some(cfg.config)
}

/// The Config half as flat `--format` strings: lists space-joined like
/// `.Layers` above, `-` where the image omits the field.
pub(super) fn config_flat(
    c: &Option<podvm_image::oci::RunConfig>,
) -> (String, String, String, String, String) {
    let none = || "-".to_string();
    match c {
        None => (none(), none(), none(), none(), none()),
        Some(c) => (
            if c.env.is_empty() {
                none()
            } else {
                c.env.join(" ")
            },
            c.cmd.as_ref().map(|v| v.join(" ")).unwrap_or_else(none),
            c.entrypoint
                .as_ref()
                .map(|v| v.join(" "))
                .unwrap_or_else(none),
            if c.working_dir.is_empty() {
                none()
            } else {
                c.working_dir.clone()
            },
            if c.user.is_empty() {
                none()
            } else {
                c.user.clone()
            },
        ),
    }
}

/// The Config half as a JSON document with docker's key names, null where
/// the image omits the field. Reported beside what podvm applied, never
/// merged with it.
pub(super) fn config_json(c: &Option<podvm_image::oci::RunConfig>) -> serde_json::Value {
    let null = serde_json::Value::Null;
    match c {
        None => serde_json::json!({
            "Env": null, "Cmd": null, "Entrypoint": null,
            "WorkingDir": null, "User": null,
        }),
        Some(c) => serde_json::json!({
            "Env": if c.env.is_empty() { null.clone() } else { serde_json::Value::from(c.env.clone()) },
            "Cmd": c.cmd.clone().map(serde_json::Value::from).unwrap_or(null.clone()),
            "Entrypoint": c.entrypoint.clone().map(serde_json::Value::from).unwrap_or(null.clone()),
            "WorkingDir": if c.working_dir.is_empty() { null.clone() } else { serde_json::Value::from(c.working_dir.clone()) },
            "User": if c.user.is_empty() { null } else { serde_json::Value::from(c.user.clone()) },
        }),
    }
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
        // ⭐ TODO/cli.md T-1417. What the image declares, with docker's key
        // names and null where it omits a field, beside what podvm applied.
        "Config": config_json(&run_config_of(store, r)),
        // ⛔ T-0505, and nested here because it is nested in --format too. A
        // caller that reads one and not the other must not find two shapes.
        "Exec": { "Mode": EXEC_MODE, "Shares": EXEC_SHARES },
    })
    .to_string()
}

pub(super) fn pick(fields: &[(&str, String)], key: &str) -> String {
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

/// TODO/image.md T-1419. The record's blobs that have no file in the
/// store, by digest.
///
/// Presence only, never a hash: `images` marks blob-less records while
/// listing, and hashing every blob of every listed image on every listing
/// is `verify`'s job. An unparseable digest counts as missing, because no
/// file under that name can be the blob the record names.
pub(super) fn missing_blobs(store: &Store, r: &Record) -> Vec<String> {
    let mut out = Vec::new();
    for b in r.blobs() {
        match podvm_image::digest::Digest::parse(b) {
            Ok(d) if store.blob_path(&d).exists() => {}
            _ => out.push(b.to_string()),
        }
    }
    out
}

pub(super) fn unknown(verb: &str, flag: &str, usage: &str) -> i32 {
    eprintln!("podvm {verb}: unknown option {flag:?}");
    eprint!("{usage}");
    EXIT_FLAG_ERROR
}

pub(super) fn fail(e: Error) -> i32 {
    eprintln!("podvm: {e}");
    e.exit_code()
}
