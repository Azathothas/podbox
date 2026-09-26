//! The `podbox windows` flag surface. `TODO/milestones.md` T-1112.
//!
//! ⚠ A `windows` subcommand is not a docker verb, so there is no parity row
//! to take the surface from: the table's two rows for this verb say what it
//! is and what it costs, and this file is the whole of what it accepts.

use std::path::PathBuf;

use podbox_image::error::EXIT_FLAG_ERROR;

pub(crate) const USAGE: &str = "\
usage: podbox windows <doctor|setup|fetch|run> [options] [--] [command]
  doctor                       report the machine profile and the accelerator a
                               guest run would use, and refuse where none holds
  fetch    --url <url>         download a base image into the cache, bounded by
                               --max-bytes and RLIMIT_FSIZE and verified
                               against --sha256 where one is pinned
  setup    --image <disk>      install the guest agent into a Windows disk
                               image, once, writing <disk>.podbox.qcow2 beside
                               it. Pass THAT to `run`. This is the only step
                               that uses the guest console.
  run      --image <disk>      boot a disposable guest, run one cmd.exe line,
                               and print its stdout, stderr and exit code.
options:
  --image <path>               the Windows disk image (vhdx, qcow2 or raw)
  --url <url>                  the base image to download
  --sha256 <hex>               the pinned digest for --url; without it the
                               download is bounded but unverified
  --max-bytes <n>              the download ceiling, over RLIMIT_FSIZE never
  --podbox-mem <size>          guest memory, for example 4G
  --podbox-cpus <n>            guest cpus
  --podbox-timeout <seconds>   how long a run may take before it is stopped
  --podbox-qemu-arg <arg>      one extra emulator argument, repeatable
  --guest <dos|windows>        which guest to boot (default windows).
                               dos boots FreeDOS from its own base cache and
                               needs no --image; windows boots the disk image
";

/// One parsed `podbox windows` invocation.
/// Which guest a run boots. DOS is FreeDOS from the base cache with one
/// typed line; Windows is a disk image with the autostarted agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Guest {
    Dos,
    Windows,
}

/// One parsed `podbox windows` invocation.
pub(crate) struct Args {
    pub sub: String,
    pub guest: Option<Guest>,
    pub image: Option<PathBuf>,
    pub mem: Option<u64>,
    pub cpus: Option<u32>,
    pub timeout: Option<u64>,
    pub emu_args: Vec<String>,
    pub command: String,
    pub url: Option<String>,
    pub sha256: Option<String>,
    pub max_bytes: Option<u64>,
}

/// One value after a flag, or the flag error that says which one is missing.
fn value<'a>(it: &mut std::slice::Iter<'a, String>, flag: &str) -> Result<&'a String, i32> {
    it.next().ok_or_else(|| {
        eprintln!("podbox windows: {flag} needs a value");
        EXIT_FLAG_ERROR
    })
}

