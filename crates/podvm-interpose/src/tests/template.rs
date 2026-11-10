use super::*;

type Fd1 = unsafe extern "C" fn(*mut c_char) -> c_int;
type Fd2 = unsafe extern "C" fn(*mut c_char, c_int) -> c_int;
type Fd3 = unsafe extern "C" fn(*mut c_char, c_int, c_int) -> c_int;
type MkdtempFn = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

/// A rootfs whose path is longer than each guest template below, so the
/// mapped name does not fit the template that the payload gives.
struct Rootfs {
    top: std::path::PathBuf,
    dir: std::path::PathBuf,
    guest: String,
}

impl Rootfs {
    fn new(tag: &str) -> Rootfs {
        let top = std::env::temp_dir().join(format!("podvm-{tag}-{}", std::process::id()));
        let mut dir = top.clone();
        for _ in 0..4 {
            dir.push("a-deep-rootfs-directory-name");
        }
        std::fs::create_dir_all(dir.join("tmp")).expect("rootfs");
        let guest = format!("/podvm-{tag}-guest");
        assert!(dir.as_os_str().len() > guest.len(), "the fixture is deep");
        Rootfs { top, dir, guest }
    }

    /// Run `f` with the table `GUEST:ROOTFS` set, and nothing after it.
    fn mapped<R>(&self, f: impl FnOnce() -> R) -> R {
        // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
        unsafe {
            std::env::set_var(
                "PODVM_MAPS",
                format!("{}:{}", self.guest, self.dir.display()),
            )
        };
        let r = f();
        // SAFETY: as above.
        unsafe { std::env::remove_var("PODVM_MAPS") };
        r
    }
}

impl Drop for Rootfs {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.top).ok();
    }
}

/// A NUL-terminated buffer that holds `text`.
fn template(text: &str) -> Vec<u8> {
    let mut buf = text.as_bytes().to_vec();
    buf.push(0);
    buf
}

/// The guest name in `buf` up to its NUL.
fn guest_name(buf: &[u8]) -> String {
    let n = buf.iter().position(|&b| b == 0).expect("NUL");
    String::from_utf8(buf[..n].to_vec()).expect("utf-8")
}

/// Check one descriptor call: the buffer keeps the guest name with only
/// the six `X` before `suffix` replaced, the file is in the rootfs under
/// that name, and bytes written through the descriptor read back there.
fn check_fd(root: &Rootfs, name: &str, want: &str, buf: &[u8], fd: c_int, e: c_int, suffix: &str) {
    assert!(fd >= 0, "{name} answers a descriptor, errno {e}");
    let got = guest_name(buf);
    assert_eq!(
        got.len(),
        want.len(),
        "{name}: the guest name keeps its length"
    );
    let cut = want.len() - suffix.len() - 6;
    assert_eq!(
        &got[..cut],
        &want[..cut],
        "{name}: the head is the guest one"
    );
    assert_eq!(&got[cut + 6..], suffix, "{name}: the suffix stays");
    let run = &got[cut..cut + 6];
    assert_ne!(run, "XXXXXX", "{name}: the X run is replaced");
    let leaf = got.rsplit('/').next().expect("leaf");
    let file = root.dir.join("tmp").join(leaf);
    assert!(
        file.is_file(),
        "{name}: the file is in the rootfs: {}",
        file.display()
    );
    let mut st = std::fs::File::from(unsafe {
        <std::os::fd::OwnedFd as std::os::fd::FromRawFd>::from_raw_fd(fd)
    });
    std::io::Write::write_all(&mut st, b"cache").expect("write");
    assert_eq!(
        std::fs::read(&file).expect("read"),
        b"cache",
        "{name}: fd and file agree"
    );
}

/// One descriptor call through the exported `name`, mapped, with errno.
fn run_fd(
    root: &Rootfs,
    buf: &mut [u8],
    call: impl FnOnce(*mut c_char) -> c_int,
) -> (c_int, c_int) {
    root.mapped(|| {
        clear_errno();
        let fd = call(buf.as_mut_ptr() as *mut c_char);
        (fd, errno())
    })
}

