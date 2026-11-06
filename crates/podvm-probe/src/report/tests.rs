use super::*;

/// The document, taken from a real run of the shipping path.
fn one_document() -> String {
    let f = crate::run();
    let sel = Selection::choose(&f);
    document(&f, &sel)
}

/// A value of `key` at the top level of a flat JSON object, without a
/// parser. ⚠ Only sound for the shapes asserted below, which are all
/// `"key":"value"` on one line, and it is test scaffolding rather than
/// something shipped.
fn field(doc: &str, key: &str) -> Option<String> {
    let at = doc.find(&format!("\"{key}\":\""))? + key.len() + 4;
    let rest = &doc[at..];
    Some(rest[..rest.find('"')?].to_string())
}

#[test]
fn the_cache_key_agrees_with_the_identity_block_it_is_derived_from() {
    // ⛔ docs/conventions/forbidden-patterns.md: a value in two places with
    // no check that they agree drifts, and the copy a reader trusts is the
    // wrong one. This is that check. TODO/probe.md T-0111.
    let doc = one_document();
    let id = crate::identity::read();
    let key = crate::identity::confinement_key(&id);
    for (name, value) in key.components() {
        if let Some(v) = value {
            assert!(
                doc.contains(&format!("\"{name}\":")),
                "the document has no {name}"
            );
            if matches!(
                name,
                "uid_map"
                    | "gid_map"
                    | "setgroups"
                    | "seccomp"
                    | "seccomp_filters"
                    | "cap_eff"
                    | "cap_bnd"
            ) {
                assert_eq!(
                    id_field(&id, name).as_deref(),
                    Some(v),
                    "cache_key.{name} and identity.{name} disagree"
                );
            }
        }
    }
}

fn id_field(id: &Identity, name: &str) -> Option<String> {
    match name {
        "uid_map" => id.uid_map.clone(),
        "gid_map" => id.gid_map.clone(),
        "setgroups" => id.setgroups.clone(),
        "seccomp" => id.seccomp.clone(),
        "seccomp_filters" => id.seccomp_filters.clone(),
        "cap_eff" => id.cap_eff.clone(),
        "cap_bnd" => id.cap_bnd.clone(),
        _ => None,
    }
}

#[test]
fn the_document_carries_a_complete_cache_key_on_this_machine() {
    let doc = one_document();
    assert!(doc.contains("\"cache_key\":{"), "{doc}");
    assert!(doc.contains("\"complete\":true"), "{doc}");
    let mnt = field(&doc, "mnt_ns").expect("mnt_ns in the document");
    assert!(mnt.starts_with("mnt:["), "{mnt}");
}

#[test]
fn two_documents_from_one_machine_agree_on_the_rung_and_the_key() {
    // ⚠ The rung and the key, not the whole document: the writable probe
    // reports free blocks, which move between two runs on a live machine.
    let (a, b) = (one_document(), one_document());
    assert_eq!(field(&a, "rung"), field(&b, "rung"));
    assert_eq!(field(&a, "mnt_ns"), field(&b, "mnt_ns"));
    assert_eq!(field(&a, "boot_id"), field(&b, "boot_id"));
}

fn denied() -> crate::verdict::Outcome {
    crate::verdict::Outcome::denied(crate::sys::EPERM)
}

/// Six machine legs, all clear. Synthetic, so the shape is asserted
/// without depending on whether this machine carries an emulator.
/// The accel leg carries the producer's list format, like a real
/// run: an `Ok` without a list promises no accelerator.
fn machine_ok() -> Findings {
    Findings {
        rows: crate::probes::MACHINE_LEGS
            .iter()
            .map(|&name| {
                if name == "qemu-system-x86_64 -accel help" {
                    (
                        name,
                        crate::verdict::Outcome::ok_with("accelerators: kvm tcg"),
                    )
                } else {
                    (name, crate::verdict::Outcome::ok())
                }
            })
            .collect(),
        ..Findings::empty()
    }
}

