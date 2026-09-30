//! The relay operator: connect one session, move standard descriptors as
//! bare frames, exit with the session.
//!
//! `operator --relay wss://HOST --name NAME --connect-token TOKEN [--ready-wait SECS]`
//!
//! ⛔ This binary is a valid SSH `ProxyCommand`. SSH writes and reads this
//! process's standard descriptors, and the operator moves those bytes to the
//! pair's session and back. It reports the error (plus the usage line for
//! usage failures) and exits with the runtime code, so the SSH client
//! prints its own diagnosis beside it.
//! Flags override the `PODSSH_RELAY`, `PODSSH_NAME`, and
//! `PODSSH_CONNECT_TOKEN` environment; the token never appears in a log line
//! or an error.
//!
//! ⛔ Every read waits bounded. The dial carries a connect timeout, the DNS
//! lookup carries the tighter of that and `DNS_DEADLINE`, the wait for
//! `ready` defaults to 20 s, relay-leg writes carry `WRITE_DEADLINE`, and
//! the goodbye after stdin ends to 10 s. A relay that stops reading ends
//! the op inside the write bound.
//! Stdin before `ready` is queued up to 1 MiB, then fails loud.

#![forbid(unsafe_code)]

use std::time::Duration;

use podbox_ssh::mux::OperatorConfig;
use podbox_ssh::{Error, Kind};

const USAGE: &str = "usage: operator --relay wss://HOST[:PORT] --name NAME --connect-token TOKEN [--ready-wait SECS]";

fn main() {
    std::process::exit(run(&std::env::args().skip(1).collect::<Vec<_>>()));
}

fn run(argv: &[String]) -> i32 {
    match run_inner(argv) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            if e.kind() == Kind::Usage {
                eprintln!("{USAGE}");
            }
            e.exit_code()
        }
    }
}

fn run_inner(argv: &[String]) -> Result<(), Error> {
    let cfg = parse_args(argv)?;
    // ⛔ Standard descriptors belong to the SSH client. A failure to take
    // them is a server-side error on this end, reported like any other.
    let mut stdio = podbox_ssh::Stdio::new().map_err(|e| Error::server("operator stdio", e))?;
    podbox_ssh::mux::run_operator(&cfg, &mut stdio)
}

fn parse_args(argv: &[String]) -> Result<OperatorConfig, Error> {
    let usage = || Error::new(Kind::Usage, "operator args", USAGE);
    let mut relay = std::env::var("PODSSH_RELAY").unwrap_or_default();
    let mut name = std::env::var("PODSSH_NAME").unwrap_or_default();
    let mut token = std::env::var("PODSSH_CONNECT_TOKEN").unwrap_or_default();
    let mut ready_wait: Option<Duration> = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--relay" => {
                i += 1;
                relay = argv.get(i).ok_or_else(usage)?.clone();
            }
            "--name" => {
                i += 1;
                name = argv.get(i).ok_or_else(usage)?.clone();
            }
            "--connect-token" => {
                i += 1;
                token = argv.get(i).ok_or_else(usage)?.clone();
            }
            "--ready-wait" => {
                i += 1;
                let secs = argv.get(i).ok_or_else(usage)?;
                let n: u64 = secs.parse().map_err(|_| usage())?;
                if n == 0 || n > 300 {
                    return Err(usage());
                }
                ready_wait = Some(Duration::from_secs(n));
            }
            _ => return Err(usage()),
        }
        i += 1;
    }
    if relay.trim().is_empty() || name.trim().is_empty() || token.trim().is_empty() {
        return Err(usage());
    }
    // ⛔ Validated here, before any dial: a bad origin, name, or token is a
    // usage failure naming the field, never a connection attempt.
    podbox_ssh::mux::parse_origin(&relay)?;
    podbox_ssh::mux::validate_name(&name)?;
    podbox_ssh::mux::validate_token(&token)?;
    let mut cfg = OperatorConfig::new(relay, name, token);
    if let Some(w) = ready_wait {
        cfg.ready_wait = w;
    }
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn full_flags_parse() {
        let cfg = parse_args(&args(&[
            "--relay",
            "wss://example.com",
            "--name",
            "n1",
            "--connect-token",
            "tok",
            "--ready-wait",
            "7",
        ]))
        .unwrap();
        assert_eq!(cfg.relay, "wss://example.com");
        assert_eq!(cfg.ready_wait, Duration::from_secs(7));
        let cfg = parse_args(&args(&[
            "--relay",
            "wss://example.com",
            "--name",
            "n1",
            "--connect-token",
            "tok",
        ]))
        .unwrap();
        assert_eq!(cfg.ready_wait, Duration::from_secs(20));
    }

    #[test]
    fn missing_and_bad_fields_are_usage_errors() {
        for argv in [
            args(&[]),
            args(&["--relay", "wss://example.com"]),
            args(&["--relay", "wss://example.com", "--name", "n1"]),
            args(&[
                "--relay",
                "http://example.com",
                "--name",
                "n1",
                "--connect-token",
                "tok",
            ]),
            args(&[
                "--relay",
                "wss://example.com",
                "--name",
                "has space",
                "--connect-token",
                "tok",
            ]),
            args(&[
                "--relay",
                "wss://example.com",
                "--name",
                "n1",
                "--connect-token",
                "has space",
            ]),
            args(&[
                "--relay",
                "wss://example.com",
                "--name",
                "n1",
                "--connect-token",
                "tok",
                "--ready-wait",
                "0",
            ]),
            args(&[
                "--relay",
                "wss://example.com",
                "--name",
                "n1",
                "--connect-token",
                "tok",
                "--ready-wait",
                "301",
            ]),
            args(&[
                "--relay",
                "wss://example.com",
                "--name",
                "n1",
                "--connect-token",
                "tok",
                "--ready-wait",
                "soon",
            ]),
        ] {
            // ⛔ No `unwrap_err`: the config holds a token and must never
            // grow a `Debug` that prints it into a test failure.
            let e = match parse_args(&argv) {
                Ok(_) => panic!("{argv:?} parsed"),
                Err(e) => e,
            };
            assert_eq!(e.kind(), Kind::Usage, "{argv:?}: {e}");
        }
    }

    #[test]
    fn failure_exit_code_is_the_runtime_code() {
        assert_eq!(run(&[]), podbox_ssh::EXIT_RUNTIME_ERROR);
    }
}
