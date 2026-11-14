//! TODO/image.md T-1842: the input errors of the image verbs exit 1, as
//! docker 29.9.2 does (measured 2026-11-14), and a bad host of
//! `--insecure-registry` is refused with the value named (podcrates
//! `v0.45.0`, `Refusal::Host`, podcrates@T-585).

use podvm_image::error::{Error, EXIT_CLI_ERROR};

#[test]
fn image_exit_codes_of_input_errors_are_1() {
    assert_eq!(
        Error::NoSuchImage("nosuch".into()).exit_code(),
        EXIT_CLI_ERROR
    );
    assert_eq!(
        Error::Reference("Bad:Name".into()).exit_code(),
        EXIT_CLI_ERROR
    );
    let e = podvm_image::transport::resolve(&["bad host!".to_string()], None).unwrap_err();
    assert_eq!(e.exit_code(), EXIT_CLI_ERROR, "{e}");
    assert!(e.to_string().contains("bad host!"), "{e}");
    assert!(podvm_image::transport::resolve(&["registry.test:5000".to_string()], None).is_ok());
}