#[test]
fn the_document_carries_six_machine_legs_with_no_null_verdict() {
    // TODO/machine.md T-1301's Prove reads exactly this shape. All three
    // assessments' rows run, as a real run carries all of them, so no
    // leg anywhere is null.
    let mut f = machine_ok();
    f.rows.extend(supervise_ok().rows);
    f.rows.extend(supervision_ok().rows);
    let sel = Selection::choose(&f);
    let doc = document(&f, &sel);
    assert!(doc.contains("\"tiers\":{\"machine\":{\"legs\":["), "{doc}");
    for name in crate::probes::MACHINE_LEGS {
        assert!(doc.contains(&format!("\"name\":\"{name}\"")), "{doc}");
    }
    assert!(doc.contains("\"group\":\"machine\""), "{doc}");
    assert!(!doc.contains("\"verdict\":null"), "{doc}");
    assert!(doc.contains("\"refusal\":null"), "{doc}");
    assert!(doc.contains("\"profile\":\"full\""), "{doc}");
}

#[test]
fn a_document_without_machine_rows_carries_nulls_and_a_refusal() {
    // A document written before the legs existed: every leg null, and
    // the refusal names the absence rather than reading as a tier. Six
    // machine legs, three mediation legs and two supervision legs, so
    // eleven nulls, one refusal per assessment.
    let f = Findings::empty();
    let sel = Selection::choose(&f);
    let doc = document(&f, &sel);
    assert_eq!(doc.matches("\"verdict\":null").count(), 11, "{doc}");
    assert!(doc.contains("machine tier refused"), "{doc}");
    assert!(doc.contains("supervise tier refused"), "{doc}");
    assert!(doc.contains("supervision refused"), "{doc}");
    assert!(doc.contains("\"profile\":null"), "{doc}");
    let refusal = field(&doc, "refusal").expect("a refusal string");
    assert!(refusal.contains("machine tier refused"), "{refusal}");
    assert!(refusal.contains("(not probed)"), "{refusal}");
}

/// Three supervise legs, all clear. Synthetic, so the shape is asserted
/// without depending on whether this machine permits the tier.
fn supervise_ok() -> Findings {
    Findings {
        rows: crate::probes::SUPERVISE_LEGS
            .iter()
            .map(|&name| (name, crate::verdict::Outcome::ok()))
            .collect(),
        ..Findings::empty()
    }
}

/// Both supervision legs, all clear. Synthetic beside the mediation
/// fixture above.
fn supervision_ok() -> Findings {
    Findings {
        rows: crate::probes::SUPERVISION_LEGS
            .iter()
            .map(|&name| (name, crate::verdict::Outcome::ok()))
            .collect(),
        ..Findings::empty()
    }
}

/// The target shape: mediation denied, supervision holding. The
/// document refuses mediation in its own sentence and carries
/// supervision with a null refusal; the evidence states the
/// fallback instead of staying silent about it.
fn fallback_shape() -> Findings {
    let mut f = supervision_ok();
    f.rows.extend(
        [
            "seccomp(NEW_LISTENER) [supervise]",
            "process_vm_readv(own pid) [supervise]",
            "ptrace(PTRACE_TRACEME) [supervise]",
        ]
        .iter()
        .map(|&name| (name, crate::verdict::Outcome::denied(crate::sys::EPERM))),
    );
    f
}

#[test]
fn the_fallback_shape_refuses_mediation_beside_held_supervision() {
    let f = fallback_shape();
    let sel = Selection::choose(&f);
    let doc = document(&f, &sel);
    assert!(doc.contains("\"supervision\":{\"legs\":["), "{doc}");
    for name in crate::probes::SUPERVISION_LEGS {
        assert!(doc.contains(&format!("\"name\":\"{name}\"")), "{doc}");
    }
    assert!(doc.contains("supervise tier refused"), "{doc}");
    assert!(!doc.contains("supervision refused"), "{doc}");
    let e = evidence(&f, &sel);
    assert!(e.contains("supervision holds: every leg ok"), "{e}");
    assert!(e.contains("supervise tier refused"), "{e}");
}

#[test]
fn the_banner_states_mediation_off_where_supervision_covers() {
    // TODO/supervise.md T-0606: the fallback is stated, never silent.
    let f = fallback_shape();
    let sel = Selection::choose(&f);
    let b = entry_banner(&f, &sel, Rung::Chroot);
    assert!(b.contains("no syscall mediation on this machine"), "{b}");
    assert!(b.contains("pidfd and waitid"), "{b}");
}

#[test]
fn the_banner_states_no_fallback_where_mediation_holds() {
    // Where the tier holds there is no fallback to state: the line
    // stays off, so a mediated run never reads as a degraded one.
    let mut f = supervise_ok();
    f.rows.extend(supervision_ok().rows);
    let sel = Selection::choose(&f);
    let b = entry_banner(&f, &sel, Rung::Chroot);
    assert!(!b.contains("no syscall mediation"), "{b}");
}

