use super::*;

pub(super) fn drive_inner(
    root: &Path,
    binary: &str,
    scratch: &Path,
    curl_timeout: u64,
    boot_timeout: u64,
    report: &mut Report,
) -> i32 {
    let payloads = scratch.join("payloads");
    if fs::create_dir_all(&payloads).is_err() {
        eprintln!("SKIP: cannot stage payloads");
        return 2;
    }
    for class in CLASSES {
        let source = root.join(format!("experiments/154-bench-{class}.c"));
        let target = payloads.join(format!("bench-{class}"));
        let target_text = target.to_string_lossy().into_owned();
        let source_text = source.to_string_lossy().into_owned();
        let mut argv: Vec<&str> = BENCH_CFLAGS.split(' ').collect();
        argv.extend(["-o", &target_text, &source_text]);
        match run_capture("gcc", &argv, root, &[], 300) {
            Some((true, _, _)) => {}
            _ => {
                eprintln!("SKIP: bench-{class} did not build static");
                return 2;
            }
        }
        match run_capture("file", &[&target.to_string_lossy()], root, &[], 30) {
            Some((true, stdout, _)) if stdout.contains("statically linked") => {}
            _ => {
                eprintln!("SKIP: bench-{class} is not static");
                return 2;
            }
        }
    }

    report.say("== conditions");
    report.say(&format!(
        "date              {}",
        run_capture("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], root, &[], 10)
            .map(|(_, t, _)| t.trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!(
        "host kernel       {}",
        run_capture("uname", &["-r"], root, &[], 10)
            .map(|(_, t, _)| t.trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!(
        "podvm            {}",
        run_capture(binary, &["version"], root, &[], 60)
            .map(|(_, t, _)| t.trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!(
        "qemu              {}",
        run_capture("qemu-system-x86_64", &["--version"], root, &[], 30)
            .map(|(_, t, _)| t.lines().next().unwrap_or("").trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!(
        "gcc               {}",
        run_capture("gcc", &["--version"], root, &[], 30)
            .map(|(_, t, _)| t.lines().next().unwrap_or("").trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!("image             {}", alpine_ref()));
    report.say(&format!(
        "kernel            {KURL} (sha256 {KERNEL_SHA256})"
    ));
    report.say(&format!("qemu flags        {QEMU_FLAGS}"));
    report.say(&format!("payload cflags    gcc {BENCH_CFLAGS}"));
    report.say(&format!(
        "host io backing   {}",
        run_capture("df", &["-T", &scratch.to_string_lossy()], root, &[], 30)
            .map(|(_, t, _)| t.lines().last().unwrap_or("").trim().to_string())
            .unwrap_or_default()
    ));
    for class in CLASSES {
        let digest = run_capture(
            "sha256sum",
            &[&payloads.join(format!("bench-{class}")).to_string_lossy()],
            root,
            &[],
            30,
        )
        .map(|(_, t, _)| t.split_whitespace().next().unwrap_or("").to_string())
        .unwrap_or_default();
        report.say(&format!("payload bench-{class}  {digest}"));
    }
    report.say("");

    report.say("== 1. host runs");
    let host_dir = scratch.join("host");
    if fs::create_dir_all(&host_dir).is_err() {
        eprintln!("SKIP: cannot stage host runs");
        return 2;
    }
    for class in CLASSES {
        let target = payloads.join(format!("bench-{class}"));
        let log = scratch.join(format!("host-{class}.log"));
        if !bench3(
            &target.to_string_lossy(),
            &host_dir,
            &log,
            &[],
            &[],
            RUN_TIMEOUT,
        ) {
            report.miss(&format!("host bench-{class} failed"));
            continue;
        }
        let text = fs::read_to_string(&log).unwrap_or_default();
        let count = text
            .split('\n')
            .filter(|line| line.starts_with(&format!("workload={class} ")))
            .count();
        if count == 3 {
            report.pass(&format!("host bench-{class} ran three times"));
        } else {
            report.miss(&format!("host bench-{class} ran {count} of 3 times"));
        }
    }

    report.say("");
    report.say("== 2. chroot runs through podvm");
    // A store of this run's own, so the measurement never touches the
    // operator's store.
    let store = scratch.join("store").to_string_lossy().into_owned();
    let store_env = [("PODVM_STORE", store.as_str())];
    match run_capture(binary, &["pull", alpine_ref()], root, &store_env, 900) {
        Some((true, _, _)) => {}
        Some((false, stdout, stderr)) => {
            report.miss("the pull failed");
            for line in format!("{stdout}{stderr}")
                .split('\n')
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .iter()
                .rev()
            {
                report.say(line);
            }
        }
        None => {
            report.miss("the pull failed");
        }
    }
    let mut rootfs = String::new();
    if !report.failed {
        match run_capture(binary, &["extract", alpine_ref()], root, &store_env, 900) {
            Some((true, stdout, _))
                if !stdout.trim().is_empty() && Path::new(stdout.trim()).is_dir() =>
            {
                rootfs = stdout.trim().to_string();
            }
            _ => {
                report.miss("no rootfs extracted");
            }
        }
    }
    let mut chroot_logs: BTreeSet<String> = BTreeSet::new();
    if !rootfs.is_empty() {
        if fs::create_dir_all(Path::new(&rootfs).join("bench")).is_err() {
            eprintln!("SKIP: cannot stage payloads into the rootfs");
            return 2;
        }
        for class in CLASSES {
            if fs::copy(
                payloads.join(format!("bench-{class}")),
                Path::new(&rootfs).join(format!("bench/bench-{class}")),
            )
            .is_err()
            {
                eprintln!("SKIP: cannot stage payloads into the rootfs");
                return 2;
            }
        }
        report.say(&format!(
            "chroot io backing {}",
            run_capture("df", &["-T", &rootfs], root, &[], 30)
                .map(|(_, t, _)| t.lines().last().unwrap_or("").trim().to_string())
                .unwrap_or_default()
        ));
        if fs::create_dir_all(scratch.join("chroot")).is_err() {
            eprintln!("SKIP: cannot stage chroot runs");
            return 2;
        }
        for class in CLASSES {
            let log = scratch.join(format!("chroot-{class}.log"));
            let err_log = scratch.join(format!("chroot-{class}.err"));
            let guest_path = format!("/bench/bench-{class}");
            let mut combined = String::new();
            let mut errors = String::new();
            for _ in 1..=3 {
                // The script breaks the loop on a failed run and counts
                // lines, so a short log fails the row below, not here.
                match run_capture(
                    binary,
                    &["run", "--pull=never", alpine_ref(), &guest_path],
                    &scratch.join("chroot"),
                    &store_env,
                    RUN_TIMEOUT,
                ) {
                    Some((true, stdout, stderr)) => {
                        combined.push_str(&stdout);
                        errors.push_str(&stderr);
                    }
                    Some((false, stdout, stderr)) => {
                        combined.push_str(&stdout);
                        errors.push_str(&stderr);
                        errors.push_str("run rc=1\n");
                        break;
                    }
                    None => {
                        errors.push_str("run timed out\n");
                        break;
                    }
                }
            }
            fs::write(&log, &combined).ok();
            fs::write(&err_log, &errors).ok();
            chroot_logs.insert(class.to_string());
            let count = combined
                .split('\n')
                .filter(|line| line.starts_with(&format!("workload={class} ")))
                .count();
            if count == 3 {
                report.pass(&format!(
                    "chroot bench-{class} ran through podvm three times"
                ));
            } else {
                report.miss(&format!("chroot bench-{class} ran {count} of 3 times"));
                for line in errors
                    .split('\n')
                    .rev()
                    .take(3)
                    .collect::<Vec<_>>()
                    .iter()
                    .rev()
                {
                    report.say(line);
                }
            }
        }
    }

    report.say("");
    report.say("== 3. guest assembly and batch boot");
    let mut boot_log = String::new();
    if !report.failed && !rootfs.is_empty() {
        if base_archive(Path::new(&rootfs), &scratch.join("base.cpio")) {
            report.say("  base archive built");
        } else {
            report.miss("the base cpio did not build");
        }
    }
    if !report.failed {
        let init = GUEST_INIT;
        let extras = extras_archive(init.as_bytes());
        if fs::write(scratch.join("extras.cpio"), &extras).is_err() || extras.is_empty() {
            report.miss("the extras archive did not build");
        }
        let mut full = fs::read(scratch.join("base.cpio")).unwrap_or_default();
        full.extend_from_slice(&extras);
        if full.is_empty() || fs::write(scratch.join("full.cpio"), &full).is_err() {
            report.miss("concat failed");
        } else {
            report.pass(&format!("assembly {} bytes", full.len()));
        }
    }
    if !report.failed {
        let kernel = scratch.join("vmlinuz-virt");
        let archive = scratch.join("alpine-netboot.tar.gz");
        match run_capture(
            "curl",
            &[
                "-fsSL",
                "--max-time",
                &curl_timeout.to_string(),
                "-o",
                &archive.to_string_lossy(),
                KURL,
            ],
            root,
            &[],
            curl_timeout + 120,
        ) {
            Some((true, _, _)) => {}
            _ => {
                report.miss("the kernel did not fetch");
            }
        }
        if !report.failed {
            let scratch_dir = scratch.to_string_lossy().into_owned();
            match run_capture(
                "tar",
                &[
                    "-xzf",
                    &archive.to_string_lossy(),
                    "-C",
                    &scratch_dir,
                    KARCHIVE_MEMBER,
                ],
                root,
                &[],
                120,
            ) {
                Some((true, _, _))
                    if fs::rename(scratch.join(KARCHIVE_MEMBER), &kernel).is_ok() => {}
                _ => {
                    report.miss("the kernel did not extract from the netboot archive");
                }
            }
            let _ = fs::remove_file(&archive);
        }
        if !report.failed {
            match run_capture("sha256sum", &[&kernel.to_string_lossy()], root, &[], 60) {
                Some((true, stdout, _)) if stdout.starts_with(KERNEL_SHA256) => {
                    report.say("  kernel hash matches the pin");
                }
                Some((true, stdout, _)) => {
                    report.say(&format!(
                        "  kernel sha256: {}",
                        stdout.split_whitespace().next().unwrap_or("")
                    ));
                    report.miss("the kernel hash does not match the pin");
                }
                _ => {
                    report.miss("the kernel hash does not match the pin");
                }
            }
        }
    }
    if !report.failed && scratch.join("full.cpio").is_file() {
        let mut argv: Vec<&str> = QEMU_FLAGS.split(' ').collect();
        let kernel = scratch.join("vmlinuz-virt").to_string_lossy().into_owned();
        let initrd = scratch.join("full.cpio").to_string_lossy().into_owned();
        argv.extend([
            "-kernel",
            &kernel,
            "-initrd",
            &initrd,
            "-append",
            "console=ttyS0 panic=-1",
        ]);
        let boot = scratch.join("boot.log");
        let ran = run_to_file("qemu-system-x86_64", &argv, root, &boot, boot_timeout);
        boot_log = fs::read(&boot)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        match ran {
            Some(ok) => report.say(&format!(
                "  qemu exited {} with {} console bytes",
                if ok { "0" } else { "nonzero" },
                boot_log.len()
            )),
            None => report.say(&format!(
                "  qemu did not return in {boot_timeout}s (killed) or failed to spawn; {} console bytes on file",
                boot_log.len()
            )),
        }
        // The guest ends with `reboot -f`: with -no-reboot qemu then exits
        // (T-1702). `poweroff -f` halts on pc,acpi=off and qemu stays up.
        report.say("  qemu ran to the boot log");
        if boot_log.contains("VMR-BENCH-DONE") {
            report.pass("the guest ran the batch to VMR-BENCH-DONE");
        } else {
            report.miss("no done marker in the boot log");
            for line in boot_log
                .split('\n')
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .iter()
                .rev()
            {
                report.say(line);
            }
        }
    }

    report.say("");
    report.say("== 4. one row per class");
    let boot_text = fs::read_to_string(scratch.join("boot.log")).unwrap_or(boot_log);
    for class in CLASSES {
        let host_text =
            fs::read_to_string(scratch.join(format!("host-{class}.log"))).unwrap_or_default();
        let chroot_text = if chroot_logs.contains(class) {
            fs::read_to_string(scratch.join(format!("chroot-{class}.log"))).unwrap_or_default()
        } else {
            String::new()
        };
        let he = elapsed_median(class, &host_text);
        let ce = elapsed_median(class, &chroot_text);
        let ge = elapsed_median(class, &boot_text);
        let mut all = BTreeSet::new();
        for sum in checksums(class, &host_text)
            .into_iter()
            .chain(checksums(class, &chroot_text))
            .chain(checksums(class, &boot_text))
        {
            all.insert(sum);
        }
        let joined = all.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
        if all.len() == 1 && !he.is_empty() && !ce.is_empty() && !ge.is_empty() {
            report.pass(&format!(
                "{class}: host {he}s chroot {ce}s ({}x) guest {ge}s ({}x) checksum {joined}",
                ratio(&ce, &he),
                ratio(&ge, &he)
            ));
        } else {
            let hc = checksums(class, &host_text).join(" ");
            let cc = checksums(class, &chroot_text).join(" ");
            let gc = checksums(class, &boot_text).join(" ");
            report.miss(&format!(
                "{class}: host=[{he}/{hc}] chroot=[{ce}/{cc}] guest=[{ge}/{gc}]"
            ));
        }
    }
    if report.failed {
        1
    } else {
        0
    }
}
