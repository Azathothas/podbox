//! TODO/cli.md T-0805: every failure carries the operation, the errno, the
//! mechanism and the remedy.
//!
//! TOOL.md section 8 is the field guide this module encodes. Each row maps one
//! observed failure to its four parts, so a new failure mode arrives as a table
//! row rather than a new code path. The ownership row drives the extraction
//! report below. The spawn row drives the run diagnostic T-0809 wires.

/// One row of the field guide: what the caller saw, and what it means.
pub struct FailureRow {
    /// The fragment that identifies the failure in program output.
    pub observation: &'static str,
    /// The operation that met the denial.
    pub operation: &'static str,
    /// The errno or signal the caller observed.
    pub errno: &'static str,
    /// What produced it, quoted from measured state where a probe reads it.
    pub mechanism: &'static str,
    /// What clears it.
    pub remedy: &'static str,
}

/// The field guide, as data. TOOL.md section 8 carries the same rows in prose.
///
/// Specific observations precede general ones, so a setuid denial that also
/// reads `Invalid argument` maps to the setuid row and not the chown row.
pub const TABLE: &[FailureRow] = &[
    FailureRow {
        observation: "fork/exec",
        operation: "spawn the child process",
        errno: "EPERM",
        mechanism: "the child's setgroups call met the denial, not the execve. The clone runs first and setgroups runs before execve, and both surface the same way",
        remedy: "drop the credential, or set the field that suppresses setgroups",
    },
    FailureRow {
        observation: "setuid",
        operation: "change the process uid",
        errno: "EINVAL",
        mechanism: "the uid has no mapping in this user namespace. Only uid 0 exists there",
        remedy: "keep uid 0 and do not drop privileges",
    },
    FailureRow {
        observation: "TRACEME",
        operation: "trace the child with ptrace",
        errno: "EPERM",
        mechanism: "the filter denies ptrace, and the launchpad hint misdirects",
        remedy: "use a chroot or an interpose engine. No env var helps",
    },
    FailureRow {
        observation: "Failed to make / slave",
        operation: "remount / as slave",
        errno: "EPERM",
        mechanism: "the namespace clone succeeded and the mount died after it",
        remedy: "use chroot. Nothing mount-shaped works here",
    },
    FailureRow {
        observation: "move_mount",
        operation: "attach a detached mount",
        errno: "EPERM",
        mechanism: "the syscall executed, so the denial is an LSM and not the filter",
        remedy: "stop looking for an attach path. Detached mounts stay detached",
    },
    FailureRow {
        observation: "detached",
        operation: "open a path through a detached mount",
        errno: "EACCES",
        mechanism: "the LSM cannot resolve a path into a detached mount",
        remedy: "treat detached mounts as unusable, not only unattachable",
    },
    FailureRow {
        observation: "mount",
        operation: "mount a filesystem",
        errno: "EPERM",
        mechanism: "the filter denies mount everywhere, including inside a namespace the clone just created",
        remedy: "use chroot. Nothing mount-shaped works here",
    },
    FailureRow {
        observation: "ENETUNREACH",
        operation: "reach the network from the child",
        errno: "ENETUNREACH",
        mechanism: "the netns clone succeeded and handed the child an empty routeless namespace",
        remedy: "never pass namespace clone flags the child cannot populate",
    },
    FailureRow {
        observation: "Cannot change ownership",
        operation: "restore ownership while unpacking",
        errno: "EINVAL",
        mechanism: "the id has no mapping in this user namespace. The wall is chown to an unmapped id",
        remedy: "unpack with --no-same-owner, or interpose the call",
    },
    FailureRow {
        observation: "Invalid argument",
        operation: "restore ownership with chown",
        errno: "EINVAL",
        mechanism: "the id has no mapping in this user namespace, which reads as a bad argument",
        remedy: "extract without ownership and record the intended ids",
    },
    FailureRow {
        observation: "short write",
        operation: "write the decompressed payload",
        errno: "ENOSPC, as libarchive status -20",
        mechanism: "-20 is ARCHIVE_WARN, a status code and not an errno. archive_errno is ENOSPC",
        remedy: "use a temp dir with room. /tmp is 64 MiB on the target",
    },
    FailureRow {
        observation: "failed to chown temporary download directory",
        operation: "chown the download directory",
        errno: "EINVAL",
        mechanism: "DownloadUser names a uid with no mapping",
        remedy: "comment DownloadUser out",
    },
    FailureRow {
        observation: "Method http has died",
        operation: "fetch the package index",
        errno: "transport failure on tcp/80",
        mechanism: "tcp egress is broken",
        remedy: "use https sources plus a CA bundle",
    },
    FailureRow {
        observation: "no Release file",
        operation: "fetch the package index",
        errno: "transport failure on tcp/80",
        mechanism: "tcp egress is broken",
        remedy: "use https sources plus a CA bundle",
    },
    FailureRow {
        observation: "getpwuid",
        operation: "resolve a uid to a name",
        errno: "ENOENT",
        mechanism: "the host has no /etc/passwd",
        remedy: "synthesize one in every rootfs built",
    },
    FailureRow {
        observation: "unknown userid",
        operation: "resolve a uid to a name",
        errno: "ENOENT",
        mechanism: "the host has no /etc/passwd",
        remedy: "synthesize one in every rootfs built",
    },
    FailureRow {
        observation: "Symbolic link loop",
        operation: "descend into the package cache",
        errno: "ELOOP",
        mechanism: "a later layer whiteouts an earlier self-referential symlink, and plain tar ignores .wh. files",
        remedy: "apply whiteouts after each layer",
    },
    FailureRow {
        observation: "zypper",
        operation: "refresh the package index",
        errno: "reverted fixups",
        mechanism: "the RIS index regenerates repos.d",
        remedy: "edit /usr/share/zypp/local/service/ first",
    },
    FailureRow {
        observation: "Couldn't resolve host",
        operation: "resolve the mirror host",
        errno: "DNS failure",
        mechanism: "the image baked a build-host resolver",
        remedy: "always install the host's resolv.conf",
    },
    FailureRow {
        observation: "invalid syntax for user",
        operation: "remap root to a name",
        errno: "getpwuid failure",
        mechanism: "udocker remaps root through getpwuid(0), which fails with no /etc/passwd",
        remedy: "run with a numeric uid, which skips the remap",
    },
    FailureRow {
        observation: "dry-run",
        operation: "supervise the child",
        errno: "unread arguments",
        mechanism: "the notif supervisor fell back to Continue because it could not read the child's arguments",
        remedy: "probe the tier's legs up front and refuse the tier",
    },
    FailureRow {
        observation: "growing file",
        operation: "redirect to /dev/null",
        errno: "no device behind the path",
        mechanism: "/dev/null is absent and the shell's redirect implies O_CREAT",
        remedy: "install the regular-file shim before the payload runs",
    },
];

