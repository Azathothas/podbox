use super::*;

/// `--json`: the same findings, for a harness to diff against
/// `experiments/results/`.
pub fn document(f: &Findings, sel: &Selection) -> String {
    let mut s = String::new();
    let mut o = json::Obj::new(&mut s);
    o.str("podvm", VERSION);
    o.str("rung", sel.rung.word());
    o.str("rung_must_never_claim", sel.rung.must_never_claim());
    // ⚠ The rung and only the rung: `controls_answered` is beside it and is a
    // statement about the diagnostic. See `Selection::exit_code`.
    o.bool("strict_ok", sel.meets(Selection::STRICT_FLOOR));
    o.bool("controls_answered", sel.controls_answered);
    o.str("self_exe", &f.self_exe);

    // ⛔ TODO/enter.md T-0506 point 5: WHICH INSTRUMENT ANSWERED, beside the
    // rung rather than at the end, because a harness that reads `rung` and not
    // this has read qemu's answer as the machine's.
    // ⚠ `emulated: false` is the absence of evidence and says so in `checked`;
    // it is not a claim that this is a bare machine.
    let interp = crate::interpreter();
    // ⭐ TODO/enter.md T-1652: whether the native-static rung can arm here.
    let dispatch = podvm_dispatch::available();
    o.obj("measured_by", |m| {
        m.bool("user_dispatch", dispatch.is_ok());
        if let Err(why) = &dispatch {
            m.str("user_dispatch_refused", why);
        }
        m.bool("emulated", interp.is_emulated());
        m.str("interpreter", &interp.key());
        match &interp {
            crate::interp::Interpreter::Emulated { evidence, .. } => {
                m.str("evidence", evidence);
            }
            crate::interp::Interpreter::NoEvidence { checked } => {
                m.arr("checked", |a| {
                    for c in checked {
                        a.str(c);
                    }
                });
            }
        }
    });

    o.arr("control_notes", |a| {
        for n in &sel.control_notes {
            a.str(n);
        }
    });

    // ⭐ TODO/probe.md T-0102's shape: the control's answer, by name.
    o.obj("controls", |c| {
        for name in crate::select::control_rows() {
            let key = name.split_once('(').map(|(k, _)| k).unwrap_or(name);
            match f.get(name) {
                Some(x) if x.verdict == Verdict::Denied => {
                    c.opt_str(key, x.errno_name().as_deref());
                }
                Some(x) if x.verdict == Verdict::Ok => {
                    c.str(key, "ok");
                }
                // ⛔ A control that could not run is null, never the errno of
                // whatever stopped it: that would read as an answer.
                _ => {
                    c.null(key);
                }
            }
        }
    });

    // ⭐ TODO/enter.md T-0503's `has("ptmx")`. Derived from the two rows that
    // ran, never a second pair of calls: `usable` is the OPEN, not the stat,
    // because a device node that exists and cannot be opened is exactly the
    // degraded `-t` the entry refuses.
    let ptmx_stat = f.get("stat(/dev/ptmx)");
    let ptmx_open = f.get("open(/dev/ptmx, O_RDWR)");
    o.obj("ptmx", |x| {
        x.str("path", crate::probes::PTMX);
        x.bool(
            "present",
            ptmx_stat.map(|s| s.verdict == Verdict::Ok) == Some(true),
        );
        x.bool(
            "usable",
            ptmx_open.map(|s| s.verdict == Verdict::Ok) == Some(true),
        );
        x.opt_str("stat_verdict", ptmx_stat.map(|s| s.verdict.word()));
        x.opt_str("stat_detail", ptmx_stat.map(|s| s.reason.as_str()));
        x.opt_str(
            "stat_errno_name",
            ptmx_stat.and_then(|s| s.errno_name()).as_deref(),
        );
        x.opt_str("open_verdict", ptmx_open.map(|s| s.verdict.word()));
        x.opt_str(
            "open_errno_name",
            ptmx_open.and_then(|s| s.errno_name()).as_deref(),
        );
    });

    let m = Mounts::of(f);
    o.obj("mounts", |mo| {
        for (half, out) in m.rows() {
            mo.str(half, out.verdict.word());
        }
        mo.str("summary", &m.summary());
        mo.opt_str("capability_note", m.capability_note());
    });

    o.obj("identity", |i| {
        i.num("uid", f.identity.uid);
        i.num("gid", f.identity.gid);
        i.arr("groups", |a| {
            for g in &f.identity.groups {
                a.num(*g as i64);
            }
        });
        i.opt_str("uid_map", f.identity.uid_map.as_deref());
        i.opt_str("gid_map", f.identity.gid_map.as_deref());
        i.opt_str("setgroups", f.identity.setgroups.as_deref());
        i.opt_str("cap_eff", f.identity.cap_eff.as_deref());
        i.opt_str("cap_prm", f.identity.cap_prm.as_deref());
        i.opt_str("cap_bnd", f.identity.cap_bnd.as_deref());
        i.opt_str("seccomp", f.identity.seccomp.as_deref());
        i.opt_str("seccomp_filters", f.identity.seccomp_filters.as_deref());
        i.opt_str("no_new_privs", f.identity.no_new_privs.as_deref());
        i.opt_str("threads", f.identity.threads.as_deref());
        i.arr("unreadable", |a| {
            for u in &f.identity.unreadable {
                a.str(u);
            }
        });
    });

    o.arr("writable", |a| {
        for w in &f.writable {
            a.obj(|e| {
                e.str("path", &w.path);
                e.str("source", &w.source);
                e.bool("writable", w.writable());
                e.str("verdict", w.outcome.verdict.word());
                e.opt_num("errno", w.outcome.errno.map(|x| x.0 as i64));
                e.opt_str("errno_name", w.outcome.errno_name().as_deref());
                e.opt_unum("blocks_free", w.space.as_ref().map(|s| s.blocks_free));
                e.opt_num("block_size", w.space.as_ref().map(|s| s.block_size));
                e.opt_unum("inodes_free", w.space.as_ref().map(|s| s.inodes_free));
                e.opt_str("reason", none_if_empty(&w.outcome.reason));
            });
        }
    });

    o.arr("probes", |a| {
        for (name, out) in &f.rows {
            a.obj(|e| {
                e.str("name", name);
                e.str("verdict", out.verdict.word());
                e.opt_num("errno", out.errno.map(|x| x.0 as i64));
                e.opt_str("errno_name", out.errno_name().as_deref());
                e.opt_str("reason", none_if_empty(&out.reason));
                e.str(
                    "group",
                    match crate::probes::find(name).map(|p| p.group) {
                        Some(crate::probes::Group::Attribution) => "attribution",
                        Some(crate::probes::Group::Machine) => "machine",
                        Some(crate::probes::Group::Supervise) => "supervise",
                        Some(crate::probes::Group::Census) | None => "census",
                    },
                );
            });
        }
    });

    // ⭐ TODO/machine.md T-1301. The machine tier's legs, one per fact, each
    // with the verdict the run established, and the profile they
    // establish (`full`, `tcg`, or null where no profile runs). A leg
    // whose row is absent -- a document written before the legs existed
    // -- carries a null verdict, which is missing rather than any
    // verdict, so a stale cache cannot read as a measured tier.
    let mach = crate::machine::assess(f);
    let refusal = mach.refusal();
    let profile = mach.profile.map(|p| p.word());
    // ⭐ TODO/supervise.md T-0606, assessed beside the machine tier so the
    // document carries one `tiers` object and not two writers of one key.
    let sup = crate::supervise::assess(f);
    let sup_refusal = sup.refusal();
    o.obj("tiers", |t| {
        t.obj("machine", |m| {
            m.arr("legs", |a| {
                for leg in &mach.legs {
                    a.obj(|e| {
                        e.str("name", leg.name);
                        match &leg.outcome {
                            Some(out) => {
                                e.str("verdict", out.verdict.word());
                                e.opt_num("errno", out.errno.map(|x| x.0 as i64));
                                e.opt_str("errno_name", out.errno_name().as_deref());
                                e.opt_str("reason", none_if_empty(&out.reason));
                            }
                            None => {
                                e.null("verdict");
                                e.null("errno");
                                e.null("errno_name");
                                e.null("reason");
                            }
                        }
                    });
                }
            });
            m.opt_str("refusal", refusal.as_deref());
            m.opt_str("profile", profile);
        });
        t.obj("supervise", |m| {
            m.arr("legs", |a| {
                for leg in &sup.legs {
                    a.obj(|e| {
                        e.str("name", leg.name);
                        match &leg.outcome {
                            Some(out) => {
                                e.str("verdict", out.verdict.word());
                                e.opt_num("errno", out.errno.map(|x| x.0 as i64));
                                e.opt_str("errno_name", out.errno_name().as_deref());
                                e.opt_str("reason", none_if_empty(&out.reason));
                            }
                            None => {
                                e.null("verdict");
                                e.null("errno");
                                e.null("errno_name");
                                e.null("reason");
                            }
                        }
                    });
                }
            });
            m.opt_str("refusal", sup_refusal.as_deref());
        });
        // TODO/supervise.md T-0606. Supervision beside mediation: the
        // pair the lifecycle runs on, available where both hold even as
        // mediation is refused. Same legs-and-refusal shape, so one
        // reader serves both assessments.
        let vis = crate::supervise::assess_supervision(f);
        let vis_refusal = vis.refusal();
        t.obj("supervision", |m| {
            m.arr("legs", |a| {
                for leg in &vis.legs {
                    a.obj(|e| {
                        e.str("name", leg.name);
                        match &leg.outcome {
                            Some(out) => {
                                e.str("verdict", out.verdict.word());
                                e.opt_num("errno", out.errno.map(|x| x.0 as i64));
                                e.opt_str("errno_name", out.errno_name().as_deref());
                                e.opt_str("reason", none_if_empty(&out.reason));
                            }
                            None => {
                                e.null("verdict");
                                e.null("errno");
                                e.null("errno_name");
                                e.null("reason");
                            }
                        }
                    });
                }
            });
            m.opt_str("refusal", vis_refusal.as_deref());
        });
    });

    // ⭐ TODO/machine.md T-1306. The non-goals as measured stances, one per
    // blocked design. A stance is per run, never a constant: refused where
    // the mechanism is denied, open where it works here, unestablished
    // where the rows cannot say. The per-leg verdicts also live in
    // `probes` above; the agreement between the two is held by
    // `nongoals`' own tests, which assert the detail names the rows'
    // errnos.
    let goals = crate::nongoals::assess(f);
    o.arr("non_goals", |a| {
        for g in &goals {
            a.obj(|e| {
                e.str("name", g.name);
                e.str("spec", g.spec);
                e.str("stance", crate::nongoals::word(g.stance));
                e.str("detail", &g.detail);
                e.str("remedy", g.remedy);
            });
        }
    });

    // ⭐ TODO/probe.md T-0111. The cache stores this document verbatim, so the
    // key it is validated against travels inside it and there is no second
    // serializer to drift against the first.
    //
    // ⚠ Seven of the ten components are also in the `identity` block above.
    // `docs/conventions/forbidden-patterns.md` forbids a value in two places
    // WITH NO CHECK THAT THEY AGREE, and the check is
    // `the_cache_key_agrees_with_the_identity_block_it_is_derived_from` below.
    let key = crate::confinement_key(&f.identity);
    o.obj("cache_key", |k| {
        for (name, value) in key.components() {
            k.opt_str(name, value);
        }
        k.bool("complete", key.is_complete());
    });

    o.arr("rejected", |a| {
        for r in &sel.rejected {
            a.obj(|e| {
                e.str("rung", r.rung.word());
                e.str("requirement", r.requirement);
                e.str("evidence", &r.evidence);
            });
        }
    });

    o.end();
    s.push('\n');
    s
}
