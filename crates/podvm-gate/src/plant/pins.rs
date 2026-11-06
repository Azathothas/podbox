use super::*;

fn cargo_lib(pkg: &str, test: &str) -> Vec<String> {
    vec![
        "test".to_string(),
        "-p".to_string(),
        pkg.to_string(),
        "--lib".to_string(),
        test.to_string(),
    ]
}

fn cargo_bin(pkg: &str, bin: &str, test: &str) -> Vec<String> {
    vec![
        "test".to_string(),
        "-p".to_string(),
        pkg.to_string(),
        "--bin".to_string(),
        bin.to_string(),
        test.to_string(),
    ]
}

fn cargo_test(pkg: &str, target: &str, test: &str) -> Vec<String> {
    vec![
        "test".to_string(),
        "-p".to_string(),
        pkg.to_string(),
        "--test".to_string(),
        target.to_string(),
        test.to_string(),
    ]
}

// The interposer is excluded from the workspace, so its tests run through
// its own manifest on the gnu target, which is the gate workflow's own
// command for them.
fn cargo_interpose(test: &str) -> Vec<String> {
    vec![
        "test".to_string(),
        "--manifest-path".to_string(),
        "crates/podvm-interpose/Cargo.toml".to_string(),
        "--target".to_string(),
        "x86_64-unknown-linux-gnu".to_string(),
        test.to_string(),
    ]
}

fn interpose_env() -> Vec<(String, String)> {
    vec![(
        "RUSTFLAGS".to_string(),
        "-C target-feature=-crt-static".to_string(),
    )]
}

