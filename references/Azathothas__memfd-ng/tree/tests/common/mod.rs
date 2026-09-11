#![allow(dead_code)] // Each test binary uses part of this module.

//! Provide shared test programs and environment controls.

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, Once};

static SERIAL: Mutex<()> = Mutex::new(());

/// Lock environment changes across tests.
///
/// Recover the guard if an earlier panic poisoned the mutex.
pub fn serial() -> MutexGuard<'static, ()> {
    match SERIAL.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Hold the serial lock and record environment changes.
///
/// `Drop` restores each changed variable. This prevents one test from
/// changing the environment of another test.
pub struct EnvGuard {
    saved: Vec<(OsString, Option<OsString>)>,
    _serial: MutexGuard<'static, ()>,
}

impl EnvGuard {
    /// Take the serial lock and apply `vars`. Each earlier value is saved.
    pub fn set(vars: &[(&str, &str)]) -> Self {
        let mut g = EnvGuard {
            saved: Vec::new(),
            _serial: serial(),
        };
        for (k, v) in vars {
            g.apply(k, Some(v));
        }
        g
    }

    /// Take the serial lock without changing an environment variable.
    pub fn empty() -> Self {
        EnvGuard {
            saved: Vec::new(),
            _serial: serial(),
        }
    }

    /// Set one variable and save its earlier value.
    pub fn put(&mut self, key: &str, value: &str) -> &mut Self {
        self.apply(key, Some(value));
        self
    }

    /// Remove one variable and save its earlier value.
    pub fn unset(&mut self, key: &str) -> &mut Self {
        self.apply(key, None);
        self
    }

    fn apply(&mut self, key: &str, value: Option<&str>) {
        self.saved.push((OsString::from(key), env::var_os(key)));
        match value {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // Restore in reverse order to support a key that changed more than once.
        for (key, prev) in self.saved.iter().rev() {
            match prev {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }
    }
}

/// Create an empty test directory under `CARGO_TARGET_TMPDIR`.
///
/// The target file system permits execution. This avoids assumptions about
/// the mount options for system directories.
pub fn exec_tmpdir(tag: &str) -> PathBuf {
    let dir = target_tmpdir().join(format!("ladder-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create exec tmpdir");
    dir
}

fn target_tmpdir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

/// Compile `code` with the selected C compiler and return the binary path.
fn cc_build(name: &str, code: &str, link_static: bool) -> PathBuf {
    // Use a path for this process. Test binaries can compile fixtures at the
    // same time. Separate paths prevent execution of a partial file.
    let pid = std::process::id();
    let dir = target_tmpdir();
    std::fs::create_dir_all(&dir).expect("create fixture tmpdir");
    let src = dir.join(format!("{name}.{pid}.c"));
    let bin = dir.join(format!("{name}.{pid}"));
    std::fs::write(&src, code).expect("write fixture source");
    // MEMFD_NG_TEST_CC lets cross-environments (qemu-user CI) pick the guest
    // toolchain; fixtures must match the architecture of the test binary.
    let cc = env::var("MEMFD_NG_TEST_CC").unwrap_or_else(|_| "cc".to_string());
    let mut cmd = Command::new(cc);
    if link_static {
        cmd.arg("-static");
    }
    let ok = cmd
        .args(["-O2", "-o"])
        .arg(&bin)
        .arg(&src)
        .output()
        .expect("cc is required to build test fixtures")
        .status
        .success();
    assert!(ok, "cc failed to build {name}");
    bin
}

const STUB_SRC: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <signal.h>
/* stub <mode> [args...]
   print   : write argv[2..] space-joined + newline
   exit N  : exit with N
   env V   : print getenv(V) or "(unset)"
   pwd     : print cwd
   fd-target S : count descriptors whose procfs target contains S
   cat     : copy stdin to stdout, exit 0
   sleep   : sleep 60 (for kill tests)
   pgroup  : print "pgid=<getpgid(0)> sid=<getsid(0)>"
   crash   : raise(SIGSEGV) (dies by signal, for 128+signal propagation)
*/
int main(int argc, char **argv, char **envp) {
    (void)envp;
    if (argc < 2) return 64;
    if (!strcmp(argv[1], "print")) {
        for (int i = 2; i < argc; i++) printf("%s%s", argv[i], i + 1 < argc ? " " : "");
        printf("\n");
        return 0;
    }
    if (!strcmp(argv[1], "exit")) return argc > 2 ? atoi(argv[2]) : 0;
    if (!strcmp(argv[1], "env")) {
        const char *v = argc > 2 ? getenv(argv[2]) : NULL;
        puts(v ? v : "(unset)");
        return 0;
    }
    if (!strcmp(argv[1], "pwd")) {
        char buf[4096];
        puts(getcwd(buf, sizeof buf) ? buf : "(fail)");
        return 0;
    }
    if (!strcmp(argv[1], "fd-target")) {
        int n = 0;
        const char *needle = argc > 2 ? argv[2] : "";
        for (int fd = 3; fd < 1024; fd++) {
            char path[64], target[4096];
            snprintf(path, sizeof path, "/proc/self/fd/%d", fd);
            ssize_t len = readlink(path, target, sizeof target - 1);
            if (len > 0) {
                target[len] = '\0';
                if (strstr(target, needle)) n++;
            }
        }
        printf("%d\n", n);
        return 0;
    }
    if (!strcmp(argv[1], "cat")) {
        char buf[4096];
        ssize_t n;
        while ((n = read(0, buf, sizeof buf)) > 0) write(1, buf, n);
        return 0;
    }
    if (!strcmp(argv[1], "sleep")) { sleep(60); return 0; }
    if (!strcmp(argv[1], "pgroup")) {
        printf("pgid=%d sid=%d\n", (int)getpgid(0), (int)getsid(0));
        return 0;
    }
    if (!strcmp(argv[1], "crash")) { raise(SIGSEGV); return 0; }
    return 64;
}
"#;

pub fn static_stub() -> &'static PathBuf {
    static INIT: Once = Once::new();
    static mut SLOT: Option<PathBuf> = None;
    unsafe {
        INIT.call_once(|| {
            (*core::ptr::addr_of_mut!(SLOT)) =
                Some(cc_build("memfd_ng_stub_static", STUB_SRC, true))
        });
        (*core::ptr::addr_of!(SLOT)).as_ref().unwrap()
    }
}

pub fn dynamic_stub() -> &'static PathBuf {
    static INIT: Once = Once::new();
    static mut SLOT: Option<PathBuf> = None;
    unsafe {
        INIT.call_once(|| {
            (*core::ptr::addr_of_mut!(SLOT)) =
                Some(cc_build("memfd_ng_stub_dynamic", STUB_SRC, false))
        });
        (*core::ptr::addr_of!(SLOT)).as_ref().unwrap()
    }
}

pub fn stub_code() -> &'static Vec<u8> {
    static INIT: Once = Once::new();
    static mut SLOT: Option<Vec<u8>> = None;
    unsafe {
        INIT.call_once(|| {
            (*core::ptr::addr_of_mut!(SLOT)) =
                Some(std::fs::read(static_stub()).expect("read static stub"))
        });
        (*core::ptr::addr_of!(SLOT)).as_ref().unwrap()
    }
}

