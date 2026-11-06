use super::*;

/// 364 tail-5: `logs -f --tail 3` prints the last three lines, then follows
/// new lines to the container's end instead of replaying the file. The
/// container already ended (mklog waits), so follow prints the tail and
/// exits.
#[test]
fn logs_follow_starts_from_the_tail() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("follow");
    if let Some(reason) = live_skip_reason(&store) {
        eprintln!("SKIP logs follow: {reason}");
        cleanup(&store);
        return;
    }
    if !pull_or_skip(DEBIAN, &store) {
        cleanup(&store);
        return;
    }
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let name = format!("t364b-{}-{n}", std::process::id());
    let (create, start, wait) = mklog(
        &store,
        &name,
        DEBIAN,
        &[
            "/bin/bash",
            "-c",
            "for i in 1 2 3 4 5; do echo line$i; done",
        ],
    );
    assert_mklog_ok(&create, &start, &wait, &name);

    let follow = run_podvm(
        &["logs", "-f", "--tail", "3", &name],
        &store,
        Duration::from_secs(120),
    );
    assert_eq!(
        follow.status.code(),
        Some(0),
        "logs -f --tail 3 exited {:?}, stderr was {:?}",
        follow.status.code(),
        stderr_text(&follow, &["logs -f --tail 3"])
    );
    assert_eq!(
        follow.stdout,
        b"line3\nline4\nline5\n",
        "logs -f --tail 3 printed {:?}",
        stdout_text(&follow, &["logs -f --tail 3"])
    );

    let rm = run_podvm(&["rm", &name], &store, Duration::from_secs(120));
    assert_eq!(
        rm.status.code(),
        Some(0),
        "rm {name} exited {:?}, stderr was {:?}",
        rm.status.code(),
        stderr_text(&rm, &["rm"])
    );
    cleanup(&store);
}

/// 364 df-4: `system df --bogus` is a flag error and `system df --help`
/// prints usage at 0. Hermetic: `df` decides its arguments before it
/// touches the store.
#[test]
fn system_df_flag_shapes_need_no_store() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("df4");
    let bogus = run_podvm(
        &["system", "df", "--bogus"],
        &store,
        Duration::from_secs(60),
    );
    assert_eq!(
        bogus.status.code(),
        Some(FLAG_ERROR),
        "system df --bogus exited {:?} (want {FLAG_ERROR}), stderr was {:?}",
        bogus.status.code(),
        stderr_text(&bogus, &["system df --bogus"])
    );
    let help = run_podvm(&["system", "df", "--help"], &store, Duration::from_secs(60));
    assert_eq!(
        help.status.code(),
        Some(0),
        "system df --help exited {:?}, stderr was {:?}",
        help.status.code(),
        stderr_text(&help, &["system df --help"])
    );
    let stdout = stdout_text(&help, &["system df --help"]);
    assert!(
        stdout.contains("podvm system df"),
        "system df --help printed no usage: {stdout:?}"
    );
    cleanup(&store);
}

/// 364 df-2, hermetic shape: the totals lines are present on a fresh store,
/// and accounting leaves the store untouched (the image count is identical
/// before and after).
#[test]
fn system_df_reports_totals_and_touches_nothing() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("dfempty");
    let before = run_podvm(&["images", "-q"], &store, Duration::from_secs(60));
    assert_eq!(
        before.status.code(),
        Some(0),
        "images -q exited {:?}, stderr was {:?}",
        before.status.code(),
        stderr_text(&before, &["images -q"])
    );
    let df = run_podvm(&["system", "df"], &store, Duration::from_secs(60));
    assert_eq!(
        df.status.code(),
        Some(0),
        "system df exited {:?}, stderr was {:?}",
        df.status.code(),
        stderr_text(&df, &["system df"])
    );
    let stdout = stdout_text(&df, &["system df"]);
    assert!(
        stdout.contains("REPOSITORY"),
        "system df printed no header: {stdout:?}"
    );
    for line in [
        "Images:",
        "Containers:",
        "Stored blobs:",
        "Extracted rootfs:",
        "Reclaimable:",
    ] {
        assert!(
            stdout.contains(line),
            "system df output has no {line:?} line: {stdout:?}"
        );
    }
    let after = run_podvm(&["system", "df"], &store, Duration::from_secs(60));
    assert_eq!(
        after.status.code(),
        Some(0),
        "second system df exited {:?}",
        after.status.code()
    );
    let recount = run_podvm(&["images", "-q"], &store, Duration::from_secs(60));
    assert_eq!(
        before.stdout,
        recount.stdout,
        "df changed the store: {:?} before, {:?} after",
        stdout_text(&before, &["images -q"]),
        stdout_text(&recount, &["images -q"])
    );
    cleanup(&store);
}

