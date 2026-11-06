# Meta 1: the test surface, checked from the implementation side

## Meta 1 scope and method

Round 1 and round 2 read the 121 scripts. This round reads the Rust tree and
asks a different question: does each proposed conversion fit the code it would
touch. A conversion plan is a claim about a crate, a file, a command and a
dependency. Each of those four is checkable. Each was checked.

**What I read, in full.**

- The root `Cargo.toml` and all ten `crates/*/Cargo.toml` files.
- Every `#[cfg(test)] mod` in every `crates/*/src/**/*.rs` and in
  `crates/podbox-ssh/tests/*.rs`, read at the site rather than by name.
- `crates/podbox-ssh/tests/common.rs` (205 lines) in full, plus
  `ssh_over_unix_e2e.rs` (150) and `session_interactive.rs` (484) in full, and
  the head of `mux_two_client.rs` (1507 lines, first 120 read in full and the
  rest searched by symbol).
- `docs/conventions/code.md` (36 lines) in full, and the Verification section
  that carries the three test kinds.
- `TODO/INDEX.md` in full, `TODO/deps.md` T-0901 through T-0912,
  `TODO/interpose.md` around line 236, `.github/workflows/gate.yml`,
  `scripts/plant.sh` lines 45-60, `scripts/check-todo.py` lines 225-232 and
  296-304.
- All ten group reports' verdict lines and every RUST-TEST, RUST-TOOL and
  SPLIT conversion plan, plus `refactor/03-peer-review-2/round-2.md` in full.

**What I counted, and with what command.**

| Quantity | Command | Result |
| --- | --- | --- |
| `#[cfg(test)]` modules, per crate | `grep -rn --include='*.rs' '#\[cfg(test)\]' crates/podbox-$c \| grep -v /target/ \| wc -l` | 118 attributes, 117 real module headers, 1 false positive |
| `#[test]` / `#[tokio::test]`, per crate | same, with `#[test]` and `#[tokio::test]` | 980 total, 0 async |
| `#[test]` under `src/` only | `grep -rn --include='*.rs' -c '#\[test\]' crates/*/src crates/*/build.rs \| awk -F: '{s+=$2}'` | **950** |
| `#[test]` under `tests/` | `grep -rn --include='*.rs' -c '#\[test\]' crates/*/tests \| awk -F: '{s+=$2}'` | **30** |
| `tests/` directories in the tree | `find crates -type d -name tests -not -path '*/target/*'` | **1**: `crates/podbox-ssh/tests` |
| `crates/podbox-interpose` `#[test]` | `grep -rc '#\[test\]' crates/podbox-interpose/src/*.rs \| awk -F: '{s+=$2}'` | **48** |
| Parity table rows | `awk 'NR>=232 && NR<=537' crates/podbox-cli/src/parity.rs \| grep -c 'Row {'` and the same range with `grep -c 'verb: "'` | **265 both ways** |
| Verdict distribution per group | read each `**Verdict:` / `### Verdict:` line in file order, mapped to the preceding `## ` heading | see the counts section |

The `#[cfg(test)]` count needs one correction stated plainly. 118 attributes
were found, and 117 are followed by `mod tests {`. The 118th is
`crates/podbox-image/src/store.rs:1102`, which is the call
`tests::note_the_refusal(path, fd);` inside an existing module, not a module
header. The 117 real headers are all `mod tests {` except one,
`crates/podbox-extract/src/lib.rs:40`, which reads `mod drive;` and pulls in
the crate's acceptance module. **So 117 `#[cfg(test)]` module declarations
exist, and 116 are named `tests`.**

**What I could not settle.**

- No build. `cargo test` fails on this host at `error: linker 'cc' not found`,
  which round 2 recorded as O4 and which I did not retry. Every claim about
  what compiles is a claim about what I read, not about what a compiler
  accepted.
- Whether a `cdylib`-only crate can host a `tests/` directory is settled by
  cargo's own rule rather than by an observed run here. `crates/podbox-interpose`
  declares `crate-type = ["cdylib"]` and no `rlib`. Cargo builds an integration
  test by linking the crate's **library target**, and a `cdylib` is not linkable
  that way. I did not run cargo to watch it fail; the rule is what makes the
  proposals wrong, and the proposals are wrong either way if I am right.
- `TODO/PROGRESS.md` was not read. Nothing here depends on the work order.
- The bodies of 4 of the 121 scripts were not read. They are named where a
  plan is checked and no claim about them is made beyond what the reports and
  the entries say.

---

## Meta 1 — the real test surface

| crate | src files | `#[cfg(test)]` modules | `#[test]` fns | idiom | helper surface |
| --- | --- | --- | --- | --- | --- |
| `podbox-probe` | 17 | 17 | 108 | inline `mod tests` only | `findings(rows)` / `all_ok()` in `nongoals.rs:232`; `Outcome::ok()` / `::denied(sys::EPERM)` constructors from `verdict.rs`; `crate::sys` errno constants |
| `podbox-image` | 19 | 18 | 150 | inline; plus `mod drive`-style modules | `scratch(name)` and `record(...)` in `store.rs`; `Scratch::new` / `layer(&[E::…])` in `extract`; `rootfs_tar()` in `layout.rs:433`; `serve_once(body)` one-shot `TcpListener` at `registry.rs:955`; `STORE_TESTS` mutex for serialised store tests |
| `podbox-enter` | 11 | 10 | 120 | inline `mod tests` at `abi.rs:1074` | `fake()` / `fake_import()` / `tiny_elf()` at the end of `abi.rs`; `std::env::temp_dir()` + pid at `abi.rs:1094` and `:1364`; `PODBOX_TEST_NOBODY_ROOT` / `PODBOX_TEST_NOBODY_GUEST` orchestrator env pair |
| `podbox-extract` | 9 | 7 | 47 | inline; `#[cfg(test)] mod drive;` at `lib.rs:40` pulls the whole acceptance module in | `Scratch::new(name)` and `layer(&[E::…])` with `E::File` / `E::Symlink` / `E::Hardlink` / `E::Dir` / `E::Owned` constructors; hand-built layers via `std::os::unix::fs::symlink` |
| `podbox-complete` | 9 | 8 | 51 | inline `mod tests` at `identity.rs:320`, `write.rs:639` | `scratch(name)` creating `etc/` at `identity.rs:323` and `write.rs:643`; temp rootfs with planted symlinks |
| `podbox-supervise` | 3 | 3 | 14 | inline | reads its own source through `env!("CARGO_MANIFEST_DIR")` to assert no thread spawns on the spawn path |
| `podbox-interpose` | 9 | 7 | 48 | inline; `mod tests` in `identity.rs:282`, `device.rs:251`, `memo.rs:281`, `lib.rs:2896` | `table()` at `device.rs:255`; the crate builds for **no** target other than itself and has no dev-dependencies |
| `podbox-ssh` | 12 + 4 bins | 13 | 114 src + 30 `tests/` | **the only crate with `tests/`** | `crates/podbox-ssh/tests/common.rs`: `require_binary` (28), `fresh_tmp` (48), `make_keys` (68), `current_user` (89), `ensure_privsep` (103), `run_ssh` (112), `collect` (155), `wait_for` (194); plus `Proc` and `Term` in each suite and `env!("CARGO_BIN_EXE_*")` at 9 sites |
| `podbox-cli` | 31 | 28 | 244 | inline `mod tests` in 29 files; no `tests/` | `PODBOX_STORE` set and removed at `complete.rs:784,806`; `PODBOX_QEMU` / `PODBOX_OVMF_*` at `windows/plan.rs:191-221`; `std::env::temp_dir()` in 13 files |
| `podbox-windows` | 7 | 7 | 54 | inline | `PODBOX_DOS_BASE` / `PODBOX_WINDOWS_BASE` at `dos.rs:330-336`; `PODBOX_WINDOWS_KEEP` at `lib.rs:589-678` |
| **total** | **137** | **117** | **980** | | |

**The three conventions a conversion must follow.**

1. **Tests are inline, not in `tests/`.** 950 of 980 test functions live in
   `#[cfg(test)] mod tests` beside the code. `crates/podbox-ssh/tests/` holds
   30 and is the only directory, and it exists because those tests spawn real
   `sshd`, `ssh` and `ssh-keygen` binaries. A plan that adds a `tests/` file to
   a crate that has none is proposing a structural change, and must say so.
2. **There are no dev-dependencies anywhere.** `grep -rn 'dev-dependencies'
   crates/*/Cargo.toml Cargo.toml` returns nothing. `tempfile` appears in no
   manifest and no lock entry. Every temp-dir fixture in the tree is hand-rolled
   `std::env::temp_dir()` with a pid or counter suffix: 55 sites across 38 files.
   Any plan that says "add a `tempfile::TempDir`" is adding a dependency the
   project never took. Group-1 says this correctly and then proposes it anyway
   at `group-1.md:661` and `:668`.
