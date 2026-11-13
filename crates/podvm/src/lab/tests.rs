//! TODO/net.md T-1662: the host side of `podvm lab`.

use super::*;
use pod_netlab::Family::{V4, V6};
use pod_netlab::{Counter, Expectations, Probe, Verdict};

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn expect_of(name: &str) -> Expectations {
    let (n, k) = knobs_of(&args(&[name])).unwrap();
    Lab::new(PREFIX, &n, k).unwrap().expect()
}

fn verdicts(e: &Expectations, p: Probe) -> Vec<Verdict> {
    e.get(p)
        .unwrap_or_else(|| panic!("no probe {p}"))
        .verdicts
        .clone()
}

fn must(e: &Expectations, c: Counter) -> Must {
    e.counters
        .iter()
        .find(|(k, _)| *k == c)
        .map(|(_, m)| *m)
        .unwrap()
}

/// The first clause of the Prove: the verdicts of two scenarios derive
/// from their knobs, and differ where the knobs differ.
#[test]
fn the_verdicts_of_two_scenarios_derive_from_their_knobs() {
    let open = expect_of("dual-nat");
    assert_eq!(verdicts(&open, Probe::Tcp(V4, 443)), [Verdict::Allow]);
    assert_eq!(verdicts(&open, Probe::Tcp(V6, 22)), [Verdict::Allow]);
    assert_eq!(verdicts(&open, Probe::Inbound(V4)), [Verdict::Dropped]);
    assert_eq!(must(&open, Counter::Return), Must::Move);
    let shut = expect_of("blackout");
    assert_eq!(verdicts(&shut, Probe::Tcp(V4, 443)), [Verdict::Dropped]);
    assert_eq!(
        must(&shut, Counter::Return),
        Must::Stay,
        "nothing passes a blackout"
    );
    assert_eq!(must(&shut, Counter::Dropped), Must::Move);
}

#[test]
fn flags_give_the_knobs_of_the_same_scenario() {
    let flags = scenario("web-only").unwrap().knobs.flags();
    let (name, knobs) = knobs_of(&flags).unwrap();
    assert_eq!(name, CUSTOM);
    assert_eq!(knobs, scenario("web-only").unwrap().knobs);
}

#[test]
fn the_plan_names_the_steps_the_probes_and_the_counters() {
    let (n, k) = knobs_of(&args(&["dual-nat"])).unwrap();
    let text = plan_text(&n, k).unwrap();
    for part in [
        "lab dual-nat:",
        "== up, in the guest",
        "== down, in the guest",
        "pv-dual-nat-g",
        "tcp4-443",
    ] {
        assert!(text.contains(part), "no {part:?} in the plan:\n{text}");
    }
    assert!(
        text.lines()
            .any(|l| l.contains("tcp4-443") && l.contains("ALLOW")),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|l| l.contains("dropped") && l.contains("must stay")),
        "{text}"
    );
}

#[test]
fn each_scenario_plans() {
    let text = scenarios_text();
    assert_eq!(text.lines().count(), pod_netlab::scenario::NAMES.len());
    for s in scenarios() {
        plan_text(s.name, s.knobs).unwrap_or_else(|e| panic!("{}: {e}", s.name));
    }
}

#[test]
fn a_bad_knob_set_is_refused_by_name() {
    assert!(knobs_of(&args(&["no-such-lab"]))
        .unwrap_err()
        .contains("no-such-lab"));
    assert!(knobs_of(&args(&["--no-such-knob"])).is_err());
    assert!(knobs_of(&args(&["--stack", "v7"])).is_err());
    assert!(knobs_of(&[]).is_err());
    assert_eq!(main(&args(&["plan", "--stack", "v7"])), EXIT_FLAG_ERROR);
}

#[test]
fn the_guest_verbs_refuse_and_name_what_they_wait_for() {
    for v in GUEST_VERBS {
        assert_eq!(main(&args(&[v, "dual-nat"])), EXIT_FLAG_ERROR, "{v}");
        let why = guest_refusal(v);
        assert!(
            why.contains("podcrates@T-345") && why.contains("podvm lab plan"),
            "{why}"
        );
    }
}
