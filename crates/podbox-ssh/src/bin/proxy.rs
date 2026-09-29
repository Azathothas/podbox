//! The stdio-to-target shuttle: `proxy (tcp HOST PORT | unix PATH
//! [--hold-for-banner SECS] | exec PROG [ARGS...])`.
//!
//! ⛔ This binary is a valid SSH `ProxyCommand`. SSH writes and reads this
//! process's standard descriptors, and the pump moves those bytes to the
//! target. It reports one line on failure and exits with the runtime code,
//! so the SSH client prints its own diagnosis beside it.
//!
//! `unix` takes `--hold-for-banner SECS` for servers that open their side
//! late: the proxy buffers both directions until a server version line
//! arrives, then replays in order and pumps. Without it the client's
//! first bytes can land before the server's first open and be lost.
//!
//! ⛔ Every wait here is bounded. The dial carries a connect timeout per
//! address, the hold carries its own deadline, and the pump returns when
//! either side closes. An argument that does not parse is a typed error,
//! never a panic.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::time::Duration;

use podbox_ssh::{parse_port, Dialer, Error, Kind, Stdio, Target};

/// How long one dial waits per resolved address.
const DIAL_TIMEOUT: Duration = Duration::from_secs(10);

const USAGE: &str =
    "usage: proxy (tcp HOST PORT | unix PATH [--hold-for-banner SECS] | exec PROG [ARGS...])";

/// One parsed invocation: where to dial, and whether to hold the
/// client's first bytes for the server's version line.
#[derive(Debug)]
struct ProxyArgs {
    target: Target,
    hold_secs: Option<u64>,
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(run(&argv));
}

fn run(argv: &[String]) -> i32 {
    match run_inner(argv) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            e.exit_code()
        }
    }
}

fn run_inner(argv: &[String]) -> Result<(), Error> {
    let args = parse_args(argv)?;
    let dialer = Dialer::new(DIAL_TIMEOUT);
    let mut stream = dialer.dial(&args.target)?;
    // ⛔ Standard descriptors belong to the SSH client. A failure to take
    // them is a server-side error on this end, reported like any other.
    let mut stdio = Stdio::new().map_err(|e| Error::server("proxy stdio", e))?;
    if let Some(secs) = args.hold_secs {
        podbox_ssh::hold_until_banner(&mut stdio, &mut *stream, secs)?;
    }
    podbox_ssh::pump(&mut stdio, &mut *stream)
}

/// Parse the proxy command line into a target plus the banner hold.
/// Untrusted input maps to typed usage errors; nothing here panics on
/// argument shape. The hold belongs to `unix` alone: a late-opening
/// server behind any other target is a different entry's problem, and
/// a flag that parses where it cannot act is dead config.
fn parse_args(argv: &[String]) -> Result<ProxyArgs, Error> {
    let usage = || Error::new(Kind::Usage, "proxy args", USAGE);
    match argv.first().map(String::as_str) {
        Some("tcp") => {
            // ⛔ Exactly a host and a port. A third word is not ignored: it
            // is a caller mistake, and ignoring it would dial somewhere the
            // caller did not review.
            let [_, host, port] = argv else {
                return Err(usage());
            };
            if host.trim().is_empty() {
                return Err(Error::new(Kind::Usage, "proxy args", "tcp needs a host"));
            }
            Ok(ProxyArgs {
                target: Target::Tcp {
                    host: host.to_string(),
                    port: parse_port(port)?,
                },
                hold_secs: None,
            })
        }
        Some("unix") => {
            if argv.len() < 2 || argv[1].trim().is_empty() {
                return Err(Error::new(Kind::Usage, "proxy args", "unix needs a path"));
            }
            let mut hold_secs = None;
            match &argv[2..] {
                [] => {}
                [flag, secs] if flag == "--hold-for-banner" => {
                    hold_secs = Some(parse_hold_secs(secs)?);
                }
                _ => {
                    return Err(usage());
                }
            }
            Ok(ProxyArgs {
                target: Target::Unix {
                    path: PathBuf::from(&argv[1]),
                },
                hold_secs,
            })
        }
        Some("exec") => {
            if argv.len() < 2 {
                return Err(usage());
            }
            Ok(ProxyArgs {
                target: Target::Exec {
                    program: argv[1].clone(),
                    args: argv[2..].to_vec(),
                },
                hold_secs: None,
            })
        }
        _ => Err(usage()),
    }
}

