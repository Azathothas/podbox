use super::*;

pub(super) fn command_of(c: &Container) -> String {
    // TODO/supervise.md T-1800: the width counts characters, and the cut
    // falls between two of them. A byte index cut a multibyte character in
    // two, and the release build aborted on the panic.
    let joined = c.argv.join(" ");
    if joined.chars().count() > 20 {
        format!("{}...", joined.chars().take(17).collect::<String>())
    } else {
        joined
    }
}

pub(super) fn ps_field_names() -> Vec<&'static str> {
    vec![
        "ID",
        "Names",
        "Image",
        "Command",
        "CreatedAt",
        "Status",
        "State",
        "Ports",
        "Pid",
    ]
}

pub(super) fn ps_fields(c: &Container, no_trunc: bool) -> Vec<(&'static str, String)> {
    vec![
        ("ID", if no_trunc { c.id.clone() } else { c.short_id() }),
        ("Names", c.name.clone()),
        ("Image", c.image.clone()),
        ("Command", c.argv.join(" ")),
        ("CreatedAt", c.created_at.clone()),
        ("Status", c.status()),
        ("State", c.state.word().to_string()),
        // ⛔ Always empty, and the usage says why rather than leaving a reader
        // to wonder whether podvm forgot.
        ("Ports", String::new()),
        (
            "Pid",
            c.pid.map(|p| p.to_string()).unwrap_or_else(|| "0".into()),
        ),
    ]
}

/// `podvm logs [-f|--follow] [--tail N] <container>`
pub(super) const LOGS_USAGE: &str = "usage: podvm logs [-f|--follow] [--tail N] <container>";

/// What `logs` was asked for. One parser, so the flag cannot be accepted in
/// one position and lost in another (TODO/supervise.md T-1318).
#[derive(Debug)]
pub(super) struct LogsArgs {
    pub(super) follow: bool,
    pub(super) tail: Option<u64>,
    pub(super) want: String,
}

pub(super) fn parse_logs(args: &[String]) -> std::result::Result<LogsArgs, i32> {
    let mut follow = false;
    let mut tail: Option<u64> = None;
    let mut want: Option<String> = None;
    let mut expecting: Option<&'static str> = None;
    for a in args {
        if expecting.take().is_some() {
            match parse_tail(a) {
                Ok(n) => tail = Some(n),
                Err(why) => {
                    eprintln!("podvm logs: {why}");
                    return Err(EXIT_FLAG_ERROR);
                }
            }
            continue;
        }
        match a.as_str() {
            "-f" | "--follow" => follow = true,
            "--tail" => expecting = Some("--tail"),
            "-h" | "--help" => {
                println!("{LOGS_USAGE}");
                return Err(0);
            }
            other if other.starts_with("--tail=") => match parse_tail(&other[7..]) {
                Ok(n) => tail = Some(n),
                Err(why) => {
                    eprintln!("podvm logs: {why}");
                    return Err(EXIT_FLAG_ERROR);
                }
            },
            other if other.starts_with('-') => {
                crate::parity::admit("logs", other, LOGS_USAGE)?;
                return Err(crate::parity::no_arm("logs", other));
            }
            // ⚠ The first positional wins and the rest are ignored, as before:
            // `logs` names one container.
            other if want.is_none() => want = Some(other.to_string()),
            _ => {}
        }
    }
    if let Some(flag) = expecting {
        eprintln!("podvm logs: {flag} needs a value");
        return Err(EXIT_FLAG_ERROR);
    }
    let Some(want) = want else {
        eprintln!("{LOGS_USAGE}");
        return Err(EXIT_CLI_ERROR);
    };
    Ok(LogsArgs { follow, tail, want })
}

/// Parse a `--tail` value: a non-negative integer, nothing else. Zero
/// prints no lines and still exits 0; anything that is not a number is a
/// flag error naming the value rather than a silent whole log.
fn parse_tail(raw: &str) -> std::result::Result<u64, String> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "--tail takes a non-negative line count, not {raw:?}"
        ));
    }
    raw.parse::<u64>()
        .map_err(|_| format!("--tail takes a non-negative line count, not {raw:?}"))
}

