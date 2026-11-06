use super::*;

/// The verb's own row, if podvm names it at all.
pub fn verb(name: &str) -> Option<&'static Row> {
    TABLE.iter().find(|r| r.verb == name && r.flag.is_none())
}

/// The row for one flag of one verb, under any spelling the parser accepts.
///
/// ⛔ This is what makes the table binding rather than descriptive. A parser
/// asks here first, so a flag with no row cannot be quietly accepted and a row
/// with no parser arm cannot be quietly ignored.
/// Which verb's rows a verb's flags are listed under.
///
/// ⭐ `create` is served by `run`'s PARSER, so it takes `run`'s flag set. The
/// table holds ONE copy of those rows rather than two that can diverge, and this
/// is the single place that says where a verb's rows live. ⛔ Without it,
/// naming the caller's verb in the diagnostics would also have made `flag`
/// refuse every flag `create` accepts, which is the shape of hole
/// `docs/conventions/code.md` calls a second, quieter surface. T-0801.
///
/// The `image` and `system` groups dispatch to one handler per subverb, so
/// each subverb's rows live under its own name and the refusal names the
/// path the caller typed, as `create` does for `run`. T-0808.
pub fn rows_of(verb: &str) -> &str {
    match verb {
        "create" => "run",
        "image ls" | "image list" => "images",
        "image rm" | "image remove" => "rmi",
        "image prune" => "prune",
        "image tag" => "tag",
        "image inspect" => "inspect",
        "image pull" => "pull",
        "image extract" => "extract",
        "system info" => "info",
        "system install-names" => "install-names",
        "system abi" => "abi",
        "system df" => "df",
        v => v,
    }
}

pub fn flag(verb: &str, name: &str) -> Option<&'static Row> {
    // ⚠ `--flag=value` is one argument to the shell and two things here. The
    // row is for the flag, so the value is cut off before the lookup.
    let name = name.split('=').next().unwrap_or(name);
    let rows = rows_of(verb);
    TABLE.iter().find(|r| r.verb == rows && r.names(name))
}

/// Decide whether a verb's parser may go on to handle `arg`, and say why not.
///
/// ⛔ **This is the guard T-0801 is for.** `Ok(())` means the table has a row
/// and the flag is not refused; `Err(code)` means the caller has already been
/// told, in one line, either that podvm has no such flag or that podvm has it
/// and will not honour it. A parser that decided this itself would be a second
/// table, and the one nobody reads is the one that drifts.
///
/// ⚠ A `Stub` row passes: it is accepted and does nothing, which is a
/// difference the table states rather than a refusal.
pub fn admit(verb: &str, arg: &str, usage: &str) -> Result<(), i32> {
    match flag(verb, arg) {
        Option::None => {
            // ⚠ Point at the verb whose ROWS were consulted, which is not always
            // the one the caller typed: `podvm create` is served by `run`'s
            // parser. Saying "this verb" would send a caller to a row set that
            // does not exist.
            let under = rows_of(verb);
            let where_ = if under == verb {
                "lists every flag this verb takes".to_string()
            } else {
                format!("lists every flag it takes under `{under}`, whose parser serves it")
            };
            eprintln!(
                "podvm {verb}: unknown option {arg:?}. It has no row in the parity \
                 table; `podvm system info` {where_}"
            );
            eprint!("{usage}");
            Err(podvm_image::error::EXIT_FLAG_ERROR)
        }
        Some(r) if r.status == Status::None => {
            // ⛔ Refused UP FRONT, with the reason, rather than accepted and
            // silently lost. `dockless`'s posture, and the whole point of the
            // table being consulted rather than described.
            eprintln!(
                "podvm {verb}: {} is in the parity table with status None: {}",
                r.flag.unwrap_or(arg),
                r.note
            );
            Err(podvm_image::error::EXIT_FLAG_ERROR)
        }
        Some(_) => Ok(()),
    }
}

/// Admit-first, one read path (T-0808): refuse every dash-arg the table
/// does not list before any match arm runs, so an arm for a flag with no
/// row is unreachable. A value that starts with `-` must use the `=` form.
pub fn admit_all(verb: &str, args: &[String], usage: &str) -> Option<i32> {
    for a in args {
        if a.starts_with('-')
            && let Err(c) = admit(verb, a, usage)
        {
            return Some(c);
        }
    }
    None
}

/// The arm a verb's parser reaches when [`admit`] passed a flag and the parser
/// has no case for it.
///
/// ⛔ **That is a defect in podvm, not a caller mistake**, and it is reported
/// as one. ⚠ It is also the case a test has to be able to see, and since
/// T-0802 it can no longer be seen in the exit code: a flag error and a runtime
/// failure are both docker's 125, so the sentinel the parser tests used to
/// compare against became equal to the legitimate refusals it was distinguishing
/// itself from. Under `cfg(test)` this panics with the flag's own name, which is
/// a stronger assertion than the code comparison it replaces; in a shipped
/// binary it returns 125 and says what happened, because a runtime that panics
/// on its own argument surface is worse than one that refuses.
pub fn no_arm(verb: &str, flag: &str) -> i32 {
    let msg = format!(
        "podvm {verb}: {flag:?} is in the parity table and this parser has no arm \
         for it. That is a bug in podvm (TODO/cli.md T-0801)"
    );
    if cfg!(test) {
        panic!("{msg}");
    }
    eprintln!("{msg}");
    podvm_image::error::EXIT_RUNTIME_ERROR
}

/// The whole table as a JSON array, one object per row.
///
/// ⭐ T-0801's `Prove` reads exactly this: `length >= 60`, and every `status`
/// one of the four words.
pub fn json() -> String {
    let rows: Vec<serde_json::Value> = TABLE
        .iter()
        .map(|r| {
            serde_json::json!({
                "verb": r.verb,
                "flag": r.flag,
                "status": r.status.word(),
                "note": r.note,
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// The same table for a person, one row per line.
pub fn text() -> String {
    let mut out = String::new();
    let mut last = "";
    for r in TABLE {
        if r.verb != last {
            last = r.verb;
        }
        let name = match r.flag {
            Option::None => r.verb.to_string(),
            Some(f) => format!("{} {}", r.verb, f),
        };
        out.push_str(&format!("{:<34} {:<9} {}\n", name, r.status.word(), r.note));
    }
    out
}