// T-1579's pins: one case per wave-2 source and per batch-4 test file.
// Each breaks its file's own test and asserts the failure names the file.
pub(super) fn run_pins(plant: &mut Plant, root: &Path, scratch: &str) {
    let none: Vec<(String, String)> = Vec::new();

    let test = "every_image_usage_is_plain_ascii";
    plant.case_rust_test(
        "32a images",
        "crates/podvm/src/images/tests.rs",
        test,
        &cargo_bin("podvm", "podvm", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm/src/images/tests.rs", test),
    );

    let test = "system_usage_is_plain_ascii";
    plant.case_rust_test(
        "32b system",
        "crates/podvm/src/system/tests.rs",
        test,
        &cargo_bin("podvm", "podvm", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm/src/system/tests.rs", test),
    );

    let test = "every_parity_note_is_plain_ascii";
    plant.case_rust_test(
        "32c parity",
        "crates/podvm/src/parity/tests.rs",
        test,
        &cargo_bin("podvm", "podvm", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm/src/parity/tests.rs", test),
    );

    let test = "a_write_through_an_absolute_symlink_does_not_escape";
    plant.case_rust_test(
        "32d write",
        "crates/podvm-complete/src/write/tests.rs",
        test,
        &cargo_lib("podvm-complete", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm-complete/src/write/tests.rs", test),
    );

    let test = "a_decimal_fd_parses_and_anything_else_does_not";
    plant.case_rust_test(
        "32e memo",
        "crates/podvm-interpose/src/memo.rs",
        test,
        &cargo_interpose(test),
        &interpose_env(),
        scratch,
        &|| pin_test(root, "crates/podvm-interpose/src/memo.rs", test),
    );

    let test = "uid_alone_means_its_own_group";
    plant.case_rust_test(
        "32f identity",
        "crates/podvm-interpose/src/identity.rs",
        test,
        &cargo_interpose(test),
        &interpose_env(),
        scratch,
        &|| pin_test(root, "crates/podvm-interpose/src/identity.rs", test),
    );

    let test = "a_sigkilled_payload_is_137";
    plant.case_rust_test(
        "32g exit",
        "crates/podvm-probe/src/exit.rs",
        test,
        &cargo_lib("podvm-probe", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm-probe/src/exit.rs", test),
    );

    let test = "refused_run_flags_exit_125_naming_status_none";
    plant.case_rust_test(
        "33a curated_refusals",
        "crates/podvm/tests/curated_refusals.rs",
        test,
        &cargo_test("podvm", "curated_refusals", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm/tests/curated_refusals.rs", test),
    );

    let test = "run_help_names_the_detach_flag";
    plant.case_rust_test(
        "33b detached_stdio",
        "crates/podvm/tests/detached_stdio.rs",
        test,
        &cargo_test("podvm", "detached_stdio", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm/tests/detached_stdio.rs", test),
    );

    let test = "logs_tail_with_a_non_count_is_a_flag_error";
    plant.case_rust_test(
        "33c qol",
        "crates/podvm/tests/qol.rs",
        test,
        &cargo_test("podvm", "qol", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm/tests/qol.rs", test),
    );

    let test = "empty_store_lists_nothing_and_marks_nothing";
    plant.case_rust_test(
        "33d store_gates",
        "crates/podvm/tests/store_gates.rs",
        test,
        &cargo_test("podvm", "store_gates", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm/tests/store_gates.rs", test),
    );

    let test = "refusal_triggers_only_where_the_open_row_is_denied";
    plant.case_rust_test(
        "33e tty_refusal",
        "crates/podvm-enter/tests/tty_refusal.rs",
        test,
        &cargo_test("podvm-enter", "tty_refusal", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm-enter/tests/tty_refusal.rs", test),
    );

    let test = "pull_the_fixture_then_pull_it_again_for_up_to_date";
    plant.case_rust_test(
        "33f acquisition",
        "crates/podvm-image/tests/acquisition.rs",
        test,
        &cargo_test("podvm-image", "acquisition", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm-image/tests/acquisition.rs", test),
    );

    let test = "the_reference_is_qualified_before_it_is_pulled";
    plant.case_rust_test(
        "33g across_distributions",
        "crates/podvm-image/tests/across_distributions.rs",
        test,
        &cargo_test("podvm-image", "across_distributions", test),
        &none,
        scratch,
        &|| {
            pin_test(
                root,
                "crates/podvm-image/tests/across_distributions.rs",
                test,
            )
        },
    );

    let test = "plant_common_mod_pin";
    plant.case_rust_test(
        "33h common mod",
        "crates/podvm-image/tests/common/mod.rs",
        test,
        &cargo_test("podvm-image", "acquisition", test),
        &none,
        scratch,
        &|| pin_module(root, "crates/podvm-image/tests/common/mod.rs", test),
    );

    let test = "the_fixture_serves_manifest_blobs_and_shutdown";
    plant.case_rust_test(
        "33i common registry",
        "crates/podvm-image/tests/common/registry.rs",
        test,
        &cargo_test("podvm-image", "registry_fixture", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm-image/tests/common/registry.rs", "start"),
    );

    let test = "concurrent_pulls_against_one_fixture_all_resolve_to_the_seeded_digest";
    plant.case_rust_test(
        "33j parallel_layers",
        "crates/podvm-image/tests/parallel_layers.rs",
        test,
        &cargo_test("podvm-image", "parallel_layers", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm-image/tests/parallel_layers.rs", test),
    );

    let test = "the_fixture_serves_manifest_blobs_and_shutdown";
    plant.case_rust_test(
        "33k registry_fixture",
        "crates/podvm-image/tests/registry_fixture.rs",
        test,
        &cargo_test("podvm-image", "registry_fixture", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm-image/tests/registry_fixture.rs", test),
    );

    let test = "the_record_and_the_blobs_name_the_served_bytes";
    plant.case_rust_test(
        "33m store_digest",
        "crates/podvm-image/tests/store_digest.rs",
        test,
        &cargo_test("podvm-image", "store_digest", test),
        &none,
        scratch,
        &|| pin_test(root, "crates/podvm-image/tests/store_digest.rs", test),
    );
}
