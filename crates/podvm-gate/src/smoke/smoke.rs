use super::*;

fn smoke_unrun(triple: &str, message: &str) -> ExitCode {
    eprintln!("SMOKE-FAIL {triple}: {message}");
    ExitCode::from(2)
}

fn doc_line(doc: &str, key: &str) -> String {
    for line in doc.lines() {
        if let Some((name, value)) = line.split_once(": ")
            && name == key
        {
            return value.to_string();
        }
    }
    String::new()
}

pub(super) fn do_smoke(bin: &str, triple: &str, qemu: &str, root: &Path) -> ExitCode {
    if bin.is_empty() || triple.is_empty() {
        eprintln!("SMOKE-FAIL usage: podvm-smoke BINARY TRIPLE [QEMU]");
        return ExitCode::from(2);
    }
    if !Path::new(bin).is_file() {
        return smoke_unrun(triple, &format!("no binary at {bin}"));
    }
    if !qemu.is_empty() && which(qemu).is_none() {
        return smoke_unrun(triple, &format!("no emulator {qemu} on PATH"));
    }
    // Run the binary under test, directly or through the emulator, with
    // both streams merged the way the script's 2>&1 merged them.
    let invoke = |args: &[&str]| -> Result<Run, String> {
        let mut full: Vec<String> = Vec::new();
        if !qemu.is_empty() {
            full.push(qemu.to_string());
        }
        full.push(bin.to_string());
        for arg in args {
            full.push(arg.to_string());
        }
        let (prog, rest) = full.split_first().unwrap();
        run_bounded(prog, rest, root, &[], 300)
    };

    // Group 1: `version` exits 0 and prints the artefact's version line.
    let first = match invoke(&["version"]) {
        Ok(run) => run,
        Err(e) => return smoke_unrun(triple, &e),
    };
    let out = format!("{}{}", first.stdout, first.stderr);
    let out_trimmed = out.trim_end().to_string();
    if first.rc != 0 {
        return smoke_fail(
            triple,
            &format!("version exits {}, not 0: {out_trimmed}", first.rc),
        );
    }
    if !out_trimmed.starts_with("podvm ") {
        return smoke_fail(
            triple,
            &format!("version prints [{out_trimmed}], not the version line"),
        );
    }

    // Group 2: `version --verbose` names the triple.
    let verbose = match invoke(&["version", "--verbose"]) {
        Ok(run) => run,
        Err(e) => return smoke_unrun(triple, &e),
    };
    if verbose.rc != 0 {
        return smoke_fail(
            triple,
            &format!("version --verbose exits {}, not 0", verbose.rc),
        );
    }
    let doc = format!("{}{}", verbose.stdout, verbose.stderr);
    print!("{doc}");
    if doc_line(&doc, "target") != triple {
        return smoke_fail(
            triple,
            &format!("verbose names target [{}]", doc_line(&doc, "target")),
        );
    }
    if doc_line(&doc, "crt-static") != "yes" {
        return smoke_fail(
            triple,
            &format!(
                "verbose names crt-static [{}], not yes",
                doc_line(&doc, "crt-static")
            ),
        );
    }

    // Groups 3: both interposer digests are 64 lowercase hex digits. Off
    // x86_64 a binary embeds no interposer since TODO/interpose.md T-1721
    // (the objects are x86_64 only), so `absent` is the decided value there.
    let native = triple.starts_with("x86_64-");
    for key in ["interpose-gnu", "interpose-musl"] {
        let value = doc_line(&doc, key);
        if !native && value == "absent" {
            println!("SMOKE-EMBED {triple} {key} absent (TODO/interpose.md T-1721)");
            continue;
        }
        if value.len() != 64 {
            return smoke_fail(
                triple,
                &format!("{key} is [{value}], not a 64-digit digest"),
            );
        }
        if !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return smoke_fail(triple, &format!("{key} is [{value}], not lowercase hex"));
        }
    }

    // Group 4: no PT_INTERP.
    if which("readelf").is_none() {
        return smoke_unrun(
            triple,
            "readelf is not installed, so static-ness is unchecked",
        );
    }
    let elf = match run_bounded(
        "readelf",
        &["-l".to_string(), bin.to_string()],
        root,
        &[],
        60,
    ) {
        Ok(run) => run,
        Err(e) => return smoke_unrun(triple, &e),
    };
    let interp = elf
        .stdout
        .lines()
        .filter(|line| line.contains("INTERP"))
        .count();
    if interp != 0 {
        return smoke_fail(
            triple,
            &format!("{interp} PT_INTERP entries, so the binary is not static"),
        );
    }

    // Group 6: the embedded pair's e_machine beside the artefact's own. A
    // present object with no machine is a build that did not record; an
    // artefact embedding an object of another machine is the wrong-arch
    // embed this group exists to catch. Off x86_64 the decided form is no
    // embed at all (TODO/interpose.md T-1721, PodUtils/podrunner#52).
    if which("od").is_none() {
        return smoke_unrun(triple, "od is not installed, so e_machine is unchecked");
    }
    let bytes = fs::read(bin).unwrap_or_default();
    if bytes.len() < 20 {
        return smoke_unrun(triple, "e_machine unreadable");
    }
    let machine = u16::from_le_bytes([bytes[18], bytes[19]]);
    if machine == 0 {
        return smoke_unrun(triple, "e_machine unreadable");
    }
    let outer = format!("{machine:x}");
    let gnu_m = doc_line(&doc, "interpose-gnu-machine");
    let musl_m = doc_line(&doc, "interpose-musl-machine");
    if gnu_m.is_empty() {
        return smoke_fail(triple, "no interpose-gnu-machine line in version --verbose");
    }
    if musl_m.is_empty() {
        return smoke_fail(
            triple,
            "no interpose-musl-machine line in version --verbose",
        );
    }
    println!("SMOKE-EMBED-MACHINE {triple} outer=0x{outer} gnu={gnu_m} musl={musl_m}");
    if triple.starts_with("x86_64-") {
        if gnu_m != format!("0x{outer}") {
            return smoke_fail(
                triple,
                &format!("gnu embed {gnu_m} is not the artefact 0x{outer}"),
            );
        }
        if musl_m != format!("0x{outer}") {
            return smoke_fail(
                triple,
                &format!("musl embed {musl_m} is not the artefact 0x{outer}"),
            );
        }
    } else if gnu_m == format!("0x{outer}") && musl_m == format!("0x{outer}") {
        println!("SMOKE-EMBED-MACHINE {triple} embeds match the artefact");
    } else if gnu_m == "absent" && musl_m == "absent" {
        println!("SMOKE-EMBED-MACHINE {triple} embeds none (TODO/interpose.md T-1721)");
    } else {
        return smoke_fail(
            triple,
            &format!("embeds gnu={gnu_m} musl={musl_m}, not the artefact 0x{outer} and not none"),
        );
    }

    // Group 5: the binary pulls and extracts for its actual job, not only
    // versions. A synthetic one-file image travels by save/load through the
    // binary under test; then the tarball's layer bytes are flipped and the
    // same load must refuse. Native legs only.
    if qemu.is_empty() {
        let code = group_five(bin, triple, qemu, root);
        if code != 0 {
            return ExitCode::from(code);
        }
    }

    let size = fs::metadata(bin).map(|meta| meta.len()).unwrap_or(0);
    println!(
        "SMOKE-OK {triple} qemu={} bytes={size}",
        if qemu.is_empty() { "native" } else { qemu }
    );
    ExitCode::from(0)
}

