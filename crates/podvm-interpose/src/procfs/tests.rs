use super::*;

fn z(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

#[test]
fn fd_spellings_match_exactly() {
    assert_eq!(fd_number(&z(b"/proc/self/fd/0")), Some(0));
    assert_eq!(fd_number(&z(b"/proc/self/fd/63")), Some(63));
    assert_eq!(fd_number(&z(b"/dev/fd/63")), Some(63));
    // A trailing slash is ENOTDIR on a live /proc, never a descriptor.
    assert_eq!(fd_number(&z(b"/proc/self/fd/3/")), None);
    assert_eq!(fd_number(&z(b"/proc/self/fd/")), None);
    assert_eq!(fd_number(&z(b"/proc/self/fd/x")), None);
    assert_eq!(fd_number(&z(b"/proc/self/fd/3x")), None);
    assert_eq!(fd_number(&z(b"/proc/self/fd/-1")), None);
    // No NUL terminator is not a path the kernel took.
    assert_eq!(fd_number(b"/proc/self/fd/3"), None);
    // Past any descriptor table: refused, never wrapped.
    assert_eq!(fd_number(&z(b"/proc/self/fd/2147483648")), None);
    assert_eq!(fd_number(&z(b"/proc/self/fd/99999999999999999999")), None);
    // Other pids' descriptors are not answerable without their tables.
    assert_eq!(fd_number(&z(b"/proc/1234/fd/3")), None);
    assert_eq!(fd_number(&z(b"proc/self/fd/3")), None);
    // The standard stream spellings name their descriptors by definition.
    assert_eq!(fd_number(&z(b"/dev/stdin")), Some(0));
    assert_eq!(fd_number(&z(b"/dev/stdout")), Some(1));
    assert_eq!(fd_number(&z(b"/dev/stderr")), Some(2));
    assert_eq!(fd_number(&z(b"/dev/stdin/")), None);
}

#[test]
fn exe_matches_only_itself() {
    assert!(is_self_exe(&z(b"/proc/self/exe")));
    assert!(!is_self_exe(&z(b"/proc/self/exe/")));
    assert!(!is_self_exe(&z(b"/proc/self/cwd")));
    assert!(!is_self_exe(b"/proc/self/exe"));
}

/// T-1409: exactly `1` arms the loader-family override, and only the
/// exe spelling answers from the guest even where the real call
/// succeeds. Anything else stays on the old rule (emulation only where
/// the real call fails), so an accidental value never reroutes what the
/// payload sees itself as.
#[test]
fn the_exe_override_needs_its_flag_and_its_spelling() {
    assert!(parse_exe_override(b"1"));
    assert!(!parse_exe_override(b""));
    assert!(!parse_exe_override(b"0"));
    assert!(!parse_exe_override(b"true"));
    assert!(!parse_exe_override(b"11"));
    assert!(should_override_exe(true, &z(b"/proc/self/exe")));
    assert!(!should_override_exe(false, &z(b"/proc/self/exe")));
    assert!(!should_override_exe(true, &z(b"/proc/self/cwd")));
    assert!(!should_override_exe(true, &z(b"/proc/self/exe/")));
    assert!(!should_override_exe(true, b"/proc/self/exe"));
}

#[test]
fn mount_files_sort_into_two_renderings() {
    assert_eq!(mount_file(&z(b"/proc/mounts")), Some(MountFile::Mounts));
    assert_eq!(
        mount_file(&z(b"/proc/self/mounts")),
        Some(MountFile::Mounts)
    );
    assert_eq!(mount_file(&z(b"/etc/mtab")), Some(MountFile::Mounts));
    assert_eq!(
        mount_file(&z(b"/proc/self/mountinfo")),
        Some(MountFile::MountInfo)
    );
    assert_eq!(mount_file(&z(b"/proc/cpuinfo")), None);
    assert_eq!(mount_file(&z(b"/proc/self/mountinfo/")), None);
}

#[test]
fn compatible_flags_duplicate_and_the_rest_fall_through() {
    // Same mode both sides, no status flags: the duplicate.
    assert_eq!(open_answer(O_RDONLY, O_RDONLY), OpenAnswer::Dup);
    assert_eq!(open_answer(O_RDWR, O_RDWR), OpenAnswer::Dup);
    // CLOEXEC rides the dup variant; NOCTTY and LARGEFILE are ignored.
    assert_eq!(open_answer(O_RDONLY, O_RDONLY | O_CLOEXEC), OpenAnswer::Dup);
    assert_eq!(
        open_answer(O_RDONLY, O_RDONLY | O_NOCTTY | O_LARGEFILE),
        OpenAnswer::Dup
    );
    // A weaker request than the description holds is fine.
    assert_eq!(open_answer(O_RDWR, O_RDONLY), OpenAnswer::Dup);
    assert_eq!(open_answer(O_RDWR, O_WRONLY), OpenAnswer::Dup);
    // A stronger one is the kernel's EACCES.
    assert_eq!(open_answer(O_WRONLY, O_RDONLY), OpenAnswer::Fail(EACCES));
    assert_eq!(open_answer(O_RDONLY, O_WRONLY), OpenAnswer::Fail(EACCES));
    assert_eq!(open_answer(O_RDONLY, O_RDWR), OpenAnswer::Fail(EACCES));
    // A directory flag on a non-directory is ENOTDIR, exactly.
    assert_eq!(
        open_answer(O_RDONLY, O_RDONLY | O_DIRECTORY),
        OpenAnswer::Fail(ENOTDIR)
    );
    // Creation, truncation, path-only, tmpfile and nofollow have no
    // duplicate semantics: the fall-through, never an approximation.
    for f in [O_CREAT, O_TRUNC, O_EXCL, O_PATH, O_TMPFILE, O_NOFOLLOW] {
        assert_eq!(
            open_answer(O_RDONLY, O_RDONLY | f),
            OpenAnswer::Pass,
            "{f:o}"
        );
    }
    // A status flag either side does not carry is a different
    // description, not a dup with a tweak.
    assert_eq!(
        open_answer(O_RDONLY, O_RDONLY | O_NONBLOCK),
        OpenAnswer::Pass
    );
    assert_eq!(open_answer(O_RDONLY | O_APPEND, O_RDONLY), OpenAnswer::Pass);
    assert_eq!(open_answer(O_RDONLY, O_RDONLY | O_SYNC), OpenAnswer::Pass);
}

#[test]
fn links_render_as_the_kernel_renders_them() {
    let mut out = [0u8; 32];
    let n = fd_link(PIPE_PREFIX, 12345, &mut out);
    assert_eq!(&out[..n], b"pipe:[12345]");
    let n = fd_link(SOCKET_PREFIX, 7, &mut out);
    assert_eq!(&out[..n], b"socket:[7]");
    let n = fd_link(PIPE_PREFIX, 0, &mut out);
    assert_eq!(&out[..n], b"pipe:[0]");
}

#[test]
fn fstype_names_what_it_knows_and_says_the_rest() {
    assert_eq!(Fstype::of_magic(0xEF53), Fstype::Ext4);
    assert_eq!(Fstype::of_magic(0x0102_1994), Fstype::Tmpfs);
    assert_eq!(Fstype::of_magic(0x794C_7630), Fstype::Overlay);
    assert_eq!(Fstype::of_magic(0x9126_800E), Fstype::Btrfs);
    assert_eq!(Fstype::of_magic(0x5846_5342), Fstype::Xfs);
    assert_eq!(Fstype::of_magic(0x6464_6F63), Fstype::Unknown);
    assert_eq!(Fstype::Unknown.word(), b"unknown");
}

#[test]
fn mount_escapes_keep_one_path_one_field() {
    let mut out = [0u8; 256];
    let n = escape_mount(b"/mnt/my drive", &mut out);
    assert_eq!(&out[..n], b"/mnt/my\\040drive");
    let n = escape_mount(b"/plain", &mut out);
    assert_eq!(&out[..n], b"/plain");
}

#[test]
fn the_mounts_fixture_names_root_and_the_recorded_mounts() {
    let mut area = [0u8; 4096];
    area[0..4].copy_from_slice(b"none");
    area[4..10].copy_from_slice(b"/mnt/x");
    let mounts = [RecMount {
        src_start: 0,
        src_len: 4,
        tgt_start: 4,
        tgt_len: 6,
        ro: false,
    }];
    let mut out = [0u8; FIXTURE_MAX];
    let n = render_mounts(Fstype::Ext4, true, &area, &mounts, &mut out);
    let text = std::str::from_utf8(&out[..n]).unwrap();
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "podvm / ext4 rw 0 0");
    assert_eq!(lines.next().unwrap(), "none /mnt/x none rw 0 0");
    assert_eq!(lines.next(), None);
}