#[test]
fn the_document_carries_three_supervise_legs_with_no_null_verdict() {
    // TODO/supervise.md T-0606's Prove reads exactly this shape: the
    // tier holds, so its refusal is null while the machine tier's is not
    // (its rows never ran in this fixture).
    let f = supervise_ok();
    let sel = Selection::choose(&f);
    let doc = document(&f, &sel);
    assert!(doc.contains("\"supervise\":{\"legs\":["), "{doc}");
    for name in crate::probes::SUPERVISE_LEGS {
        assert!(doc.contains(&format!("\"name\":\"{name}\"")), "{doc}");
    }
    assert!(doc.contains("\"group\":\"supervise\""), "{doc}");
    assert!(doc.contains("\"refusal\":null"), "{doc}");
}

#[test]
fn the_evidence_names_the_tcg_profile_where_kvm_is_denied() {
    // The lane shape: kvm absent, the emulator listing tcg. No
    // refusal prints; the profile and the denied leg do.
    let mut f = machine_ok();
    f.rows[1] = (
        "open(/dev/kvm, O_RDWR)",
        crate::verdict::Outcome::denied(crate::sys::ENOENT),
    );
    let sel = Selection::choose(&f);
    let e = evidence(&f, &sel);
    assert!(e.contains("machine, one leg per fact"), "{e}");
    assert!(e.contains("profile: tcg"), "{e}");
    assert!(e.contains("not hardware isolation"), "{e}");
    assert!(e.contains("open(/dev/kvm, O_RDWR)=ENOENT"), "{e}");
    assert!(!e.contains("machine tier refused"), "{e}");
    let doc = document(&f, &sel);
    assert!(doc.contains("\"profile\":\"tcg\""), "{doc}");
    assert!(doc.contains("\"refusal\":null"), "{doc}");
}

#[test]
fn the_evidence_refuses_where_no_accelerator_holds() {
    // Neither leg of acceleration: the node denied and tcg unlisted.
    // The refusal names both, in run order.
    let mut f = machine_ok();
    f.rows[1] = (
        "open(/dev/kvm, O_RDWR)",
        crate::verdict::Outcome::denied(crate::sys::ENOENT),
    );
    f.rows[5] = (
        "qemu-system-x86_64 -accel help",
        crate::verdict::Outcome::ok_with("accelerators: kvm"),
    );
    let sel = Selection::choose(&f);
    let e = evidence(&f, &sel);
    assert!(e.contains("machine tier refused"), "{e}");
    assert!(e.contains("open(/dev/kvm, O_RDWR)=ENOENT"), "{e}");
    assert!(e.contains("tcg not listed"), "{e}");
    let doc = document(&f, &sel);
    assert!(doc.contains("\"profile\":null"), "{doc}");
}

/// Every row T-1306 cites, all clear. Synthetic, so the shape is
/// asserted without depending on what this machine denies.
fn cited_ok() -> Findings {
    Findings {
        rows: [
            "bind(127.0.0.1:0)+listen",
            "open(/dev/kvm, O_RDWR)",
            "clone(CLONE_NEWUTS)",
            "unshare(CLONE_NEWUSER)",
            "unshare(CLONE_NEWNS)",
            "ptrace(PTRACE_TRACEME)",
            "setuid(1000)",
            "setgroups(0,NULL)",
            "prlimit(RLIMIT_FSIZE)",
        ]
        .iter()
        .map(|&name| (name, crate::verdict::Outcome::ok()))
        .collect(),
        ..Findings::empty()
    }
}

#[test]
fn the_document_carries_six_non_goal_stances() {
    // TODO/machine.md T-1306's Prove reads exactly this shape.
    let f = cited_ok();
    let sel = Selection::choose(&f);
    let doc = document(&f, &sel);
    assert!(doc.contains("\"non_goals\":[{"), "{doc}");
    for name in [
        "tcp listener",
        "kvm acceleration",
        "runc-style oci",
        "user-mode linux guest",
        "uid_map identity switch",
        "guest file over the ceiling",
    ] {
        assert!(doc.contains(&format!("\"name\":\"{name}\"")), "{doc}");
    }
}