fn bin_run(bin: &str, qemu: &str, root: &Path, store: &Path, args: &[&str]) -> Result<Run, String> {
    let mut full: Vec<String> = Vec::new();
    if !qemu.is_empty() {
        full.push(qemu.to_string());
    }
    full.push(bin.to_string());
    for arg in args {
        full.push(arg.to_string());
    }
    let (prog, rest) = full.split_first().unwrap();
    let store_str = store.to_string_lossy().into_owned();
    run_bounded(
        prog,
        rest,
        root,
        &[("PODVM_STORE".to_string(), store_str)],
        300,
    )
}

fn group_five(bin: &str, triple: &str, qemu: &str, root: &Path) -> u8 {
    let work = env::temp_dir().join(format!("podvm-smoke-{}", std::process::id()));
    let cleanup = |code: u8| -> u8 {
        let _ = fs::remove_dir_all(&work);
        code
    };
    if fs::create_dir_all(&work).is_err() {
        smoke_unrun(triple, "no temp dir");
        return 2;
    }
    let store = work.join("store");
    let seed = work.join("seed");
    if fs::create_dir_all(&seed).is_err() {
        smoke_unrun(triple, "no seed dir");
        return cleanup(2);
    }
    if fs::write(seed.join("payload.txt"), "smoke-payload\n").is_err() {
        smoke_unrun(triple, "no seed file");
        return cleanup(2);
    }
    let rootfs_tar = work.join("rootfs.tar");
    if run_bounded(
        "tar",
        &[
            "-cf".to_string(),
            rootfs_tar.to_string_lossy().into_owned(),
            "payload.txt".to_string(),
        ],
        &seed,
        &[],
        60,
    )
    .is_err()
    {
        smoke_unrun(triple, "no seed tar");
        return cleanup(2);
    }
    let seed_tar = work.join("seed.tar");
    let evil_tar = work.join("evil.tar");
    let steps: &[(&str, Vec<String>)] = &[
        (
            "import",
            vec![
                bin.to_string(),
                "import".to_string(),
                rootfs_tar.to_string_lossy().into_owned(),
                "smoke:t1".to_string(),
            ],
        ),
        (
            "save",
            vec![
                bin.to_string(),
                "save".to_string(),
                "-o".to_string(),
                seed_tar.to_string_lossy().into_owned(),
                "smoke:t1".to_string(),
            ],
        ),
        (
            "load",
            vec![
                bin.to_string(),
                "load".to_string(),
                "-i".to_string(),
                seed_tar.to_string_lossy().into_owned(),
            ],
        ),
        (
            "extract",
            vec![
                bin.to_string(),
                "extract".to_string(),
                "smoke:t1".to_string(),
            ],
        ),
    ];
    for (name, full) in steps {
        let (prog, rest) = full.split_first().unwrap();
        let store_str = store.to_string_lossy().into_owned();
        match run_bounded(
            prog,
            rest,
            root,
            &[("PODVM_STORE".to_string(), store_str)],
            300,
        ) {
            Ok(run) if run.rc == 0 => {}
            _ => {
                smoke_fail(triple, &format!("{name} of the seed image exits non-zero"));
                return cleanup(1);
            }
        }
    }
    let inspect = match bin_run(
        bin,
        qemu,
        root,
        &store,
        &["inspect", "--format", "{{.RootfsPath}}", "smoke:t1"],
    ) {
        Ok(run) if run.rc == 0 => run,
        _ => {
            smoke_fail(triple, "inspect of the loaded image exits non-zero");
            return cleanup(1);
        }
    };
    let rootfs = inspect.stdout.trim().to_string();
    let payload = fs::read_to_string(Path::new(&rootfs).join("payload.txt")).unwrap_or_default();
    if payload != "smoke-payload\n" {
        smoke_fail(triple, "payload bytes differ after extract");
        return cleanup(1);
    }
    println!("SMOKE-PULL-EXTRACT-OK {triple} honest leg: pull-by-load plus extract reads the payload bytes");
    if fs::copy(&seed_tar, &evil_tar).is_err() {
        smoke_fail(triple, "no repack copy");
        return cleanup(1);
    }
    // Flip the layer blob inside the store-format tarball, not the outer
    // tar framing: `load` hashes each entry's bytes against the descriptor
    // that names it, so a flipped ENTRY fails closed with a digest
    // mismatch.
    let list = match run_bounded(
        "tar",
        &["-tf".to_string(), seed_tar.to_string_lossy().into_owned()],
        root,
        &[],
        60,
    ) {
        Ok(run) => run.stdout,
        Err(_) => {
            smoke_fail(triple, "no blob entry in the seed tarball");
            return cleanup(1);
        }
    };
    let blob = list
        .lines()
        .find(|line| line.contains("blobs/sha256/"))
        .unwrap_or("")
        .to_string();
    if blob.is_empty() {
        smoke_fail(triple, "no blob entry in the seed tarball");
        return cleanup(1);
    }
    let evil_dir = work.join("evil");
    if fs::create_dir_all(&evil_dir).is_err()
        || run_bounded(
            "tar",
            &[
                "-xf".to_string(),
                seed_tar.to_string_lossy().into_owned(),
                "-C".to_string(),
                evil_dir.to_string_lossy().into_owned(),
            ],
            root,
            &[],
            60,
        )
        .is_err()
    {
        smoke_fail(triple, "no repack dir");
        return cleanup(1);
    }
    let blob_path = evil_dir.join(&blob);
    let mut blob_bytes = fs::read(&blob_path).unwrap_or_default();
    if blob_bytes.len() < 101 {
        smoke_unrun(triple, "no dd");
        return cleanup(2);
    }
    blob_bytes[100] = b'X';
    if fs::write(&blob_path, &blob_bytes).is_err() {
        smoke_unrun(triple, "no dd");
        return cleanup(2);
    }
    if run_bounded(
        "tar",
        &[
            "-cf".to_string(),
            evil_tar.to_string_lossy().into_owned(),
            "oci-layout".to_string(),
            "index.json".to_string(),
            "blobs".to_string(),
        ],
        &evil_dir,
        &[],
        60,
    )
    .is_err()
    {
        smoke_fail(triple, "no repack");
        return cleanup(1);
    }
    let evil = bin_run(
        bin,
        qemu,
        root,
        &store,
        &["load", "-i", &evil_tar.to_string_lossy()],
    );
    match evil {
        Ok(run) if run.rc == 0 => {
            smoke_fail(triple, "load of the flipped tarball exits 0");
            return cleanup(1);
        }
        Err(_) => {
            smoke_unrun(triple, "evil load could not run");
            return cleanup(2);
        }
        _ => {}
    }
    println!("SMOKE-PULL-EXTRACT-OK {triple} flipped leg: the corrupted fixture refuses");
    cleanup(0)
}

// ------------------------------------------------------ the diagnostics half