/// T-1699: `mkostemp` of a guest template in a deep rootfs creates the
/// file in the rootfs. The buffer keeps a guest name of its own length,
/// and its `X` run names the created file. The old T-0705 rule answered
/// `ERANGE` here, because the mapped name is longer than the template.
/// T-1701: `mkstemp64` and `mkostemp64` do the same.
#[test]
fn mkostemp_creates_in_the_rootfs_and_keeps_the_guest_name() {
    let root = Rootfs::new("t1699");
    let want = format!("{}/tmp/entry-XXXXXX", root.guest);
    let f: Fd2 = lookup(RTLD_DEFAULT, "mkostemp");
    let mut buf = template(&want);
    let (fd, e) = run_fd(&root, &mut buf, |t| unsafe { f(t, 0) });
    check_fd(&root, "mkostemp", &want, &buf, fd, e, "");
    let f: Fd2 = lookup(RTLD_DEFAULT, "mkostemp64");
    let mut buf = template(&want);
    let (fd, e) = run_fd(&root, &mut buf, |t| unsafe { f(t, 0) });
    check_fd(&root, "mkostemp64", &want, &buf, fd, e, "");
    let f: Fd1 = lookup(RTLD_DEFAULT, "mkstemp64");
    let mut buf = template(&want);
    let (fd, e) = run_fd(&root, &mut buf, |t| unsafe { f(t) });
    check_fd(&root, "mkstemp64", &want, &buf, fd, e, "");
}

/// T-1701: `mkstemps` and `mkostemps` replace the six `X` before the
/// suffix and keep the suffix, in a deep rootfs.
#[test]
fn mkstemps_keeps_the_suffix_and_the_guest_name() {
    let root = Rootfs::new("t1701");
    let want = format!("{}/tmp/entry-XXXXXX.cache", root.guest);
    let f: Fd2 = lookup(RTLD_DEFAULT, "mkstemps");
    let mut buf = template(&want);
    let (fd, e) = run_fd(&root, &mut buf, |t| unsafe { f(t, 6) });
    check_fd(&root, "mkstemps", &want, &buf, fd, e, ".cache");
    let f: Fd3 = lookup(RTLD_DEFAULT, "mkostemps");
    let mut buf = template(&want);
    let (fd, e) = run_fd(&root, &mut buf, |t| unsafe { f(t, 6, 0) });
    check_fd(&root, "mkostemps", &want, &buf, fd, e, ".cache");
}

/// T-1701: a suffix length that is negative, that leaves no room for the
/// run, or that puts a byte other than `X` in the run answers `EINVAL`,
/// as libc does, and the buffer does not change.
#[test]
fn mkstemps_refuses_a_bad_suffix_length() {
    let root = Rootfs::new("t1701e");
    let want = format!("{}/tmp/entry-XXXXXX.cache", root.guest);
    let f: Fd2 = lookup(RTLD_DEFAULT, "mkstemps");
    for len in [-1, 5, 7, want.len() as c_int] {
        let mut buf = template(&want);
        let (fd, e) = run_fd(&root, &mut buf, |t| unsafe { f(t, len) });
        assert_eq!((fd, e), (-1, EINVAL), "suffix length {len}");
        assert_eq!(guest_name(&buf), want, "suffix length {len}");
    }
}

/// T-1699: `mkdtemp` makes the directory in the rootfs and answers the
/// caller's own buffer with a guest name. A template with no `X` run
/// answers `EINVAL`, as libc does, also where the mapped copy loses its
/// trailing slash to normalisation and would end in six `X`.
#[test]
fn mkdtemp_answers_the_guest_template_and_refuses_no_x_run() {
    let root = Rootfs::new("t1699d");
    let want = format!("{}/tmp/dir-XXXXXX", root.guest);
    let mut buf = template(&want);
    let mut bad = template(&format!("{want}/"));
    let f: MkdtempFn = lookup(RTLD_DEFAULT, "mkdtemp");
    let (r, r_bad, e_bad) = root.mapped(|| {
        let r = unsafe { f(buf.as_mut_ptr() as *mut c_char) };
        clear_errno();
        let r_bad = unsafe { f(bad.as_mut_ptr() as *mut c_char) };
        (r, r_bad, errno())
    });
    assert_eq!(r, buf.as_mut_ptr() as *mut c_char, "the caller's buffer");
    let got = guest_name(&buf);
    assert_eq!(got.len(), want.len());
    let run = &got[want.len() - 6..];
    assert_ne!(run, "XXXXXX");
    assert!(root.dir.join("tmp").join(format!("dir-{run}")).is_dir());
    assert!(r_bad.is_null(), "no X run is refused");
    assert_eq!(e_bad, EINVAL);
}