/// `podvm logs [-f|--follow] <container>`
pub fn logs(args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded = match crate::parity::expand("logs", args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member("logs", &member, LOGS_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all("logs", args, LOGS_USAGE) {
        return c;
    }
    let o = match parse_logs(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let s = match store() {
        Ok(s) => s,
        Err(c) => return c,
    };
    if o.follow {
        // ⚠ The payload's own bytes, to stdout, unaltered. `logs` is the one
        // verb whose stdout is not podvm's, following or not.
        let mut out = std::io::stdout().lock();
        // ⭐ TODO/cli.md T-1337: from the tail, not from the start.
        // A reconnect that replays the whole file is the defect
        // `--tail` exists to remove.
        let r = match o.tail {
            Some(n) => podvm_supervise::follow_tail(&s, &o.want, &mut out, n),
            None => podvm_supervise::follow(&s, &o.want, &mut out),
        };
        return match r {
            Ok(()) => 0,
            Err(e) => fail("logs", e),
        };
    }
    match podvm_supervise::logs(&s, &o.want) {
        Ok(bytes) => {
            // ⚠ The payload's own bytes, to stdout, unaltered. `logs` is the one
            // verb whose stdout is not podvm's.
            // ⭐ TODO/cli.md T-1337: the tail is a slice of the same read,
            // so no `--tail` changes the default by a byte.
            let bytes = match o.tail {
                Some(n) => {
                    let start = podvm_supervise::tail_offset(&bytes, n) as usize;
                    &bytes[start..]
                }
                None => &bytes[..],
            };
            // TODO/cli.md T-1816: a failed write is 125, not 0.
            let mut out = std::io::stdout().lock();
            match out.write_all(bytes).and_then(|()| out.flush()) {
                Ok(()) => 0,
                Err(e) => crate::names::stdout::failed(&e),
            }
        }
        Err(e) => fail("logs", e),
    }
}

/// `podvm stop [-t N] <container>...`
pub fn stop(args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: `stop -t5` reads as `-t 5` before admission.
    let expanded = match crate::parity::expand("stop", args) {
        Ok(a) => a,
        Err(member) => {
            return crate::parity::refuse_member(
                "stop",
                &member,
                "usage: podvm stop [-t seconds] <container> [container...]",
            )
        }
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(
        "stop",
        args,
        "usage: podvm stop [-t seconds] <container> [container...]",
    ) {
        return c;
    }
    let mut grace = STOP_GRACE_MS;
    let mut names = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("usage: podvm stop [-t seconds] <container> [container...]");
                return 0;
            }
            "-t" | "--time" | "--timeout" => match it.next().and_then(|v| grace_ms(v)) {
                Some(v) => grace = v,
                None => {
                    eprintln!("podvm stop: -t takes a number of seconds");
                    return EXIT_FLAG_ERROR;
                }
            },
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(
                    "stop",
                    other,
                    "usage: podvm stop [-t seconds] <container> [container...]",
                ) {
                    return c;
                }
                return crate::parity::no_arm("stop", other);
            }
            other => names.push(other.to_string()),
        }
    }
    if names.is_empty() {
        eprintln!("usage: podvm stop [-t seconds] <container> [container...]");
        return EXIT_CLI_ERROR;
    }
    let s = match store() {
        Ok(s) => s,
        Err(c) => return c,
    };
    let mut code = 0;
    for want in &names {
        // TODO/image.md T-1767: the caller prints the name, so `restart`,
        // which also starts, prints it once.
        match stop_one(&s, "stop", want, grace) {
            Ok(name) => println!("{name}"),
            Err(c) => code = c,
        }
    }
    code
}

/// The grace of `stop -t SECONDS` in milliseconds. `None` for a value that
/// is not a whole number of seconds or that overflows (T-1691). Zero is
/// valid: it sends SIGKILL at once, as `docker stop -t 0` does (call C12).
pub(crate) fn grace_ms(v: &str) -> Option<u64> {
    v.parse::<u64>().ok()?.checked_mul(1000)
}

/// Stop one container: the per-name half of `stop`, shared with
/// `restart` (TODO/cli.md T-1331). Returns the name on success.
fn stop_one(s: &podvm_image::Store, verb: &str, want: &str, grace: u64) -> Result<String, i32> {
    match podvm_supervise::stop(s, want, grace) {
        Ok((c, killed)) => {
            if killed && grace == 0 {
                // T-1691 (call C12): `-t 0` asked for the SIGKILL, as docker.
                eprintln!("podvm {verb}: {} was killed at once (-t 0)", c.name);
            } else if killed {
                // ⛔ Said, never silent. A caller that asked for a graceful
                // stop and got a SIGKILL has to be able to tell.
                eprintln!(
                    "podvm {verb}: {} did not exit within {} s of SIGTERM and was killed",
                    c.name,
                    grace / 1000
                );
            }
            Ok(c.name)
        }
        Err(e) => Err(fail(verb, e)),
    }
}