#[test]
fn the_mountinfo_fixture_sequences_its_own_ids() {
    let mut area = [0u8; 4096];
    area[0..4].copy_from_slice(b"none");
    area[4..10].copy_from_slice(b"/mnt/x");
    let mounts = [RecMount {
        src_start: 0,
        src_len: 4,
        tgt_start: 4,
        tgt_len: 6,
        ro: true,
    }];
    let mut out = [0u8; FIXTURE_MAX];
    let n = render_mountinfo(8, 1, Fstype::Ext4, true, &area, &mounts, &mut out);
    let text = std::str::from_utf8(&out[..n]).unwrap();
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "1 1 8:1 / / rw - ext4 podvm rw");
    assert_eq!(lines.next().unwrap(), "2 1 0:0 / /mnt/x ro - none none ro");
    assert_eq!(lines.next(), None);
}

#[test]
fn mode_ino_and_device_words_read_back() {
    let mut st = [0u8; STAT_LEN];
    st[ST_MODE..ST_MODE + 4].copy_from_slice(&0o10600u32.to_ne_bytes());
    st[ST_INO..ST_INO + 8].copy_from_slice(&123456u64.to_ne_bytes());
    assert_eq!(mode_of(&st) & S_IFMT, S_IFIFO);
    assert_eq!(ino_of(&st), 123456);
    // makedev(8, 1) is 0x801: the inverse names both halves.
    assert_eq!(major_of(0x801), 8);
    assert_eq!(minor_of(0x801), 1);
}

