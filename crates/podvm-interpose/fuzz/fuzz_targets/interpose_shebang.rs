//! TODO/gate.md T-1684: the `#!` reader of the interposer over arbitrary
//! bytes. It must not panic, and each range it returns must lie inside
//! the head, on the first line, with no blank in the name.
#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/exec/parse.rs"]
#[allow(dead_code)]
mod parse;

fuzz_target!(|data: &[u8]| {
    let Some((&flag, rest)) = data.split_first() else {
        return;
    };
    let head = &rest[..rest.len().min(parse::HEAD - 1)];
    let Some(sb) = parse::shebang(head, flag & 1 == 1) else {
        return;
    };
    let line = head.iter().position(|&c| c == b'\n').unwrap_or(head.len());
    let (s, e) = sb.interp;
    assert!(2 <= s && s < e && e <= line);
    assert!(head[s..e]
        .iter()
        .all(|&c| c != b' ' && c != b'\t' && c != 0));
    if let Some((a, b)) = sb.arg {
        assert!(e <= a && a < b && b <= line);
    }
});