3. **Environment variables are set in-process, not by a shell.** 23 `set_var` /
   `remove_var` sites across four crates, always paired so a failure cannot
   leak a value into a sibling test. `PODBOX_STORE` is the only one that names a
   store path; `PODBOX_TEST_NOBODY_ROOT` / `_GUEST` is the only orchestrator
   pair, and the test that reads them **returns early** when they are absent
   (`abi.rs:1150-1152`) while the worker half **panics** rather than passing
   vacuously (`abi.rs:1156-1159`).

**The fixture surface that exists and can be reused.**

- Real external binaries: only in `podbox-ssh/tests/`, via `require_binary`,
  which **panics naming the binary** rather than skipping (`common.rs:41`).
- Real external processes: `Proc` in `session_interactive.rs:107` and
  `mux_two_client.rs:48`, both kill-and-reap on drop.
- A fake relay: `mux_two_client.rs` lines 120-1507, a high-fidelity `reverse-v1`
  mock on loopback TCP.
- A one-shot HTTP origin: `registry.rs:955` `serve_once`, plain HTTP/1.0, not
  HTTPS and not an OCI registry.
- **No OCI registry fixture exists in Rust.** T-0206's registry fixture is the
  shell script `experiments/180-registry-fixture.sh` driving a downloaded `zot`
  binary, per `TODO/image.md:483-485`. There is no Rust equivalent in any crate.

---

## Meta 1 — conversions already done

Every existing Rust test whose doc comment or comment names an experiment file,
with its line. This is the section that changes the plan most, because most of
it is work the reports did not know about.

### Already done, and the reports name it

| test | file:line | script whose clause it carries |
| --- | --- | --- |
| `the_soname_discriminator_is_the_one_that_was_measured` | `crates/podbox-enter/src/abi.rs:1170` | `80-interposer-abi.sh` check A. Doc comment at `:1168` names `experiments/results/interposer-abi.txt` |
| `a_shipped_libc_defines_its_symbols_in_dynsym` | `crates/podbox-enter/src/abi.rs:1220` | `80` check D. Doc comment at `:1212-1214` states the `.symtab` trap and the 0 / 3136 counts. Skip at `:1228-1231` is the honest third state |
| `an_import_at_a_version_the_libc_does_not_declare_is_refused` | `crates/podbox-enter/src/abi.rs:1250` | `80` check C. Doc comment at `:1246` |
| `a_glibc_object_against_a_musl_rootfs_is_refused_in_those_words` | `crates/podbox-enter/src/abi.rs:1274` | `80` check E, glibc arm |
| `a_matching_object_and_libc_are_admitted` | `crates/podbox-enter/src/abi.rs:1293` | `80` check E, the control |
| `this_binarys_own_header_parses` | `crates/podbox-enter/src/abi.rs:1199` | the reader `80` check D depends on |
| `an_import_from_the_runtimes_beside_libc_admits_against_their_union` | `crates/podbox-enter/src/abi.rs:1315` | the `libgcc_s` `_Unwind_Resume@GCC_3.0` case of `80` |

`80-interposer-abi.sh` is therefore **half converted already**: 4 of 5 checks
have predicates in `abi.rs`. Group-9's plan says so and is right.

| test | file:line | script |
| --- | --- | --- |
| `prompt_echo_command_and_no_terminal` | `crates/podbox-ssh/tests/session_interactive.rs:255` | `388-interactive-shell.sh` clause 3 |
| `editing_repairs_a_typo` | `:276` | clause 4 |
| `history_recalls_the_previous_command` | `:288` | clause 5 |
| `state_persists_across_commands` | `:304` | clause 6 |
| `signal_kills_the_command_not_the_shell` | `:323` | clause 7 |
| `signal_without_trap_ends_the_session_with_130` | `:351` | clause 11 |
| `shell_exit_code_passes_through` | `:374` | clause 8 |
| `one_shot_command_is_refused_naming_the_variable` | `:389` | clause 9 |
| `subsystem_sftp_never_reaches_the_shell` | `:410` | clause 10 |
| `ctrl_d_on_empty_line_ends_the_session` | `:431` | not in the script |
| `client_eof_submits_the_partial_line_then_ends` | `:450` | not in the script |
| `large_output_does_not_wedge_the_session` | `:472` | not in the script |

**12 test functions, 9 script clauses, 3 added.** Group-9 counts 12 and is
right. The module comment at `:12-20` carries the same needle rule the script
carries at `388:12-14`. `388-interactive-shell.sh` is **fully converted**.

### Already done, and the reports under-count it

| test | file:line | script whose clause it carries |
| --- | --- | --- |
| `six_non_goals_come_back_in_order` | `crates/podbox-probe/src/nongoals.rs:253` | `149-podvm-non-goals.sh` clause 0 |
| `a_denied_mechanism_refuses_naming_its_errno` | `:261` | clause 1 |
| `a_permitted_mechanism_turns_the_non_goal_off` | `:280` | clause 2 |
| `a_missing_row_is_unestablished_rather_than_any_stance` | `:290` | clause 3 |
| `the_ceiling_row_names_its_number_or_says_there_is_none` | `:302` | clause 0, ceiling row |
| `the_document_carries_six_non_goal_stances` | `crates/podbox-probe/src/report.rs:1069` | `149` clause 0, on the rendered document |
| `a_denied_kvm_names_its_errno_in_the_document_and_the_evidence` | `crates/podbox-probe/src/report.rs:1088` | `149` clause 4 **and** clause 5. It asserts `open(/dev/kvm, O_RDWR)=ENOENT` in both the document and the evidence, and the evidence contains `non-goals, one stance per blocked design` at `:1105-1107` |

Group-9's `149` plan says clause 5 "becomes a pure test on the report renderer"
and proposes it as new work at `crates/podbox-probe/src/report.rs`. **It is
already written**, and at `:1105-1107` it asserts the exact string
`149:96` requires. The plan's only remaining gap for `149` is the six row names
pinned as a constant, which the plan names correctly.

| test | file:line | script |
| --- | --- | --- |
| `the_first_run_measures_and_the_second_is_served_from_the_cache` | `crates/podbox-image/src/probe_cache.rs:233` | `170-probe-cache.sh` clause 2 |
| `a_cache_written_under_another_mount_namespace_is_refused_and_says_which` | `:247` | clause 3 |
| `a_cache_keyed_on_the_boot_id_alone_would_not_have_caught_that` | `:268` | clause 3, the failure control |
| `an_answer_taken_by_another_instrument_is_never_served` | `:296` | clause 3, the instrument case |

Group-6's `170` plan names all four at exactly these lines. **Correct.**

| test | file:line | script |
| --- | --- | --- |
| `every_measured_passwd_shape_ends_up_naming_files_first` | `crates/podbox-complete/src/identity.rs:336` | `90-nsswitch-contract.sh` check A, and `125-across-distributions.sh`'s six measured shapes. Doc comment at `:332-334` names the experiment |

`TODO/complete.md:659` names this test, and group-9's `90` plan names it. Check
A of `90` is already in Rust.

| test | file:line | script |
| --- | --- | --- |
| `a_shared_blob_survives_removing_one_of_the_two_images_that_reach_it` | `crates/podbox-image/src/store.rs:1950` | `160-store-gc.sh` clause 1 and clause 4 |
| `an_image_a_container_holds_is_refused_by_rmi_and_skipped_by_prune` | `crates/podbox-image/src/store.rs:1976` | `160` clauses 2 and 3 |
| `a_symlink_pointing_out_of_the_store_is_refused` | `crates/podbox-image/src/contain.rs:118` | `160` **clause 5**, the whole clause. Doc comment at `:119-120` says the string check "passes this and deletes /etc" |
| `the_t_0215_instrument_sees_a_lock_that_is_held` | `crates/podbox-image/src/store.rs:1629` | `153-store-lock-race.sh` clause A. Doc comment at `:1615-1627` names `experiments/153-store-lock-race.sh` and its `--nocapture` discipline |
| `a_fork_while_the_lock_is_held_does_not_extend_it` | `crates/podbox-image/src/store.rs:2019` | `153` clause C, one of the four skip names |
| `a_spawned_process_does_not_inherit_the_lock` | `crates/podbox-image/src/store.rs:2114` | `153`, the other skip name |
| `a_lock_handed_to_the_payload_outlives_this_process_dropping_it` | `crates/podbox-image/src/store.rs:2293` | `153` clause E's plant target |
| `releasing_a_lock_frees_it_even_while_a_duplicate_descriptor_lives` | `crates/podbox-image/src/store.rs:2369` | `153` clause D's plant target |

Group-2's `153` plan names `store.rs:2019`, `:2114` and the two plants at their
**exact lines**. Every one of the four skip names and both plant targets is
correct. `160-store-gc.sh` is 4 of 5 clauses already in Rust.

