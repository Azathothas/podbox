#!/bin/sh
# T-0415 probe (ignored, not committed). Measures what devices::apply does
# with a non-regular shape at a device path, on a machine that refuses
# mknod the way production does. The test module is written into the GUEST
# copy only; this tree gains no file.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
cat > crates/podbox-complete/src/t0415probe.rs <<'EOF'
use crate::write::Root;
use crate::Report;

fn scratch(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("podbox-t0415-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(p.join("dev")).unwrap();
    std::fs::create_dir_all(p.join("etc")).unwrap();
    std::fs::write(p.join("etc/hostname"), b"inside").unwrap();
    std::fs::write(p.join("outside-target"), b"outside").unwrap();
    p
}

fn kindname(root: &Root, rel: &str) -> String {
    match root.kind(rel) {
        Ok(k) => match k {
            crate::write::Kind::Missing => "missing".into(),
            crate::write::Kind::Regular { len, .. } => format!("regular({len})"),
            crate::write::Kind::Dir => "dir".into(),
            crate::write::Kind::Symlink(t) => format!("symlink({t})"),
            crate::write::Kind::CharDev { major, minor } => format!("char({major}:{minor})"),
            crate::write::Kind::Other(m) => format!("other({m:o})"),
        },
        Err(e) => format!("KINDERR {e}"),
    }
}

fn show(tag: &str, d: &std::path::Path) {
    let root = Root::open(d.to_str().unwrap()).unwrap();
    let before = kindname(&root, "dev/null");
    let mut r = Report::default();
    crate::devices::apply(&root, &mut r);
    let rows: Vec<String> = r
        .fixups
        .iter()
        .filter(|f| f.entry == "T-0401" && f.path == "dev/null")
        .map(|f| {
            format!(
                "{:?}/degraded={}/{}",
                f.action,
                f.degraded,
                f.detail.chars().take(80).collect::<String>()
            )
        })
        .collect();
    let after = kindname(&root, "dev/null");
    println!("T0415 {tag} before={before} rows={rows:?} after={after}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn probe_shapes_at_dev_null() {
    println!("T0415 uid={} (mknod refused here means the shim arm)", unsafe { libc_uid() });
    let d = scratch("clean");
    show("clean", &d);
    let d = scratch("dir");
    std::fs::create_dir(d.join("dev/null")).unwrap();
    show("dir", &d);
    let d = scratch("sym-in");
    std::os::unix::fs::symlink("etc/hostname", d.join("dev/null")).unwrap();
    show("sym-in", &d);
    let d = scratch("sym-out");
    std::os::unix::fs::symlink(d.join("outside-target"), d.join("dev/null")).unwrap();
    show("sym-out", &d);
    let d = scratch("sym-dangle");
    std::os::unix::fs::symlink("nope-absent", d.join("dev/null")).unwrap();
    show("sym-dangle", &d);
    let d = scratch("fifo");
    assert!(std::process::Command::new("mkfifo").arg(d.join("dev/null")).status().unwrap().success());
    show("fifo", &d);
}

extern "C" {
    fn getuid() -> u32;
}
unsafe fn libc_uid() -> u32 {
    unsafe { getuid() }
}
EOF
printf '\n#[cfg(test)] mod t0415probe;\n' >> crates/podbox-complete/src/lib.rs
cargo test -p podbox-complete --lib t0415 -- --nocapture 2>&1 | grep -E "^T0415|test result|^error" | head -20
echo "== probe job done"
