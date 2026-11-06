use super::*;

fn extract_json_int(text: &str, key: &str) -> Option<i64> {
    let needle = format!("\"{key}\"");
    let mut search: &str = text;
    loop {
        let at = search.find(&needle)?;
        let mut rest = search[at + needle.len()..].trim_start();
        if !rest.starts_with(':') {
            search = &search[at + needle.len()..];
            continue;
        }
        rest = rest[1..].trim_start();
        let mut digits = 0;
        let mut negative = false;
        if rest.starts_with('-') {
            negative = true;
            rest = &rest[1..];
        }
        let mut value: i64 = 0;
        for byte in rest.bytes() {
            if !byte.is_ascii_digit() {
                break;
            }
            value = value
                .saturating_mul(10)
                .saturating_add((byte - b'0') as i64);
            digits += 1;
        }
        if digits == 0 {
            search = &search[at + needle.len()..];
            continue;
        }
        return Some(if negative { -value } else { value });
    }
}

pub(super) fn do_diagnostics(root: &Path, with_powershell: bool) -> ExitCode {
    use std::time::{SystemTime, UNIX_EPOCH};
    println!("== conditions");
    let date = run_bounded(
        "date",
        &["-u".to_string(), "+%Y-%m-%dT%H:%M:%SZ".to_string()],
        root,
        &[],
        30,
    )
    .map(|run| run.stdout.trim().to_string())
    .unwrap_or_else(|_| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "-".to_string())
    });
    println!("{date}");
    let host = run_bounded("uname", &["-s".to_string()], root, &[], 30)
        .map(|run| run.stdout.trim().to_string())
        .unwrap_or_else(|_| "-".to_string());
    let python = run_bounded(
        "python3",
        &[
            "-c".to_string(),
            "import sys; print(sys.version.split()[0])".to_string(),
        ],
        root,
        &[],
        30,
    )
    .map(|run| run.stdout.trim().to_string())
    .unwrap_or_else(|_| "-".to_string());
    println!("host={host} python={python}");
    println!("Inputs: fixed stub-v1 checks; late message after 20 control lines.");
    let shell = which("sh");
    let powershell = which("pwsh");
    if shell.is_none() || (with_powershell && powershell.is_none()) {
        println!("cannot run: required interpreter is absent");
        return ExitCode::from(2);
    }
    let work = env::temp_dir().join(format!("podvm-gate-output-{}", std::process::id()));
    if fs::create_dir_all(&work).is_err() {
        println!("cannot run: required interpreter is absent");
        return ExitCode::from(2);
    }
    let cleanup = |code: ExitCode| -> ExitCode {
        let _ = fs::remove_dir_all(&work);
        code
    };
    for name in DIAGNOSTIC_CHECKS {
        if fs::write(work.join(format!("{name}.sh")), "exit 0\n").is_err()
            || fs::write(work.join(format!("{name}.ps1")), "exit 0\n").is_err()
        {
            println!("cannot run: required interpreter is absent");
            return cleanup(ExitCode::from(2));
        }
    }
    let profiles: &[(&str, bool)] = &[("sh", true), ("ps1", with_powershell)];
    for (kind, enabled) in profiles {
        if !enabled {
            if *kind == "ps1" {
                println!("PowerShell profile: not requested; Windows proof uses --powershell");
            }
            continue;
        }
        let interpreter = if *kind == "sh" {
            shell.clone().unwrap()
        } else {
            powershell.clone().unwrap()
        };
        let source_path = root.join(format!("scripts/common/check-gate.{kind}"));
        let source = match fs::read_to_string(&source_path) {
            Ok(text) => text,
            Err(_) => {
                println!("cannot run: required interpreter is absent");
                return cleanup(ExitCode::from(2));
            }
        };
        let runner = work.join(format!("check-gate.{kind}"));
        // The runner is a copy the fixture may mutate; the tree is never
        // written. Newlines stay LF exactly as the script wrote them.
        if fs::write(&runner, source.replace("\r\n", "\n")).is_err() {
            println!("cannot run: required interpreter is absent");
            return cleanup(ExitCode::from(2));
        }
        let stub = work.join(format!("check-docs.{kind}"));
        let noise = (0..20)
            .map(|_| {
                if *kind == "sh" {
                    "echo control".to_string()
                } else {
                    "Write-Output 'control'".to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let fail_body = if *kind == "sh" {
            format!("{noise}\necho {MARKER}\nexit 1\n")
        } else {
            format!("{noise}\nWrite-Output '{MARKER}'\nexit 1\n")
        };
        if fs::write(&stub, &fail_body).is_err() {
            println!("cannot run: required interpreter is absent");
            return cleanup(ExitCode::from(2));
        }
        let (flags, json_flag, strict) = if *kind == "sh" {
            ("--fast", "--json", "--strict")
        } else {
            ("-Fast", "-Json", "-Strict")
        };
        let prefix: Vec<String> = if *kind == "sh" {
            vec![interpreter.to_string_lossy().into_owned()]
        } else {
            vec![
                interpreter.to_string_lossy().into_owned(),
                "-NoProfile".to_string(),
                "-File".to_string(),
            ]
        };
        let run_with = |extra: &[&str]| -> Result<Run, String> {
            let mut args: Vec<String> = prefix.clone();
            args.push(runner.to_string_lossy().into_owned());
            for flag in extra {
                args.push(flag.to_string());
            }
            let (prog, rest) = args.split_first().unwrap();
            run_bounded(prog, rest, &work, &[], RUN_TIMEOUT_SECS)
        };
        // The late failure is printed, and the JSON carries it as a count.
        match run_with(&[flags]) {
            Ok(run) if run.rc == 1 && run.stdout.contains(MARKER) => {}
            _ => {
                eprintln!("SMOKE-FAIL diagnostics: the late failure did not print on {kind}");
                return cleanup(ExitCode::from(1));
            }
        }
        match run_with(&[flags, json_flag]) {
            Ok(run) => {
                if run.rc != 1 || extract_json_int(&run.stdout, "failed") != Some(1) {
                    eprintln!("SMOKE-FAIL diagnostics: the JSON verdict is wrong on {kind}");
                    return cleanup(ExitCode::from(1));
                }
                if run.stdout.contains(MARKER) {
                    eprintln!("SMOKE-FAIL diagnostics: the marker leaked into JSON on {kind}");
                    return cleanup(ExitCode::from(1));
                }
            }
            Err(e) => {
                eprintln!("SMOKE-FAIL diagnostics: {e}");
                return cleanup(ExitCode::from(1));
            }
        }
        // The lost-output mutation: truncate the failure log and the late
        // message must vanish while the run still fails.
        let mutated = if *kind == "sh" {
            let current = fs::read_to_string(&runner).unwrap_or_default();
            current.replacen(
                "sed 's/^/          /' \"$OUT/log\" ;;",
                "sed 's/^/          /' \"$OUT/log\" | head -12 ;;",
                1,
            )
        } else {
            let current = fs::read_to_string(&runner).unwrap_or_default();
            current.replacen(
                "Get-Content -LiteralPath $logFile -ErrorAction",
                "Get-Content -LiteralPath $logFile -TotalCount 12 -ErrorAction",
                1,
            )
        };
        let current = fs::read_to_string(&runner).unwrap_or_default();
        if mutated == current {
            eprintln!("SMOKE-FAIL diagnostics: mutation did not reach its runner on {kind}");
            return cleanup(ExitCode::from(1));
        }
        if fs::write(&runner, &mutated).is_err() {
            eprintln!("SMOKE-FAIL diagnostics: cannot write the runner on {kind}");
            return cleanup(ExitCode::from(1));
        }
        match run_with(&[flags]) {
            Ok(run) if run.rc == 1 && !run.stdout.contains(MARKER) => {}
            _ => {
                eprintln!("SMOKE-FAIL diagnostics: the truncated run kept the message on {kind}");
                return cleanup(ExitCode::from(1));
            }
        }
        if fs::write(&runner, &current).is_err() {
            eprintln!("SMOKE-FAIL diagnostics: cannot restore the runner on {kind}");
            return cleanup(ExitCode::from(1));
        }
        // The clean control passes, and the unavailable twin skips.
        if fs::write(&stub, "exit 0\n").is_err() {
            eprintln!("SMOKE-FAIL diagnostics: cannot write the stub on {kind}");
            return cleanup(ExitCode::from(1));
        }
        match run_with(&[flags]) {
            Ok(run) if run.rc == 0 && !run.stdout.contains(MARKER) => {}
            _ => {
                eprintln!("SMOKE-FAIL diagnostics: the clean control failed on {kind}");
                return cleanup(ExitCode::from(1));
            }
        }
        if fs::write(work.join("check-twins.sh"), "echo unavailable\nexit 2\n").is_err() {
            eprintln!("SMOKE-FAIL diagnostics: cannot write the twin on {kind}");
            return cleanup(ExitCode::from(1));
        }
        match run_with(&[json_flag]) {
            Ok(run) => {
                if run.rc != 0
                    || extract_json_int(&run.stdout, "skipped") != Some(1)
                    || extract_json_int(&run.stdout, "failed") != Some(0)
                {
                    eprintln!("SMOKE-FAIL diagnostics: the twin did not skip on {kind}");
                    return cleanup(ExitCode::from(1));
                }
            }
            Err(e) => {
                eprintln!("SMOKE-FAIL diagnostics: {e}");
                return cleanup(ExitCode::from(1));
            }
        }
        match run_with(&[json_flag, strict]) {
            Ok(run) => {
                if run.rc != 1 || extract_json_int(&run.stdout, "skipped") != Some(1) {
                    eprintln!("SMOKE-FAIL diagnostics: strict did not fail the skip on {kind}");
                    return cleanup(ExitCode::from(1));
                }
            }
            Err(e) => {
                eprintln!("SMOKE-FAIL diagnostics: {e}");
                return cleanup(ExitCode::from(1));
            }
        }
        println!("ok: {kind} late failure, lost-output mutation, JSON, clean control, and unavailable twin");
    }
    println!("verdict GATE-DIAGNOSTICS-OK");
    cleanup(ExitCode::from(0))
}

pub(super) fn print_help() {
    println!("podvm-smoke: the per-arch release smoke and the gate diagnostics.");
    println!();
    println!("  podvm-smoke BINARY TRIPLE [QEMU]   version, digests, static-ness,");
    println!("                                      embeds, and pull plus extract");
    println!("  podvm-smoke --diagnostics          the failed-gate diagnostic drive");
    println!("  podvm-smoke --diagnostics --powershell");
    println!("                                      the same, with the Windows runner too");
    println!();
    println!("Exit: 0 green, 1 something failed, 2 it could not run.");
}