/// `podvm restart <container>...`
pub(super) const RESTART_USAGE: &str = "usage: podvm restart <container> [container...]";

/// `podvm restart`: stop-then-start in one verb (TODO/cli.md T-1331).
///
/// A composite, not a policy: each container stops with the default
/// grace and starts again, and a failure names which half failed, so a
/// caller can tell a container that never stopped from one that
/// stopped and never came back.
pub fn restart(args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission
    // (this verb takes none today; the call keeps the rule complete).
    let expanded = match crate::parity::expand("restart", args) {
        Ok(a) => a,
        Err(member) => return crate::parity::refuse_member("restart", &member, RESTART_USAGE),
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all("restart", args, RESTART_USAGE) {
        return c;
    }
    let mut names = Vec::new();
    for want in args {
        match want.as_str() {
            "-h" | "--help" => {
                println!("{RESTART_USAGE}");
                return 0;
            }
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit("restart", other, RESTART_USAGE) {
                    return c;
                }
                return crate::parity::no_arm("restart", other);
            }
            other => names.push(other.to_string()),
        }
    }
    if names.is_empty() {
        eprintln!("{RESTART_USAGE}");
        return EXIT_CLI_ERROR;
    }
    let s = match store() {
        Ok(s) => s,
        Err(c) => return c,
    };
    let mut code = 0;
    for want in &names {
        match stop_one(&s, "restart", want, STOP_GRACE_MS) {
            Ok(_) => match start_one(&s, "restart", want) {
                Ok(_) => {}
                Err(c) => {
                    eprintln!("podvm restart: {want}: stopped, then the start half failed");
                    code = c;
                }
            },
            Err(c) => {
                // TODO/supervise.md T-1805: a missing name is said once.
                if c != EXIT_CLI_ERROR {
                    eprintln!("podvm restart: {want}: the stop half failed; not started");
                }
                code = c;
            }
        }
    }
    code
}

/// `podvm kill [-s SIG] <container>...`
pub fn kill(args: &[String]) -> i32 {
    // ⭐ TODO/cli.md T-1330: bundled shorts expand before admission.
    let expanded = match crate::parity::expand("kill", args) {
        Ok(a) => a,
        Err(member) => {
            return crate::parity::refuse_member(
                "kill",
                &member,
                "usage: podvm kill [-s SIGNAL] <container> [container...]",
            )
        }
    };
    let args: &[String] = &expanded;
    if let Some(c) = crate::parity::admit_all(
        "kill",
        args,
        "usage: podvm kill [-s SIGNAL] <container> [container...]",
    ) {
        return c;
    }
    let mut sig = 9;
    let mut names = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("usage: podvm kill [-s SIGNAL] <container> [container...]");
                return 0;
            }
            "-s" | "--signal" => match it.next() {
                Some(v) => match signal_number(v) {
                    Some(n) => sig = n,
                    None => {
                        eprintln!("podvm kill: {v:?} is not a signal podvm knows");
                        return EXIT_FLAG_ERROR;
                    }
                },
                None => {
                    eprintln!("podvm kill: -s needs a signal");
                    return EXIT_FLAG_ERROR;
                }
            },
            other if other.starts_with('-') => {
                if let Err(c) = crate::parity::admit(
                    "kill",
                    other,
                    "usage: podvm kill [-s SIGNAL] <container> [container...]",
                ) {
                    return c;
                }
                return crate::parity::no_arm("kill", other);
            }
            other => names.push(other.to_string()),
        }
    }
    if names.is_empty() {
        eprintln!("usage: podvm kill [-s SIGNAL] <container> [container...]");
        return EXIT_CLI_ERROR;
    }
    let s = match store() {
        Ok(s) => s,
        Err(c) => return c,
    };
    let mut code = 0;
    for want in &names {
        match podvm_supervise::kill(&s, want, sig) {
            Ok(c) => println!("{}", c.name),
            Err(e) => code = fail("kill", e),
        }
    }
    code
}