pub fn dynamic_code() -> &'static Vec<u8> {
    static INIT: Once = Once::new();
    static mut SLOT: Option<Vec<u8>> = None;
    unsafe {
        INIT.call_once(|| {
            (*core::ptr::addr_of_mut!(SLOT)) =
                Some(std::fs::read(dynamic_stub()).expect("read dynamic stub"))
        });
        (*core::ptr::addr_of!(SLOT)).as_ref().unwrap()
    }
}

/// Hand-assembled x86_64 ELF64: 64-byte ehdr + 56-byte phdr + `exit(42)`.
/// No toolchain, no libc, fully deterministic — the smallest real image.
#[cfg(target_arch = "x86_64")]
pub const TINY_ELF_EXIT42: &[u8] = &[
    // ehdr
    0x7f, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x02, 0x00, 0x3e, 0x00, 0x01, 0x00,
    0x00, 0x00, 0x78, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, // e_entry 0x400078
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_phoff 64
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_shoff 0
    0x00, 0x00, 0x00, 0x00, // e_flags
    0x40, 0x00, // e_ehsize 64
    0x38, 0x00, // e_phentsize 56
    0x01, 0x00, // e_phnum 1
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // phdr @64: PT_LOAD R+X, vaddr 0x400000, filesz/memsz 0x90
    0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x88, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // code @0x78: mov eax,60; mov edi,42; syscall
    0xb8, 0x3c, 0x00, 0x00, 0x00, 0xbf, 0x2a, 0x00, 0x00, 0x00, 0x0f, 0x05,
];

/// File prefix used by the temporary-file sequence.
pub const FALLBACK_PREFIX: &str = ".memfd-ng-";

/// Return true when the running Linux kernel is at least `major.minor`.
pub fn kernel_at_least(major: u64, minor: u64) -> bool {
    let info = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
    let mut parts = info.split('.');
    let k_major: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let k_minor: u64 = parts
        .next()
        .and_then(|p| p.trim_start_matches("0").parse().ok())
        .unwrap_or(0);
    (k_major, k_minor) >= (major, minor)
}

/// Remove temporary files from earlier test runs.
pub fn clear_stale_fallback_files(dir: &PathBuf) {
    for entry in std::fs::read_dir(dir)
        .expect("read dir")
        .filter_map(|e| e.ok())
    {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(FALLBACK_PREFIX)
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Fail when temporary files remain in the given directory.
pub fn assert_no_fallback_leftovers(dir: &PathBuf) {
    let leftovers: Vec<_> = std::fs::read_dir(dir)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(FALLBACK_PREFIX))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temporary files remain: {leftovers:?}"
    );
}

pub fn tmpdir() -> PathBuf {
    env::temp_dir()
}