/// 364 df-1: `system df` exits 0 and rows the pulled and extracted image
/// with nonzero stored and extracted bytes.
#[test]
fn system_df_rows_the_pulled_image_with_nonzero_bytes() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("df1");
    if let Some(reason) = live_skip_reason(&store) {
        eprintln!("SKIP system df: {reason}");
        cleanup(&store);
        return;
    }
    if !pull_or_skip(DEBIAN, &store) {
        cleanup(&store);
        return;
    }
    // 364's tail clauses run containers before df-1, so the image is
    // extracted by the time df rows it. A bare pull leaves ROOTFS at 0 B
    // and the script's own last-field check would refuse it too.
    let extract = run_podvm(&["extract", DEBIAN], &store, Duration::from_secs(900));
    if !extract.status.success() {
        eprintln!(
            "SKIP system df: could not extract {DEBIAN}: {:?}",
            stderr_text(&extract, &["extract"]).trim()
        );
        cleanup(&store);
        return;
    }
    let df = run_podvm(&["system", "df"], &store, Duration::from_secs(120));
    assert_eq!(
        df.status.code(),
        Some(0),
        "system df exited {:?}, stderr was {:?}",
        df.status.code(),
        stderr_text(&df, &["system df"])
    );
    let stdout = stdout_text(&df, &["system df"]);
    assert!(
        stdout.contains("REPOSITORY"),
        "system df printed no header: {stdout:?}"
    );
    let row = stdout
        .lines()
        .find(|l| l.contains("debian"))
        .unwrap_or_else(|| panic!("system df has no debian row: {stdout:?}"))
        .to_string();
    // 364 reads the last two columns (stored, rootfs) and refuses a zero
    // in either field exactly, not a "0 B" anywhere in the row.
    let fields: Vec<&str> = row.split_whitespace().collect();
    assert!(
        fields.len() >= 4,
        "debian row is too short to bill: {row:?} in {stdout:?}"
    );
    let stored = fields[fields.len() - 4..fields.len() - 2].join(" ");
    let rootfs = fields[fields.len() - 2..].join(" ");
    assert!(
        stored != "0 B" && rootfs != "0 B",
        "debian row bills nothing: stored={stored:?} rootfs={rootfs:?} in {row:?}"
    );
    cleanup(&store);
}

/// 364 df-2, live shape: with an image present the totals lines are still
/// there and the store is untouched by df (the image count is identical
/// before and after).
#[test]
fn system_df_leaves_a_live_store_untouched() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("df2");
    if let Some(reason) = live_skip_reason(&store) {
        eprintln!("SKIP system df totals: {reason}");
        cleanup(&store);
        return;
    }
    if !pull_or_skip(DEBIAN, &store) {
        cleanup(&store);
        return;
    }
    let before = run_podvm(&["images", "-q"], &store, Duration::from_secs(60));
    assert_eq!(
        before.status.code(),
        Some(0),
        "images -q exited {:?}, stderr was {:?}",
        before.status.code(),
        stderr_text(&before, &["images -q"])
    );
    let df = run_podvm(&["system", "df"], &store, Duration::from_secs(120));
    assert_eq!(
        df.status.code(),
        Some(0),
        "system df exited {:?}, stderr was {:?}",
        df.status.code(),
        stderr_text(&df, &["system df"])
    );
    let stdout = stdout_text(&df, &["system df"]);
    for line in [
        "Images:",
        "Containers:",
        "Stored blobs:",
        "Extracted rootfs:",
        "Reclaimable:",
    ] {
        assert!(
            stdout.contains(line),
            "system df output has no {line:?} line: {stdout:?}"
        );
    }
    let again = run_podvm(&["system", "df"], &store, Duration::from_secs(120));
    assert_eq!(
        again.status.code(),
        Some(0),
        "second system df exited {:?}",
        again.status.code()
    );
    let after = run_podvm(&["images", "-q"], &store, Duration::from_secs(60));
    assert_eq!(
        stdout_text(&before, &["images -q"]),
        stdout_text(&after, &["images -q"]),
        "df changed the store"
    );
    cleanup(&store);
}

