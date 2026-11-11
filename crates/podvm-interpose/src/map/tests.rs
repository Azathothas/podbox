use super::*;

/// A fake environ image: static, NUL-terminated, process-stable like the
/// real one, so the table borrows it the same way.
fn env_of<const N: usize>(entries: [&'static [u8]; N]) -> Vec<*const c_char> {
    let mut v: Vec<*const c_char> = entries
        .iter()
        .map(|e| e.as_ptr() as *const c_char)
        .collect();
    v.push(core::ptr::null());
    v
}

#[test]
fn an_absent_variable_is_an_empty_table() {
    let env = env_of([b"PATH=/bin\0", b"HOME=/root\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    assert_eq!(t.n, 0);
    let env2: Vec<*const c_char> = vec![core::ptr::null()];
    let t2 = unsafe { parse(env2.as_ptr()) };
    assert_eq!(t2.n, 0);
    let t3 = unsafe { parse(core::ptr::null()) };
    assert_eq!(t3.n, 0);
}

/// `rewrite` through one unsafe block, so the cases read as cases.
fn rw(t: &Table, path: &[u8], out: &mut [u8; OUT]) -> isize {
    unsafe { rewrite(t, path.as_ptr(), path.len(), out) }
}

/// `unrewrite` through one unsafe block, beside `rw`.
fn unw(t: &Table, path: &[u8], out: &mut [u8; OUT]) -> isize {
    unsafe { unrewrite(t, path.as_ptr(), path.len(), out) }
}

#[test]
fn pairs_parse_and_match_at_a_boundary() {
    let env = env_of([b"PODVM_MAPS=/mapped:/real,/m2:/r2\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    assert_eq!(t.n, 2);
    // Matches, with the rest carried over.
    let mut out = [0u8; OUT];
    let n = rw(&t, b"/mapped/a", &mut out);
    assert_eq!(&out[..n as usize], b"/real/a");
    // ⛔ `/mapped2` is not under `/mapped`.
    assert_eq!(rw(&t, b"/mapped2/a", &mut out), -1);
    // Unmatched is untouched.
    assert_eq!(rw(&t, b"/other/a", &mut out), -1);
}

#[test]
fn the_longest_prefix_wins_whatever_the_order() {
    let env = env_of([b"PODVM_MAPS=/:/root-fallback,/mapped:/real\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    assert_eq!(t.n, 2);
    let mut out = [0u8; OUT];
    let n = rw(&t, b"/mapped/a", &mut out);
    assert_eq!(&out[..n as usize], b"/real/a");
    let n = rw(&t, b"/etc/hostname", &mut out);
    assert_eq!(&out[..n as usize], b"/root-fallback/etc/hostname");
}

#[test]
fn malformed_pairs_are_skipped_not_fatal() {
    let env = env_of([b"PODVM_MAPS=/good:/r,nocolon,/:/,:/empty-from,empty-to:\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    // `/good:/r` and `/:/` parse; the other three do not.
    assert_eq!(t.n, 2);
}

#[test]
fn the_memo_directory_is_never_rewritten() {
    let env = env_of([b"PODVM_MAPS=/:/anywhere\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    let mut out = [0u8; OUT];
    assert_eq!(rw(&t, b"/.podvm/ownership.memo", &mut out), -1);
    assert_eq!(rw(&t, b"/.podvm", &mut out), -1);
    // ⭐ But its siblings still map: the guard is the directory, not the
    // prefix string.
    let n = rw(&t, b"/.podvm2/x", &mut out);
    assert_eq!(&out[..n as usize], b"/anywhere/.podvm2/x");
}

#[test]
fn machinery_paths_never_rewrite() {
    // Broad enough to catch everything: without the exclusion every one
    // of these would join under `/anywhere`.
    let env = env_of([b"PODVM_MAPS=/:/anywhere\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    let mut out = [0u8; OUT];
    // The built-in tree: the leaves the machinery names, the tree
    // itself, and deeper paths under it.
    assert_eq!(rw(&t, b"/proc", &mut out), -1);
    assert_eq!(rw(&t, b"/proc/self/fd", &mut out), -1);
    assert_eq!(rw(&t, b"/proc/self/fd/3", &mut out), -1);
    assert_eq!(rw(&t, b"/proc/self/cwd", &mut out), -1);
    assert_eq!(rw(&t, b"/proc/self/exe", &mut out), -1);
    assert_eq!(rw(&t, b"/proc/sys/kernel/ostype", &mut out), -1);
    // ⭐ The boundary holds: siblings of the tree still map.
    let n = rw(&t, b"/proctor/x", &mut out);
    assert_eq!(&out[..n as usize], b"/anywhere/proctor/x");
    let n = rw(&t, b"/procself", &mut out);
    assert_eq!(&out[..n as usize], b"/anywhere/procself");
}

#[test]
fn results_naming_machinery_stay_real() {
    // TO broad enough to catch everything: the reverse side holds the
    // same exclusion, so machinery names never read back virtual.
    let env = env_of([b"PODVM_MAPS=/v:/\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    let mut out = [0u8; OUT];
    assert_eq!(unw(&t, b"/proc/self/exe", &mut out), -1);
    assert_eq!(unw(&t, b"/proc/self/fd/3", &mut out), -1);
    // Beside it the reverse map still answers.
    let n = unw(&t, b"/etc/x", &mut out);
    assert_eq!(&out[..n as usize], b"/v/etc/x");
}

/// `user_excluded` over byte strings, so the cases read as cases.
fn ux(v: &[u8], path: &[u8]) -> bool {
    user_excluded(v.as_ptr(), v.len(), path.as_ptr(), path.len())
}

/// `builtin_excluded` over a byte string.
fn bx(path: &[u8]) -> bool {
    builtin_excluded(path.as_ptr(), path.len())
}

#[test]
fn user_exclusions_add_prefixes_at_a_boundary() {
    let cases: &[(&[u8], &[u8], bool)] = &[
        (b"/x", b"/x/a", true),
        (b"/x", b"/x", true),
        (b"/x", b"/x2", false),
        (b"/proc", b"/proc/self/exe", true),
        (b"/proctor", b"/proctor/x", true),
        (b"/proctor", b"/proc/x", false),
        // Empty items are skipped, not matched.
        (b"/x::/yy", b"/yy", true),
        (b"/x::/yy", b"/", false),
        (b"", b"/x", false),
        (b":", b"/x", false),
    ];
    for (v, p, want) in cases {
        assert_eq!(ux(v, p), *want);
    }
}

#[test]
fn builtins_hold_without_any_user_list() {
    // No environ involved: the built-in is code, and it answers the
    // same with the user list absent.
    assert!(bx(b"/proc"));
    assert!(bx(b"/proc/self/fd/3"));
    assert!(bx(b"/proc/self/cwd"));
    assert!(bx(b"/proc/self/exe"));
    assert!(bx(b"/proc/sys/kernel/ostype"));
    assert!(!bx(b"/proctor"));
    assert!(!bx(b"/procself"));
    assert!(!bx(b"/etc/hosts"));
}

#[test]
fn unrewrite_maps_real_prefixes_back_to_virtual() {
    let env = env_of([b"PODVM_MAPS=/mapped:/real,/m2:/r2\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    let mut out = [0u8; OUT];
    // Matches on the TO side, with the rest carried over.
    let n = unw(&t, b"/real/a", &mut out);
    assert_eq!(&out[..n as usize], b"/mapped/a");
    // ⛔ `/real2` is not under `/real`.
    assert_eq!(unw(&t, b"/real2/a", &mut out), -1);
    // Unmatched is untouched.
    assert_eq!(unw(&t, b"/other/a", &mut out), -1);
    // An exact TO match answers the bare FROM.
    let n = unw(&t, b"/r2", &mut out);
    assert_eq!(&out[..n as usize], b"/m2");
    // A trailing slash on TO is ignored for the match.
    let env = env_of([b"PODVM_MAPS=/m:/real/\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    let n = unw(&t, b"/real/x", &mut out);
    assert_eq!(&out[..n as usize], b"/m/x");
    // A TO of `/` matches everything, like the FROM side.
    let env = env_of([b"PODVM_MAPS=/m:/\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    let n = unw(&t, b"/etc/x", &mut out);
    assert_eq!(&out[..n as usize], b"/m/etc/x");
    // The longest TO wins whatever the order.
    let env = env_of([b"PODVM_MAPS=/short:/real,/long:/real/sub\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    let n = unw(&t, b"/real/sub/a", &mut out);
    assert_eq!(&out[..n as usize], b"/long/a");
}

/// TODO/enter.md T-1411. The root join reads back with one separator:
/// a real `/store/rootfs/tmp` under FROM `/` answers `/tmp`, never
/// `//tmp`. The `getcwd` wrapper reads through here, so the doubled
/// slash was what `-w /tmp` printed on the userland rung.
#[test]
fn unrewrite_keeps_one_separator_on_a_root_prefix() {
    let env = env_of([b"PODVM_MAPS=/:/store/rootfs\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    let mut out = [0u8; OUT];
    let n = unw(&t, b"/store/rootfs/tmp", &mut out);
    assert_eq!(&out[..n as usize], b"/tmp");
    let n = unw(&t, b"/store/rootfs/a/b", &mut out);
    assert_eq!(&out[..n as usize], b"/a/b");
    // An exact root match answers the bare slash, with no remainder
    // for the join to double.
    let n = unw(&t, b"/store/rootfs", &mut out);
    assert_eq!(&out[..n as usize], b"/");
}

/// The forward mirror: a target ending in the separator the remainder
/// starts with joins with one, not two.
#[test]
fn rewrite_keeps_one_separator_where_the_target_ends_in_one() {
    let env = env_of([b"PODVM_MAPS=/m:/real/\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    let mut out = [0u8; OUT];
    let n = rw(&t, b"/m/x", &mut out);
    assert_eq!(&out[..n as usize], b"/real/x");
}

#[test]
fn unrewrite_never_maps_the_memo_directory() {
    let env = env_of([b"PODVM_MAPS=/v:/.podvm,/w:/work\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    let mut out = [0u8; OUT];
    // The guard beats the match: TO `/.podvm` would match, and the
    // answer stays -1.
    assert_eq!(unw(&t, b"/.podvm/x", &mut out), -1);
    assert_eq!(unw(&t, b"/.podvm", &mut out), -1);
    // Beside it the reverse map still answers.
    let n = unw(&t, b"/work/f", &mut out);
    assert_eq!(&out[..n as usize], b"/w/f");
}

#[test]
fn absolutize_copies_absolute_paths_and_joins_the_cwd() {
    let t = unsafe { parse(core::ptr::null()) };
    let mut out = [0u8; OUT];
    let n = unsafe { absolutize(&t, AT_FDCWD, c"/x/y".as_ptr(), &mut out) };
    assert_eq!(&out[..n as usize], b"/x/y");
    // Relative joins the working directory this test runs in.
    let n = unsafe { absolutize(&t, AT_FDCWD, c"rel".as_ptr(), &mut out) };
    assert!(n > 4, "{n}");
    assert_eq!(&out[n as usize - 4..n as usize], b"/rel");
    // A descriptor with nothing behind it has no answer.
    let n = unsafe { absolutize(&t, 9173, c"rel".as_ptr(), &mut out) };
    assert_eq!(n, -1);
}

/// `normalise` over a byte string, so the cases read as cases.
fn norm(path: &[u8]) -> Vec<u8> {
    let mut buf = [0u8; OUT];
    buf[..path.len()].copy_from_slice(path);
    let n = normalise(&mut buf, path.len());
    buf[..n].to_vec()
}

/// T-1643: `.`, `..` and `//` fold before the match. A `..` at the root
/// stays at the root, so a relative path cannot name the parent of the
/// rootfs, and `/tmp/..` no longer matches a `/tmp` pair.
#[test]
fn normalise_folds_dots_and_doubled_slashes() {
    let cases: &[(&[u8], &[u8])] = &[
        (b"/", b"/"),
        (b"//", b"/"),
        (b"/a/b", b"/a/b"),
        (b"//a//b", b"/a/b"),
        (b"/a/./b/.", b"/a/b/"),
        (b"/a/b/..", b"/a/"),
        (b"/a/b/../c", b"/a/c"),
        (b"/..", b"/"),
        (b"/../../etc/x", b"/etc/x"),
        (b"/tmp/../etc", b"/etc"),
        (b"/a/b/", b"/a/b/"),
        (b"/a/.b/..c", b"/a/.b/..c"),
        (b"/./", b"/"),
    ];
    for (input, want) in cases {
        assert_eq!(
            norm(input),
            want.to_vec(),
            "{}",
            String::from_utf8_lossy(input)
        );
    }
}

/// A table `/:<TO>`, with `TO` read at run time. The table borrows its
/// text for the life of the process, as it borrows environ, so the test
/// leaks the one string.
fn root_table(to: &std::path::Path) -> Table {
    use std::os::unix::ffi::OsStrExt;
    let mut s = b"PODVM_MAPS=/:".to_vec();
    s.extend_from_slice(to.as_os_str().as_bytes());
    s.push(0);
    let s: &'static [u8] = Box::leak(s.into_boxed_slice());
    let env: &'static [*const c_char] =
        Box::leak(vec![s.as_ptr() as *const c_char, core::ptr::null()].into_boxed_slice());
    unsafe { parse(env.as_ptr()) }
}

/// Absolutize, then rewrite, the way `prepare` does it.
fn resolve(t: &Table, dirfd: c_int, path: &core::ffi::CStr) -> Vec<u8> {
    let mut abs = [0u8; OUT];
    let n = unsafe { absolutize(t, dirfd, path.as_ptr(), &mut abs) };
    assert!(n > 0, "absolutize {path:?} answered {n}");
    let mut out = [0u8; OUT];
    let m = rw(t, &abs[..n as usize], &mut out);
    assert!(m > 0, "rewrite {path:?} answered {m}");
    out[..m as usize].to_vec()
}

/// T-1643: a cwd inside the table target absolutizes to a guest path,
/// and the rewrite then adds the TO prefix one time. The test's own
/// working directory lies under its parent, which is the TO side here,
/// as the rootfs is the TO side of the `/` pair on the userland rung.
/// Before the fix the join kept the host base, and the result carried
/// the prefix two times.
#[test]
fn a_cwd_inside_the_target_gets_the_prefix_once() {
    use std::os::unix::ffi::OsStrExt;
    let cwd = std::env::current_dir().expect("cwd");
    let parent = cwd.parent().expect("a cwd below the root");
    let leaf = cwd.file_name().expect("leaf").as_bytes();
    let t = root_table(parent);
    // The guest base: the leaf under `/`, with no host prefix.
    let mut abs = [0u8; OUT];
    let n = unsafe { absolutize(&t, AT_FDCWD, c"rel".as_ptr(), &mut abs) };
    let mut want = b"/".to_vec();
    want.extend_from_slice(leaf);
    want.extend_from_slice(b"/rel");
    assert_eq!(&abs[..n as usize], &want[..]);
    // `AT_FDCWD`: one prefix.
    let mut host = cwd.as_os_str().as_bytes().to_vec();
    host.extend_from_slice(b"/rel");
    assert_eq!(resolve(&t, AT_FDCWD, c"rel"), host);
    // `..` stays inside the target, and `./` folds.
    let mut up = parent.as_os_str().as_bytes().to_vec();
    up.extend_from_slice(b"/etc/x");
    assert_eq!(resolve(&t, AT_FDCWD, c"../../.././etc//x"), up);
    // A descriptor: `/proc/self/fd/<n>` names a host path too, and it
    // gets the same one prefix.
    let dir = std::fs::File::open(cwd.join("src")).expect("open src");
    let fd = std::os::fd::AsRawFd::as_raw_fd(&dir);
    let mut host = cwd.as_os_str().as_bytes().to_vec();
    host.extend_from_slice(b"/src/x");
    assert_eq!(resolve(&t, fd, c"x"), host);
}

/// TODO/interpose.md T-1705. A rewrite is idempotent: a host path under the
/// root pair or under the host side of a volume passes unchanged, and a
/// guest path still gets the prefix. A host side of `/` is not a reason to
/// pass: each path is under it.
#[test]
fn a_host_path_under_a_host_side_is_not_prefixed_again() {
    let env = env_of([b"PODVM_MAPS=/:/store/rootfs,/data:/srv/vol,/hostroot:/\0"]);
    let t = unsafe { parse(env.as_ptr()) };
    assert_eq!(t.n, 3);
    let mut out = [0u8; OUT];
    // The loader's answer for a library of the image: a host path.
    assert_eq!(rw(&t, b"/store/rootfs/usr/lib/libcoreclr.so", &mut out), -1);
    assert_eq!(rw(&t, b"/store/rootfs", &mut out), -1);
    // A file of a volume, by its host path.
    assert_eq!(rw(&t, b"/srv/vol/x", &mut out), -1);
    // The boundary: a sibling of the rootfs is a guest path.
    let n = rw(&t, b"/store/rootfs2/x", &mut out);
    assert_eq!(&out[..n as usize], b"/store/rootfs/store/rootfs2/x");
    // A guest path still gets the prefix.
    let n = rw(&t, b"/usr/lib/libcoreclr.so", &mut out);
    assert_eq!(&out[..n as usize], b"/store/rootfs/usr/lib/libcoreclr.so");
    let n = rw(&t, b"/data/x", &mut out);
    assert_eq!(&out[..n as usize], b"/srv/vol/x");
    let n = rw(&t, b"/hostroot/etc/hostname", &mut out);
    assert_eq!(&out[..n as usize], b"/etc/hostname");
}
