use super::compare::{compare, parse_expected};
use super::scrub::{scrub, REDACTED};
use super::*;

/// The plant of the Prove: a token in a detail is REDACTED in the line.
#[test]
fn a_planted_token_is_redacted() {
    let token = "ghp_0123456789abcdefABCDEF0123456789abcd";
    let r = Record::new("env", "GITHUB_TOKEN", Verdict::Allow, token);
    let line = r.line();
    assert!(!line.contains(token), "{line}");
    assert!(line.ends_with(REDACTED), "{line}");
    for (input, gone) in [
        ("HTTPS_PROXY=http://user:hunter2@proxy:3128", "hunter2"),
        ("API_KEY=abc", "abc"),
        (
            "Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N",
            "eyJ",
        ),
        ("aB3dE5gH7jK9mN1pQ3sT5vX7zA9cE1gI3k", "aB3d"),
        ("AKIAIOSFODNN7EXAMPLE", "AKIA"),
    ] {
        let s = scrub(input);
        assert!(!s.contains(gone), "{input} -> {s}");
    }
    // A digest and a plain word stay.
    let digest = "sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc";
    assert_eq!(scrub(digest), digest);
    assert_eq!(scrub("EPERM"), "EPERM");
}

#[test]
fn a_line_parses_back_to_its_record() {
    let r = Record::new("escalation", "unshare-user", Verdict::Deny, "EPERM");
    assert_eq!(Record::parse(&r.line()), Some(r));
    assert_eq!(Record::parse("# a note"), None);
    assert_eq!(Record::parse("a\tb\tMAYBE\tc"), None);
    assert_eq!(Verdict::of_errno(2), Verdict::Absent);
    assert_eq!(Verdict::of_errno(1), Verdict::Deny);
    assert_eq!(Verdict::of_errno(111), Verdict::Refused);
}

fn report(lines: &[(&str, &str, Verdict)]) -> Vec<Record> {
    lines
        .iter()
        .map(|(s, t, v)| Record::new(s, t, *v, ""))
        .collect()
}

/// The Prove: two reports of one tier compare clean; a planted escape in
/// an invariant section fails a strict compare.
#[test]
fn the_compare_passes_one_tier_and_fails_a_planted_escape() {
    let a = report(&[
        ("escalation", "unshare-user", Verdict::Deny),
        ("escape-surface", "host-store", Verdict::Absent),
        ("identity", "pid", Verdict::Allow),
    ]);
    let o = compare(&a, &a, &[]);
    assert_eq!(o.code(true), 0);
    assert_eq!(o.same, 3);
    // The plant: the host store is reachable.
    let mut b = a.clone();
    b[1].verdict = Verdict::Allow;
    let o = compare(&b, &a, &[]);
    assert_eq!(o.code(true), 1, "{}", o.summary());
    assert_eq!(o.code(false), 0, "only --strict fails");
    assert_eq!(o.unexplained().len(), 1);
    // A cause in the expected-differences file explains it.
    let e = parse_expected("# why\nescape-surface\thost-store\tplanted for the test\n");
    assert_eq!(compare(&b, &a, &e).code(true), 0);
    // A difference outside the invariant sections never fails.
    let mut c = a.clone();
    c[2].verdict = Verdict::Unknown;
    assert_eq!(compare(&c, &a, &[]).code(true), 0);
    // Records on one side only are counted, not failed.
    let o = compare(&a[..2], &a, &[]);
    assert_eq!((o.only_reference, o.only_here, o.code(true)), (1, 0, 0));
}
