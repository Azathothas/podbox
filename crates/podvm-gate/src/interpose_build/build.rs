use super::*;

pub(super) fn set_skip(rc: &mut i32) {
    // A failure outranks a skip, so a skip never clears one.
    if *rc != 1 {
        *rc = 2;
    }
}

pub(super) fn build_one(root: &Path, krate: &Path, target: &str, ceiling: &str, tmp: &Path) -> i32 {
    let mut rc = 0;
    if !target_installed(target, root) {
        eprintln!("SKIP {target}: target not installed");
        return 2;
    }
    println!("== {target}");
    let zig_linker = root.join("scripts/zig-cc.sh");
    let mut rustflags = "-C target-feature=-crt-static".to_string();
    let mut stub_env: Vec<(String, String)> = Vec::new();
    if target.contains("musl") {
        if command_present("zig") && zig_linker.is_file() {
            rustflags.push_str(&format!(" -C linker={}", zig_linker.display()));
            let zig_version = capture("zig", &["version".to_string()], root, &[])
                .map(|(_, stdout, _)| stdout.trim().to_string())
                .unwrap_or_default();
            println!("   linker: scripts/zig-cc.sh (zig {zig_version})");
        } else {
            eprintln!("SKIP {target}: no zig, and the default linker produces a GLIBC object");
            eprintln!("      ./scripts/common/bootstrap-env.sh zig");
            return 2;
        }
    } else if target.contains("gnu") {
        // THE `libdl` IMPORT LIBRARY. The `dlsym` reference must bind the
        // library that defines it on old systems, and on a modern host that
        // is not `libc.so.6`. The stub carries SONAME libdl.so.2 with no
        // implementation, so the reference binds libdl.so.2, which glibc
        // keeps as a compatibility object on both sides of the merge. musl
        // has no `libdl`, so this stays on the glibc arm the way `zig cc`
        // stays on the musl one.
        let stub_dir = krate.join("target/dl-stub");
        if fs::create_dir_all(&stub_dir).is_err() {
            eprintln!("SKIP {target}: no stub directory");
            return 2;
        }
        let stub = stub_dir.join("libdl-stub.so");
        let stub_rc = Command::new("cc")
            .args([
                "-shared",
                "-nostdlib",
                &format!("-Wl,--version-script={}", krate.join(DL_STUB_MAP).display()),
                "-Wl,-soname,libdl.so.2",
                "-o",
                &stub.to_string_lossy(),
                &krate.join(DL_STUB_SRC).to_string_lossy(),
            ])
            .current_dir(root)
            .stdin(Stdio::null())
            .status()
            .map(|status| status.code().unwrap_or(1))
            .unwrap_or(127);
        if stub_rc != 0 {
            eprintln!("FAIL {target}: the libdl stub did not build");
            return 1;
        }
        let soname = capture(
            "readelf",
            &["-dW".to_string(), stub.to_string_lossy().into_owned()],
            root,
            &[],
        )
        .and_then(|(_, stdout, _)| {
            stdout
                .lines()
                .find(|line| line.contains("SONAME"))
                .and_then(|line| {
                    let open = line.find('[')?;
                    let close = line[open..].find(']')?;
                    Some(line[open + 1..open + close].to_string())
                })
        })
        .unwrap_or_default();
        println!("   stub: {} (SONAME {soname})", stub.display());
        // PREPENDED THROUGH A LINKER WRAPPER, not a late link argument: user
        // link arguments travel after the objects, so by the time the stub
        // is seen the `dlsym` reference is already bound.
        rustflags.push_str(&format!(
            " -C linker={}",
            root.join("scripts/gnu-link-stub.sh").display()
        ));
        stub_env.push((
            "PODVM_DL_STUB".to_string(),
            stub.to_string_lossy().into_owned(),
        ));
    }
    // THE VERSION SCRIPT. Without it a Rust cdylib exports its runtime
    // into every process it is loaded into, and this object is loaded into
    // every process of a container. An ABSOLUTE path: the build runs with
    // the crate as its working directory and the linker is invoked from
    // somewhere else again.
    rustflags.push_str(&format!(
        " -C link-arg=-Wl,--version-script={}",
        krate.join(MAP).display()
    ));
    let so = krate
        .join("target")
        .join(target)
        .join("release")
        .join("libpodvm_interpose.so");
    let build_rc = Command::new("cargo")
        .args(["build", "--release", "--target", target])
        .env("RUSTFLAGS", &rustflags)
        .envs(stub_env.iter().cloned())
        .current_dir(krate)
        .status()
        .map(|status| status.code().unwrap_or(1))
        .unwrap_or(127);
    if build_rc != 0 {
        eprintln!("FAIL {target}");
        return 1;
    }
    if let Ok(meta) = fs::metadata(&so) {
        println!("   {} {}", meta.len(), so.display());
    }
    let needed = dt_needed(&so, root);
    println!("   DT_NEEDED: {}", needed.join(" "));
    let want = want_libc(target);
    if needed.iter().any(|name| name == want) {
        println!(
            "   ok: it names {want}, so it is {}-linked",
            libc_kind(target)
        );
    } else {
        eprintln!(
            "FAIL {target}: DT_NEEDED is [{}] and must name {want}",
            needed.join(" ")
        );
        rc = 1;
    }
    // THE GLIBC CEILING. No needed version above it may enter.
    let newest = newest_need(&glibc_needs(&so, root));
    match newest {
        Some(top) => {
            if version_at_or_below(&top, GLIBC_CEILING) {
                println!("   ok: newest need {top}, within {GLIBC_CEILING}");
            } else {
                eprintln!("FAIL {target}: needs {top}, above the {GLIBC_CEILING} ceiling");
                rc = 1;
            }
        }
        None => println!("   ok: no versioned needs"),
    }
    // `gettid` MUST NOT LEAVE. The crate defines the number itself as a
    // bare assembly label: local binding by default, no dynamic entry, no
    // payload-visible export. Uppercase only: a localized entry stays in
    // `.dynsym` as `t`, which the loader binds inside the object and no
    // payload resolves.
    let leaked = dynamic_names(&so, root).into_iter().any(|(kind, name)| {
        name == "gettid" && kind.len() == 1 && kind.chars().all(|ch| ch.is_ascii_uppercase())
    });
    if leaked {
        eprintln!("FAIL {target}: gettid is dynamically exported");
        rc = 1;
    } else {
        println!("   ok: gettid is not dynamically exported");
    }
    // `libdl` MUST BE NEEDED on glibc. The SONAME carries its own version,
    // which has nothing to do with the `libc.so.6` beside it.
    if target.contains("gnu") {
        if needed.iter().any(|name| name == "libdl.so.2") {
            println!("   ok: libdl.so.2 is needed");
        } else {
            eprintln!(
                "FAIL {target}: DT_NEEDED is [{}] and names no libdl",
                needed.join(" ")
            );
            rc = 1;
        }
    }
    // THE EXPORTED SET. The version script is the source of this list and
    // the object is what is compared with it: a name in the map that the
    // object does not export is a silent non-interposition, and one the
    // object exports that the map does not list is a symbol some other
    // library in the payload's process resolves to podvm. Without this a
    // link that stopped applying the version script produces a working
    // object exporting hundreds of names and exits 0.
    let declared = declared_exports(&krate.join(MAP));
    let safe_target = target.replace(|ch: char| !ch.is_ascii_alphanumeric(), "_");
    let declared_file = tmp.join(format!("podvm-declared-{safe_target}.txt"));
    let exported_file = tmp.join(format!("podvm-exported-{safe_target}.txt"));
    let _ = fs::write(&declared_file, declared.join("\n") + "\n");
    match exported_symbols(&so, root) {
        Some(exported) => {
            let _ = fs::write(&exported_file, exported.join("\n") + "\n");
            if declared == exported {
                println!(
                    "   ok: exports {} names, exactly what interpose.map declares",
                    exported.len()
                );
            } else {
                eprintln!(
                    "FAIL {target}: exports differ from interpose.map (declared {}, exported {})",
                    declared.len(),
                    exported.len()
                );
                let mut only_declared: Vec<&String> = declared
                    .iter()
                    .filter(|name| !exported.contains(name))
                    .collect();
                let mut only_exported: Vec<&String> = exported
                    .iter()
                    .filter(|name| !declared.contains(name))
                    .collect();
                only_declared.sort();
                only_exported.sort();
                for name in only_declared {
                    eprintln!("      < {name}");
                }
                for name in only_exported {
                    eprintln!("      > {name}");
                }
                rc = 1;
            }
        }
        None => {
            eprintln!(
                "FAIL {target}: nm could not read the exports of {}",
                so.display()
            );
            rc = 1;
        }
    }
    let personalities = dynamic_names(&so, root)
        .into_iter()
        .filter(|(_, name)| name == "rust_eh_personality")
        .count();
    if personalities > 0 {
        eprintln!("FAIL {target}: rust_eh_personality is dynamically exported");
        rc = 1;
    } else {
        println!("   ok: rust_eh_personality is not exported");
    }
    // THE SIZE. This asserts the same ceiling where the object is linked,
    // so every dev check and CI build holds it rather than only a clone
    // reading the record.
    let bytes = fs::metadata(&so).map(|meta| meta.len()).unwrap_or(u64::MAX);
    let limit: u64 = ceiling.parse().unwrap_or(u64::MAX);
    if bytes >= limit {
        eprintln!("FAIL {target}: {bytes} bytes is at or over the interpose ceiling of {ceiling}");
        rc = 1;
    } else {
        println!("   ok: {bytes} bytes, under the interpose ceiling of {ceiling}");
    }
    rc
}