/// 364 df-3: a digest pull is reclaimable, to the byte. df's Reclaimable
/// line equals `image prune -f`'s Total reclaimed space with a dangling
/// image, and df after reads zero.
#[test]
fn system_df_reclaimable_matches_prune_to_the_byte() {
    let _guard = SERIAL.lock().unwrap();
    let store = fresh_store("df3");
    if let Some(reason) = live_skip_reason(&store) {
        eprintln!("SKIP system df reclaimable: {reason}");
        cleanup(&store);
        return;
    }
    if !pull_or_skip(DEBIAN, &store) {
        cleanup(&store);
        return;
    }
    if !pull_or_skip(ALPINE, &store) {
        cleanup(&store);
        return;
    }
    let inspect = run_podvm(
        &["inspect", "--format", "{{.RepoDigests}}", ALPINE],
        &store,
        Duration::from_secs(60),
    );
    assert_eq!(
        inspect.status.code(),
        Some(0),
        "inspect of {ALPINE} exited {:?}, stderr was {:?}",
        inspect.status.code(),
        stderr_text(&inspect, &["inspect"])
    );
    let digest_ref = stdout_text(&inspect, &["inspect"]).trim().to_string();
    assert!(
        !digest_ref.is_empty(),
        "inspect printed no RepoDigests for {ALPINE}"
    );
    let rmi = run_podvm(&["rmi", ALPINE], &store, Duration::from_secs(120));
    assert_eq!(
        rmi.status.code(),
        Some(0),
        "rmi {ALPINE} exited {:?}, stderr was {:?}",
        rmi.status.code(),
        stderr_text(&rmi, &["rmi"])
    );
    let digest_pull = run_podvm(&["pull", &digest_ref], &store, Duration::from_secs(900));
    if !digest_pull.status.success() {
        eprintln!(
            "SKIP system df reclaimable: digest pull of {digest_ref} failed: {:?}",
            stderr_text(&digest_pull, &["pull"]).trim()
        );
        cleanup(&store);
        return;
    }
    let images = run_podvm(&["images"], &store, Duration::from_secs(60));
    let images_stdout = stdout_text(&images, &["images"]);
    assert!(
        images_stdout.contains("<none>"),
        "digest pull is not dangling: {images_stdout:?}"
    );

    let df = run_podvm(&["system", "df"], &store, Duration::from_secs(120));
    assert_eq!(
        df.status.code(),
        Some(0),
        "system df exited {:?}",
        df.status.code()
    );
    let df_stdout = stdout_text(&df, &["system df"]);
    let reclaim_line = df_stdout
        .lines()
        .find(|l| l.starts_with("Reclaimable:"))
        .unwrap_or_else(|| panic!("system df has no Reclaimable line: {df_stdout:?}"))
        .to_string();
    // 364 strips the trailing "(what ...)" note before comparing.
    let r1 = reclaim_line
        .trim_start_matches("Reclaimable:")
        .trim()
        .split(" (")
        .next()
        .unwrap_or("")
        .to_string();
    assert!(
        !r1.is_empty() && r1 != "0 B",
        "Reclaimable reads {r1:?} with a dangling image: {df_stdout:?}"
    );

    let prune = run_podvm(&["image", "prune", "-f"], &store, Duration::from_secs(300));
    assert_eq!(
        prune.status.code(),
        Some(0),
        "image prune -f exited {:?}, stderr was {:?}",
        prune.status.code(),
        stderr_text(&prune, &["image prune -f"])
    );
    let prune_stdout = stdout_text(&prune, &["image prune -f"]);
    let r2 = prune_stdout
        .lines()
        .find(|l| l.contains("Total reclaimed space:"))
        .and_then(|l| l.split("Total reclaimed space:").nth(1))
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    assert!(
        !r2.is_empty(),
        "prune printed no Total reclaimed space line: {prune_stdout:?}"
    );
    assert_eq!(
        r1, r2,
        "prune freed {r2:?} but df reported {r1:?}: {prune_stdout:?}"
    );

    let df_after = run_podvm(&["system", "df"], &store, Duration::from_secs(120));
    assert_eq!(
        df_after.status.code(),
        Some(0),
        "system df after prune exited {:?}",
        df_after.status.code()
    );
    let after_stdout = stdout_text(&df_after, &["system df"]);
    let after_line = after_stdout
        .lines()
        .find(|l| l.starts_with("Reclaimable:"))
        .unwrap_or("");
    assert!(
        after_line.starts_with("Reclaimable: 0 B"),
        "df after prune reads {after_line:?}, want Reclaimable 0 B: {after_stdout:?}"
    );
    cleanup(&store);
}
