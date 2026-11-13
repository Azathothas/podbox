//! `podvm lab`: a network lab inside one guest (TODO/net.md T-1662, issue
//! 178).
//!
//! The host side, on the offline part of `pod-netlab` (podcrates@T-207):
//! `scenarios` lists the offline scenarios with their knobs, and `plan`
//! checks a knob set and prints the steps that build and remove the lab in
//! the guest and the verdict that each probe must give. The crate runs no
//! command. The verbs that run the lab in a guest (`up`, `down`, `check`,
//! `try`, `matrix`) refuse by name until podcrates gives the guest parts
//! (the probe program, the world and the run: podcrates@T-345 to
//! podcrates@T-347).

#[cfg(test)]
mod tests;

use pod_netlab::{scenario, scenarios, Bound, Expect, Knobs, Lab, Must, Seen, Settings};
use podvm_image::error::EXIT_FLAG_ERROR;

const USAGE: &str = "\
usage: podvm lab scenarios
       podvm lab plan SCENARIO
       podvm lab plan [--stack S] [--nat N] [--policy P] [KNOB-FLAG]...

A network lab inside one guest: a guest, a router and a world namespace,
with NAT, a policy and impairment (TODO/net.md T-1662). `scenarios` lists
the offline scenarios with their knobs. `plan` checks the knobs of a
scenario, or of the flags of sand-net, and prints the steps that build
and remove the lab in the guest and the verdict that each probe must give.
up, down, check, try and matrix run the lab in a guest; they wait for the
guest parts of pod-netlab (podcrates@T-345 to podcrates@T-347).";

/// The prefix of the namespaces of a lab (`pv-NAME-g`, `-r`, `-w`).
pub const PREFIX: &str = "pv";
/// The name of a lab that flags give, not a scenario.
pub const CUSTOM: &str = "custom";
/// The guest directory of the lab's files, and the group of the probes:
/// the probes run as root in the guest.
const SETTINGS_DIR: &str = "/run/podvm-lab";
const PING_GID: u32 = 0;

/// The verbs that run a lab in a guest, refused until the guest parts exist.
const GUEST_VERBS: [&str; 5] = ["up", "down", "check", "try", "matrix"];

pub fn main(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        None => {
            eprintln!("{USAGE}");
            EXIT_FLAG_ERROR
        }
        Some("-h" | "--help") => {
            println!("{USAGE}");
            0
        }
        Some("scenarios") => {
            print!("{}", scenarios_text());
            0
        }
        Some("plan") => match knobs_of(&args[1..]).and_then(|(n, k)| plan_text(&n, k)) {
            Ok(text) => {
                print!("{text}");
                0
            }
            Err(e) => {
                if !e.is_empty() {
                    eprintln!("podvm lab plan: {e}");
                }
                EXIT_FLAG_ERROR
            }
        },
        Some(v) if GUEST_VERBS.contains(&v) => {
            eprintln!("{}", guest_refusal(v));
            EXIT_FLAG_ERROR
        }
        Some(other) => {
            eprintln!("podvm lab: {other} is not a verb of lab\n{USAGE}");
            EXIT_FLAG_ERROR
        }
    }
}

/// The refusal of a verb that runs the lab in a guest.
pub fn guest_refusal(verb: &str) -> String {
    format!(
        "podvm lab {verb}: refused: the lab runs in a guest, and the guest parts of \
         pod-netlab (the probe program, the world and the run, podcrates@T-345 to \
         podcrates@T-347) are not released. `podvm lab plan` gives the steps and the \
         expected verdicts now (TODO/net.md T-1662)"
    )
}

/// Each offline scenario, one line each: its name and its flags.
pub fn scenarios_text() -> String {
    let mut out = String::new();
    for s in scenarios() {
        let flags = s.knobs.flags().join(" ");
        let from = if s.given { "issue" } else { "name" };
        out.push_str(&format!("{:<22} {from:<5} {flags}\n", s.name));
    }
    out
}

/// The name and the knobs of `plan`'s arguments: one scenario name, or
/// the flags of sand-net (each flag admitted by the parity table first).
pub fn knobs_of(args: &[String]) -> Result<(String, Knobs), String> {
    match args {
        [] => Err("give a scenario or knob flags (podvm lab scenarios lists them)".into()),
        [name] if !name.starts_with('-') => scenario(name)
            .map(|s| (s.name.to_string(), s.knobs))
            .ok_or_else(|| {
                format!("{name} is not an offline scenario (podvm lab scenarios lists them)")
            }),
        _ => {
            for a in args.iter().filter(|a| a.starts_with("--")) {
                // The table refuses an unlisted flag and says so itself: an empty
                // error tells `main` to add nothing.
                crate::parity::admit("lab", a, USAGE).map_err(|_| String::new())?;
            }
            Knobs::parse_flags(args)
                .map(|k| (CUSTOM.to_string(), k))
                .map_err(|e| e.to_string())
        }
    }
}

/// The plan of a lab: its steps up and down, and its expectations.
pub fn plan_text(name: &str, knobs: Knobs) -> Result<String, String> {
    let lab = Lab::new(PREFIX, name, knobs).map_err(|e| e.to_string())?;
    let settings = Settings {
        state_dir: SETTINGS_DIR.into(),
        ping_gid: PING_GID,
    };
    let up = lab.up(&settings).map_err(|e| e.to_string())?;
    let down = lab.down(&settings).map_err(|e| e.to_string())?;
    let expect = lab.expect();
    let mut out = format!("lab {name}: {}\n", lab.knobs().flags().join(" "));
    out.push_str(&format!("== up, in the guest ({} steps)\n", up.len()));
    for s in &up {
        out.push_str(&format!("  {}\n", s.line()));
    }
    out.push_str(&format!("== down, in the guest ({} steps)\n", down.len()));
    for s in &down {
        out.push_str(&format!("  {}\n", s.line()));
    }
    out.push_str("== the verdict of each probe, from the knobs\n");
    for e in &expect.probes {
        out.push_str(&format!("  {}\n", expect_line(e)));
    }
    out.push_str("== the counters between the two reads of the check\n");
    for (c, m) in &expect.counters {
        let word = match m {
            Must::Move => "must move",
            Must::Stay => "must stay",
        };
        out.push_str(&format!("  {:<10} {word}\n", c.name()));
    }
    Ok(out)
}

/// One probe: its name, its verdicts, what it must see, its bound and why.
pub fn expect_line(e: &Expect) -> String {
    let verdicts: Vec<String> = e.verdicts.iter().map(ToString::to_string).collect();
    let mut line = format!("{:<16} {:<10}", e.probe.to_string(), verdicts.join("|"));
    match e.seen {
        Some(Seen::Addr(a)) => line.push_str(&format!(" sees {a}")),
        Some(Seen::PortKept) => line.push_str(" keeps its port"),
        Some(Seen::PortMoved) => line.push_str(" gets another port"),
        None => {}
    }
    match e.bound {
        Some(Bound::AtLeastMs(ms)) => line.push_str(&format!(" at least {ms} ms")),
        Some(Bound::Between(a, b)) => line.push_str(&format!(" {a} to {b} echoed")),
        None => {}
    }
    line.push_str(&format!("  ({})", e.why));
    line
}