/// Parse the verb. `Ok(None)` is `-h/--help`: printed, and the exit is 0.
pub(crate) fn parse(args: &[String]) -> Result<Option<Args>, i32> {
    let mut it = args.iter();
    let sub = match it.next() {
        Some(s) => s.clone(),
        None => {
            eprintln!("podbox windows: no subcommand\n{USAGE}");
            return Err(podbox_image::error::EXIT_FLAG_ERROR);
        }
    };
    if sub == "-h" || sub == "--help" {
        print!("{USAGE}");
        return Ok(None);
    }
    let mut a = Args {
        guest: None,
        sub,
        image: None,
        mem: None,
        cpus: None,
        timeout: None,
        emu_args: Vec::new(),
        command: String::new(),
        url: None,
        sha256: None,
        max_bytes: None,
    };
    let mut words: Vec<String> = Vec::new();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--" => {
                words.extend(it.by_ref().cloned());
                break;
            }
            "--image" => a.image = Some(PathBuf::from(value(&mut it, "--image")?)),
            "--url" => a.url = Some(value(&mut it, "--url")?.clone()),
            "--sha256" => {
                let v = value(&mut it, "--sha256")?;
                if v.len() != 64 || !v.chars().all(|c| c.is_ascii_hexdigit()) {
                    eprintln!("podbox windows: --sha256 {v} is not a sha256 digest");
                    return Err(EXIT_FLAG_ERROR);
                }
                a.sha256 = Some(v.to_ascii_lowercase());
            }
            "--max-bytes" => {
                let v = value(&mut it, "--max-bytes")?;
                a.max_bytes = Some(v.parse().map_err(|_| {
                    eprintln!("podbox windows: --max-bytes {v} is not a count of bytes");
                    EXIT_FLAG_ERROR
                })?);
                if a.max_bytes == Some(0) {
                    eprintln!("podbox windows: --max-bytes 0 downloads nothing");
                    return Err(EXIT_FLAG_ERROR);
                }
            }
            "--podbox-mem" => {
                let v = value(&mut it, "--podbox-mem")?;
                a.mem = Some(crate::tier::parse_mem(v).ok_or_else(|| {
                    eprintln!("podbox windows: --podbox-mem {v} is not a size");
                    EXIT_FLAG_ERROR
                })?);
            }
            "--podbox-cpus" => {
                let v = value(&mut it, "--podbox-cpus")?;
                let n: u32 = v.parse().map_err(|_| {
                    eprintln!("podbox windows: --podbox-cpus {v} is not a count");
                    EXIT_FLAG_ERROR
                })?;
                if n == 0 {
                    eprintln!("podbox windows: --podbox-cpus 0 is not a machine");
                    return Err(EXIT_FLAG_ERROR);
                }
                a.cpus = Some(n);
            }
            "--podbox-timeout" => {
                let v = value(&mut it, "--podbox-timeout")?;
                a.timeout = Some(v.parse().map_err(|_| {
                    eprintln!("podbox windows: --podbox-timeout {v} is not a count of seconds");
                    EXIT_FLAG_ERROR
                })?);
            }
            "--podbox-qemu-arg" => a
                .emu_args
                .push(value(&mut it, "--podbox-qemu-arg")?.clone()),
            "--guest" => {
                let v = value(&mut it, "--guest")?;
                a.guest = Some(match v.as_str() {
                    "dos" => Guest::Dos,
                    "windows" => Guest::Windows,
                    _ => {
                        eprintln!("podbox windows: --guest takes dos or windows, not {v}");
                        return Err(EXIT_FLAG_ERROR);
                    }
                });
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(None);
            }
            other if other.starts_with('-') => {
                eprintln!("podbox windows: {other} is not a flag this verb has\n{USAGE}");
                return Err(EXIT_FLAG_ERROR);
            }
            other => words.push(other.to_string()),
        }
    }
    a.command = words.join(" ");
    Ok(Some(a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(xs: &[&str]) -> Result<Option<Args>, i32> {
        parse(&xs.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn help_prints_and_exits_zero() {
        assert!(matches!(p(&["--help"]), Ok(None)));
        assert!(matches!(p(&["run", "-h"]), Ok(None)));
    }

    #[test]
    fn no_subcommand_and_an_unknown_flag_are_flag_errors() {
        assert!(p(&[]).is_err());
        assert!(p(&["run", "--nope"]).is_err());
        assert!(
            p(&["wat"]).is_ok(),
            "an unknown subcommand parses; dispatch refuses it"
        );
    }

    #[test]
    fn the_command_is_everything_after_the_flags() {
        let a = p(&["run", "--image", "/x.vhdx", "--", "cmd", "/c", "ver"])
            .unwrap()
            .unwrap();
        assert_eq!(a.command, "cmd /c ver");
        assert_eq!(a.image, Some(PathBuf::from("/x.vhdx")));
    }

    #[test]
    fn a_sha256_that_is_not_a_digest_is_refused() {
        assert!(p(&["fetch", "--url", "u", "--sha256", "abc"]).is_err());
        assert!(p(&["fetch", "--url", "u", "--sha256", &"z".repeat(64)]).is_err());
        let a = p(&["fetch", "--url", "u", "--sha256", &"A".repeat(64)])
            .unwrap()
            .unwrap();
        assert_eq!(
            a.sha256,
            Some("a".repeat(64)),
            "pins normalise to lowercase"
        );
    }

    #[test]
    fn a_zero_memory_or_cpu_or_ceiling_is_refused() {
        assert!(p(&["run", "--podbox-mem", "0"]).is_err());
        assert!(p(&["run", "--podbox-cpus", "0"]).is_err());
        assert!(p(&["fetch", "--url", "u", "--max-bytes", "0"]).is_err());
        assert!(p(&["run", "--podbox-mem", "1G"]).unwrap().unwrap().mem == Some(1 << 30));
    }

    #[test]
    fn the_guest_flag_names_dos_or_windows_and_nothing_else() {
        assert_eq!(
            p(&["run", "--guest", "dos"]).unwrap().unwrap().guest,
            Some(Guest::Dos)
        );
        assert_eq!(
            p(&["run", "--guest", "windows"]).unwrap().unwrap().guest,
            Some(Guest::Windows)
        );
        assert_eq!(p(&["run"]).unwrap().unwrap().guest, None);
        assert!(p(&["run", "--guest", "plan9"]).is_err());
        assert!(p(&["run", "--guest"]).is_err());
    }

    #[test]
    fn a_flag_without_its_value_is_refused_rather_than_read_off_the_end() {
        assert!(p(&["run", "--image"]).is_err());
        assert!(p(&["fetch", "--url"]).is_err());
        assert!(p(&["fetch", "--max-bytes"]).is_err());
    }
}