/// A memo file with one complete mount, one torn chain and one
/// ownership record: the scan takes the complete mount with its
/// read-only flag and nothing else.
#[test]
fn the_scan_takes_complete_mounts_and_drops_the_rest() {
    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    fn header(flags: u32, cs: u32, ct: u32) -> [u8; 32] {
        let mut r = [0u8; 32];
        r[0..8].copy_from_slice(&u64::MAX.to_le_bytes());
        r[8..16].copy_from_slice(&crate::emulate::OP_MOUNT.to_le_bytes());
        r[16..20].copy_from_slice(&flags.to_le_bytes());
        r[20..24].copy_from_slice(&cs.to_le_bytes());
        r[24..28].copy_from_slice(&ct.to_le_bytes());
        r
    }
    fn cont(piece: &[u8]) -> [u8; 32] {
        let mut r = [0u8; 32];
        r[0..8].copy_from_slice(&u64::MAX.to_le_bytes());
        r[8..16].copy_from_slice(&crate::emulate::CONT.to_le_bytes());
        r[16..16 + piece.len()].copy_from_slice(piece);
        r
    }
    fn owner() -> [u8; 32] {
        let mut r = [0u8; 32];
        r[0..8].copy_from_slice(&8u64.to_le_bytes());
        r[8..16].copy_from_slice(&9u64.to_le_bytes());
        r
    }

    let d = std::env::temp_dir().join(format!("podvm-procfs-scan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let path = d.join("memo");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        // Complete: flags ro, source "none" (one record), target
        // "/mnt/x" (one record).
        f.write_all(&header(1, 1, 1)).unwrap();
        f.write_all(&cont(b"none")).unwrap();
        f.write_all(&cont(b"/mnt/x")).unwrap();
        // Torn: a header whose target chain never arrives, broken by
        // an ownership record.
        f.write_all(&header(0, 1, 1)).unwrap();
        f.write_all(&cont(b"src")).unwrap();
        f.write_all(&owner()).unwrap();
        // Complete read-write mount behind the tear.
        f.write_all(&header(0, 1, 1)).unwrap();
        f.write_all(&cont(b"tmpfs")).unwrap();
        f.write_all(&cont(b"/mnt/y")).unwrap();
    }
    let f = std::fs::File::open(&path).unwrap();
    let mut area = [0u8; 4096];
    let mut recs: [RecMount; 64] = [RecMount {
        src_start: 0,
        src_len: 0,
        tgt_start: 0,
        tgt_len: 0,
        ro: false,
    }; 64];
    let taken = unsafe { scan_mounts(f.as_raw_fd(), &mut area, &mut recs) };
    assert_eq!(taken, 2);
    assert_eq!(recs[0].src(&area), b"none");
    assert_eq!(recs[0].tgt(&area), b"/mnt/x");
    assert!(recs[0].ro);
    assert_eq!(recs[1].src(&area), b"tmpfs");
    assert_eq!(recs[1].tgt(&area), b"/mnt/y");
    assert!(!recs[1].ro);
    let _ = std::fs::remove_dir_all(&d);
}
