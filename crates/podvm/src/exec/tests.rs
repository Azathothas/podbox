use super::*;

fn v(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| (*s).to_string()).collect()
}

/// ⭐ TODO/cli.md T-1416, as in `run`: `-q` forces the banner off,
/// `--verbose` prints it, and `-v` is no flag of `exec`, as docker's
/// `exec` takes no volume (TODO/enter.md T-1653).
#[test]
fn quiet_and_verbose_parse() {
    let o = parse(&v(&["-q", "img", "true"])).unwrap();
    assert!(o.ask.quiet);
    let o = parse(&v(&["--verbose", "img", "true"])).unwrap();
    assert!(o.ask.verbose);
    assert_eq!(
        parse(&v(&["-v", "img", "true"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
}

#[test]
fn exec_usage_is_plain_ascii() {
    // TODO/cli.md T-1336: `exec --help` prints on a constrained host
    // with no terminal; a glyph there is unrenderable bytes.
    let bad = EXEC_USAGE.bytes().filter(|b| *b > 0x7F).count();
    assert_eq!(bad, 0, "exec usage carries {bad} non-ASCII bytes");
}

#[test]
fn parsing_stops_at_the_image_name() {
    let o = parse(&v(&["-e", "A=1", "alpine", "ls", "-l", "-e"])).unwrap();
    assert_eq!(o.env, v(&["A=1"]));
    assert_eq!(o.image.as_deref(), Some("alpine"));
    assert_eq!(o.command, v(&["ls", "-l", "-e"]));
}

/// ⛔ The difference from `run`, and it is deliberate rather than an
/// omission: an image's Cmd is what `run` starts.
#[test]
fn exec_has_no_default_command() {
    assert_eq!(parse(&v(&["alpine"])).unwrap_err(), EXIT_CLI_ERROR);
    assert!(parse(&v(&["alpine", "true"])).is_ok());
}

/// ⭐ TODO/cli.md T-0801. Every flag the table ADMITS for this verb reaches
/// an arm of this parser. The other direction is held by construction: an
/// argument starting with `-` goes through `parity::admit` first, so an arm
/// for a flag with no row is unreachable rather than a second surface.
#[test]
fn every_flag_the_table_admits_is_handled_by_this_parser() {
    for r in crate::parity::TABLE
        .iter()
        .filter(|r| r.verb == "exec" && r.flag.is_some())
    {
        if r.status == crate::parity::Status::None {
            // ⚠ A `None` row is refused BY `admit`, so reaching an arm is
            // exactly what it must not do. Asserted the other way round.
            let got = parse(&v(&[
                r.flag.unwrap().split(',').next().unwrap().trim(),
                "img",
                "true",
            ]));
            assert_eq!(
                got.unwrap_err(),
                EXIT_FLAG_ERROR,
                "{:?} was not refused",
                r.flag
            );
            continue;
        }
        for spelling in r.flag.unwrap().split(',') {
            let f = spelling.trim();
            if f == "-h" || f == "--help" {
                assert_eq!(parse(&v(&[f])).unwrap_err(), 0, "{f} did not print usage");
                continue;
            }
            // ⚠ ASSERTED ON THE EXIT CODE, not on a shape parsing. A
            // valued flag needs a value, a boolean one does not, and one
            // with a closed set of values (`--pull`) rejects any value this
            // test could invent: all three are legitimate and only one of
            // them parses. What no legitimate arm ever returns is
            // EXIT_RUNTIME_ERROR, which is the fallback arm's own code and
            // means exactly "the table admits this flag and nothing
            // implements it".
            // ⚠ The assertion is `parity::no_arm`'s panic. See the same
            // test in `run.rs` for why the code comparison it replaced
            // stopped being able to fail.
            for shape in [v(&[f, "V", "img", "true"]), v(&[f, "img", "true"])] {
                let _ = parse(&shape);
            }
        }
    }
}

#[test]
fn a_flag_needing_a_value_does_not_swallow_the_image() {
    assert_eq!(parse(&v(&["--platform"])).unwrap_err(), EXIT_FLAG_ERROR);
    assert_eq!(parse(&v(&["-w"])).unwrap_err(), EXIT_FLAG_ERROR);
}

/// ⭐ TODO/machine.md T-1302, as in `run`: two tier values, and emulator
/// arguments that collect whole rather than splitting on whitespace.
#[test]
fn the_tier_flag_takes_two_values_and_qemu_args_collect_whole() {
    assert_eq!(
        parse(&v(&["--podvm-tier=machine", "img", "true"]))
            .unwrap()
            .tier,
        Some(crate::tier::Want::Machine)
    );
    assert_eq!(
        parse(&v(&["--podvm-tier", "chroot", "img", "true"]))
            .unwrap()
            .tier,
        Some(crate::tier::Want::Chroot)
    );
    assert_eq!(
        parse(&v(&["--podvm-tier=sandbox", "img", "true"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    let o = parse(&v(&["--podvm-qemu-arg", "a b", "img", "true"])).unwrap();
    assert_eq!(o.qemu_args, v(&["a b"]));
}

/// ⭐ TODO/machine.md T-1305, as in `run`: the memory spelling parses to
/// bytes in both forms, and anything that is not a size is a flag error.
#[test]
fn the_mem_flag_parses_to_bytes_in_both_forms() {
    assert_eq!(
        parse(&v(&["--podvm-mem", "512M", "img", "true"]))
            .unwrap()
            .mem,
        Some(512 << 20)
    );
    assert_eq!(
        parse(&v(&["--podvm-mem=1G", "img", "true"])).unwrap().mem,
        Some(1 << 30)
    );
    assert_eq!(
        parse(&v(&["--podvm-mem=bogus", "img", "true"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
    assert_eq!(
        parse(&v(&["--podvm-mem", "0", "img", "true"])).unwrap_err(),
        EXIT_FLAG_ERROR
    );
}

/// ⭐ The banner and the machine-readable field are one pair of constants,
/// so a caller reading either gets the same answer.
#[test]
fn the_banner_says_what_inspect_reports() {
    let d = degradation();
    assert!(d.contains(crate::images::EXEC_MODE), "{d}");
    assert!(d.contains(crate::images::EXEC_SHARES), "{d}");
    assert!(d.contains("T-0505"), "{d}");
}
