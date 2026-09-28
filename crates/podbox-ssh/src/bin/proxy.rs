//! The stdio-to-target shuttle: `proxy (tcp HOST PORT | unix PATH | exec
//! PROG [ARGS...])`.
//!
//! ⛔ This binary is a valid SSH `ProxyCommand`. SSH writes and reads this
//! process's standard descriptors, and the pump moves those bytes to the
//! target. It reports one line on failure and exits with the runtime code,
//! so the SSH client prints its own diagnosis beside it.
//!
//! ⛔ Every wait here is bounded. The dial carries a connect timeout per
//! address, and the pump returns when either side closes. An argument that
//! does not parse is a typed error, never a panic.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::time::Duration;

use podbox_ssh::{parse_port, Dialer, Error, Kind, Stdio, Target, EXIT_RUNTIME_ERROR};

/// How long one dial waits per resolved address.
const DIAL_TIMEOUT: Duration = Duration::from_secs(10);

const USAGE: &str = "usage: proxy (tcp HOST PORT | unix PATH | exec PROG [ARGS...])";

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
    let target = parse_args(argv)?;
    let dialer = Dialer::new(DIAL_TIMEOUT);
    let mut stream = dialer.dial(&target)?;
    // ⛔ Standard descriptors belong to the SSH client. A failure to take
    // them is a server-side error on this end, reported like any other.
    let mut stdio = Stdio::new().map_err(|e| Error::server("proxy stdio", e))?;
    podbox_ssh::pump(&mut stdio, &mut *stream)
}

/// Parse the proxy command line into a target. Untrusted input maps to
/// typed usage errors; nothing here panics on argument shape.
fn parse_args(argv: &[String]) -> Result<Target, Error> {
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
            Ok(Target::Tcp {
                host: host.to_string(),
                port: parse_port(port)?,
            })
        }
        Some("unix") => {
            let [_, path] = argv else {
                return Err(usage());
            };
            if path.trim().is_empty() {
                return Err(Error::new(Kind::Usage, "proxy args", "unix needs a path"));
            }
            Ok(Target::Unix {
                path: PathBuf::from(path),
            })
        }
        Some("exec") => {
            if argv.len() < 2 {
                return Err(usage());
            }
            Ok(Target::Exec {
                program: argv[1].clone(),
                args: argv[2..].to_vec(),
            })
        }
        _ => Err(usage()),
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
        assert_eq!(
            parse_args(&args(&["tcp", "example.com", "22"])).unwrap(),
            Target::Tcp {
                host: "example.com".to_string(),
                port: 22,
            }
        );
        for argv in [
            args(&[]),
            args(&["tcp"]),
            args(&["tcp", "example.com"]),
            args(&["tcp", "example.com", "22", "extra"]),
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
        assert_eq!(
            parse_args(&args(&["tcp", "localhost", "22"])).unwrap(),
            Target::Tcp {
                host: "localhost".to_string(),
                port: 22,
            }
        );
    }

    #[test]
    fn unix_needs_exactly_a_path() {
        assert_eq!(
            parse_args(&args(&["unix", "/tmp/x.sock"])).unwrap(),
            Target::Unix {
                path: PathBuf::from("/tmp/x.sock"),
            }
        );
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
    fn exec_carries_program_and_args() {
        assert_eq!(
            parse_args(&args(&["exec", "ssh", "-W", "h:22"])).unwrap(),
            Target::Exec {
                program: "ssh".to_string(),
                args: vec!["-W".to_string(), "h:22".to_string()],
            }
        );
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