| test | file:line | script |
| --- | --- | --- |
| `the_walk_and_openat2_refuse_the_same_traversal` | `crates/podbox-extract/src/safety.rs:583` | `220-extract-path-safety.sh` check A |
| `a_hard_link_out_of_the_destination_is_refused` | `crates/podbox-extract/src/drive.rs:382` | `220` check D |
| `a_root_level_whiteout_is_found_and_a_glob_would_miss_it` | `crates/podbox-extract/src/whiteout.rs:89` | `70-whiteout-contract.sh` check D. Doc comment at `:84-88` names the script |
| `ordinary_layer_paths_survive` | `crates/podbox-extract/src/safety.rs:530` | `70` check A. Doc comment at `:525-528` names `whiteout-contract.txt` check A |
| `absolute_and_dotdot_are_refused_lexically` | `crates/podbox-extract/src/safety.rs:508` | `220` checks B and C |
| `absolute_symlink_target_is_rebased_not_refused` | `crates/podbox-extract/src/safety.rs:543` | `70` check C, and `220` check E |
| `relative_symlink_target_resolves_from_its_own_directory` | `crates/podbox-extract/src/safety.rs:549` | `220` check E, the legitimate-link control |
| `a_layer_may_delete_a_path_and_recreate_it_in_the_same_layer` | `crates/podbox-extract/src/drive.rs:352` | `70`, the `TODO/extract.md:399` layer-ordering half |
| `the_sidecar_does_not_claim_an_ownership_the_kernel_did_not_apply` | `crates/podbox-extract/src/drive.rs:480` | `70` check B |
| `a_write_through_an_absolute_symlink_does_not_escape` | `crates/podbox-complete/src/write.rs:653` | `85-completion-symlink-escape.sh` doors 1 and 2 |
| `a_write_through_a_symlinked_directory_is_refused` | `crates/podbox-complete/src/write.rs:678` | `85` door 3 |

Group-7's `220` table cites `safety.rs:508`, `:543`, `:549` and `drive.rs:366`,
`:382` — **all correct at the line I checked**. `220` is 5 of 6 checks done.
Group-4's `70` plan is 4 of 4 checks done as predicates, so the whole RUST-TEST
half of `70` is deletion plus a record.

| test | file:line | script |
| --- | --- | --- |
| `fchmodat_forwards_flags` | `crates/podbox-interpose/src/lib.rs:2955` | `162-tar-symlink-modes.sh`. Doc comment at `:2949-2953` names T-1311 |
| `wall_errnos_carry_their_names` | `crates/podbox-interpose/src/lib.rs:3114` | `106-interpose-identity.sh` clause G |
| `the_fallback_shape_refuses_mediation_beside_held_supervision` | `crates/podbox-probe/src/report.rs:951` | `359-supervision-split.sh`, the denied-notify shape the script's own header says no lane can show |
| `the_banner_states_mediation_off_where_supervision_covers` | `crates/podbox-probe/src/report.rs:967` | `359` clause 4, banner selection |
| `the_banner_states_no_fallback_where_mediation_holds` | `crates/podbox-probe/src/report.rs:977` | `359` clause 4, the negative |
| `three_clear_legs_hold_the_tier` | `crates/podbox-probe/src/supervise.rs:158` | `359` clause A |
| `denied_notify_legs_refuse_mediation_beside_available_supervision` | `crates/podbox-probe/src/supervise.rs:285` | `359` clauses A and B, the refusal biconditional |
| `two_clear_legs_hold_supervision` | `crates/podbox-probe/src/supervise.rs:253` | `359` clause B |
| `three_clear_legs_hold_the_tier`'s siblings `a_denied_leg_refuses_and_names_its_errno` | `crates/podbox-probe/src/supervise.rs:167` | `359` clause A, the refused arm |
| `a_sigkilled_payload_is_137` | `crates/podbox-probe/src/exit.rs:131` | `330-exit-codes.sh` |
| `every_code_has_a_row_and_every_row_parses` | `crates/podbox-probe/src/exit.rs:139` | `330` section 0 |
| `the_two_names_for_125_are_one_number` | `crates/podbox-probe/src/exit.rs:163` | `330` section 0 |
| `nothing_on_the_spawn_path_can_spawn_a_thread` | `crates/podbox-supervise/src/launcher.rs:662` | `340-detached-stdio.sh`. Doc comment at `:659` names `230-lifecycle-loop.sh` clause 3 |
| `find_matches_exact_spelling` … `answer_serves_creation_as_the_node_itself` | `crates/podbox-interpose/src/device.rs:260,270,284,292,315,333` | `363-device-map.sh` clauses 1-6, the six-row opener matrix |
| `the_soname_discriminator…` family in `abi.rs` | `crates/podbox-enter/src/abi.rs:1074-1360` | `60-interposer-libc.sh` and `105-interpose-ownership.sh` check F |

**Already done, by source reference rather than by comment.** These carry an
experiment number in a doc comment with no test function of the same name; each
one is a live reference the plan must not break.

| site | file:line | names |
| --- | --- | --- |
| crate contract | `crates/podbox-image/src/store.rs:36` | `experiments/210-store-concurrency.sh` |
| lock instrument | `crates/podbox-image/src/store.rs:1626` | `experiments/153-store-lock-race.sh` |
| lock comment | `crates/podbox-image/src/store.rs:1260,1461` | `153` |
| pull concurrency | `crates/podbox-image/src/pull.rs:530` | `experiments/190-parallel-layers.sh` |
| insecure registry | `crates/podbox-image/src/pull.rs:888` | `experiments/280-insecure-registry.sh` |
| syscalls | `crates/podbox-probe/src/sys.rs:131,617,692,1776,1815` | `experiments/260-multiarch.sh` |
| attribution rows | `crates/podbox-probe/src/probes.rs:1532` | `experiments/results/attribute.txt` |
| unmapped ids | `crates/podbox-probe/src/probes.rs:776` | `experiments/20-enter-target.sh` |
| distro sweep | `crates/podbox-image/src/registry.rs:332` | `experiments/240-distro-sweep.sh` |
| distroless | `crates/podbox-complete/src/identity.rs:12,26,37,215,332` | `experiments/125-across-distributions.sh` |
| symlink escape | `crates/podbox-complete/src/write.rs:25,82,491` | `85-completion-symlink-escape.sh`, `240` |
| binfmt magic | `crates/podbox-enter/src/binfmt.rs:16,149`, `crates/podbox-probe/src/binfmt.rs:204` | `experiments/260-multiarch.sh` |
| exit 137 | `crates/podbox-probe/src/exit.rs:17,56,59,127` | `experiments/330-exit-codes.sh` |
| probe parity | `crates/podbox-probe/src/sys.rs:1776` | `experiments/130-probe-parity.sh` |
| build embedding | `crates/podbox-cli/build.rs:10` | `experiments/158-interpose-embedding.sh` |
| detached stdio | `crates/podbox-supervise/src/launcher.rs:241` | `experiments/340-detached-stdio.sh` |
| lifecycle | `crates/podbox-supervise/src/launcher.rs:659`, `crates/podbox-supervise/src/lib.rs:194` | `230-lifecycle-loop.sh` |
| memo var | `crates/podbox-supervise/src/table.rs:185` | `experiments/105-interpose-ownership.sh` |
| identity var | `crates/podbox-cli/src/interpose.rs:311,320,860` | `106`, `105` |
| interpose build | `crates/podbox-cli/src/interpose.rs:9` | `experiments/results/interposer-abi.txt` |
| Windows lane | `crates/podbox-cli/src/windows/doctor.rs:63`, `src/windows/dos.rs:61` | `369-windows-tcg-dos.sh` |
| machine ssh | `crates/podbox-cli/src/machine/ssh.rs:230` | `experiments/147-podvm-exec.sh` |
| fleet | `crates/podbox-cli/src/tier.rs:127` | `experiments/148-podvm-fleet.sh` |
| mux | `crates/podbox-ssh/src/mux.rs:83,467` | `experiments/results/mux-two-client.txt` |
| mux drive | `crates/podbox-ssh/tests/mux_two_client.rs:18` | `experiments/387-mux-two-client.sh` |

**Count of conversions already done: 8 scripts have a named Rust test for a
majority of their clauses, and 4 more have a named test for a specific clause.**
The eight are `80`, `388`, `149`, `90` (check A), `170` (clauses 2-3), `160`
(all five clauses), `220` (5 of 6), `70` (4 of 4 predicates). The four are
`362` (targets named, tests not yet written), `363` (matrix covered, `O_EXCL`
row missing), `359` (the banner and leg tests exist), `153` (the instrument
control and both plant targets exist).

---

## Meta 1 — proposal check table

One row per proposal that lands Rust code. `target?` asks whether the named
crate and file exist. `owner?` asks whether `TODO/INDEX.md`'s category table
puts that behaviour in that crate. `level` is checked against the three kinds in
`docs/conventions/code.md:27-29`. `command` is checked against
`.github/workflows/gate.yml:197` and `:211`, the two commands that actually run
in this repository today.

