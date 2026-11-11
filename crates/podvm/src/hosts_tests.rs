//! TODO/deps.md T-1679: each host of another party that podvm's code names
//! is in `pod-services`, podcrates' one list of such hosts.
//!
//! The test reads the Rust sources of the workspace and takes each host of
//! a URL (`https://HOST`) and each registry of an image reference
//! (`"HOST/...`). The repository's own tools (`podvm-gate`,
//! `podvm-release`) and the tests are out of scope: they are the tools of
//! podvm's own repository (the inventory of `pod-services`, "stay").

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Hosts that podvm names and that `pod-services` does not list yet. Each
/// was reported to podcrates (COORD-044); the test fails when the crate
/// lists one, so this list only shrinks.
const WAITING: &[&str] = &[
    // distroless/static-debian12 of the profile distroless-static (T-1659).
    "gcr.io",
    // The mirror of the measure workload for a host where public.ecr.aws
    // answers 429 (T-1687).
    "mirror.gcr.io",
];

/// Hosts that are podvm's own or no host of another party.
const OWN: &[&str] = &["localhost", "example.com", "example.org"];

/// The names that RFC 2606 reserves: no party answers them.
fn reserved(h: &str) -> bool {
    [".test", ".example", ".invalid", ".localhost"]
        .iter()
        .any(|t| h.ends_with(t))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name != "tests" {
                rust_files(&p, out);
            }
        } else if name.ends_with(".rs") && name != "tests.rs" && !name.ends_with("_tests.rs") {
            out.push(p);
        }
    }
}

fn is_host(h: &str) -> bool {
    h.contains('.')
        && h.split('.').all(|l| {
            !l.is_empty()
                && l.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        && h.rsplit('.')
            .next()
            .is_some_and(|tld| tld.bytes().all(|b| b.is_ascii_lowercase()))
}

/// The hosts that `text` names: of a URL, and of a quoted image reference.
fn hosts_of(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for scheme in ["https://", "http://"] {
        for part in text.split(scheme).skip(1) {
            let h: String = part
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
                .collect();
            if is_host(&h) {
                out.insert(h);
            }
        }
    }
    for part in text.split('"').skip(1) {
        if let Some((h, rest)) = part.split_once('/')
            && is_host(h)
            && !rest.starts_with('/')
            && !part.contains(' ')
        {
            out.insert(h.to_string());
        }
    }
    out
}

/// Each host that `pod-services` lists: of an endpoint, and of a fact whose
/// value is a host or a URL.
fn listed() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for s in pod_services::all() {
        for e in s.endpoints() {
            out.insert(e.host().to_string());
        }
        for f in s.facts() {
            let values: Vec<&str> = s
                .texts(f.name())
                .map(|t| t.iter().map(|x| x.as_ref()).collect())
                .or_else(|| s.text(f.name()).map(|v| vec![v]))
                .unwrap_or_default();
            for v in values {
                let v = v
                    .trim_start_matches("https://")
                    .trim_start_matches("http://");
                let h = v.split('/').next().unwrap_or("");
                if is_host(h) {
                    out.insert(h.to_string());
                }
            }
        }
    }
    out
}

#[test]
fn each_host_that_podvm_names_is_in_pod_services() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for e in std::fs::read_dir(&crates).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name == "podvm-gate" || name == "podvm-release" {
            continue;
        }
        rust_files(&e.path().join("src"), &mut files);
    }
    assert!(files.len() > 100, "the scan found {} files", files.len());
    let mut named: BTreeSet<String> = BTreeSet::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        named.extend(hosts_of(&text));
    }
    let listed = listed();
    let missing: Vec<&String> = named
        .iter()
        .filter(|h| !listed.contains(*h) && !OWN.contains(&h.as_str()) && !reserved(h))
        .filter(|h| !WAITING.contains(&h.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "podvm names hosts that pod-services does not list: {missing:?}; report \
         each to podcrates (TODO/deps.md T-1679), podvm keeps no second list"
    );
    for w in WAITING {
        assert!(
            !listed.contains(*w),
            "pod-services lists {w} now: drop it from WAITING"
        );
        assert!(
            named.contains(*w),
            "podvm no longer names {w}: drop it from WAITING"
        );
    }
    // The values that podvm reads resolve.
    assert!(crate::machine::guest::kernel::url().starts_with("https://dl-cdn.alpinelinux.org/"));
    assert_eq!(crate::machine::guest::kernel::kernel_sha256().len(), 64);
    assert_eq!(
        crate::images::default_login_server(),
        "https://index.docker.io/v1/"
    );
}

#[test]
fn the_scan_finds_a_url_and_an_image_reference() {
    let h = hosts_of(
        r#"x = "https://dl-cdn.alpinelinux.org/a"; image = "quay.io/jupyter/b:1"; "a b/c" "//x""#,
    );
    assert_eq!(
        h,
        ["dl-cdn.alpinelinux.org", "quay.io"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    );
}
