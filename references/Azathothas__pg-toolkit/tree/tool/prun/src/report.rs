//! What the launcher decided, and why, when somebody asks.
//!
//! ⭐ A launcher is invisible by design: it is gone by the time the payload
//! prints anything, and every question about it - which loader, which search
//! path, which host directory - is answered by guessing unless it can be made
//! to say. `PRUN_REPORT=1` is that, and the equivalence apparatus reads it.
//!
//! ⛔ It writes to stderr and never to stdout. A payload's stdout is the
//! program's, and a launcher that put one line into it has broken every
//! pipeline the program is used in.
//!
//! SPDX-License-Identifier: 0BSD

/// A record of decisions, printed only when asked for.
pub struct Report {
    on: bool,
    lines: Vec<String>,
}

impl Report {
    pub fn new(on: bool) -> Report {
        Report { on, lines: Vec::new() }
    }

    /// A variable the launcher set.
    pub fn say(&mut self, var: &str, val: &str) {
        if self.on {
            self.lines.push(format!("  {var}={val}"));
        }
    }

    /// A decision that is not a variable.
    pub fn note(&mut self, what: &str) {
        if self.on {
            self.lines.push(format!("  {what}"));
        }
    }

    /// ⛔ A refusal is printed WHETHER OR NOT the report was asked for. A
    /// launcher that silently declined to do something and started the payload
    /// anyway is how a bundle runs against the wrong loader and nobody knows.
    pub fn refused(&mut self, what: &str) {
        eprintln!("prun: {what}");
    }

    /// Print everything collected, under one heading.
    pub fn flush(&mut self, heading: &str) {
        if !self.on || self.lines.is_empty() {
            return;
        }
        eprintln!("prun: {heading}");
        for l in &self.lines {
            eprintln!("{l}");
        }
        self.lines.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_nobody_asked_for_collects_nothing() {
        let mut r = Report::new(false);
        r.say("A", "1");
        r.note("something");
        assert!(r.lines.is_empty());
    }

    #[test]
    fn a_report_that_was_asked_for_keeps_order() {
        let mut r = Report::new(true);
        r.say("A", "1");
        r.note("then this");
        assert_eq!(r.lines, vec!["  A=1".to_string(), "  then this".to_string()]);
    }
}
