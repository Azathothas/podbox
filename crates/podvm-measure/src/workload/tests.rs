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