#[test]
fn a_denied_kvm_names_its_errno_in_the_document_and_the_evidence() {
    let mut f = cited_ok();
    for (n, o) in f.rows.iter_mut() {
        if *n == "open(/dev/kvm, O_RDWR)" {
            *o = crate::verdict::Outcome::denied(crate::sys::ENOENT);
        }
    }
    let sel = Selection::choose(&f);
    let doc = document(&f, &sel);
    assert!(
        doc.contains("\"name\":\"kvm acceleration\",\"spec\":\"E-30/60\",\"stance\":\"refused\""),
        "{doc}"
    );
    assert!(doc.contains("open(/dev/kvm, O_RDWR)=ENOENT"), "{doc}");
    let e = evidence(&f, &sel);
    assert!(
        e.contains("non-goals, one stance per blocked design"),
        "{e}"
    );
    assert!(e.contains("open(/dev/kvm, O_RDWR)=ENOENT"), "{e}");
}

#[test]
fn the_chroot_banner_names_the_missing_procfs() {
    // TODO/complete.md T-0413 part 1, as emulated. The entered rung
    // mounts nothing, so the banner names the absent /proc, what the
    // interposer answers anyway, and what still fails before the
    // payload meets either.
    let f = Findings::empty();
    let sel = Selection::choose(&f);
    let b = entry_banner(&f, &sel, Rung::Chroot);
    assert!(b.contains("this mode does NOT provide:"), "{b}");
    assert!(b.contains("/proc"), "{b}");
    assert!(b.contains("emulates"), "{b}");
    assert!(b.contains("/proc/self/fd"), "{b}");
    assert!(b.contains("Interpose.Emulated.procfs"), "{b}");
}

#[test]
fn the_namespace_banner_names_no_procfs_mount() {
    // TODO/enter.md T-1339: the rung mounts a tmpfs on `/tmp` and no
    // /proc, so the emulated-procfs sentence holds and the never-claim
    // names the unmounted /proc beside the unisolated user, pid and
    // network. The mount topology itself reads mount-only.
    let f = Findings::empty();
    let sel = Selection::choose(&f);
    let b = entry_banner(&f, &sel, Rung::Namespace);
    assert!(b.contains("namespaces: mount-only"), "{b}");
    assert!(b.contains("tmpfs on /tmp"), "{b}");
    assert!(b.contains("network: host-shared"), "{b}");
    assert!(b.contains("pids: host-shared"), "{b}");
    assert!(b.contains("this mode does NOT provide:"), "{b}");
    assert!(b.contains("/proc"), "{b}");
    assert!(b.contains("emulates"), "{b}");
}

#[test]
fn two_clear_legs_mean_no_spawn_note() {
    let ok = crate::verdict::Outcome::ok();
    assert!(spawn_wall_note(Some(&ok), Some(&ok)).is_none());
    assert!(spawn_wall_note(None, None).is_none());
}

#[test]
fn a_denied_clone_names_the_clone_and_clears_setgroups() {
    let ok = crate::verdict::Outcome::ok();
    let note = spawn_wall_note(Some(&denied()), Some(&ok)).expect("a note prints");
    assert!(note.contains("clone flags"), "{note}");
    assert!(note.contains("Setgroups is clear"), "{note}");
    assert!(!note.contains("NoSetGroups"), "{note}");
}

#[test]
fn a_denied_setgroups_names_the_field_and_clears_the_clone() {
    let ok = crate::verdict::Outcome::ok();
    let note = spawn_wall_note(Some(&ok), Some(&denied())).expect("a note prints");
    assert!(note.contains("NoSetGroups"), "{note}");
    assert!(note.contains("credential"), "{note}");
    assert!(note.contains("clone is clear"), "{note}");
}

#[test]
fn two_denied_legs_name_both_and_tell_them_apart_by_shape() {
    let note = spawn_wall_note(Some(&denied()), Some(&denied())).expect("a note prints");
    assert!(note.contains("clone flags"), "{note}");
    assert!(note.contains("NoSetGroups"), "{note}");
    assert!(note.contains("denied EPERM"), "{note}");
}

#[test]
fn an_unmeasured_leg_is_never_attributed() {
    let note = spawn_wall_note(Some(&denied()), None).expect("a note prints");
    assert!(note.contains("not measured"), "{note}");
    assert!(note.contains("clone flags"), "{note}");
    assert!(!note.contains("Setgroups is clear"), "{note}");
}
