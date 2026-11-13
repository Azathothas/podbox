//! T-1703: the descriptor routes, the foreign ELF, and `system` and
//! `popen` through the routed spawn. The suite runs with one thread
//! (gate.yml), so a test may set the variables of the table.

use super::descriptor::{fd_path, is_link, route_fd, Names};
use super::tests::{elf, put, strings, tree};
use super::*;
use std::ffi::CString;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::symlink;

unsafe extern "C" {
    fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
    fn memfd_create(name: *const c_char, flags: u32) -> c_int;
    fn write(fd: c_int, buf: *const core::ffi::c_void, n: usize) -> isize;
    fn fgets(buf: *mut c_char, n: c_int, f: *mut core::ffi::c_void) -> *mut c_char;
    fn fputs(s: *const c_char, f: *mut core::ffi::c_void) -> c_int;
}

const F_GETFD: c_int = 1;
const FD_CLOEXEC: c_int = 1;

/// The launch of a descriptor route: path, argv and envp.
fn launch_fd(
    fd: c_int,
    args: &[&str],
    n: &mut Names,
) -> Result<(String, Vec<String>, Vec<String>), c_int> {
    let a: Vec<CString> = args.iter().map(|s| CString::new(*s).unwrap()).collect();
    let mut av: Vec<*const c_char> = a.iter().map(|s| s.as_ptr()).collect();
    av.push(core::ptr::null());
    let e = [
        CString::new("A=1").unwrap(),
        CString::new("PODVM_GUEST_EXE=/bin/sh").unwrap(),
    ];
    let ev = [e[0].as_ptr(), e[1].as_ptr(), core::ptr::null()];
    let mut s = Box::new(Scratch::new());
    match unsafe { route_fd(b"fexecve", fd, av.as_ptr(), ev.as_ptr(), &mut s, n) } {
        Route::Launch(l) => {
            let path = unsafe { std::ffi::CStr::from_ptr(l.path) }
                .to_string_lossy()
                .into_owned();
            Ok((path, strings(l.argv), strings(l.envp)))
        }
        Route::Pass(_) => Ok(("pass".to_string(), Vec::new(), Vec::new())),
        Route::Fail => Err(crate::errno()),
    }
}

/// The table of one test. The test binary defines the interposed entry
/// points, so each file call of the test goes through the table while it
/// is set: a test opens its host files before, and the drop clears the
/// table also where an assertion fails.
struct Table1703;

fn set_table(root: &str, lib: &str) -> Table1703 {
    unsafe {
        std::env::set_var("PODVM_MAPS", format!("/:{root}"));
        std::env::set_var("PODVM_LIB_PATH", lib);
    }
    Table1703
}

impl Drop for Table1703 {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("PODVM_MAPS");
            std::env::remove_var("PODVM_LIB_PATH");
        }
    }
}

/// `fexecve` of a file with a guest name enters through the guest loader
/// as an `execve` of the name; a `memfd` enters through
/// `/proc/self/fd/N`, which keeps its descriptor over the exec and gets
/// `FD_CLOEXEC` back where the exec fails. With no table the call passes.
#[test]
fn a_descriptor_enters_through_the_guest_loader() {
    let d = tree("t1703-fd");
    let root = d.to_string_lossy().to_string();
    let lib = format!("{root}/lib");
    put(&d.join("lib/ld-test.so"), b"\x7fELF");
    let prog = elf(3, b"/lib/ld-test.so\0");
    put(&d.join("bin/prog"), &prog);
    let host = |g: &str| format!("{root}{g}");
    let f = std::fs::File::open(d.join("bin/prog")).unwrap();
    let table = set_table(&root, &lib);

    let mut n = Names::new();
    let (path, argv, envp) = launch_fd(f.as_raw_fd(), &["prog", "a1"], &mut n).unwrap();
    assert_eq!(path, host("/lib/ld-test.so"));
    assert_eq!(
        argv,
        [
            &host("/lib/ld-test.so"),
            "--library-path",
            &lib,
            "--argv0",
            "prog",
            &host("/bin/prog"),
            "a1"
        ]
    );
    // T-1756: the table that the caller's envp lacks comes with the exec.
    let maps = format!("PODVM_MAPS=/:{root}");
    let lib_var = format!("PODVM_LIB_PATH={lib}");
    assert_eq!(envp, ["A=1", "PODVM_GUEST_EXE=/bin/prog", &maps, &lib_var]);

    let name = CString::new("t1703").unwrap();
    let m = unsafe { memfd_create(name.as_ptr(), 1) }; // MFD_CLOEXEC
    assert!(m >= 0);
    assert_eq!(
        unsafe { write(m, prog.as_ptr().cast(), prog.len()) },
        prog.len() as isize
    );
    let mut n = Names::new();
    let (path, argv, _) = launch_fd(m, &["mem", "b1"], &mut n).unwrap();
    let mut fdp = [0u8; 32];
    let k = fd_path(b"/proc/self/fd/", m, &mut fdp);
    let fdp = String::from_utf8_lossy(&fdp[..k]).into_owned();
    assert_eq!(path, host("/lib/ld-test.so"));
    assert_eq!(
        argv,
        [
            &host("/lib/ld-test.so"),
            "--library-path",
            &lib,
            "--argv0",
            "mem",
            &fdp,
            "b1"
        ]
    );
    assert_eq!(
        unsafe { fcntl(m, F_GETFD, 0) } & FD_CLOEXEC,
        0,
        "the loader must open the descriptor after the exec"
    );
    n.exec_failed();
    assert_eq!(unsafe { fcntl(m, F_GETFD, 0) } & FD_CLOEXEC, FD_CLOEXEC);

    drop(table);
    let mut n = Names::new();
    assert_eq!(launch_fd(m, &["mem"], &mut n).unwrap().0, "pass");
    drop(unsafe { std::fs::File::from_raw_fd(m) });
    let _ = std::fs::remove_dir_all(&d);
}

