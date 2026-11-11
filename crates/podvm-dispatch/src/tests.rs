//! TODO/enter.md T-1652: the parts of the handler that need no payload.

use crate::elf;
use crate::path::fd_link;

#[test]
fn the_proc_link_of_a_descriptor_and_of_the_cwd() {
    let mut b = [0u8; 32];
    assert_eq!(fd_link(1003, &mut b), b"/proc/self/fd/1003\0");
    assert_eq!(fd_link(0, &mut b), b"/proc/self/fd/0\0");
    assert_eq!(fd_link(crate::sys::AT_FDCWD, &mut b), b"/proc/self/cwd\0");
}

#[test]
fn a_shebang_gives_the_interpreter_and_one_argument() {
    assert_eq!(
        elf::shebang(b"#!/bin/sh\n"),
        Some((b"/bin/sh".to_vec(), None))
    );
    assert_eq!(
        elf::shebang(b"#! /usr/bin/env  python3 \r\nx"),
        Some((b"/usr/bin/env".to_vec(), Some(b"python3".to_vec())))
    );
    assert_eq!(elf::shebang(b"#!\n"), None);
    assert_eq!(elf::shebang(b"\x7fELF"), None);
}

#[test]
fn the_check_takes_a_static_program_and_refuses_a_dynamic_one() {
    let me = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    let read = |off: u64, out: &mut [u8]| {
        let s = me
            .get(off as usize..off as usize + out.len())
            .unwrap_or(&[]);
        out[..s.len()].copy_from_slice(s);
        s.len() as i64
    };
    assert_eq!(
        elf::static_x86_64(&me[..64], read),
        Ok(()),
        "this test binary is static"
    );
    let sh = std::fs::read("/bin/sh").unwrap_or_default();
    if sh.len() > 64 {
        let read = |off: u64, out: &mut [u8]| {
            let s = sh
                .get(off as usize..off as usize + out.len())
                .unwrap_or(&[]);
            out[..s.len()].copy_from_slice(s);
            s.len() as i64
        };
        assert_eq!(
            elf::static_x86_64(&sh[..64], read),
            Err("a dynamic program (it names an interpreter)")
        );
    }
    assert!(elf::static_x86_64(b"#!/bin/sh\n", |_, _| 0).is_err());
}

/// T-1657: the pure parts of the syscall tier.
#[test]
fn the_tier_counts_growth_and_names_its_release() {
    use crate::tier::*;
    assert!(under_tmp(b"/tmp") && under_tmp(b"/tmp/a/b"));
    assert!(!under_tmp(b"/tmpx") && !under_tmp(b"/var/tmp"));
    assert_eq!(growth(0, 10, 0), 10);
    assert_eq!(
        growth(5, 10, 20),
        0,
        "an overwrite inside the file grows nothing"
    );
    assert_eq!(growth(15, 10, 20), 5);
    assert!(fits(990, 10, 1000) && !fits(991, 10, 1000));
    assert!(fits(5000, 10, 0), "no cap");
    assert!(fits(5000, -10, 1000), "a shrink always fits");
    let mut u = [b'z'; 390];
    fill_uname(&mut u);
    assert_eq!(&u[130..130 + RELEASE.len()], RELEASE);
    assert_eq!(u[130 + RELEASE.len()], 0);
    assert_eq!(&u[0..5], b"zzzzz", "the other fields stay the host's");
}
