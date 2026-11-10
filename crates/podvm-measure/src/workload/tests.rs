use super::*;

#[test]
fn medians_need_exactly_three_runs() {
    let log = "workload=int elapsed=1.0 checksum=aa\nworkload=int elapsed=3.0 checksum=aa\nworkload=int elapsed=2.0 checksum=aa\n";
    assert_eq!(elapsed_median("int", log), "2.0");
    assert_eq!(
        elapsed_median("int", "workload=int elapsed=1.0 checksum=aa\n"),
        ""
    );
    assert_eq!(elapsed_median("sys", log), "");
    let frac = "workload=io elapsed=0.5 checksum=aa\nworkload=io elapsed=1.5 checksum=aa\nworkload=io elapsed=1.0 checksum=aa\n";
    assert_eq!(elapsed_median("io", frac), "1.0");
}

#[test]
fn checksums_deduplicate_per_class() {
    let log = "workload=mem elapsed=1.0 checksum=aa\nworkload=mem elapsed=2.0 checksum=bb\nworkload=sys elapsed=1.0 checksum=cc\n";
    assert_eq!(
        checksums("mem", log),
        vec!["aa".to_string(), "bb".to_string()]
    );
    assert_eq!(checksums("sys", log), vec!["cc".to_string()]);
}

#[test]
fn ratios_print_one_decimal_or_a_dash() {
    assert_eq!(ratio("2.0", "1.0"), "2.0");
    assert_eq!(ratio("1.0", "3.0"), "0.3");
    assert_eq!(ratio("1.0", "0.0"), "-");
    assert_eq!(ratio("", "1.0"), "-");
}

#[test]
fn extras_archive_matches_the_lib_shape() {
    // dev/ directory, dev/console 5:1, an executable init, a trailer:
    // four records, each starting with the newc magic, 4-aligned.
    let archive = extras_archive(b"#!/bin/sh\n");
    assert!(archive.len().is_multiple_of(4));
    let magic = b"070701";
    let mut count = 0;
    for window in archive.windows(magic.len()) {
        if window == magic {
            count += 1;
        }
    }
    assert_eq!(count, 4);
    assert!(archive.windows(12).any(|window| window == b"dev/console\0"));
    assert!(archive.windows(11).any(|window| window == b"TRAILER!!!\0"));
}

#[test]
fn guest_init_resets_so_qemu_exits() {
    // T-1702: `poweroff -f` halts on pc,acpi=off; qemu stays up.
    assert!(GUEST_INIT.ends_with("echo VMR-BENCH-DONE\nreboot -f\n"));
    assert!(!GUEST_INIT.contains("poweroff"));
    assert!(QEMU_FLAGS.contains("-no-reboot"));
}

#[test]
fn run_to_file_keeps_output_and_kills_at_the_timeout() {
    // T-1702: a child that prints and then hangs keeps its output on file
    // and does not outlive the timeout.
    let dir = std::env::temp_dir().join(format!("podvm-t1702-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let log = dir.join("log");
    let ran = run_to_file("sh", &["-c", "echo console; sleep 30"], &dir, &log, 1);
    assert_eq!(ran, None);
    assert_eq!(fs::read_to_string(&log).unwrap(), "console\n");
    let ok = run_to_file("sh", &["-c", "echo done"], &dir, &log, 10);
    assert_eq!(ok, Some(true));
    fs::remove_dir_all(&dir).ok();
}