| script | proposed target | exists? | right owner? | level correct? | command runs? | dep refused? | verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `80-interposer-abi.sh` | `podbox-enter/src/abi.rs` + `podbox-cli/src/system.rs` | yes | yes (`enter` owns T-0709) | yes, pure | `cargo test -p podbox-enter abi` runs | no | **OK, mostly done** — 4 of 5 checks already written; the gap is one test on the `abi` wrapper's exit 2 |
| `388-interactive-shell.sh` | `podbox-ssh/tests/session_interactive.rs` | yes | yes | yes, integration | `cargo test -p podbox-ssh --test session_interactive` runs | no | **OK, done** — nothing to write |
| `149-podvm-non-goals.sh` | `podbox-probe/src/nongoals.rs` + `src/report.rs` | yes | yes | yes, pure | `cargo test -p podbox-probe nongoals` runs | no | **FIX** — the plan proposes the report-renderer test as new work at `report.rs`; it exists at `:1069` and `:1088`. Only the six-names constant is new |
| `90-nsswitch-contract.sh` | `podbox-complete/src/identity.rs` + `podbox-enter/src/abi.rs` | yes | yes | yes, pure for A and D | `cargo test -p podbox-complete identity` runs | no | **OK for A, already done**; check D's target is right and new |
| `170-probe-cache.sh` | `podbox-image/src/probe_cache.rs` | yes | yes | yes, pure | `cargo test -p podbox-image probe_cache` runs | no | **OK, already done** — four tests at `:233,247,268,296` |
| `160-store-gc.sh` | `podbox-image/src/store.rs` + `podbox-cli/src/images.rs` | yes | yes | yes, pure | `cargo test -p podbox-image store` runs | no | **REJECT the plan as written** — all five clauses are already in Rust: `store.rs:1950`, `store.rs:1976`, and `contain.rs:118` for clause 5, whose doc comment at `:119-120` states the exact defect. The plan proposes adding `a_symlinked_blob_resolving_outside_the_store_is_refused` to `contain.rs`, where a test of that name and shape already is. The one real gap is the CLI text, and there the plan's assertion is wrong: `images.rs:963-989` **prints** the skip lines and `prune` returns `0`. Only `rmi` (`images.rs:502`, `EXIT_RUNTIME_ERROR` at `:558,:573`) returns 125 |
| `220-extract-path-safety.sh` | `podbox-extract/src/drive.rs` | yes | yes | yes, pure + fixture | `cargo test -p podbox-extract drive` runs | no | **OK, one new test** — check F is the only gap and `drive.rs` is the right home |
| `70-whiteout-contract.sh` | `whiteout.rs`, `sidecar.rs`, `safety.rs`, `remove.rs` | yes except `remove.rs` | yes | yes | `cargo test -p podbox-extract whiteout` runs | no | **FIX** — the plan puts the layer-ordering test in `remove.rs`. `remove.rs` has **no** `#[cfg(test)]` module; the test is at `drive.rs:352`. `TODO/extract.md:399` names the test, not the file. The plan read the entry and inferred the file |
| `85-completion-symlink-escape.sh` | `podbox-complete/src/write.rs` + new `podbox-cli/tests/completion_containment.rs` | `write.rs` yes; `podbox-cli/tests/` no | yes | yes, library pure and reachability integration | `cargo test -p podbox-cli --test completion_containment` would run after the directory is created | no | **OK with a stated structural change** — this would be the **first** `tests/` directory outside `podbox-ssh`. The plan says so and that is correct |
| `362-windows-refusal.sh` | `lifecycle.rs`, `windows/dos.rs`, `windows/mod.rs` | yes | yes | yes, pure with a fixture | `cargo test -p podbox-cli windows::` runs | no | **OK** — I read all three message sites at `:1657-1665`, `:60-66` and `:85-91` and every fragment the script asserts is present |
| `398-gate-diagnostics.py` | `podbox-gate/tests/diagnostics.rs` or inline | `podbox-gate` does not exist | yes, a gate belongs with the gate | integration, correct | cannot run: the crate is new | no | **OK, blocked on the crate** |
| `30-attribution-census.sh` | new `podbox-probe/tests/attribution_expectations.rs` | no `tests/` in `podbox-probe` | yes | integration over tracked files, correct | would run | no | **FIX** — the plan cites `experiments/results/attribute.txt:18-19` for the Landlock rows. The file has 17 lines; the rows are at 16-17. Round 2 correction 8 caught this in group-4 and group-4's own plan text still carries it |
| `95-podman-vfs-ignorechown.sh` | `podbox-image/tests/engine_option_matrix.rs` | no | **no** — the subject is podman, not podbox | deployment, correct | `cargo test -p podbox-image --test engine_option_matrix -- --ignored` needs a podman machine the base cannot provide | no | **REJECT** — VC-3 already sends this to KEEP-SHELL. The plan is a shell drive under a Rust name |
| `140-space-precheck.sh` | `podbox-image/src/space.rs` + `tests/space_precheck.rs` | `space.rs` yes; the second is new | yes | pure + fault | `cargo test -p podbox-image space` runs | no | **OK with two line corrections** — the plan cites the three existing tests at `space.rs:249,268,240`; they are at `:246,263,238`. The conclusions are unaffected |
| `355-parity-curated.sh` | `podbox-cli/tests/curated_surface.rs` + `ascii_output.rs` | no `tests/` in `podbox-cli` | yes | split pure / integration, correct | would run | no | **OK with a stated structural change** |
| `352-ascii-output.sh` | `podbox-cli/tests/ascii_output.rs` | no | yes | integration, correct | would run | no | **OK** |
| `153-store-lock-race.sh` | new `crates/podbox-lockrace` | no | yes | fault, correct | `cargo run -p podbox-lockrace` cannot run until the crate exists | no | **OK, blocked on the crate** — and its four skip names and two plant targets are cited at their exact current lines |
| `251-tty-refusal-no-ptmx.sh` | new `crates/podbox-ptmx-cover` | no | yes | integration, correct | n/a | no | **OK, blocked on the crate** |
| `360-perf-harness.sh` | new `crates/podbox-perf` | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** |
| `366-namespace-base.sh` | new `crates/podbox-nsdrive` | no | yes | integration, correct | n/a | no | **OK, blocked on the crate** |
| `build-state.py` | new `crates/podbox-buildstate` | no | yes | pure, correct | n/a | no | **FIX** — the plan offers `tempfile::TempDir` as a fixture. No crate in the workspace has a `[dev-dependencies]` section and `tempfile` is in no manifest and no lock entry. `std::env::temp_dir()` with a pid suffix is the tree's pattern and the plan names it as the alternative |
| `check-todo.py` | new `crates/podbox-gate`, bin `podbox-gate` | no | yes (`gate` category → `scripts/`) | integration, correct | n/a | no | **OK, blocked on the crate** — and `docs/code-map.md:28` does call it "Independent task, citation, and document-state gate", so the name is justified |
| `todo-count.py` | `podbox-gate`, second bin `podbox-count` | no | yes | pure, correct | n/a | no | **OK, blocked on the crate** |
| `zig-cc.sh` | `podbox-gate/src/triple.rs` | no | yes | pure, correct | n/a | no | **OK, blocked on the crate** |
| `plant.sh` | new `crates/podbox-plant` | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** — and it must carry the CI step at `gate.yml:56` and the 11 `Prove` lines `TODO/gate.md` names, per VC-5 |
| `110-bloat-delta.sh` | new `crates/podbox-size` | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** — and `scripts/check-todo.py:227` names `experiments/110-bloat-delta.sh` as `CEILING_SCRIPT`, so the move and `check-todo.py:227` and the three plants at `scripts/plant.sh:278-292` are one change |
| `310-session-startup.sh` | new `crates/podbox-session` | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** |
| `build-interpose.sh` | bin `podbox-interpose-build` | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** |
| `verify-release.sh` | new `crates/podbox-verify` | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** |
| `120-reproducible-build.sh` | new crate, bin `podbox-prove-t0211` or similar | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** and on correction 1: the mutation anchor is stale |
| `157-lock-inheritance-prove.sh` | new `crates/podbox-rebuild` | no | yes | fault, correct | n/a | no | **OK, blocked on the crate and on correction 1.** I read `experiments/157-lock-inheritance-prove.sh:157` and `crates/podbox-image/src/store.rs:1083`: the pattern names `lock.fd`, the source reads `fd`. Round 2 is right. Group-8's re-anchored pattern `s/if !sys::close_in_children\(fd\) \{/if !true \{/` at `:751` matches the current source, and its second pattern at `store.rs:1032` also matches |
| `105-interpose-ownership.sh` check A | `crates/podbox-interpose/tests/exported_set.rs` | **no, and impossible** | yes | integration, correct | **cannot run** | no | **REJECT** — `crates/podbox-interpose/Cargo.toml:15` declares `crate-type = ["cdylib"]` with no `rlib`. Cargo links an integration test against the library target; a `cdylib` is not that. The plan's own command, `cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu --test exported_set`, cannot work without adding `rlib` to the crate type, which is a change to the one crate whose whole point is that it depends on nothing |
| `105-interpose-ownership.sh` check B | `crates/podbox-interpose/tests/struct_offsets.rs` | no, same | yes | integration, correct | cannot run | no | **REJECT**, same reason |
| `105-interpose-ownership.sh` check G | `crates/podbox-interpose/src/memo.rs` | yes | yes | pure, correct | `cargo test --manifest-path ... memo::tests` runs | no | **OK, with a number corrected** — `memo.rs` already has a `mod tests` at `:281` with two tests, and `SCAN_CEILING` is at `:66`. The plan says "the file already carries 55 test attributes across the crate"; the crate carries **48** |
| `106-interpose-identity.sh` | `crates/podbox-interpose/src/identity.rs` | yes | yes | pure, correct | `cargo test --manifest-path ... identity` runs, with `RUSTFLAGS='-C target-feature=-crt-static'` as `gate.yml:210` sets | no | **FIX, one clause** — the plan says "a new `#[cfg(test)] mod tests`". `identity.rs:282` already has one with three tests. The new work is adding to it, not creating it |
| `387-mux-two-client.sh` | KEEP-SHELL; the Rust half is `mux_two_client.rs` | yes | yes | integration, correct | `cargo test -p podbox-ssh --test mux_two_client` runs | no | **OK** — `mux_two_client.rs:18` names the live drive as the acceptance gate, which is the right relationship |
| `394-ssh-package.sh` | new bin `package-ssh` in `podbox-ssh` + `tests/package.rs` | the bin does not exist; `tests/` does | yes | unit for 1-4, integration for 5-10, correct | would run after the bin is added | no | **OK** — and it correctly reuses `podbox_enter::abi::Elf::read` rather than shelling to `readelf` |
| `397-exported-build.py` | new bin `verify-export` in `podbox-cli` + `tests/verify_export.rs` | neither exists | yes | unit for parsing, integration for digest, correct | would run | no | **OK, with a stated structural change** — first `tests/` in `podbox-cli` |
| `session-start.sh` | new `crates/podbox-dev` | no | yes | integration, correct | n/a | no | **OK, blocked on the crate** — and its claim that nine files name the shell spelling is a documentation change the plan correctly bounds with a shim |
| `260-multiarch.sh` | SPLIT; the struct-size half to `podbox-probe/src/sys.rs` | yes | yes | pure, correct | `cargo test -p podbox-probe sys` runs | no | **OK** — `sys.rs:1815`'s `the_stat_buffer_is_the_size_the_kernel_writes` already names `260` and already pins 144 / 128 |
| `130-probe-parity.sh` | `podbox-probe/src/sys.rs` + new `tests/probe_parity.rs` | `sys.rs` yes; the other new | yes | pure + integration split, correct | would run | no | **OK** — `sys.rs:1776` already names the script as the re-check of the 16 attribution numbers |
| `151-spawn-ambiguity.sh` | SPLIT: pure half to `podbox-probe`, Go half stays shell | yes | yes | correct per VC-7 | n/a | no | **OK** |
| `150-image-acquisition.sh` | `podbox-image/src/digest.rs`, `src/reference.rs`, new `tests/store_digest.rs` | two of three exist | yes | pure then integration, correct | would run | no | **REJECT the fixture claim** — the plan says "Reuse that; do not write a third one", naming T-0206's loopback registry. T-0206's fixture is `experiments/180-registry-fixture.sh` driving a downloaded `zot` binary (`TODO/image.md:483-485`). The only Rust loopback is `registry.rs:955` `serve_once`, a one-shot HTTP/1.0 responder that is not an OCI registry and is not reusable across a `tests/` binary. **There is no Rust registry fixture to reuse** |
| `250-negative-tests.sh` | SPLIT into four crate tests | yes | yes | correct | n/a | no | **OK** |
| `145-podvm-parity.sh` | `podbox-podvm` (new) or a `spread.rs` under an existing crate | no | yes | pure, correct | n/a | no | **OK, blocked on the crate** |
| `125-across-distributions.sh` | RUST-TOOL; the measurement moves to `podbox-complete/src/identity.rs` | yes | yes | pure | `cargo test -p podbox-complete identity` runs | no | **OK, already done** — the six shapes are at `identity.rs:336` |
| `190-parallel-layers.sh` | RUST-TOOL; the policy is at `pull.rs` | yes | yes | integration | n/a | no | **OK** |
| `155-proc-absence.sh` | SPLIT into the interpose suite | yes | yes | correct | n/a | no | **OK** |
| `300-run.sh` | SPLIT into `podbox-probe/src/report.rs` and a live drive | yes | yes | correct | n/a | no | **OK** |
| `270-multiarch-image.sh` | SPLIT into `podbox-image/src/platform.rs` and a live index drive | yes | yes | correct | n/a | no | **OK** |
| `200-registry-auth.sh` | KEEP-SHELL with a Rust piece in `podbox-image/src/credentials.rs` | yes | yes | correct | n/a | no | **OK** — `credentials.rs` already carries 22 tests |
| `364-qol.sh` | SPLIT: integration + fault + pure units | yes | yes | correct | n/a | no | **OK** |
| `60-interposer-libc.sh` | RUST-TOOL, a new bin | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** — but its plant, "add `crate-type = [\"cdylib\", \"rlib\"]` and assert the tool reports A did not refuse", is a change to `crates/podbox-interpose/Cargo.toml:15`. That is a real edit to the excluded crate and must be named as one |
| `395-reconcile-repository.py`, `399-publication.py` | RUST-TOOL, new crates | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** |
| `document-state.py`, `release-notes.sh` | RUST-TOOL, `podbox-docs` | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** |
| `dev.sh`, `nightly-smoke.sh` | `podbox-gate` | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** |
| `release-licenses.py` | RUST-TOOL, second bin in `podbox-buildstate` | no | yes | deployment, correct | n/a | no | **OK, blocked on the crate** |
| `330-exit-codes.sh` | `podbox-probe/src/exit.rs` + `podbox-complete/src/lib.rs` | yes | yes | pure, correct | `cargo test -p podbox-probe exit` runs | no | **OK** — three tests already exist at `exit.rs:131,139,163` |
| `159-interpose-placement.sh` | `podbox-cli/src/interpose.rs` | yes | yes | pure for `place` / `merge_preload`, integration for the rest, correct | `cargo test -p podbox-cli interpose::tests` runs | no | **OK** — 20 tests already there |
| `363-device-map.sh` | `podbox-interpose/src/device.rs` + `podbox-enter/src/device.rs` + `podbox-cli/src/parity.rs` | all yes | yes | correct | `cargo test --manifest-path ... device` runs for the interposer half | no | **OK** — the `O_CREAT\\|O_EXCL` gap it names is real; `device.rs:333` covers creation without the `EXCL` arm |
| `391-machine-bridge.sh` | KEEP-SHELL; a contract test in `podbox-ssh` is optional | yes | yes | n/a | n/a | no | **OK** |
| `143`'s neighbours: `320`, `340`, `358`, `365`, `373`, `146`, `390` (group-5) | crate-local inline `mod tests` in `podbox-cli`, `podbox-probe`, `podbox-supervise` | yes | yes | correct | `cargo test -p podbox-cli` / `-p podbox-supervise` run | no | **OK** |
| `152`, `326`, `351`, `356`, `370`, `372` (group-6) | SPLIT, mixed inline and live drive | yes | yes | correct | n/a | no | **OK** |