/// A hold deadline in seconds: a positive count, like every other
/// duration flag in this tree. Zero would return before the server
/// could ever answer, which is a refusal wearing a flag.
fn parse_hold_secs(secs: &str) -> Result<u64, Error> {
    match secs.parse::<u64>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(Error::new(
            Kind::Usage,
            "proxy args",
            "--hold-for-banner SECS names a positive count of seconds",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn tcp_needs_exactly_a_host_and_a_port() {
        let parsed = parse_args(&args(&["tcp", "example.com", "22"])).unwrap();
        assert_eq!(
            parsed.target,
            Target::Tcp {
                host: "example.com".to_string(),
                port: 22,
            }
        );
        assert_eq!(parsed.hold_secs, None);
        for argv in [
            args(&[]),
            args(&["tcp"]),
            args(&["tcp", "example.com"]),
            args(&["tcp", "example.com", "22", "extra"]),
            args(&["tcp", ""]),
            args(&["tcp", "", "22"]),
            args(&["tcp", "example.com", "http"]),
            args(&["tcp", "example.com", "0"]),
        ] {
            let e = parse_args(&argv).unwrap_err();
            assert_eq!(e.kind(), Kind::Usage, "{argv:?}: {e}");
        }
    }

    #[test]
    fn tcp_accepts_a_dotless_host() {
        // ⛔ A name with no dot is a name. The proxy must not demand a dot.
        let parsed = parse_args(&args(&["tcp", "localhost", "22"])).unwrap();
        assert_eq!(
            parsed.target,
            Target::Tcp {
                host: "localhost".to_string(),
                port: 22,
            }
        );
    }

    #[test]
    fn unix_needs_exactly_a_path() {
        let parsed = parse_args(&args(&["unix", "/tmp/x.sock"])).unwrap();
        assert_eq!(
            parsed.target,
            Target::Unix {
                path: PathBuf::from("/tmp/x.sock"),
            }
        );
        assert_eq!(parsed.hold_secs, None);
        for argv in [
            args(&["unix"]),
            args(&["unix", ""]),
            args(&["unix", "/tmp/x.sock", "extra"]),
        ] {
            let e = parse_args(&argv).unwrap_err();
            assert_eq!(e.kind(), Kind::Usage, "{argv:?}: {e}");
        }
    }

    #[test]
    fn unix_hold_names_a_positive_deadline() {
        let parsed =
            parse_args(&args(&["unix", "/tmp/x.sock", "--hold-for-banner", "60"])).unwrap();
        assert_eq!(
            parsed.target,
            Target::Unix {
                path: PathBuf::from("/tmp/x.sock"),
            }
        );
        assert_eq!(parsed.hold_secs, Some(60));
        for argv in [
            args(&["unix", "/tmp/x.sock", "--hold-for-banner"]),
            args(&["unix", "/tmp/x.sock", "--hold-for-banner", "0"]),
            args(&["unix", "/tmp/x.sock", "--hold-for-banner", "-3"]),
            args(&["unix", "/tmp/x.sock", "--hold-for-banner", "soon"]),
            args(&["unix", "/tmp/x.sock", "--hold-for-banner", "60", "extra"]),
            args(&["unix", "/tmp/x.sock", "--hold"]),
            args(&["tcp", "example.com", "22", "--hold-for-banner", "60"]),
        ] {
            let e = parse_args(&argv).unwrap_err();
            assert_eq!(e.kind(), Kind::Usage, "{argv:?}: {e}");
        }
    }

    #[test]
    fn exec_carries_program_and_args() {
        let parsed = parse_args(&args(&["exec", "ssh", "-W", "h:22"])).unwrap();
        assert_eq!(
            parsed.target,
            Target::Exec {
                program: "ssh".to_string(),
                args: vec!["-W".to_string(), "h:22".to_string()],
            }
        );
        assert_eq!(parsed.hold_secs, None);
        let e = parse_args(&args(&["exec"])).unwrap_err();
        assert_eq!(e.kind(), Kind::Usage);
        let e = parse_args(&args(&["telnet"])).unwrap_err();
        assert_eq!(e.kind(), Kind::Usage);
        assert!(e.to_string().contains("usage"), "{e}");
    }

    #[test]
    fn failure_exit_code_is_the_runtime_code() {
        assert_eq!(run(&[]), podbox_ssh::EXIT_RUNTIME_ERROR);
    }
}
