use super::*;

/// `testdata/fixture.sqfs`: `mksquashfs -comp xz -b 4096 -all-root` of a
/// tree with a file of stored, xz and zero blocks (`mixed.bin`, 19 000
/// bytes, and its hard link `a/link.bin`, an extended file inode), a file
/// with a sparse block (`sparse.bin`), a file in a fragment
/// (`a/b/small.txt`), and a directory of 1 002 entries (`a/b`, an extended
/// directory whose listing spans metadata blocks).
const FIXTURE: &[u8] = include_bytes!("testdata/fixture.sqfs");

fn sha(b: &[u8]) -> String {
    super::super::kernel::sha256_hex(b)
}

#[test]
fn each_kind_of_file_reads_back_with_its_digest() {
    let img = Image::new(FIXTURE).unwrap();
    let mixed = img.read("mixed.bin").unwrap();
    assert_eq!(mixed.len(), 6000 + 8000 + 5000);
    assert_eq!(
        sha(&mixed),
        "d687ad52d9143c4adbfd095cb6398351ce2208bc5608889182ef6b649e7877c9"
    );
    assert_eq!(img.read("a/link.bin").unwrap(), mixed);
    assert_eq!(
        sha(&img.read("sparse.bin").unwrap()),
        "f6774748661caab29946aeb8a47a158a4770f16afb5f610eaa297a25468488fe"
    );
    assert_eq!(
        img.read("a/b/small.txt").unwrap(),
        b"small file in a fragment\n"
    );
}

#[test]
fn a_name_late_in_a_large_directory_is_found() {
    let img = Image::new(FIXTURE).unwrap();
    assert_eq!(img.read("a/b/zz-last.txt").unwrap(), b"z");
    let names = img.names("a/b").unwrap();
    assert_eq!(names.len(), 1002);
    assert_eq!(names.last().map(String::as_str), Some("zz-last.txt"));
    assert_eq!(img.names("/").unwrap(), ["a", "mixed.bin", "sparse.bin"]);
    assert_eq!(img.read("/a/b/entry-with-a-longer-name-999").unwrap(), b"");
}

#[test]
fn a_missing_path_a_directory_and_another_format_are_errors_by_name() {
    let img = Image::new(FIXTURE).unwrap();
    let e = img.read("a/b/nothing").unwrap_err();
    assert!(e.contains("no a/b/nothing (no nothing)"), "{e}");
    let e = img.read("a/b").unwrap_err();
    assert!(e.contains("is a directory"), "{e}");
    let e = Image::new(
        b"not a squashfs image, and longer than 96 bytes ......................................",
    )
    .err()
    .unwrap();
    assert!(e.contains("no squashfs magic"), "{e}");
    let mut gz = FIXTURE.to_vec();
    gz[20] = 1;
    let e = Image::new(&gz).err().unwrap();
    assert!(e.contains("compressor 1 is not xz"), "{e}");
}

/// The pinned modloop, where `PODVM_TEST_MODLOOP` names it: the four 9p
/// modules come out as ELF objects. Skipped where the variable is unset.
#[test]
fn the_pinned_modloop_gives_the_9p_modules() {
    let Some(p) = std::env::var_os("PODVM_TEST_MODLOOP") else {
        return;
    };
    let b = std::fs::read(p).unwrap();
    assert_eq!(sha(&b), super::super::kernel::modloop_sha256());
    let m = super::super::kernel::volume_modules(&b).unwrap();
    let names: Vec<_> = m.iter().map(|(n, _)| *n).collect();
    assert_eq!(names, ["netfs", "9pnet", "9pnet_virtio", "9p"]);
    for (n, bytes) in &m {
        assert_eq!(&bytes[..4], b"\x7fELF", "{n}");
    }
    // The digests that `unsquashfs` of squashfs-tools gives for the same
    // two files (measured 2026-11-12).
    assert_eq!(
        sha(&m[0].1),
        "06ea025d90799c2d44d3a0f6bb622c0116ca56a2cdaa7aca96654eaaedfbc726"
    );
    assert_eq!(
        sha(&m[3].1),
        "a802c5eba2065d14c6c25f8c180d93344dd1004ec3fdad6aec909d6aca163630"
    );
}