const OFF_C: &str = r#"#define _GNU_SOURCE
#include <stdio.h>
#include <sys/stat.h>
#include <stddef.h>
/* NOT <linux/stat.h>. musl's own <sys/stat.h> defines `struct statx` and the
 * kernel header then redefines it; glibc since 2.28 defines it there too under
 * _GNU_SOURCE. Two libcs declaring one kernel struct in two different headers
 * is exactly why podvm's object carries OFFSETS rather than a struct. */
int main(void){
  printf("sizeof=%zu dev=%zu ino=%zu mode=%zu uid=%zu gid=%zu\n",
    sizeof(struct stat), offsetof(struct stat, st_dev), offsetof(struct stat, st_ino),
    offsetof(struct stat, st_mode), offsetof(struct stat, st_uid),
    offsetof(struct stat, st_gid));
  /* statx too, and it is not a duplicate: coreutils' `stat` on a modern
   * glibc asks statx(2) and never reaches `stat`. src/lib.rs carries these
   * six numbers as well, and a check that asserted only the first struct
   * would have left the ones that actually answer a glibc payload unmeasured. */
  printf("statx sizeof=%zu mask=%zu uid=%zu gid=%zu ino=%zu devmaj=%zu devmin=%zu\n",
    sizeof(struct statx), offsetof(struct statx, stx_mask),
    offsetof(struct statx, stx_uid), offsetof(struct statx, stx_gid),
    offsetof(struct statx, stx_ino), offsetof(struct statx, stx_dev_major),
    offsetof(struct statx, stx_dev_minor));
  return 0;
}
"#;

