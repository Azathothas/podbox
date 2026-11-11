use super::*;

fn v(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// TODO/cli.md T-1655. `--timeout` takes whole seconds over 0, in both
/// spellings, and carries them to the run; 0, a fraction and a word are
/// flag errors.
#[test]
fn the_timeout_takes_whole_seconds_over_zero() {
    assert_eq!(
        parse("run", &v(&["--timeout", "2", "img"]))
            .unwrap()
            .timeout,
        Some(2)
    );
    assert_eq!(
        parse("run", &v(&["--timeout=30", "img"])).unwrap().timeout,
        Some(30)
    );
    assert_eq!(parse("run", &v(&["img"])).unwrap().timeout, None);
    for bad in ["0", "1.5", "two", "-1", ""] {
        assert_eq!(
            parse("run", &v(&["--timeout", bad, "img"])).unwrap_err(),
            EXIT_FLAG_ERROR,
            "{bad:?}"
        );
    }
}
