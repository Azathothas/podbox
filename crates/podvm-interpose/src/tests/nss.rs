//! TODO/interpose.md T-1753: with a table, the lookups of users, groups and
//! hosts read the image's files, and a name that only the host has is not
//! found (users and groups) or goes to libc (hosts).

use super::*;
use crate::nss::{AddrInfo, Group, Hostent, Passwd};
use std::ffi::{CStr, CString};

unsafe extern "C" {
    fn freeaddrinfo(ai: *mut AddrInfo);
}

/// The fields of a `struct passwd`, read through its layout.
fn pw_name(p: *mut Passwd) -> (String, u32, String) {
    #[repr(C)]
    struct View {
        name: *const c_char,
        passwd: *const c_char,
        uid: c_uint,
        gid: c_uint,
        gecos: *const c_char,
        dir: *const c_char,
        shell: *const c_char,
    }
    let v = unsafe { &*(p as *const View) };
    let s = |c: *const c_char| unsafe { CStr::from_ptr(c) }.to_string_lossy().into_owned();
    (s(v.name), v.uid, s(v.dir))
}

#[test]
fn the_lookups_read_the_image_files() {
    let dir = std::env::temp_dir().join(format!("podvm-nss-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let etc = dir.join("etc");
    std::fs::create_dir_all(&etc).unwrap();
    std::fs::write(
        etc.join("passwd"),
        "# the image's users\nroot:x:0:0:root:/root:/bin/sh\nqanode:x:4242:4242::/home/qanode:/bin/sh\n",
    )
    .unwrap();
    std::fs::write(
        etc.join("group"),
        "root:x:0:\nqanode:x:4242:\nqateam:x:4300:other,qanode\nempty:x:4301:\n",
    )
    .unwrap();
    std::fs::write(
        etc.join("hosts"),
        "127.0.0.1 localhost\n10.1.2.3 qafoo qafoo.alias # --add-host\n::1 qasix\n",
    )
    .unwrap();
    // SAFETY: the interposer tests run with one thread (gate.yml).
    unsafe { std::env::set_var("PODVM_MAPS", format!("/:{}", dir.display())) };
    let c = |s: &str| CString::new(s).unwrap();

    // Users: by id, by name, through `_r`, and a name only the host has.
    let p = unsafe { crate::getpwuid(4242) };
    assert!(!p.is_null(), "getpwuid 4242");
    assert_eq!(pw_name(p), ("qanode".into(), 4242, "/home/qanode".into()));
    let p = unsafe { crate::getpwnam(c("qanode").as_ptr()) };
    assert!(!p.is_null() && pw_name(p).1 == 4242, "getpwnam");
    assert!(
        unsafe { crate::getpwuid(65530) }.is_null(),
        "a uid the image lacks"
    );
    let mut pw = std::mem::MaybeUninit::<Passwd>::uninit();
    let mut buf = [0 as c_char; 256];
    let mut out: *mut Passwd = core::ptr::null_mut();
    let rc = unsafe {
        crate::getpwnam_r(
            c("root").as_ptr(),
            pw.as_mut_ptr(),
            buf.as_mut_ptr(),
            buf.len(),
            &mut out,
        )
    };
    assert_eq!(rc, 0);
    assert!(
        !out.is_null() && pw_name(out).0 == "root",
        "getpwnam_r root"
    );
    let mut tiny = [0 as c_char; 8];
    let rc = unsafe {
        crate::getpwuid_r(
            4242,
            pw.as_mut_ptr(),
            tiny.as_mut_ptr(),
            tiny.len(),
            &mut out,
        )
    };
    assert_eq!((rc, out.is_null()), (34, true), "ERANGE for a small buffer");
    let mut names = Vec::new();
    unsafe { crate::setpwent() };
    loop {
        let p = unsafe { crate::getpwent() };
        if p.is_null() {
            break;
        }
        names.push(pw_name(p).0);
    }
    assert_eq!(names, ["root", "qanode"], "getpwent walks the image's file");

    // Groups, and the group list of a user.
    let g = unsafe { crate::getgrnam(c("qateam").as_ptr()) };
    assert!(!g.is_null(), "getgrnam");
    #[repr(C)]
    struct GView {
        name: *const c_char,
        passwd: *const c_char,
        gid: c_uint,
        mem: *const *const c_char,
    }
    let v = unsafe { &*(g as *const GView) };
    assert_eq!(v.gid, 4300);
    let mem: Vec<String> = (0..)
        .map(|i| unsafe { *v.mem.add(i) })
        .take_while(|p| !p.is_null())
        .map(|p| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
        .collect();
    assert_eq!(mem, ["other", "qanode"]);
    assert!(
        !unsafe { crate::getgrgid(4301) }.is_null(),
        "getgrgid of an empty group"
    );
    let _: *mut Group = unsafe { crate::getgrgid(0) };
    let mut list = [0 as c_uint; 8];
    let mut n: c_int = 8;
    let k = unsafe { crate::getgrouplist(c("qanode").as_ptr(), 4242, list.as_mut_ptr(), &mut n) };
    assert_eq!((k, &list[..2]), (2, &[4242, 4300][..]), "getgrouplist");
    let mut n: c_int = 1;
    let k = unsafe { crate::getgrouplist(c("qanode").as_ptr(), 4242, list.as_mut_ptr(), &mut n) };
    assert_eq!((k, n), (-1, 2), "a short list asks for 2");

    // Hosts: the image's line, its family, and the list of getaddrinfo.
    let h = unsafe { crate::gethostbyname(c("QAFOO").as_ptr()) };
    assert!(!h.is_null(), "gethostbyname from the image's hosts");
    #[repr(C)]
    struct HView {
        name: *const c_char,
        aliases: *const *const c_char,
        addrtype: c_int,
        length: c_int,
        addr_list: *const *const u8,
    }
    let v = unsafe { &*(h as *const HView) };
    let addr = unsafe { std::slice::from_raw_parts(*v.addr_list, 4) };
    assert_eq!((v.addrtype, v.length, addr), (2, 4, &[10, 1, 2, 3][..]));
    assert_eq!(unsafe { CStr::from_ptr(v.name) }.to_str().unwrap(), "qafoo");
    let alias = unsafe { CStr::from_ptr(*v.aliases) }
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(alias, "qafoo.alias");
    let _: *mut Hostent = h;
    let h6 = unsafe { crate::gethostbyname2(c("qasix").as_ptr(), 10) };
    assert!(!h6.is_null(), "gethostbyname2 AF_INET6");
    assert_eq!(unsafe { (*(h6 as *const HView)).length }, 16);
    let mut res: *mut AddrInfo = core::ptr::null_mut();
    let rc = unsafe {
        crate::getaddrinfo(
            c("qafoo").as_ptr(),
            c("80").as_ptr(),
            core::ptr::null(),
            &mut res,
        )
    };
    assert_eq!(rc, 0, "getaddrinfo of the image's name");
    #[repr(C)]
    struct AView {
        flags: c_int,
        family: c_int,
        socktype: c_int,
        protocol: c_int,
        addrlen: c_uint,
        addr: *const u8,
    }
    let a = unsafe { &*(res as *const AView) };
    // sockaddr_in: family (2 bytes), port (2), then the address.
    let sin = unsafe { std::slice::from_raw_parts(a.addr, 8) };
    assert_eq!((a.family, &sin[2..8]), (2, &[0, 80, 10, 1, 2, 3][..]));
    unsafe { freeaddrinfo(res) };

    unsafe { std::env::remove_var("PODVM_MAPS") };
    // With no table, each lookup is libc's.
    assert!(!unsafe { crate::getpwuid(0) }.is_null(), "libc's root");
    let _ = std::fs::remove_dir_all(&dir);
}