/// The ownership row with live measurements: the example entry, the map
/// contents T-0105 reads, and the remedy.
///
/// The map legs share one line after the mechanism, so the quoted map always
/// reads within two lines of the example the entry's Prove greps for.
/// Returns `None` where the extraction recorded no example, in which case the
/// caller prints the counts line alone.
pub fn ownership_note(
    done: &podbox_extract::Extracted,
    id: &podbox_probe::identity::Identity,
) -> Option<String> {
    let first = done.first_dropped.as_ref()?;
    let row = TABLE.iter().find(|r| r.observation == "Invalid argument")?;
    let mut out = format!(
        "{} of {} (uid {}, gid {}): {}",
        row.operation, first.path, first.uid, first.gid, row.errno
    );
    out.push_str(&format!("\n  {}", row.mechanism));
    let mut legs = Vec::new();
    if first.uid != first.applied_uid {
        legs.push(format!(
            "uid {} is not mapped in this user namespace (/proc/self/uid_map: {})",
            first.uid,
            map_contents(id, "/proc/self/uid_map", id.uid_map.as_deref()),
        ));
    }
    if first.gid != first.applied_gid {
        legs.push(format!(
            "gid {} is not mapped in this user namespace (/proc/self/gid_map: {})",
            first.gid,
            map_contents(id, "/proc/self/gid_map", id.gid_map.as_deref()),
        ));
    }
    out.push_str(&format!("\n  {}", legs.join("; ")));
    out.push_str(&format!(
        "\n  {}; intended metadata recorded in {}",
        row.remedy,
        done.sidecar.display()
    ));
    Some(out)
}

/// The map contents, or the reason no reading exists. An absent value never
/// renders as an empty string that reads like one.
fn map_contents(id: &podbox_probe::identity::Identity, path: &str, map: Option<&str>) -> String {
    if let Some(m) = map {
        return m.to_string();
    }
    let prefix = format!("{path}: ");
    for e in &id.unreadable {
        if let Some(rest) = e.strip_prefix(&prefix) {
            return format!("unreadable: {rest}");
        }
    }
    "no reading and no reason recorded".to_string()
}

