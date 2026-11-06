use super::*;

pub(super) fn parse_json(text: &str) -> Option<Json> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        pos: 0,
    };
    let value = reader.value()?;
    reader.skip();
    if reader.pos == reader.bytes.len() {
        Some(value)
    } else {
        None
    }
}

pub(super) fn field<'a>(row: &'a Json, key: &str) -> Option<&'a Json> {
    match row {
        Json::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

// ------------------------------------------------------------- pure half ---

/// The words of one help text, for the one-verb-set comparison. Clause 1 of
/// the script compares the two help outputs byte for byte and then greps
/// four words; the set comparison below is the unit-testable half.
pub(super) fn help_words(help: &str) -> BTreeSet<String> {
    help.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_')
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// True where the parity table carries the row: the verb with a flag
/// starting at the prefix, where a missing flag reads as `-`.
pub(super) fn table_carries(rows: &[Json], verb: &str, flag: &str) -> bool {
    rows.iter().any(|row| {
        let verb_match = field(row, "verb").and_then(|value| match value {
            Json::Str(text) => Some(text.as_str()),
            _ => None,
        }) == Some(verb);
        let flag_text = match field(row, "flag") {
            Some(Json::Str(text)) => text.clone(),
            _ => "-".to_string(),
        };
        verb_match && flag_text.starts_with(flag)
    })
}

// ----------------------------------------------------------------- driver ---

struct Report {
    text: String,
    driven: u32,
    fails: u32,
    failed: bool,
}

impl Report {
    fn say(&mut self, line: &str) {
        self.text.push_str(line);
        self.text.push('\n');
    }
    fn pass(&mut self, what: &str) {
        self.driven += 1;
        self.say(&format!("  ok: {what}"));
    }
    fn miss(&mut self, what: &str) {
        self.failed = true;
        self.fails += 1;
        self.say(&format!("  FAIL: {what}"));
    }
}

pub(super) fn drive(root: &Path, bin_flag: Option<String>) -> i32 {
    let out = root.join("experiments/results/podvm-parity.txt");
    let mut report = Report {
        text: String::new(),
        driven: 0,
        fails: 0,
        failed: false,
    };
    let binary = match bin_flag {
        Some(path) => PathBuf::from(path),
        None => std::env::var("PODVM_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root.join("target/x86_64-unknown-linux-musl/release/podvm")),
    };
    if !binary.is_file() {
        eprintln!("SKIP: {} is not an executable. Build it:", binary.display());
        eprintln!("      cargo build --release --target x86_64-unknown-linux-musl");
        return 2;
    }
    let name = binary.to_string_lossy().into_owned();
    // `docker` and `podman` are the binary under other names. The dispatch
    // keys on the basename of argv[0], so a copy under each name routes
    // exactly like an installed symlink, portably.
    let scratch = std::env::temp_dir().join(format!(
        "podvm-parity-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    if fs::create_dir_all(&scratch).is_err() {
        eprintln!("SKIP: cannot create scratch");
        return 2;
    }
    let mut twins: Vec<(&str, String)> = Vec::new();
    for alias in ALIASES {
        let twin = scratch.join(alias);
        if fs::copy(&binary, &twin).is_err() {
            eprintln!("SKIP: cannot stage the {alias} name");
            fs::remove_dir_all(&scratch).ok();
            return 2;
        }
        twins.push((alias, twin.to_string_lossy().into_owned()));
    }
    let outcome = drive_inner(&name, &twins, &mut report);
    fs::remove_dir_all(&scratch).ok();
    report.say("");
    report.say(&format!(
        "== counts: {} driven, {} mismatches",
        report.driven, report.fails
    ));
    print!("{}", report.text);
    if fs::create_dir_all(out.parent().unwrap_or_else(|| Path::new("."))).is_ok() {
        fs::write(&out, &report.text).ok();
    }
    // Copy to /out where a lane runner collects evidence, like the
    // lock-inheritance proof.
    let out_dir = Path::new("/out");
    if out_dir.is_dir() {
        fs::copy(&out, out_dir.join("podvm-parity.txt")).ok();
    }
    println!();
    println!(
        "written to {}",
        out.strip_prefix(root).unwrap_or(&out).display()
    );
    if outcome == 2 {
        return 2;
    }
    if report.failed {
        return 1;
    }
    0
}

/// The names the binary answers to besides its own.
const ALIASES: [&str; 2] = ["docker", "podman"];

fn drive_inner(binary: &str, twins: &[(&str, String)], report: &mut Report) -> i32 {
    let version = run_combined(binary, &["version"], 60)
        .map(|(_, text)| text.trim().to_string())
        .unwrap_or_default();
    report.say("== conditions");
    report.say(&format!(
        "date              {}",
        run_combined("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], 10)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!(
        "host kernel       {}",
        run_combined("uname", &["-r"], 10)
            .map(|(_, t)| t.trim().to_string())
            .unwrap_or_default()
    ));
    report.say(&format!("podvm            {version}"));
    report.say("json              parsed in-process; no jq needed");
    report.say("");

    // The exit codes are data, read out of the binary like the script reads
    // them out of the exit-codes helper. A case the table does not carry is
    // a failure to read it, never a guess.
    let codes_doc = match run_combined(
        binary,
        &["system", "info", "--format", "{{json .ExitCodes}}"],
        60,
    ) {
        Some((0, text)) if !text.trim().is_empty() => text,
        _ => {
            eprintln!("SKIP: cannot read podvm's exit-code table");
            return 2;
        }
    };
    let mut codes = BTreeMap::new();
    if let Some(Json::Array(rows)) = parse_json(&codes_doc) {
        for row in &rows {
            if let (Some(Json::Str(case)), Some(Json::Number(code))) =
                (field(row, "case"), field(row, "code"))
                && let Ok(number) = code.parse::<i32>()
            {
                codes.insert(case.clone(), number);
            }
        }
    }
    let runtime_error = match codes.get("runtime-error") {
        Some(code) => *code,
        None => {
            eprintln!("SKIP: cannot read podvm's exit-code table");
            return 2;
        }
    };
    let flag_error = match codes.get("flag-error") {
        Some(code) => *code,
        None => {
            eprintln!("SKIP: cannot read podvm's exit-code table");
            return 2;
        }
    };

    // The table, read out of the binary. Every clause starts here, so a row
    // the binary does not publish is a row this binary cannot drive.
    let parity_doc = match run_combined(
        binary,
        &["system", "info", "--format", "{{json .Parity}}"],
        60,
    ) {
        Some((0, text)) if !text.trim().is_empty() => text,
        _ => {
            eprintln!("SKIP: the parity table did not print");
            return 2;
        }
    };
    let rows = match parse_json(&parity_doc) {
        Some(Json::Array(rows)) => rows,
        _ => {
            eprintln!("SKIP: the parity table did not parse");
            return 2;
        }
    };

    report.say("== 0. the tier rows are data");
    for (verb, flag) in [
        ("run", "--podvm-tier"),
        ("run", "--podvm-qemu-arg"),
        ("exec", "--podvm-tier"),
        ("exec", "--podvm-qemu-arg"),
        ("docker", "-"),
        ("podman", "-"),
    ] {
        if table_carries(&rows, verb, flag) {
            report.pass(&format!("table carries {verb} {flag}"));
        } else {
            report.miss(&format!("table has no {verb} {flag} row"));
        }
    }

    report.say("");
    report.say("== 1. every name's help text lists one set of verbs");
    let podvm_help = run_combined(binary, &["--help"], 60).map(|(_, text)| text);
    for (alias, twin) in twins {
        let twin_help = run_combined(twin, &["--help"], 60).map(|(_, text)| text);
        match (&podvm_help, twin_help) {
            (Some(left), Some(right)) if *left == right => report.pass(&format!(
                "podvm --help and {alias} --help are the same text"
            )),
            _ => report.miss(&format!("podvm --help and {alias} --help differ")),
        }
    }
    // The word grep over the binary's own text, like the script.
    let words = podvm_help.as_deref().map(help_words).unwrap_or_default();
    for word in ["run", "exec", "docker", "podman"] {
        if words.contains(word) {
            report.pass(&format!("podvm --help names {word}"));
        } else {
            report.miss(&format!("podvm --help never names {word}"));
        }
    }

    // run VERB ARGS...: the words, the code, and which of two markers the
    // output carries. The flag-error code is also the runtime code
    // (TODO/cli.md T-0802), so the number alone proves nothing. Every call
    // is bounded: a refusal that hangs is a hang with a green label on it.
    let run = |args: &[&str]| run_combined(binary, args, 120).unwrap_or((124, String::new()));
    let podman = twins
        .iter()
        .find(|(alias, _)| *alias == "podman")
        .map(|(_, path)| path.clone())
        .unwrap_or_default();
    let pvm = |args: &[&str]| run_combined(&podman, args, 120).unwrap_or((124, String::new()));

    report.say("");
    report.say("== 2. the flag routes");
    let (rc, text) = run(&[
        "run",
        "--podvm-tier=machine",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == runtime_error && text.contains("machine tier refused") {
        report.pass("run --podvm-tier=machine refuses naming the legs");
    } else {
        report.miss(&format!("run --podvm-tier=machine (rc={rc}): {text}"));
    }
    let (rc, text) = run(&[
        "run",
        "--podvm-tier=sandbox",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == flag_error && text.contains("takes machine or chroot") {
        report.pass("run --podvm-tier=sandbox is a flag error");
    } else {
        report.miss(&format!("run --podvm-tier=sandbox (rc={rc}): {text}"));
    }
    let (rc, text) = run(&[
        "run",
        "--podvm-qemu-arg=--cpu,host",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == flag_error && text.contains("needs --podvm-tier=machine") {
        report.pass("run --podvm-qemu-arg without the machine tier is a flag error");
    } else {
        report.miss(&format!(
            "run --podvm-qemu-arg without the tier (rc={rc}): {text}"
        ));
    }
    let (rc, text) = run(&[
        "run",
        "--podvm-tier=machine",
        "--podvm-qemu-arg=--cpu,host",
        "--podvm-qemu-arg",
        "a b",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == runtime_error && text.contains("machine tier refused") {
        report.pass("repeated qemu args with a space ride the machine tier to its refusal");
    } else {
        report.miss(&format!(
            "run --podvm-tier=machine with qemu args (rc={rc}): {text}"
        ));
    }

    report.say("");
    report.say("== 3. the name routes to the flag, never to a tier");
    let (rc, text) = pvm(&[
        "run",
        "--podvm-tier=machine",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == runtime_error
        && text.contains("machine tier refused")
        && text.contains("invoked as `podman`")
    {
        report.pass("podman run --podvm-tier=machine refuses naming the legs and says its name");
    } else {
        report.miss(&format!(
            "podman run --podvm-tier=machine (rc={rc}): {text}"
        ));
    }
    let (_, text) = pvm(&["run", "never-pulled-145:tag", "/bin/true"]);
    if !text.contains("machine tier refused") {
        report.pass("podman run with no flag proceeds past the tier: no name selects one");
    } else {
        report.miss(&format!("podman run with no flag: {text}"));
    }

    report.say("== 4. exec mirrors run, and other verbs refuse the flag as unlisted");
    let (rc, text) = run(&[
        "exec",
        "--podvm-tier=machine",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == runtime_error && text.contains("machine tier refused") {
        report.pass("exec --podvm-tier=machine refuses naming the legs");
    } else {
        report.miss(&format!("exec --podvm-tier=machine (rc={rc}): {text}"));
    }
    let (rc, text) = pvm(&[
        "exec",
        "--podvm-tier=machine",
        "never-pulled-145:tag",
        "/bin/true",
    ]);
    if rc == runtime_error && text.contains("machine tier refused") {
        report.pass("podman exec --podvm-tier=machine mirrors run");
    } else {
        report.miss(&format!(
            "podman exec --podvm-tier=machine (rc={rc}): {text}"
        ));
    }
    let (rc, text) = run(&["ps", "--podvm-tier=machine"]);
    if rc == flag_error && text.contains("no row in the parity table") {
        report.pass("ps --podvm-tier is refused as unlisted");
    } else {
        report.miss(&format!("ps --podvm-tier (rc={rc}): {text}"));
    }
    0
}
