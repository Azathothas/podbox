//! TODO/gate.md T-1684: `kind`, `interp_entry` and the loader-name
//! readers of the interposer over arbitrary bytes. They must not panic,
//! and their answers must agree with the bytes they read.
#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/exec/parse.rs"]
#[allow(dead_code)]
mod parse;

use parse::{Kind, HEAD, PHENT};

fuzz_target!(|data: &[u8]| {
    let head = &data[..data.len().min(HEAD - 1)];
    match parse::kind(head) {
        Kind::Script => assert!(head.starts_with(b"#!")),
        Kind::Elf { phnum, .. } => {
            assert!(head.len() >= 64 && head.starts_with(b"\x7fELF"));
            assert!(phnum <= u16::MAX as usize);
        }
        Kind::Foreign => assert!(head.starts_with(b"\x7fELF")),
        Kind::Other => assert!(!head.starts_with(b"#!")),
    }
    if let Some((_, _)) = parse::interp_entry(data) {
        assert!(data.len() >= PHENT);
    }
    let name = parse::basename(data);
    assert!(!name.contains(&b'/'));
    let _ = parse::loader_lacks_argv0(name);
    let _ = parse::loader_takes_inhibit_cache(name);
});
