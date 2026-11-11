use super::head::*;
use super::*;
use std::ffi::CString;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;

/// A 64-bit x86_64 ELF header with one program header at offset 64 of
/// type `ty`, and, where `interp` is given, the name at offset 120.
pub(super) fn elf(ty: u32, interp: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; 120];
    b[..4].copy_from_slice(b"\x7fELF");
    b[4] = 2;
    b[5] = 1;
    b[16..18].copy_from_slice(&3u16.to_le_bytes());
    b[18..20].copy_from_slice(&62u16.to_le_bytes());
    b[32..40].copy_from_slice(&64u64.to_le_bytes());
    b[54..56].copy_from_slice(&(PHENT as u16).to_le_bytes());
    b[56..58].copy_from_slice(&1u16.to_le_bytes());
    b[64..68].copy_from_slice(&ty.to_le_bytes());
    b[72..80].copy_from_slice(&120u64.to_le_bytes());
    b[96..104].copy_from_slice(&(interp.len() as u64).to_le_bytes());
    b.extend_from_slice(interp);
    b
}

#[test]
fn the_head_tells_elf_script_and_other_apart() {
    let dynamic = elf(3, b"/lib/ld-test.so\0");
    let Kind::Elf { phoff, phnum } = kind(&dynamic[..HEAD.min(dynamic.len())]) else {
        panic!("a native ELF header did not read as ELF");
    };
    assert_eq!((phoff, phnum), (64, 1));
    assert_eq!(interp_entry(&dynamic[64..64 + PHENT]), Some((120, 16)));
    // PT_LOAD only: a static executable.
    assert_eq!(interp_entry(&elf(1, b"")[64..64 + PHENT]), None);
    assert_eq!(kind(b"#!/bin/sh\n"), Kind::Script);
    assert_eq!(kind(b"plain text"), Kind::Other);
    let mut foreign = dynamic.clone();
    foreign[18] = 183; // EM_AARCH64
    assert_eq!(kind(&foreign), Kind::Foreign);
}

/// The reader agrees with the kernel on a real dynamic file: the test
/// binary itself, which the gnu target links with a loader.
#[test]
fn the_reader_finds_the_interpreter_of_a_real_binary() {
    let p = CString::new("/proc/self/exe").unwrap();
    let mut head = [0u8; HEAD];
    let mut out = [0u8; OUT];
    let f = unsafe { read_head(p.as_ptr(), &mut head, &mut out) }.expect("read the test binary");
    assert!(
        matches!(f, File::Dynamic),
        "the test binary has no PT_INTERP"
    );
    let n = crate::map::strnlen(out.as_ptr() as *const c_char, OUT);
    assert!(
        out[..n].starts_with(b"/lib"),
        "{:?}",
        String::from_utf8_lossy(&out[..n])
    );
}

#[test]
fn a_shebang_line_parses_as_the_kernel_parses_it() {
    let at = |h: &[u8], r: (usize, usize)| h[r.0..r.1].to_vec();
    let h = b"#!/bin/sh\necho hi\n";
    let sb = shebang(h, true).unwrap();
    assert_eq!(at(h, sb.interp), b"/bin/sh");
    assert_eq!(sb.arg, None);
    let h = b"#! \t/usr/bin/env  node -x \t\nrest";
    let sb = shebang(h, true).unwrap();
    assert_eq!(at(h, sb.interp), b"/usr/bin/env");
    // One argument: the rest of the line, outer blanks removed.
    assert_eq!(at(h, sb.arg.unwrap()), b"node -x");
    // A short file without a newline: its end ends the name.
    let h = b"#!/bin/sh";
    assert_eq!(at(h, shebang(h, true).unwrap().interp), b"/bin/sh");
    // A full head without a newline: the name may be cut, so ENOEXEC.
    assert_eq!(shebang(h, false), None);
    assert_eq!(shebang(b"#!  \n", true), None);
    assert_eq!(shebang(b"#x", true), None);
}

#[test]
fn a_glibc_loader_from_2_16_takes_inhibit_cache() {
    assert!(loader_takes_inhibit_cache(b"ld-linux-x86-64.so.2"));
    assert!(loader_takes_inhibit_cache(b"ld-2.31.so"));
    assert!(loader_takes_inhibit_cache(b"ld-2.16.so"));
    assert!(!loader_takes_inhibit_cache(b"ld-2.15.so"));
    assert!(!loader_takes_inhibit_cache(b"ld-musl-x86_64.so.1"));
    assert!(!loader_takes_inhibit_cache(b"ld-test.so"));
}