**Rejected or fixed: 7. Blocked on a new crate: 17. Already done: 9. OK: the
rest.**

---

## Meta 1 — final verdict counts

**The header tables and the bodies disagree in six of ten groups.** Each
group's body carries one `**Verdict:` or `### Verdict:` line per script. I
counted those. The header tables carry a distribution. In groups 1, 4, 5, 6, 7
and 9 the two disagree. The **bodies** are the verdicts; the headers are
summaries of them and are wrong.

| group | body n | header n | agree? |
| --- | --- | --- | --- |
| 1 | 9 | 9 | **no** — body SPLIT 3 / RUST-TOOL 2; header SPLIT 2 / RUST-TOOL 3 |
| 2 | 12 | 12 | yes |
| 3 | 13 | 13 | yes |
| 4 | 12 | 12 | **no** — body KEEP-SHELL 4 / RUST-TEST 4; header KEEP-SHELL 3 / RUST-TEST 5 |
| 5 | 12 | 12 | **no** — body SPLIT 5 / RUST-TEST 3 / RUST-TOOL 1 / KEEP-SHELL 1; header SPLIT 2 / RUST-TEST 4 / RUST-TOOL 2 / KEEP-SHELL 3 |
| 6 | 13 | 13 | **no** — body RUST-TEST 0, SPLIT 7; header RUST-TEST 1, SPLIT 6 |
| 7 | 12 | 12 | **no** — body SPLIT 4 / RUST-TEST 2; header SPLIT 3 / RUST-TEST 3 |
| 8 | 13 | 13 | yes |
| 9 | 13 | 13 | **no** — body SPLIT 7 / RUST-TEST 1 / KEEP-SHELL 1 / RUST-TOOL 3; header SPLIT 5 / RUST-TEST 4 / KEEP-SHELL 2 / RUST-TOOL 2 |
| 10 | 12 | 12 | yes |
| **total** | **121** | **121** | 4 agree, 6 differ |