/// An ELF file of another machine fails with `ENOEXEC`, never a host run.
/// `AT_SYMLINK_NOFOLLOW` sees a link as its last component.
#[test]
fn a_foreign_elf_refuses_and_nofollow_sees_a_link() {
    let d = tree("t1703-foreign");
    let root = d.to_string_lossy().to_string();
    let mut arm = elf(3, b"/lib/ld-linux-aarch64.so.1\0");
    arm[18] = super::tests::FOREIGN;
    put(&d.join("bin/arm"), &arm);
    put(&d.join("bin/plain"), &elf(1, b""));
    symlink("/bin/plain", d.join("bin/alias")).unwrap();
    let f = std::fs::File::open(d.join("bin/arm")).unwrap();
    let table = set_table(&root, &format!("{root}/lib"));
    let mut n = Names::new();
    assert_eq!(launch_fd(f.as_raw_fd(), &["arm"], &mut n), Err(ENOEXEC));
    let mut buf = [0u8; OUT];
    let alias = CString::new("/bin/alias").unwrap();
    let plain = CString::new("/bin/plain").unwrap();
    assert!(unsafe { is_link(AT_FDCWD, alias.as_ptr(), &mut buf) });
    assert!(!unsafe { is_link(AT_FDCWD, plain.as_ptr(), &mut buf) });
    drop(table);
    let _ = std::fs::remove_dir_all(&d);
}

/// Copy the host file at `from`, its links followed, to `to`.
fn copy_real(from: &str, to: &std::path::Path) -> bool {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(from, to).is_ok()
}

/// `system` and `popen` start the image's `/bin/sh` through the image's
/// loader. The rootfs holds a copy of the host's dynamic shell, its
/// loader and its libc; the running shell's `/proc/PID/exe` is the
/// rootfs loader, not the host's.
#[test]
fn system_and_popen_start_the_image_shell() {
    let d = tree("t1703-shell");
    let root = d.to_string_lossy().to_string();
    // The loader and the libc of this machine (T-1721).
    #[cfg(target_arch = "x86_64")]
    let (ld, libc) = (
        "/lib64/ld-linux-x86-64.so.2",
        "/lib/x86_64-linux-gnu/libc.so.6",
    );
    #[cfg(target_arch = "aarch64")]
    let (ld, libc) = (
        "/lib/ld-linux-aarch64.so.1",
        "/lib/aarch64-linux-gnu/libc.so.6",
    );
    let ok = copy_real("/bin/sh", &d.join("bin/sh"))
        && copy_real(ld, &d.join(&ld[1..]))
        && copy_real(libc, &d.join("lib/libc.so.6"));
    if !ok {
        eprintln!("no glibc /bin/sh on this host: the shell clause did not run");
        return;
    }
    // The test's own file calls go through the table, and its readlink
    // answers the guest name, which is the host's name too. The inode
    // tells the two loaders apart.
    let ino =
        |p: &std::path::Path| std::os::unix::fs::MetadataExt::ino(&std::fs::metadata(p).unwrap());
    let (guest_ld, host_ld) = (ino(&d.join(&ld[1..])), ino(std::path::Path::new(ld)));
    let table = set_table(&root, &format!("{root}/lib"));
    let exit3 = CString::new("exit 3").unwrap();
    let st = unsafe { super::shell::system(exit3.as_ptr()) };
    let nul = unsafe { super::shell::system(core::ptr::null()) };

    // A shell that waits on its input: its exe is readable while it runs.
    let cmd = CString::new("read x; echo got-$x").unwrap();
    let w = CString::new("w").unwrap();
    let f = unsafe { super::shell::popen(cmd.as_ptr(), w.as_ptr()) };
    assert!(!f.is_null(), "popen w: errno {}", crate::errno());
    std::thread::sleep(std::time::Duration::from_millis(300));
    // The shell is a child of the thread that ran popen: read each task.
    let child: String = std::fs::read_dir("/proc/self/task")
        .unwrap()
        .filter_map(|e| std::fs::read_to_string(e.ok()?.path().join("children")).ok())
        .collect::<Vec<_>>()
        .join(" ");
    // `/proc` passes the table: the metadata is the running loader's.
    let exe: Vec<u64> = child
        .split_whitespace()
        .filter_map(|p| std::fs::metadata(format!("/proc/{p}/exe")).ok())
        .map(|m| std::os::unix::fs::MetadataExt::ino(&m))
        .collect();
    let line = CString::new("hi\n").unwrap();
    unsafe { fputs(line.as_ptr(), f) };
    let wst = unsafe { super::shell::pclose(f) };

    let cmd = CString::new("echo out-$((6*7))").unwrap();
    let r = CString::new("re").unwrap();
    let f = unsafe { super::shell::popen(cmd.as_ptr(), r.as_ptr()) };
    assert!(!f.is_null(), "popen r: errno {}", crate::errno());
    let mut got = [0u8; 64];
    unsafe { fgets(got.as_mut_ptr() as *mut c_char, 64, f) };
    let rst = unsafe { super::shell::pclose(f) };
    drop(table);
    let _ = std::fs::remove_dir_all(&d);

    assert_eq!(st, 3 << 8, "system(\"exit 3\")");
    assert_eq!(nul, 1);
    assert!(
        exe.contains(&guest_ld),
        "the shell's exe inodes {exe:?}, the rootfs loader {guest_ld}, the host's {host_ld}"
    );
    assert_eq!(wst, 0);
    let got = std::ffi::CStr::from_bytes_until_nul(&got)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!((got.as_str(), rst), ("out-42\n", 0));
}
