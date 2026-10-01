//! `experiments/125-across-distributions.sh` hermetic half: the row
//! discipline against the loopback fixture.
//!
//! The script pulls one pinned row per distribution through an engine and
//! reads the same bytes on every row (libc, nsswitch, supplied passwd). That
//! matrix needs a live engine and a live registry, so it stays in the script
//! as the SKIP test at the bottom. This file ports only what is hermetic:
//! the four rules that make the script a measurement rather than a survey.
//!
//! Ported rules, each against the loopback fixture through the `pull` API:
//!
//! * rule 4: the reference is fully qualified before it is pulled. The pull
//!   records the canonical qualified repository it was asked for, never a
//!   display short form.
//! * rule 2: a row that could not be pulled counts apart from a row whose
//!   command ran and failed. Unknown tags are refused as transport errors
//!   and never read as broken rows.
//! * rule 3: a run where nothing ran is a failure, not agreement. A fixture
//!   pull that fails fails this test; it never passes silently.
//! * the reading itself: every row that pulls produces one. Here the reading
//!   is the platform the image declares (the os and arch the record
//!   carries), which is this crate's analog of the script's libc row.
//!
//! Not ported: the eleven live distribution rows (alpine through archlinux),
//! which need an engine and network access to Docker Hub.

mod common;

use std::sync::atomic::{AtomicU64, Ordering};

use podbox_image::platform::Platform;
use podbox_image::pull;
use podbox_image::store::Store;
use podbox_image::transport::Policy;

/// Serialises the pulls in this file, for the same reason as the matching
/// lock in the parallel-layers test: one file, one pull at a time.
static PULLS: std::sync::Mutex<()> = std::sync::Mutex::new(());

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A unique store directory for one test leg. The caller removes it.
fn fresh_dir(prefix: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("podbox-125-{prefix}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch store directory");
    dir
}

fn cleanup(dir: &std::path::Path) {
    let _ = std::fs::remove_dir_all(dir);
}

/// The deliberate act the CLI spells `--insecure-registry`: plain HTTP to the
/// loopback fixture only.
fn policy_for(endpoint: &str) -> Policy {
    Policy::with_insecure(&[endpoint])
}

/// Script rule 4: the reference is fully qualified before it is pulled, so
/// the store records the qualified repository it was asked for and no
/// shortname alias of it.
#[test]
fn the_reference_is_qualified_before_it_is_pulled() {
    let _held = PULLS.lock().unwrap_or_else(|e| e.into_inner());
    let fx = common::registry::Fixture::start();
    let policy = policy_for(&fx.endpoint);

    let parsed =
        podbox_image::Reference::parse(&fx.reference()).expect("the loopback reference parses");
    assert_eq!(
        parsed.endpoint(),
        fx.endpoint,
        "the reference resolved away from the loopback endpoint"
    );

    let dir = fresh_dir("qualified");
    let store = Store::open(&dir).expect("the test opens its store");
    let mut out = Vec::new();
    let pulled = pull::pull(
        &store,
        &fx.reference(),
        &Platform::host(),
        &policy,
        &mut out,
    )
    .expect("the qualified pull succeeds");
    assert_eq!(
        pulled.record.repository,
        format!("{}/{}", fx.endpoint, fx.repository),
        "the record is not keyed on the qualified repository"
    );
    assert_eq!(
        pulled.record.tag.as_deref(),
        Some(fx.tag.as_str()),
        "the record lost the tag it was pulled by"
    );

    cleanup(&dir);
    fx.shutdown();
}

/// The script's reading itself: every row that pulls produces one. Here the
/// reading is the platform the image declares, carried on the record.
#[test]
fn a_row_that_pulls_produces_a_reading() {
    let _held = PULLS.lock().unwrap_or_else(|e| e.into_inner());
    let fx = common::registry::Fixture::start();
    let policy = policy_for(&fx.endpoint);

    let dir = fresh_dir("reading");
    let store = Store::open(&dir).expect("the test opens its store");
    let mut out = Vec::new();
    let pulled = pull::pull(
        &store,
        &fx.reference(),
        &Platform::host(),
        &policy,
        &mut out,
    )
    .expect("the fixture row pulls");
    assert_eq!(
        pulled.record.digest, fx.manifest_digest,
        "the row did not resolve to the seeded digest"
    );
    assert_eq!(
        pulled.record.os, "linux",
        "the row declares os {}, not linux",
        pulled.record.os
    );
    assert_eq!(
        pulled.record.platform,
        Platform::host().to_string(),
        "the row declares platform {}, not the host platform",
        pulled.record.platform
    );

    cleanup(&dir);
    fx.shutdown();
}

/// Script rule 2: rows that could not be pulled count apart from rows whose
/// command ran and failed. One row pulls; two unknown tags are refused as
/// registry errors, record nothing, and never read as broken rows.
#[test]
fn unreachable_rows_count_apart_from_broken_ones() {
    let _held = PULLS.lock().unwrap_or_else(|e| e.into_inner());
    let fx = common::registry::Fixture::start();
    let policy = policy_for(&fx.endpoint);

    let mut ran = 0u32;
    let mut nopull = 0u32;
    let mut broken = 0u32;

    let rows = [
        fx.tag.clone(),
        "no-such-tag-a".to_string(),
        "no-such-tag-b".to_string(),
    ];
    for tag in &rows {
        let want = format!("{}/{}:{tag}", fx.endpoint, fx.repository);
        let dir = fresh_dir("row");
        let store = Store::open(&dir).expect("a row opens its store");
        let mut out = Vec::new();
        match pull::pull(&store, &want, &Platform::host(), &policy, &mut out) {
            Ok(pulled) => {
                ran += 1;
                assert_eq!(
                    pulled.record.digest, fx.manifest_digest,
                    "row {tag} pulled but resolved away from the seed"
                );
            }
            Err(podbox_image::Error::Http { what, detail }) => {
                nopull += 1;
                assert!(
                    what.contains(tag) || detail.contains(tag),
                    "row {tag} was refused without naming the row: {what}: {detail}"
                );
                let records = store.list().expect("the index reads after a refusal");
                if !records.is_empty() {
                    broken += 1;
                }
            }
            Err(other) => {
                broken += 1;
                eprintln!("row {tag} failed outside the registry refusal: {other}");
            }
        }
        cleanup(&dir);
    }
    assert_eq!(ran, 1, "expected one pulled row, got {ran}");
    assert_eq!(nopull, 2, "expected two refused rows, got {nopull}");
    assert_eq!(
        broken, 0,
        "{broken} row(s) pulled and the harness could not run there"
    );

    fx.shutdown();
}

/// The eleven live distribution rows (alpine through archlinux): they need a
/// container engine and network access to Docker Hub, neither of which a
/// hermetic test may assume. A live engine with registry access would reopen
/// this; until then SKIP, never green.
#[test]
fn the_live_distribution_matrix_needs_an_engine_and_a_registry() {
    let _held = PULLS.lock().unwrap_or_else(|e| e.into_inner());
    eprintln!("SKIP: the eleven live rows need a container engine and Docker Hub access; the hermetic discipline around them is the three tests above");
}