#[test]
fn only_a_glibc_loader_before_2_33_lacks_argv0() {
    assert!(loader_lacks_argv0(b"ld-2.31.so"));
    assert!(loader_lacks_argv0(b"ld-2.17.so"));
    assert!(!loader_lacks_argv0(b"ld-2.33.so"));
    assert!(!loader_lacks_argv0(b"ld-linux-x86-64.so.2"));
    assert!(!loader_lacks_argv0(b"ld-musl-x86_64.so.1"));
    assert!(!loader_lacks_argv0(b"ld-2.x.so"));
    assert_eq!(basename(b"/lib/ld.so"), b"ld.so");
    assert_eq!(basename(b"ld.so"), b"ld.so");
}

pub(super) fn tree(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-t1646-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for sub in ["lib", "lib64", "usr/bin", "bin"] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
    }
    d
}

pub(super) fn put(p: &Path, bytes: &[u8]) {
    std::fs::write(p, bytes).unwrap();
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A table that anchors guest `/` at `root`, as T-1407 builds it.
fn table_at(root: &str) -> (Vec<u8>, Table) {
    let entry = format!("PODVM_MAPS=/:{root}\0").into_bytes();
    let env = [entry.as_ptr() as *const c_char, core::ptr::null()];
    let t = unsafe { crate::map::parse(env.as_ptr()) };
    (entry, t)
}

fn walk(t: &Table, g: &str) -> Result<(String, String), c_int> {
    let (mut guest, mut host) = ([0u8; OUT], [0u8; OUT]);
    let (gn, hn) = unsafe { walk::resolve(t, g.as_bytes(), &mut guest, &mut host) }?;
    Ok((
        String::from_utf8_lossy(&guest[..gn]).into_owned(),
        String::from_utf8_lossy(&host[..hn]).into_owned(),
    ))
}

/// The walk resolves each link in guest names. The loader case is the
/// fixture of `podvm_enter::userland`'s
/// `an_absolute_link_loader_resolves_inside_the_rootfs` (T-1408), and the
/// walk gives the host path `loader_plan` gives there.
#[test]
fn the_walk_follows_links_inside_the_rootfs() {
    let d = tree("walk");
    let root = d.to_string_lossy().to_string();
    put(&d.join("lib/real-ld.so"), b"\x7fELF");
    symlink("/lib/real-ld.so", d.join("lib64/ld-linux-x86-64.so.2")).unwrap();
    put(&d.join("bin/busybox"), b"\x7fELF");
    symlink("/bin/busybox", d.join("bin/ls")).unwrap();
    symlink("../usr/bin", d.join("lib/bin")).unwrap();
    symlink("loop-b", d.join("bin/loop-a")).unwrap();
    symlink("loop-a", d.join("bin/loop-b")).unwrap();
    let (_keep, t) = table_at(&root);
    let got = walk(&t, "/lib64/ld-linux-x86-64.so.2").unwrap();
    assert_eq!(
        got,
        (
            "/lib/real-ld.so".to_string(),
            format!("{root}/lib/real-ld.so")
        )
    );
    let got = walk(&t, "/bin/ls").unwrap();
    assert_eq!(
        got,
        ("/bin/busybox".to_string(), format!("{root}/bin/busybox"))
    );
    // A relative link, and `..` that stops at the guest root.
    let got = walk(&t, "/../lib/bin/../../bin/./ls").unwrap();
    assert_eq!(got.0, "/bin/busybox");
    // An absent leaf stays as it is: the exec answers for it.
    assert_eq!(walk(&t, "/bin/absent").unwrap().0, "/bin/absent");
    assert_eq!(walk(&t, "/bin/loop-a"), Err(ELOOP));
    // A path outside the table follows no link.
    assert_eq!(walk(&t, "/proc/self/exe").unwrap().1, "/proc/self/exe");
    let _ = std::fs::remove_dir_all(&d);
}

/// The strings of a launch vector.
pub(super) fn strings(v: *const *const c_char) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while !unsafe { *v.add(i) }.is_null() {
        let p = unsafe { *v.add(i) };
        out.push(
            unsafe { std::ffi::CStr::from_ptr(p) }
                .to_string_lossy()
                .into_owned(),
        );
        i += 1;
    }
    out
}