/// The extraction report both `extract` and `run` print: the counts, then the
/// four-part note where the tree's ownership differs from the image's.
///
/// One function for both verbs, so the two cannot describe one extraction two
/// ways.
pub(crate) fn report_dropped(
    out: &mut dyn std::io::Write,
    done: &podbox_extract::Extracted,
    id: &podbox_probe::identity::Identity,
) {
    if done.ownership_dropped == 0 {
        return;
    }
    // Said out loud. A tree whose ownership differs from the image's and says
    // nothing is the dishonesty this project exists to refuse.
    let _ = writeln!(
        out,
        "podbox: {} of {} entries carry an id this machine cannot apply; \
         what the image intended is in {}",
        done.ownership_dropped,
        done.sidecar_rows,
        done.sidecar.display()
    );
    if let Some(note) = ownership_note(done, id) {
        let _ = writeln!(out, "podbox: {note}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_with_maps(
        uid_map: Option<&str>,
        gid_map: Option<&str>,
    ) -> podbox_probe::identity::Identity {
        podbox_probe::identity::Identity {
            uid_map: uid_map.map(|s| s.to_string()),
            gid_map: gid_map.map(|s| s.to_string()),
            ..Default::default()
        }
    }

    fn extracted_with_example(
        dropped: u64,
        first: Option<podbox_extract::sidecar::FirstDropped>,
    ) -> podbox_extract::Extracted {
        podbox_extract::Extracted {
            rootfs: Default::default(),
            sidecar: Default::default(),
            layers: 0,
            entries: 0,
            removed: 0,
            uncompressed_bytes: 0,
            uncompressed_estimated: false,
            skipped: 0,
            skipped_kinds: Vec::new(),
            sidecar_rows: 0,
            ownership_dropped: dropped,
            first_dropped: first,
        }
    }

    #[test]
    fn every_row_carries_all_four_parts() {
        assert!(!TABLE.is_empty());
        for r in TABLE {
            assert!(!r.observation.is_empty(), "a row has no observation");
            assert!(!r.operation.is_empty(), "a row has no operation");
            assert!(!r.errno.is_empty(), "a row has no errno");
            assert!(!r.mechanism.is_empty(), "a row has no mechanism");
            assert!(!r.remedy.is_empty(), "a row has no remedy");
        }
    }

    /// The entry's Prove, in miniature: the quoted map reads within two lines
    /// of the example, whether one id dropped or both did.
    fn assert_map_beside_example(note: &str, id_text: &str, map_text: &str) {
        let lines: Vec<&str> = note.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.contains(id_text))
            .expect("the example names the id");
        assert!(
            lines[at + 1..at + 3].iter().any(|l| l.contains(map_text)),
            "the map reads beside the example:\n{note}"
        );
    }

    #[test]
    fn the_note_names_the_entry_the_errno_the_map_and_the_remedy() {
        let done = extracted_with_example(
            900,
            Some(podbox_extract::sidecar::FirstDropped {
                path: "etc/shadow".into(),
                uid: 0,
                gid: 42,
                applied_uid: 0,
                applied_gid: 0,
            }),
        );
        let id = identity_with_maps(Some("0 1000 1"), Some("0 1000 1"));
        let note = ownership_note(&done, &id).expect("an example renders");
        assert!(note.contains("etc/shadow"), "{note}");
        assert!(note.contains("gid 42"), "{note}");
        assert!(note.contains("EINVAL"), "{note}");
        assert!(note.contains("/proc/self/gid_map: 0 1000 1"), "{note}");
        assert!(note.contains("without ownership"), "{note}");
        assert_map_beside_example(&note, "gid 42", "gid_map");
        // The uid applied cleanly, so only the gid leg renders.
        assert!(!note.contains("uid_map"), "{note}");
    }

    #[test]
    fn both_dropped_ids_still_read_beside_the_example() {
        let done = extracted_with_example(
            2,
            Some(podbox_extract::sidecar::FirstDropped {
                path: "etc/shadow".into(),
                uid: 0,
                gid: 42,
                applied_uid: 1000,
                applied_gid: 1000,
            }),
        );
        let id = identity_with_maps(Some("0 1000 1"), Some("0 1000 1"));
        let note = ownership_note(&done, &id).expect("an example renders");
        assert_map_beside_example(&note, "gid 42", "gid_map");
        assert!(note.contains("/proc/self/uid_map: 0 1000 1"), "{note}");
    }

    #[test]
    fn an_unreadable_map_reports_itself_as_unreadable() {
        let done = extracted_with_example(
            1,
            Some(podbox_extract::sidecar::FirstDropped {
                path: "etc/shadow".into(),
                uid: 0,
                gid: 42,
                applied_uid: 0,
                applied_gid: 1000,
            }),
        );
        let mut id = identity_with_maps(None, None);
        id.unreadable
            .push("/proc/self/gid_map: No such file or directory (os error 2)".into());
        let note = ownership_note(&done, &id).expect("an example renders");
        assert!(note.contains("unreadable"), "{note}");
        assert!(!note.contains("/proc/self/gid_map: \n"), "{note}");
    }

    #[test]
    fn no_example_means_no_note() {
        let done = extracted_with_example(3, None);
        let id = identity_with_maps(Some("0 1000 1"), Some("0 1000 1"));
        assert!(ownership_note(&done, &id).is_none());
    }
}