The body total is 121, which matches `refactor/00-orientation/assignment-map.md:3`.
The header total is also 121, so the two sets of numbers cannot both be right
for the same scripts.

**After VC-1 through VC-7**, counting from the bodies:

| verdict | before VC | after VC | change |
| --- | --- | --- | --- |
| KEEP-SHELL | 27 | **29** | +2 (`162` by VC-2, `95` by VC-3) |
| RUST-TOOL | 27 | **27** | 0 (VC-5 relabels `plant.sh` in place) |
| SPLIT | 35 | **36** | +1 (`151` by VC-7) |
| RUST-TEST | 20 | **18** | −2 (the two above) |
| DELETE | 12 | **11** | −1 (`162` leaves DELETE) |
| **total** | **121** | **121** | |

VC-1 (`386`), VC-4 (`10`) and VC-6 (`95`, the level statement) change evidence
or condition, not a verdict label. **The count to use is 121, and 43 of them
converge on a Rust test or tool** (18 RUST-TEST + 36 SPLIT, counting a SPLIT as
converging because every one of the 36 has a Rust half).

---

## Meta 1 — implementation waves

Nine waves. A wave is a set of entries that lands together behind one green
`cargo test --workspace` and one `py scripts/check-todo.py` pass. Boundaries
are drawn where a wave's proof needs a crate, a directory, a fixture, or a
record the previous wave does not have.

**The two commands that gate every wave.** From
`.github/workflows/gate.yml:197` and `:211`:

```sh
cargo test --workspace
RUSTFLAGS='-C target-feature=-crt-static' \
  cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu
```

plus `py scripts/check-todo.py`, which is `scripts/check-todo.py:300`'s tracked
file scan. **Neither runs on this host**: `cargo` stops at `linker 'cc' not
found`. Every wave below is proven on the Linux base lane through
`sh scripts/windows/run-in-base.sh`.

### Wave 0 — the record fixes. No Rust, no build.
`TODO/cli.md:63` 265 rows; `TODO/cli.md:182` 40/40; `TODO/cli.md:1246` ten;
`TODO/gate.md:1431` remove the TCG figure; `TODO/probe.md:171` ESRCH;
`TODO/interpose.md:87` 112; `TODO/interpose.md:1445-1446` the `lib.rs` lines;
`TODO/gate.md:434` the `Source` line; `TODO/image.md` T-0211's Done re-anchored
or reopened; `experiments/157-lock-inheritance-prove.sh:157` re-anchored to
`close_in_children(fd)`.

**Boundary reason:** wave 1's plant for `157` encodes the current source. A
stale anchor makes the plant pass vacuously, so the fix must land first.
**Proof:** `py scripts/check-todo.py` exits 0.

### Wave 1 — delete what is already in Rust. No new code.
`388-interactive-shell.sh` (12 tests, 9 clauses, 3 extra).
`80-interposer-abi.sh` (4 of 5 checks, keeping the `B` loader arm as a drive).
`170-probe-cache.sh` clauses 2-3. `160-store-gc.sh` (all five clauses, the last
at `contain.rs:118`).
`90-nsswitch-contract.sh` check A. `70-whiteout-contract.sh` all four predicates.

**Boundary reason:** no new test is written, so the proof is the existing
suite. `TODO/podssh.md:79` and `:126` repoint from the script to the test file
in the same change, because a `Prove` that names a deleted script is a red
gate. **Proof:** `cargo test --workspace` unchanged and green.

### Wave 2 — the inline pure gaps. One crate each, no new directories.
`149-podvm-non-goals.sh` (the six-names constant only; the report test already
exists at `report.rs:1069,1088`). `220-extract-path-safety.sh` check F, into
`extract/src/drive.rs`. `362-windows-refusal.sh` into
`cli/src/lifecycle.rs`, `cli/src/windows/dos.rs`, `cli/src/windows/mod.rs`.
`90-nsswitch-contract.sh` check D into `enter/src/abi.rs`.
`105-interpose-ownership.sh` check G into `interpose/src/memo.rs`.
`106-interpose-identity.sh` clauses A, B, C0 into `interpose/src/identity.rs`,
which already has a `mod tests` at `:282`.
`130-probe-parity.sh`'s pure half into `probe/src/sys.rs`.
`160-store-gc.sh`'s CLI-text half into `cli/src/images.rs`, asserting the exit
code each verb actually returns: 125 for `rmi`, 0 for `prune`.
`159-interpose-placement.sh`'s `place` / `merge_preload` half into
`cli/src/interpose.rs`.
`363-device-map.sh`'s `O_CREAT|O_EXCL` row into `interpose/src/device.rs`.
`330-exit-codes.sh`'s case-name set into `probe/src/exit.rs`.
`325-parity-drive.sh` clauses 1-2 into `cli/src/parity.rs`, whose `mod tests`
is at `:706`.

**Boundary reason:** every target is an existing `mod tests`. Nothing opens a
new directory, so one `cargo test --workspace` covers the whole wave and the
plant set is one `scripts/plant.sh` case per test. **Proof:**
`cargo test --workspace`.

### Wave 3 — the interposer `WANT` string. Small, self-contained.
`70-whiteout-contract.sh`'s one new test. `85-completion-symlink-escape.sh`'s
two new library tests in `complete/src/write.rs`.

**Boundary reason:** these two are the only library tests that need a
`symlink`-planted temp rootfs in a crate that does not already have one
(`write.rs:643` does, so this is thin). Grouping them separately keeps the
wave's fixture work in one place. **Proof:** `cargo test -p podbox-extract` and
`cargo test -p podbox-complete`.

### Wave 4 — the first `tests/` directories outside `podbox-ssh`.
`85`'s reachability half → new `crates/podbox-cli/tests/completion_containment.rs`.
`355-parity-curated.sh` → new `crates/podbox-cli/tests/curated_surface.rs`.
`352-ascii-output.sh` → new `crates/podbox-cli/tests/ascii_output.rs`.
`397-exported-build.py` → new bin `verify-export` in `podbox-cli` plus
`tests/verify_export.rs`.
`394-ssh-package.sh` → new bin `package-ssh` in `podbox-ssh` plus
`tests/package.rs`.
`30-attribution-census.sh` → new `crates/podbox-probe/tests/attribution_expectations.rs`.
`140-space-precheck.sh` clause 4 → new `crates/podbox-image/tests/space_precheck.rs`.
`150-image-acquisition.sh` clauses 2-3 → new `crates/podbox-image/tests/store_digest.rs`.

**Boundary reason:** this wave is where the tree's shape changes. Three crates
gain their first `tests/` directory and two gain a `[[bin]]`. Doing it in one
wave means one decision, one gate pass, and one `docs/code-map.md` edit.
**Boundary condition:** the `store_digest` and `completion_containment` tests
want a real extracted rootfs; without a Rust registry fixture (see the
rejected row above) the `store_digest` half is **blocked** and lands later.
**Proof:** `cargo test --workspace`, plus `cargo build -p podbox-ssh --release
--target x86_64-unknown-linux-musl` for the package half.

### Wave 5 — `crates/podbox-gate`. Three scripts, four binaries.
`check-todo.py` → bin `podbox-gate`. `todo-count.py` → bin `podbox-count`.
`zig-cc.sh` piece 2 → `podbox-gate/src/triple.rs`.
`dev.sh` → the same crate. `nightly-smoke.sh` → `podbox-gate smoke`.
`398-gate-diagnostics.py` → `podbox-gate/src/lib.rs` inline.

**Boundary reason:** these five are one crate. The gate is the project's
instrument for every other change in this plan, so moving it first would make
the later waves unprovable; moving it late means the earlier waves are proven by
the Python gate they were replacing. It goes in the middle. **One change must
repoint `scripts/check-todo.py:227` `CEILING_SCRIPT` if `110` moves with it, and
must add the new crate to `scripts/plant.sh:51` `FILES`.**
**Proof:** `cargo test -p podbox-gate` and `py scripts/check-todo.py`, with
`check-todo.py` now reading the Rust binary.

### Wave 6 — `crates/podbox-buildstate` and `crates/podbox-dev`.
`build-state.py` → `podbox-buildstate` lib + bin.
`release-licenses.py` → second bin in the same crate.
`session-start.sh` → `podbox-dev` bin `podbox-dev session-start`.

**Boundary reason:** two independent new crates with no shared subject. They
are one wave because both are pure or trivially integration and neither needs a
fixture another wave builds. `session-start.sh` needs the `AGENTS.md`,
`README.md`, `HUMAN.md`, `scripts/README.md`, `docs/containers.md`,
`docs/code-map.md`, `docs/agent-tooling.md` and `docs/hosted-sessions.md` edits
in the same change, or a shim. **Proof:** `cargo test -p podbox-buildstate` and
`cargo test -p podbox-dev`.

