//! TODO/cli.md T-1745 (V-1 of PodUtils/podcrates#124): a run through
//! `podvm-api` starts the program, and the call gives back its record.
//!
//! Hermetic, as `labels.rs`: the image is a one-file rootfs that `podvm
//! import` records, and its program is this test binary itself (static
//! musl), which runs on each rung; `--list` prints its tests and exits 0.

use std::path::Path;
use std::process::Command;

fn write_ustar(path: &Path, name: &str, data: &[u8]) {
    let mut hdr = [0u8; 512];
    hdr[..name.len()].copy_from_slice(name.as_bytes());
    hdr[100..108].copy_from_slice(b"0000755\0");
    hdr[108..116].copy_from_slice(b"0000000\0");
    hdr[116..124].copy_from_slice(b"0000000\0");
    hdr[124..136].copy_from_slice(format!("{:011o}\0", data.len()).as_bytes());
    hdr[136..148].copy_from_slice(b"00000000000\0");
    hdr[148..156].copy_from_slice(b"        ");
    hdr[156] = b'0';
    hdr[257..262].copy_from_slice(b"ustar");
    hdr[263..265].copy_from_slice(b"00");
    let sum: u32 = hdr.iter().map(|b| u32::from(*b)).sum();
    hdr[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    let mut tar = hdr.to_vec();
    tar.extend_from_slice(data);
    tar.resize(tar.len() + (512 - data.len() % 512) % 512, 0);
    tar.resize(tar.len() + 1024, 0);
    std::fs::write(path, tar).unwrap();
}

#[test]
fn a_run_through_the_api_gives_its_record() {
    let dir = std::env::temp_dir().join(format!("podvm-api-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let tar = dir.join("r.tar");
    write_ustar(
        &tar,
        "hello",
        &std::fs::read(std::env::current_exe().unwrap()).unwrap(),
    );
    let store = dir.join("store");
    let podvm = Path::new(env!("CARGO_BIN_EXE_podvm"));
    let out = Command::new(podvm)
        .args(["import", tar.to_str().unwrap(), "lab/api:1"])
        .env("PODVM_STORE", &store)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "import: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mut req = podvm_api::Run::new("lab/api:1", &["/hello", "--list"]);
    req.pull = podvm_api::Pull::Never;
    req.env = vec![("PODVM_API_TEST".into(), "a value".into())];
    req.labels = vec![("job".into(), "api".into())];
    let mut job = req.job(podvm).unwrap();
    job.command()
        .env("PODVM_STORE", &store)
        .stdout(std::process::Stdio::null());
    let report = job.run().unwrap();
    assert!(report.succeeded(), "{:?}", report.raw);
    assert_eq!(report.schema, podvm_api::SCHEMA);
    assert_eq!(report.image.as_deref(), Some("lab/api:1"));
    assert!(report.rung.is_some(), "the record names the rung");
    assert_eq!(report.labels, ["job=api"]);
    assert!(
        !report.raw.to_string().contains("a value"),
        "the record holds no value of an environment variable"
    );

    // A refusal also gives a record: an image that the store does not hold.
    let mut none = podvm_api::Run::new("lab/absent:1", &["/hello"]);
    none.pull = podvm_api::Pull::Never;
    let mut job = none.job(podvm).unwrap();
    job.command()
        .env("PODVM_STORE", &store)
        .stderr(std::process::Stdio::null());
    let refused = job.run().unwrap();
    assert!(
        refused.refused.is_some() && refused.exit_code.is_none(),
        "{:?}",
        refused.raw
    );
    let _ = std::fs::remove_dir_all(&dir);
}
