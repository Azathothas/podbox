//! TODO/interpose.md T-1744. The open flags of `crate::oflags` agree with
//! the kernel's `<asm/fcntl.h>` of the machine of the test: x86_64 in
//! `gate.yml`, aarch64 in `arm.yml`. glibc's own `O_LARGEFILE` is 0 on a
//! 64-bit target, so the kernel's header is the reference: its values are
//! what the system call reads.

use crate::oflags::*;

const PROGRAM: &str = r#"
#include <asm/fcntl.h>
#include <stdio.h>
int main(void) {
    printf("%d %d %d %d %d %d %d %d\n", O_DIRECTORY, O_NOFOLLOW, O_DIRECT,
           O_LARGEFILE, O_PATH, O_TMPFILE, O_CLOEXEC, O_CREAT | O_EXCL);
    return 0;
}
"#;

#[test]
fn the_open_flags_agree_with_the_c_headers() {
    let dir = std::env::temp_dir().join(format!("podvm-oflags-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("o.c"), PROGRAM).unwrap();
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let built = std::process::Command::new(&cc)
        .args(["-o", "o", "o.c"])
        .current_dir(&dir)
        .status()
        .unwrap_or_else(|e| panic!("{cc} does not start ({e}): the test needs a C compiler"));
    assert!(built.success(), "{cc} could not build the flag program");
    let out = std::process::Command::new(dir.join("o")).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let got: Vec<i32> = String::from_utf8(out.stdout)
        .unwrap()
        .split_whitespace()
        .map(|w| w.parse().unwrap())
        .collect();
    let ours = [
        O_DIRECTORY,
        O_NOFOLLOW,
        O_DIRECT,
        O_LARGEFILE,
        O_PATH,
        O_TMPFILE,
        O_CLOEXEC,
        O_CREAT | O_EXCL,
    ];
    let names = [
        "O_DIRECTORY",
        "O_NOFOLLOW",
        "O_DIRECT",
        "O_LARGEFILE",
        "O_PATH",
        "O_TMPFILE",
        "O_CLOEXEC",
        "O_CREAT|O_EXCL",
    ];
    assert_eq!(got.len(), ours.len(), "the program printed {got:?}");
    for ((name, want), have) in names.iter().zip(got).zip(ours) {
        assert_eq!(
            have, want,
            "{name}: the interposer has {have:#o}, the header {want:#o}"
        );
    }
}

/// The case of T-1744: an `O_NOFOLLOW` open of a link of the rootfs fails
/// with ELOOP, as the kernel gives it, and does not follow the link. With
/// the x86_64 value on aarch64 the interposer read `O_LARGEFILE` there and
/// followed the link (T-1725).
#[test]
fn an_open_with_o_nofollow_does_not_follow_a_link_of_the_rootfs() {
    use super::*;
    unsafe extern "C" {
        fn close(fd: c_int) -> c_int;
    }
    const ELOOP: c_int = 40;
    let dir = std::env::temp_dir().join(format!("podvm-nofollow-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let root = dir.join("root");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("target"), b"x").unwrap();
    std::os::unix::fs::symlink("/target", root.join("link")).unwrap();
    // SAFETY: the interposer tests run with one thread (gate.yml).
    unsafe { std::env::set_var("PODVM_MAPS", format!("/:{}", root.display())) };
    clear_errno();
    let fd = unsafe { crate::open(c"/link".as_ptr(), O_NOFOLLOW, 0) };
    let e = errno();
    let follows = unsafe { crate::open(c"/link".as_ptr(), 0, 0) };
    unsafe { std::env::remove_var("PODVM_MAPS") };
    if fd >= 0 {
        unsafe { close(fd) };
    }
    if follows >= 0 {
        unsafe { close(follows) };
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(fd < 0 && e == ELOOP, "O_NOFOLLOW: fd {fd}, errno {e}");
    assert!(
        follows >= 0,
        "without O_NOFOLLOW the link resolves in the rootfs"
    );
}
