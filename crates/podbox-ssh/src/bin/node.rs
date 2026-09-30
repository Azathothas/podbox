//! The relay node: register a pair name, serve every operator session on
//! the one socket, redial on loss.
//!
//! `node --relay wss://HOST --name NAME --node-token TOKEN [--server CMD] [--once]`
//!
//! ⛔ Nothing here listens. The sandbox dials out and registers, which is the
//! only shape that works where `bind(2)` is denied. Flags override the
//! `PODSSH_RELAY`, `PODSSH_NAME`, `PODSSH_NODE_TOKEN`, and `PODSSH_SERVER`
//! environment; the token never appears in a log line or an error, so
//! passing it on the command line stays out of every committed artefact.
//!
//! ⛔ Every read waits bounded. The dial carries a connect timeout, the DNS
//! lookup carries the tighter of that and `DNS_DEADLINE`, the hello carries
//! its own window, relay-leg writes carry `WRITE_DEADLINE`, and the redial
//! backs off 1 s to 30 s with jitter. A relay that stops reading ends the
//! socket inside the write bound, and the node redials.
//! `--once` exits after the first socket ends: 0 when at least one
//! session completed, 1 when none did.

#![forbid(unsafe_code)]

use podbox_ssh::mux::NodeConfig;
use podbox_ssh::{Error, Kind};

const USAGE: &str =
    "usage: node --relay wss://HOST[:PORT] --name NAME --node-token TOKEN [--server CMD] [--once]";

fn main() {
    std::process::exit(run(&std::env::args().skip(1).collect::<Vec<_>>()));
}

fn run(argv: &[String]) -> i32 {
    match run_inner(argv) {
        Ok(stats) => {
            eprintln!(
                "podbox-ssh node: ended: {} socket(s), {} completed, {} refused",
                stats.sockets, stats.sessions_completed, stats.sessions_refused
            );
            if stats.sessions_completed == 0 {
                1
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("{e}");
            if e.kind() == Kind::Usage {
                eprintln!("{USAGE}");
            }
            e.exit_code()
        }
    }
}

fn run_inner(argv: &[String]) -> Result<podbox_ssh::mux::NodeStats, Error> {
    let cfg = parse_args(argv)?;
    podbox_ssh::mux::run_node(&cfg)
}

fn parse_args(argv: &[String]) -> Result<NodeConfig, Error> {
    let usage = || Error::new(Kind::Usage, "node args", USAGE);
    let mut relay = std::env::var("PODSSH_RELAY").unwrap_or_default();
    let mut name = std::env::var("PODSSH_NAME").unwrap_or_default();
    let mut token = std::env::var("PODSSH_NODE_TOKEN").unwrap_or_default();
    let mut server = std::env::var("PODSSH_SERVER").unwrap_or_default();
    let mut once = false;
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
            "--node-token" => {
                i += 1;
                token = argv.get(i).ok_or_else(usage)?.clone();
            }
            "--server" => {
                i += 1;
                server = argv.get(i).ok_or_else(usage)?.clone();
            }
            "--once" => once = true,
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
    Ok(NodeConfig {
        relay,
        name,
        node_token: token,
        server_command: if server.trim().is_empty() {
            None
        } else {
            Some(server)
        },
        once,
    })
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
            "--node-token",
            "tok",
            "--server",
            "cat",
            "--once",
        ]))
        .unwrap();
        assert_eq!(cfg.relay, "wss://example.com");
        assert_eq!(cfg.name, "n1");
        assert!(cfg.once);
        assert_eq!(cfg.server_command.as_deref(), Some("cat"));
    }

    #[test]
    fn missing_and_bad_fields_are_usage_errors() {
        for argv in [
            args(&[]),
            args(&["--relay", "wss://example.com"]),
            args(&["--relay", "wss://example.com", "--name", "n1"]),
            args(&[
                "--relay",
                "wss://example.com",
                "--name",
                "n1",
                "--node-token",
                "tok",
                "--bogus",
            ]),
            args(&[
                "--relay",
                "http://example.com",
                "--name",
                "n1",
                "--node-token",
                "tok",
            ]),
            args(&[
                "--relay",
                "wss://example.com",
                "--name",
                "has space",
                "--node-token",
                "tok",
            ]),
            args(&[
                "--relay",
                "wss://example.com",
                "--name",
                "n1",
                "--node-token",
                "has space",
            ]),
            args(&["--relay"]),
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
}