// Check B. `struct stat` AND `struct statx` OFFSETS, under both libcs, by
// `offsetof` rather than by reading a header. A Linux host runs both
// directly; anywhere else this check could not run, and it says so rather
// than answering from one libc twice.
pub(super) fn check_offsets(root: &Path, tmp: &Path) -> i32 {
    println!("== B. struct stat offsets, by offsetof and under both libcs");
    // NTFS carries no POSIX mode, and chmod on such a checkout is a silent
    // no-op, so the check builds beside the tree rather than assuming the
    // temporary directory honours modes. The binaries built here run on
    // this host directly.
    let source = tmp.join("off.c");
    if fs::write(&source, OFF_C).is_err() {
        println!("  COULD NOT BUILD OR RUN");
        return 2;
    }
    let src = source.to_string_lossy().into_owned();
    let off_g = tmp.join("off_g");
    let off_m = tmp.join("off_m");
    let glibc_built = Command::new("cc")
        .args(["-O1", "-o", &off_g.to_string_lossy(), &src])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
        || (command_present("zig")
            && Command::new(root.join("scripts/zig-cc.sh"))
                .args(["-O1", "-o", &off_g.to_string_lossy(), &src])
                .env("ZIG_TARGET", "x86_64-linux-gnu.2.17")
                .current_dir(root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false));
    let musl_built = command_present("zig")
        && Command::new(root.join("scripts/zig-cc.sh"))
            .args(["-O1", "-o", &off_m.to_string_lossy(), &src])
            .env("ZIG_TARGET", "x86_64-linux-musl")
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    if !glibc_built {
        eprintln!("  the glibc offsetof program did not build (needs cc or zig)");
    }
    if !musl_built {
        eprintln!("  the musl offsetof program did not build (needs zig)");
    }
    // A Linux host runs both directly. Anywhere else they cannot execute
    // here, and the check says so.
    let uname = Command::new("uname")
        .arg("-s")
        .stdin(Stdio::null())
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default();
    let mut g_off = String::new();
    let mut m_off = String::new();
    if uname == "Linux" {
        if glibc_built {
            g_off = run_program(&off_g);
        }
        if musl_built {
            m_off = run_program(&off_m);
        }
    }
    println!(
        "  glibc  {}",
        if g_off.is_empty() {
            "COULD NOT BUILD OR RUN".to_string()
        } else {
            g_off.clone()
        }
    );
    println!(
        "  musl   {}",
        if m_off.is_empty() {
            "COULD NOT BUILD OR RUN".to_string()
        } else {
            m_off.clone()
        }
    );
    // The numbers the interposer carries, asserted here rather than
    // believed. They agree on x86_64 and that is this architecture's
    // property, not a general one.
    if g_off.is_empty() || m_off.is_empty() {
        eprintln!("  one of the two offsetof programs did not build or run");
        2
    } else if g_off == WANT_OFFSETS && m_off == WANT_OFFSETS {
        println!("  B: both libcs agree on BOTH structs, and with the offsets src/lib.rs carries");
        0
    } else {
        eprintln!("  B: the offsets differ from the ones src/lib.rs carries [{WANT_OFFSETS}]");
        1
    }
}

fn run_program(path: &Path) -> String {
    let start = Instant::now();
    let mut child = match Command::new(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return String::new(),
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return String::new();
                }
                let mut buf = Vec::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let _ = pipe.read_to_end(&mut buf);
                }
                return String::from_utf8_lossy(&buf).replace('\n', "|");
            }
            Ok(None) => {
                if start.elapsed() >= Duration::from_secs(60) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return String::new();
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return String::new(),
        }
    }
}

pub(super) fn print_help() {
    println!("podvm-interpose-build: both interposer objects, exports, and size checks.");
    println!();
    println!("  TARGETS=\"...\" podvm-interpose-build   one object per libc, then check B");
    println!();
    println!("Exit: 0 every requested target built, 1 a build failed, 2 could not run.");
}