### Wave 7 — the release and build tools.
`110-bloat-delta.sh` → `podbox-size`. `120-reproducible-build.sh` → a new
`podbox-rebuild`. `157-lock-inheritance-prove.sh` → the same crate.
`build-interpose.sh` → `podbox-interpose-build`. `verify-release.sh` →
`podbox-verify`. `document-state.py` and `release-notes.sh` → `podbox-docs`.
`395-reconcile-repository.py` and `399-publication.py` → their own crates.
`145-podvm-parity.sh` → `podbox-podvm`.

**Boundary reason:** nine new crates with no shared code. They are one wave
only because they share a shape: each is a binary with a three-state exit, each
is a deployment-level proof, and each needs the same `Cargo.toml` `members` row
and the same `TODO/INDEX.md` category row. Splitting them gives nine `members`
edits instead of one. **Boundary condition:** `157` cannot land until wave 0's
re-anchor, and `110` cannot move without `scripts/check-todo.py:227` and the
three plants at `scripts/plant.sh:278-292`. **Proof:** `cargo test -p` for each
new crate.

### Wave 8 — `crates/podbox-plant`, last.
`plant.sh` → `podbox-plant` with 44 cases, the four guards, and the CI step at
`gate.yml:56` repointed.

**Boundary reason:** `plant.sh` is the project's own instrument. Moving it
while five crates are still landing would mean two instruments at once and no
single authority. It goes last, and it goes with the 11 `Prove` lines across
`TODO/gate.md` and `TODO/deps.md` that name `./scripts/plant.sh`, per VC-5.
**Proof:** `cargo run -p podbox-plant --bin podbox-plant` exits 0, then
`py scripts/check-todo.py` through the new binary.

---

## Meta 1 — new crates required

Eighteen distinct new crate names across the plans. Every one needs a
`members` row and, if the behaviour has no home, a `TODO/INDEX.md` category
row.

| proposed crate | binary | from | `members` row | `INDEX.md` category row |
| --- | --- | --- | --- | --- |
| `crates/podbox-gate` | `podbox-gate`, `podbox-count` | `check-todo.py`, `todo-count.py`, `zig-cc.sh`, `dev.sh`, `nightly-smoke.sh`, `398` | yes | yes, new category beside `gate`, or fold under `gate` |
| `crates/podbox-buildstate` | `podbox-buildstate`, `podbox-release-licenses` | `build-state.py`, `release-licenses.py` | yes | yes, `packaging` owns both today |
| `crates/podbox-dev` | `podbox-dev session-start` | `session-start.sh` | yes | yes, `packaging` T-1005 owns it |
| `crates/podbox-plant` | `podbox-plant` | `plant.sh` | yes | yes, `gate` |
| `crates/podbox-size` | `podbox-size` | `110-bloat-delta.sh` | yes | yes, `deps` T-0910 |
| `crates/podbox-rebuild` | `podbox-prove-t0211` | `120`, `157` | yes | yes, `packaging` T-1004 and `image` T-0211 |
| `crates/podbox-interpose-build` | `podbox-interpose-build` | `build-interpose.sh` | yes | yes, `packaging` T-1002 |
| `crates/podbox-verify` | `podbox-verify` | `verify-release.sh` | yes | yes, `packaging` T-1328 |
| `crates/podbox-docs` | `document-state`, `release-notes` | `document-state.py`, `release-notes.sh` | yes | yes, `complete` T-1324 and `packaging` |
| `crates/podbox-podvm` | `podbox-podvm` | `145-podvm-parity.sh`, `154` | yes | yes, `podvm` |
| `crates/podbox-lockrace` | `podbox-lockrace` | `153-store-lock-race.sh` | yes | yes, `image` T-0215 |
| `crates/podbox-ptmx-cover` | `podbox-ptmx-cover` | `251-tty-refusal-no-ptmx.sh` | yes | yes, `enter` T-0503 |
| `crates/podbox-perf` | `podbox-perf` | `360-perf-harness.sh` | yes | yes, `gate` T-1338 |
| `crates/podbox-nsdrive` | `podbox-nsdrive` | `366-namespace-base.sh` | yes | yes, `probe` T-0111 |
| `crates/podbox-session` | `podbox-session` | `310-session-startup.sh` | yes | yes, `packaging` T-1005 |
| `crates/podbox-interpose-check` | `podbox-interpose-check` | `60-interposer-libc.sh` | **no** — must stay out, or the check runs outside the exclusion | no, `interpose` owns it |
| `crates/podbox-reconcile` | `podbox-reconcile` | `395-reconcile-repository.py` | yes | yes, `packaging` |
| `crates/podbox-publish` | `podbox-publish` | `399-publication.py` | yes | yes, `packaging` T-1314 |