fn launch(
    path: &str,
    args: &[&str],
    env: &[&str],
) -> Result<(String, Vec<String>, Vec<String>), c_int> {
    let p = CString::new(path).unwrap();
    let a: Vec<CString> = args.iter().map(|s| CString::new(*s).unwrap()).collect();
    let mut av: Vec<*const c_char> = a.iter().map(|s| s.as_ptr()).collect();
    av.push(core::ptr::null());
    let e: Vec<CString> = env.iter().map(|s| CString::new(*s).unwrap()).collect();
    let mut ev: Vec<*const c_char> = e.iter().map(|s| s.as_ptr()).collect();
    ev.push(core::ptr::null());
    let (mut s, mut buf) = (Box::new(Scratch::new()), [0u8; OUT]);
    match unsafe {
        route(
            b"execve",
            AT_FDCWD,
            p.as_ptr(),
            av.as_ptr(),
            ev.as_ptr(),
            false,
            &mut s,
            &mut buf,
        )
    } {
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

/// The three arms on one anchored rootfs: a dynamic ELF enters through the
/// guest loader with `--argv0`; a `#!` script runs its interpreter the same
/// way with the kernel's arguments; a static ELF keeps the direct exec. An
/// interpreter outside the rootfs fails with `ENOENT`, never a host run.
#[test]
fn the_route_enters_each_arm_through_the_rootfs() {
    let d = tree("route");
    let root = d.to_string_lossy().to_string();
    put(&d.join("lib/ld-test.so"), b"\x7fELF");
    put(&d.join("bin/prog"), &elf(3, b"/lib/ld-test.so\0"));
    put(&d.join("bin/static"), &elf(1, b""));
    put(&d.join("bin/lost"), &elf(3, b"/lib/absent.so\0"));
    put(&d.join("bin/scr"), b"#!/bin/prog -x\necho\n");
    symlink("/bin/prog", d.join("usr/bin/alias")).unwrap();
    let lib = format!("{root}/lib");
    let exe = "PODVM_GUEST_EXE=/bin/sh";
    let env = ["A=1", exe];
    unsafe { std::env::set_var("PODVM_MAPS", format!("/:{root}")) };
    unsafe { std::env::set_var("PODVM_LIB_PATH", &lib) };
    let host = |g: &str| format!("{root}{g}");

    let (path, argv, envp) = launch("/usr/bin/alias", &["alias", "a1"], &env).unwrap();
    assert_eq!(path, host("/lib/ld-test.so"));
    let want = [
        &host("/lib/ld-test.so"),
        "--library-path",
        &lib,
        "--argv0",
        "alias",
        &host("/bin/prog"),
        "a1",
    ];
    assert_eq!(argv, want);
    assert_eq!(envp, ["A=1", "PODVM_GUEST_EXE=/bin/prog"]);

    let (path, argv, _) = launch("/bin/scr", &["scr", "s1"], &env).unwrap();
    assert_eq!(path, host("/lib/ld-test.so"));
    let want = [
        &host("/lib/ld-test.so"),
        "--library-path",
        &lib,
        "--argv0",
        "/bin/prog",
        &host("/bin/prog"),
        "-x",
        "/bin/scr",
        "s1",
    ];
    assert_eq!(argv, want);

    let (path, argv, envp) = launch("/bin/static", &["static", "z"], &env).unwrap();
    assert_eq!(
        (path, argv),
        (
            host("/bin/static"),
            vec!["static".to_string(), "z".to_string()]
        )
    );
    assert_eq!(envp, ["A=1", "PODVM_GUEST_EXE=/bin/static"]);

    assert_eq!(launch("/bin/lost", &["lost"], &env), Err(ENOENT));
    assert_eq!(launch("/bin/none", &["none"], &env), Err(ENOENT));

    // No table: the earlier behaviour, untouched.
    unsafe { std::env::remove_var("PODVM_MAPS") };
    assert_eq!(launch("/bin/prog", &["prog"], &env).unwrap().0, "pass");
    unsafe { std::env::remove_var("PODVM_LIB_PATH") };
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/enter.md T-1724: where podvm names itself, a static child enters
/// the native-static rung through podvm, with the rootfs and its guest
/// path; a `#!` script with a static interpreter too, with the kernel's
/// arguments. A dynamic child keeps the guest loader.
#[test]
fn a_static_child_enters_the_native_static_rung_through_podvm() {
    let d = tree("dispatch");
    let root = d.to_string_lossy().to_string();
    put(&d.join("bin/static"), &elf(1, b""));
    put(&d.join("bin/scr"), b"#!/bin/static -x\necho\n");
    let env = ["A=1"];
    unsafe { std::env::set_var("PODVM_MAPS", format!("/:{root}")) };
    unsafe { std::env::set_var("PODVM_DISPATCH_SELF", "/opt/podvm") };
    let (path, argv, _) = launch("/bin/static", &["static", "z"], &env).unwrap();
    assert_eq!(path, "/opt/podvm");
    assert_eq!(argv, ["/opt/podvm", "--podvm-dispatch-enter", &root, "/bin/static", "static", "z"]);
    let (path, argv, _) = launch("/bin/scr", &["scr", "s1"], &env).unwrap();
    assert_eq!(path, "/opt/podvm");
    assert_eq!(
        argv,
        ["/opt/podvm", "--podvm-dispatch-enter", &root, "/bin/static", "/bin/static", "-x", "/bin/scr", "s1"]
    );
    unsafe { std::env::remove_var("PODVM_DISPATCH_SELF") };
    let (path, _, _) = launch("/bin/static", &["static"], &env).unwrap();
    assert_eq!(path, format!("{root}/bin/static"), "with no podvm named, the kernel runs it");
    unsafe { std::env::remove_var("PODVM_MAPS") };
    let _ = std::fs::remove_dir_all(&d);
}

/// A guest `LD_LIBRARY_PATH` goes to host names before the image's list;
/// the inherited host list is not named twice.
#[test]
fn the_library_path_keeps_the_image_list_last() {
    let (_keep, t) = table_at("/store/root");
    let lp = |env: &[&str]| {
        let e: Vec<CString> = env.iter().map(|s| CString::new(*s).unwrap()).collect();
        let mut ev: Vec<*const c_char> = e.iter().map(|s| s.as_ptr()).collect();
        ev.push(core::ptr::null());
        let mut out = [0u8; OUT];
        let n = unsafe { launch::lib_path(&t, ev.as_ptr(), &mut out) }.unwrap();
        String::from_utf8_lossy(&out[..n]).into_owned()
    };
    let img = "PODVM_LIB_PATH=/store/root/lib:/store/root/usr/lib";
    assert_eq!(
        lp(&[img, "LD_LIBRARY_PATH=/store/root/lib:/store/root/usr/lib"]),
        "/store/root/lib:/store/root/usr/lib"
    );
    assert_eq!(
        lp(&[img, "LD_LIBRARY_PATH=/opt/app/lib:/store/root/x:rel"]),
        "/store/root/opt/app/lib:/store/root/x:/store/root/lib:/store/root/usr/lib"
    );
}

/// Issue 127: a link whose absolute target exists only in the rootfs
/// answers `stat` and `access` through the walk, as a shell's command
/// lookup needs. Without the walk both miss with `ENOENT` on the host.
#[test]
fn a_lookup_through_an_absolute_link_finds_the_rootfs_file() {
    let d = tree("lookup");
    let root = d.to_string_lossy().to_string();
    put(&d.join("bin/podvm-t1646-target"), b"\x7fELF");
    symlink("/bin/podvm-t1646-target", d.join("bin/t1646-link")).unwrap();
    unsafe { std::env::set_var("PODVM_MAPS", format!("/:{root}")) };
    let p = CString::new("/bin/t1646-link").unwrap();
    let mut st = [0u8; 256];
    let rc = unsafe { crate::stat(p.as_ptr(), st.as_mut_ptr() as *mut core::ffi::c_void) };
    let errno = crate::errno();
    let ok_access = unsafe { crate::access(p.as_ptr(), X_OK) };
    let absent = CString::new("/bin/t1646-absent").unwrap();
    let miss = unsafe { crate::stat(absent.as_ptr(), st.as_mut_ptr() as *mut core::ffi::c_void) };
    let miss_errno = crate::errno();
    unsafe { std::env::remove_var("PODVM_MAPS") };
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!((rc, ok_access), (0, 0), "errno {errno}");
    assert_eq!((miss, miss_errno), (-1, ENOENT));
}
