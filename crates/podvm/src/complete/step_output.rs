//! The output of a step of the completion, TODO/image.md T-1763: an
//! unlinked file that podvm prints from only where the step fails.

/// An unlinked file for the output of one step (T-1763).
pub(super) fn step_output() -> std::io::Result<std::fs::File> {
    let path = std::env::temp_dir().join(format!(
        "podvm-step-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)?;
    let _ = std::fs::remove_file(&path);
    Ok(f)
}

/// The last `n` lines of the output of a step, in printable ASCII.
pub(super) fn tail(out: &std::fs::File, n: usize) -> Vec<String> {
    use std::io::{Read, Seek};
    let mut f = out;
    let mut buf = Vec::new();
    if f.seek(std::io::SeekFrom::Start(0)).is_err()
        || f.take(1 << 20).read_to_end(&mut buf).is_err()
    {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&buf);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(n)..]
        .iter()
        .map(|l| {
            l.chars()
                .map(|c| {
                    if c.is_ascii_graphic() || c == ' ' {
                        c
                    } else {
                        '?'
                    }
                })
                .collect()
        })
        .collect()
}

/// TODO/image.md T-1773: a step enters the rootfs by chroot, so where
/// chroot is denied (the userland rung) each step is a skipped row that
/// names why, not a failure of each run. True where it skipped them.
pub(super) fn skipped(
    verb: &str,
    steps: &[podvm_complete::Step],
    report: &mut podvm_complete::Report,
    quiet: bool,
    err: &mut dyn std::io::Write,
) -> bool {
    if podvm_probe::probes::chroot_usable(&podvm_probe::run()) {
        return false;
    }
    for s in steps {
        report.fixups.push(podvm_complete::Fixup {
            entry: s.entry,
            id: s.id,
            path: String::new(),
            action: podvm_complete::Action::Skipped,
            detail: format!(
                "`{}` runs inside the rootfs by chroot(2), which this machine denies",
                s.argv.join(" ")
            ),
            degraded: true,
        });
    }
    if !quiet {
        let _ = writeln!(
            err,
            "podvm {verb}: complete: {} step(s) skipped: a step enters the rootfs by \
             chroot(2), which this machine denies (TODO/image.md T-1773)",
            steps.len()
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn the_tail_of_a_step_is_its_last_printable_lines() {
        let mut f = step_output().expect("an unlinked file");
        for i in 0..30 {
            writeln!(f, "rehash: warning {i}").unwrap();
        }
        f.write_all(b"\n\x1b[31mlast\n").unwrap();
        let t = tail(&f, 3);
        assert_eq!(t, ["rehash: warning 28", "rehash: warning 29", "?[31mlast"]);
    }
}