**Eighteen new crates is a structural decision, not a set of files.** The
workspace has ten members today. Eighteen more is a 2.8x growth in the unit of
`Cargo.lock` resolution, of `scripts/check-todo.py`'s tracked-file scan, and of
the gate's citation check. **The reports do not name that cost anywhere.** The
cheaper shape, and the one this plan should default to, is **four new crates**
— `podbox-gate`, `podbox-buildstate`, `podbox-plant`, `podbox-verify` — with a
second `[[bin]]` in each for the scripts that share a subject. Group-1 already
does this for `check-todo.py` and `todo-count.py` ("One crate, two binaries,
because the two share the `ROW` grammar") and group-2 does it for
`build-state.py` and `release-licenses.py`. The other fourteen do not.

**On `podbox-interpose-check`:** a crate that checks whether the interposer can
be built as a `cdylib` cannot itself be a workspace member, because the check's
subject is the exclusion itself. It must be built the way
`scripts/build-interpose.sh` builds the interposer, from outside cargo, and its
proof command is the `gate.yml:211` shape. **Any plan that adds it to
`members` inverts the thing it measures.**

---

## Meta 1 — blockers outside the code

Named precisely, because a task entry that says "needs an engine" is not a
blocker a reader can act on.

### Blockers that stop a `cargo test` from running

| blocker | what it stops | evidence |
| --- | --- | --- |
| **No C linker on this host.** `cargo test --workspace` fails at `error: linker 'cc' not found`. | every wave's proof. Round 2 recorded this as O4 and did not re-run it | the toolchain on this Windows host; `scripts/zig-cc.sh` exists for a different lane |
| **Real `sshd`, `ssh`, `ssh-keygen`.** | `cargo test -p podbox-ssh --tests`. `require_binary` **panics**, never skips (`common.rs:41`) | `crates/podbox-ssh/tests/common.rs:28-43` |
| **A Linux kernel.** `podbox-probe/src/sys.rs` is Linux-only; `crates/podbox-ssh/tests/*` uses `std::os::unix::net::UnixListener` and `std::os::unix::fs::PermissionsExt` | every proof on this host | `sys.rs`, `common.rs:17` |
| **A `target` of `x86_64-unknown-linux-musl`.** `.cargo/config.toml:8` sets it globally, so a host build of the workspace needs that target installed | `cargo test --workspace` on a plain Linux host without the target | `.cargo/config.toml:7-8` |

### Blockers on a live OCI engine, root, or a network

| script | needs | where the plan already says so |
| --- | --- | --- |
| `95-podman-vfs-ignorechown.sh` | a **nested podman machine**; the WSL base has none. **VC-3 sends this to KEEP-SHELL** | `group-4.md:555` |
| `180-registry-fixture.sh` | a downloaded `zot` binary, `openssl`, a `--network=none` container | `group-6.md:294` |
| `369-windows-tcg-dos.sh`, `370-windows-guest.sh`, `362` clause 3 | **a licensed Windows image** or a FreeDOS base the script fetches | `crates/podbox-cli/src/windows/dos.rs:61` names the script |
| `367-qemu-user-aarch64.sh` | a live `binfmt_misc` aarch64 registration and `qemu-aarch64-static` | `group-7.md:1316` |
| `385-kvm-open.sh`, `357-tcg-profile.sh`, `392-kvm-guest.sh` | `/dev/kvm` **absent** (that is the subject) or present (the other arm) | group-10, group-8 |
| `290-microvm.sh` | `qemu-system-x86_64 -M microvm` and a TPM | `group-6.md:346` |
| `210-store-concurrency.sh` | real concurrent processes against a real store. A mock proves nothing, and `store.rs:36` says so | `group-10.md:428` |
| `125-across-distributions.sh`, `190-parallel-layers.sh`, `150-image-acquisition.sh` | a **registry pull**, and a quota that the project's own runs exhausted once (`TODO/image.md:444-448`) | `TODO/image.md:444-448` |
| `152-nix-acceptance.sh` | a 23 MiB TLS fetch from `nixos.org` and a `nix-build` that compiles `hello` | `group-6.md:169` |
| `240-distro-sweep.sh`, `382-restricted-sshd.sh`, `361-guest-usernet.sh` | eleven to fifteen pinned images and a rootful engine | group-4, group-7 |
| `260-multiarch.sh` | **cross-compiling to 14 architectures**, and `syscalls` 0.8.1's crate-level `#![feature(asm_experimental_arch)]` on five of them. `TODO/deps.md` T-0912 rules the dependency target-gated for that reason | `crates/podbox-probe/Cargo.toml:9-24` |
| `391-machine-bridge.sh` | `pty.openpty`, a fetched **dropbear** built with `./configure && make`, and `zig cc -static` | `group-9.md:842` |
| `400-kvm-cleanup.py` | Linux processes with controlled argv; no guest. **This one is not blocked** and runs in the required gate at `gate.yml:208` | `experiments/400-kvm-cleanup.py:1-2` |

### Blockers on the operator

| item | needs | why it cannot be automated |
| --- | --- | --- |
| `crates/podbox-gate` **rename** | a decision on whether `podbox-check-todo` keeps its name | `docs/conventions/code.md:14`: "A compatibility path needs a current caller and a stated condition." The condition is the operator's to state |
| 14 windows-lane proofs | a **licensed Windows guest image** | no licence exists in this tree |
| `docs/code-map.md`, `docs/agent-tooling.md`, `docs/architecture.md` | a decision on which crate owns a behaviour the ten reports disagree about (group-1 counts `check-todo.py` RUST-TOOL, its own body counts it twice) | the disagreement is a record decision, and `TODO/INDEX.md`'s category table is the artefact that settles it |
| The whole plan on this host | `wsl-toolkit --instance podbox` and a `cc` | `AGENTS.md` names the instance; `scripts/zig-cc.sh` is the C toolchain answer |

---

## Meta 1 — where the reports were wrong about the implementation

Nine, each with the file I opened.

**1. A `cdylib`-only crate cannot host `tests/*.rs`.** Group-8's `105` plan puts
check A in `crates/podbox-interpose/tests/exported_set.rs` and check B in
`tests/struct_offsets.rs`, and gives the commands
`cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target
x86_64-unknown-linux-gnu --test exported_set`. `crates/podbox-interpose/Cargo.toml:15`
declares `crate-type = ["cdylib"]` and nothing else. Cargo builds an
integration test by linking the crate's library target; a `cdylib` is not
linkable that way. Both commands are wrong as written, and making them work
means adding `rlib` to the one crate whose entire reason for existing is that
it has no dependencies and no shared resolution. **The check A and check B work
have to live somewhere else: the gate, or inline in `lib.rs` reading the built
`.so` from `CARGO_MANIFEST_DIR`, or a shell drive that stays.**

**2. `remove.rs` has no test module.** Group-4's `70` plan puts the
layer-ordering leg in `crates/podbox-extract/src/remove.rs` and says the test
"already exists per `TODO/extract.md:399`". `TODO/extract.md:399` names a
`Prove` line, not a file. The test is at `crates/podbox-extract/src/drive.rs:352`.
`grep -n '#\[cfg(test)\]' crates/podbox-extract/src/remove.rs` returns nothing.
The plan read the entry and inferred a file from a subject name.

**3. The T-0206 registry fixture is shell, not Rust.** Group-5's `150` plan says
of clauses 2 and 3: "the loopback registry from T-0206. `TODO/image.md:452` names
it as 'a reader over `crates/podbox-image/src/store.rs` rather than a second
store'. Reuse that; do not write a third one." `TODO/image.md:483` gives T-0206's
`Prove` as `./experiments/180-registry-fixture.sh` and `:485` its Done. The
fixture is a shell script driving a downloaded `zot` binary. The only Rust
loopback is `crates/podbox-image/src/registry.rs:955` `serve_once`, a one-shot
HTTP/1.0 responder that answers one request with one body and is not an OCI
registry. **There is nothing to reuse.** The `store_digest.rs` plan in wave 4
needs a Rust registry fixture built first, or the test is hand-rolled per
assertion, which is `docs/conventions/code.md:6` "Do not add duplicate
implementations".

**4. `160-store-gc.sh` is fully converted, and the plan proposes the last clause
again.** Group-2's `160` plan says "Four of the five clauses are already unit
tests" and then, under "Conversion plan for clause 5 (the SPLIT)", proposes
adding `a_symlinked_blob_resolving_outside_the_store_is_refused` to
`crates/podbox-image/src/contain.rs`. `contain.rs:118` is
`a_symlink_pointing_out_of_the_store_is_refused`, and its doc comment at
`:119-120` reads "⭐ The whole reason the check is on the resolved path. A
string check passes this and deletes /etc." That is clause 5's assertion and
clause 5's failure control. **All five clauses of `160` are in Rust.** The
plan's other error is on the CLI half: it says the new `images.rs` test "must
assert the exit code it returns is `EXIT_RUNTIME_ERROR` (125)". I read
`images.rs:963-989`: `prune` calls `store.delete(&rest, Held::Skip)`, prints
`skipped: {name} is in use by a running container` at `:976`, prints the
referenced line at `:980`, and returns `0` at `:988`. Only `rmi`
(`images.rs:502`) reaches `EXIT_RUNTIME_ERROR`, at `:558` and `:573`. A test
written to the plan would fail on `prune`.

**5. `identity.rs` already has a `mod tests`.** Group-10's `106` plan says
"Target: `crates/podbox-interpose/src/identity.rs`, a new `#[cfg(test)] mod
tests`." `crates/podbox-interpose/src/identity.rs:282` is one, with three tests
(`uid_alone_means_its_own_group`, `uid_and_gid_parse`,
`anything_else_is_not_an_identity`). The work is adding to it. Minor, and it
changes the diff's shape.

**6. The interposer test count is 48, not 55.** Group-8's `105` check G plan
says `memo.rs` "already carries 55 test attributes across the crate". I counted
with `grep -rc '#\[test\]' crates/podbox-interpose/src/*.rs | awk -F: '{s+=$2}'`:
**48**. This is round 2's failure mode exactly — a whole-tree count quoted
without the command. It does not change the plan, but it is the fourth instance
of the same habit.

**7. Six of ten group header tables disagree with their own bodies.** Group-1
counts `zig-cc.sh` as RUST-TOOL in its table and gives it SPLIT in its body.
Group-4's table says KEEP-SHELL 3 and its body has 4. Group-5's table says
SPLIT 2, RUST-TEST 4, RUST-TOOL 2, KEEP-SHELL 3; its body says SPLIT 5,
RUST-TEST 3, RUST-TOOL 1, KEEP-SHELL 1. Group-6's table says RUST-TEST 1; its
body has **zero** `**Verdict: RUST-TEST.**` lines. Group-7's table says RUST-TEST
3; its body has 2. Group-9's table says SPLIT 5, RUST-TEST 4; its body says
SPLIT 7, RUST-TEST 1. **Both sets total 121, so neither is caught by a sum.**
The task entries must be built from the bodies, and a reader working from the
headers will write the wrong entries for six groups.

**8. `149`'s report-renderer test already exists.** Group-9's `149` plan
proposes, as new work, "Target: `crates/podbox-probe/src/report.rs`, inside its
existing `mod tests`. Assert the rendered evidence string contains
`non-goals, one stance per blocked design`". `crates/podbox-probe/src/report.rs:1105-1107`
asserts that exact string, inside `a_denied_kvm_names_its_errno_in_the_document_and_the_evidence`
at `:1088`, which also asserts the document carries all six row names. The
`report.rs` proposal is a restatement of a test that exists. This is the single
most plan-changing error: it makes `149` look like it has two new tests to write
when it has one.

**9. Three line citations are off by two to three, in the same file.**
Group-1's `140` plan cites `space.rs:249`, `:268` and `:240` for three existing
tests; they are at `:246`, `:263` and `:238`. Group-1's `393` plan cites
`crates/podbox-image/src/space.rs:281` for `dir_bytes_counts_once_and_never_follows`;
it is at `:299`. Group-4's `30` plan cites
`experiments/results/attribute.txt:18-19` for the Landlock rows; the file has 17
lines and the rows are at 16-17 — the citation round 2 corrected in group-4's
findings, which group-4's own plan text still carries. None changes a
conclusion. All three are the same failure as the parity count: a line number
reached by reading the wrong range.

**And one place the reports were right that I expected them to be wrong.**
Group-9's `149` plan says "`crates/podbox-probe/src/nongoals.rs:227`" for the
`mod tests` header. The `#[cfg(test)]` attribute is at `:226` and `mod tests {`
at `:227`. Group-9's `80` plan says `abi.rs`'s `mod tests` is "at line 1075"; the
attribute is at `:1074` and `mod tests {` at `:1075`. Group-6's `170` plan names
all four `probe_cache.rs` tests at their exact lines. Group-2's `153` plan names
all four skip names and both plant targets at their exact lines. Group-3's `325`
plan names `parity.rs:706-707` for the module, `:1001` for the import, `:599-621`
for the `None` arm and `:959-991` for the SSH test — all correct. **The line
citations are correct wherever the report read the file and wrong wherever it
inferred a line from a subject name.** That is the pattern, and it is worth
stating as the gate on the final entries: a line citation that was inferred
rather than read must be re-opened before it goes in a `Prove`.
