use super::*;
use pod_term::echo::{BELL, EL};

/// Feed bytes, collect consequences: client bytes, shell bytes,
/// signals, and whether input ended.
fn feed(d: &mut Discipline, bytes: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<Sig>, bool) {
    let mut client = Vec::new();
    let mut shell = Vec::new();
    let mut sigs = Vec::new();
    let mut eof = false;
    for b in bytes {
        for e in d.key(*b) {
            match e {
                Event::ToLocal(c) => client.extend_from_slice(&c),
                Event::ToRemote(s) => shell.extend_from_slice(&s),
                Event::Signal(s) => sigs.push(s),
                Event::Eof => eof = true,
                Event::Size(_) => {}
            }
        }
    }
    (client, shell, sigs, eof)
}

/// True when `needle` occurs in `hay`. `Vec::contains` takes an
/// element, not a slice, so subsequence checks go through here.
fn has(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

fn typed(d: &mut Discipline, s: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<Sig>, bool) {
    feed(d, s)
}

#[test]
fn typing_echoes_and_enter_submits() {
    let mut d = Discipline::new();
    let (client, shell, sigs, eof) = typed(&mut d, b"hi\n");
    assert_eq!(client, b"hi\r\n$ ");
    assert_eq!(shell, b"hi\n");
    assert!(sigs.is_empty() && !eof);
}

#[test]
fn crlf_submits_one_line() {
    // ⛔ The pair is one line: a client speaking CRLF must not run
    // every command twice.
    let mut d = Discipline::new();
    let (client, shell, _, _) = typed(&mut d, b"a\r\n");
    assert_eq!(client, b"a\r\n$ ");
    assert_eq!(shell, b"a\n");
}

#[test]
fn lone_cr_submits_and_lone_lf_too() {
    // ⛔ Each alone, in a fresh discipline: back to back they are a
    // `\r\n` pair, which the pair test above already covers.
    let mut d = Discipline::new();
    let (_, shell, _, _) = typed(&mut d, b"\r");
    assert_eq!(shell, b"\n");
    let mut d = Discipline::new();
    let (_, shell, _, _) = typed(&mut d, b"\n");
    assert_eq!(shell, b"\n");
}

#[test]
fn backspace_erases_one_cell() {
    let mut d = Discipline::new();
    let (client, shell, _, _) = typed(&mut d, b"ab\x7f\n");
    assert_eq!(client, b"ab\x08 \x08\r\n$ ");
    assert_eq!(shell, b"a\n");
}

#[test]
fn backspace_at_start_bells() {
    let mut d = Discipline::new();
    let (client, shell, _, _) = typed(&mut d, b"\x7f");
    assert_eq!(client, BELL);
    assert!(shell.is_empty());
}

#[test]
fn ctrl_u_erases_the_line() {
    let mut d = Discipline::new();
    let (client, shell, _, _) = typed(&mut d, b"hello\x15");
    let mut expect = b"\r".to_vec();
    expect.extend_from_slice(PROMPT);
    expect.extend_from_slice(EL);
    assert_eq!(client, [b"hello".to_vec(), expect].concat());
    assert!(shell.is_empty());
    // The line is gone: Enter submits an empty line.
    let (_, shell, _, _) = typed(&mut d, b"\n");
    assert_eq!(shell, b"\n");
}

#[test]
fn ctrl_w_erases_one_word() {
    let mut d = Discipline::new();
    let (_, shell, _, _) = typed(&mut d, b"foo bar\x17\n");
    assert_eq!(shell, b"foo \n");
}

#[test]
fn ctrl_c_signals_and_discards() {
    let mut d = Discipline::new();
    let (client, shell, sigs, _) = typed(&mut d, b"abc\x03");
    assert_eq!(sigs, vec![Sig::Int]);
    assert_eq!(client, b"abc^C\r\n$ ");
    assert!(shell.is_empty());
    // The interrupted line never ran, so history never saw it.
    let (client, _, _, _) = typed(&mut d, b"\x1b[A");
    assert_eq!(client, BELL);
}

#[test]
fn ctrl_backslash_signals_quit() {
    let mut d = Discipline::new();
    let (client, _, sigs, _) = typed(&mut d, b"\x1c");
    assert_eq!(sigs, vec![Sig::Quit]);
    assert_eq!(client, b"^\\\r\n$ ");
}

#[test]
fn suspend_and_flow_control_bell_and_change_nothing() {
    // The catalogue's refused half, pinned as refused: a bell, no
    // signal, and the line intact underneath.
    let mut d = Discipline::new();
    typed(&mut d, b"ab");
    for b in [0x1au8, 0x11, 0x13] {
        let (client, shell, sigs, _) = typed(&mut d, &[b]);
        assert_eq!(client, BELL, "byte {b:#x}");
        assert!(shell.is_empty() && sigs.is_empty(), "byte {b:#x}");
    }
    let (_, shell, _, _) = typed(&mut d, b"\n");
    assert_eq!(shell, b"ab\n");
}

#[test]
fn ctrl_d_on_empty_is_eof_inside_is_delete() {
    let mut d = Discipline::new();
    let (_, _, _, eof) = typed(&mut d, b"\x04");
    assert!(eof);
    // Mid-line it deletes under the cursor with a redraw instead.
    let mut d = Discipline::new();
    typed(&mut d, b"ab\x1b[D");
    let (client, shell, _, eof) = typed(&mut d, b"\x04\n");
    assert!(!eof);
    assert_eq!(shell, b"a\n");
    assert!(has(&client, b"\x1b[K"));
}

#[test]
fn history_browses_and_reruns() {
    let mut d = Discipline::new();
    let (_, shell, _, _) = typed(&mut d, b"one\ntwo\n");
    assert_eq!(shell, b"one\ntwo\n");
    // Up recalls newest first.
    let (client, _, _, _) = typed(&mut d, b"\x1b[A");
    assert!(has(&client, b"two"));
    let (client, _, _, _) = typed(&mut d, b"\x1b[A");
    assert!(has(&client, b"one"));
    // Past the oldest is a bell, and the line stays.
    let (client, _, _, _) = typed(&mut d, b"\x1b[A");
    assert_eq!(client, BELL);
    // Down walks back to the newest; Enter runs it again.
    typed(&mut d, b"\x1b[B");
    let (_, shell, _, _) = typed(&mut d, b"\n");
    assert_eq!(shell, b"two\n");
}

#[test]
fn history_skips_empty_and_repeats() {
    let mut d = Discipline::new();
    typed(&mut d, b"\nx\nx\n");
    let (client, _, _, _) = typed(&mut d, b"\x1b[A");
    assert!(client.ends_with(b"x"));
    // One entry, not three: the repeat and the blank never stored.
    let (client, _, _, _) = typed(&mut d, b"\x1b[A");
    assert_eq!(client, BELL);
}

#[test]
fn arrows_move_and_insert_midline() {
    let mut d = Discipline::new();
    let (client, _, _, _) = typed(&mut d, b"ac\x1b[D");
    assert!(has(&client, b"\x1b[D"));
    let (client, shell, _, _) = typed(&mut d, b"b\n");
    assert!(has(&client, b"\x1b[K"));
    assert_eq!(shell, b"abc\n");
}

#[test]
fn home_and_end_are_silent_moves() {
    // The jump moves the screen's cursor with the line's (pod-term), so
    // the next key lands where the user sees the cursor.
    let mut d = Discipline::new();
    let (client, _, _, _) = typed(&mut d, b"ab\x01");
    assert_eq!(client, b"ab\x1b[2D");
    let (client, shell, _, _) = typed(&mut d, b"X\n");
    assert!(has(&client, b"\x1b[K"));
    assert_eq!(shell, b"Xab\n");
}

#[test]
fn unknown_escape_bells_and_drops() {
    let mut d = Discipline::new();
    let (client, shell, _, _) = typed(&mut d, b"\x1bxa");
    assert_eq!(client, [BELL.to_vec(), b"a".to_vec()].concat());
    assert!(shell.is_empty());
}

#[test]
fn line_cap_drops_with_bell() {
    let mut d = Discipline::new();
    let big = vec![b'y'; LINE_CAP + 5];
    let (client, _, _, _) = typed(&mut d, &big);
    // Every accepted byte echoes itself; every byte past the cap is
    // one bell and nothing else.
    assert_eq!(client.len(), LINE_CAP + 5);
    assert_eq!(client.iter().filter(|b| **b == b'\x07').count(), 5);
    let (_, shell, _, _) = typed(&mut d, b"\n");
    assert_eq!(shell.len(), LINE_CAP + 1);
}

#[test]
fn midline_erase_redraws() {
    // "abc", two lefts, erase: removes `a`, leaves "bc".
    let mut d = Discipline::new();
    typed(&mut d, b"abc\x1b[D\x1b[D");
    let (client, shell, _, _) = typed(&mut d, b"\x7f\n");
    assert!(has(&client, b"\x1b[K"));
    assert_eq!(shell, b"bc\n");
}

#[test]
fn right_arrow_moves_and_retypes() {
    // "ac", left twice, right once, "b": the insert lands between,
    // "abc".
    let mut d = Discipline::new();
    let (client, _, _, _) = typed(&mut d, b"ac\x1b[D\x1b[D\x1b[C");
    assert!(has(&client, b"\x1b[C"));
    let (client, shell, _, _) = typed(&mut d, b"b\n");
    assert!(has(&client, b"\x1b[K"));
    assert_eq!(shell, b"abc\n");
}

#[test]
fn esc_home_end_jump_silently() {
    // `ESC [ H` and `ESC [ F` move like Ctrl-A and Ctrl-E: the screen's
    // cursor moves with the line's.
    let mut d = Discipline::new();
    let (client, _, _, _) = typed(&mut d, b"ab\x1b[H");
    assert_eq!(client, b"ab\x1b[2D");
    let (client, shell, _, _) = typed(&mut d, b"X\n");
    assert!(has(&client, b"\x1b[K"));
    assert_eq!(shell, b"Xab\n");
    let mut d = Discipline::new();
    let (client, _, _, _) = typed(&mut d, b"ab\x1b[H\x1b[F");
    assert_eq!(client, b"ab\x1b[2D\x1b[2C");
    let (_, shell, _, _) = typed(&mut d, b"X\n");
    assert_eq!(shell, b"abX\n");
}

#[test]
fn unknown_bracket_sequence_bells() {
    // `ESC [ Z` is refused: one bell, the line intact underneath.
    let mut d = Discipline::new();
    typed(&mut d, b"ab");
    let (client, shell, _, _) = typed(&mut d, b"\x1b[Z");
    assert_eq!(client, BELL);
    assert!(shell.is_empty());
    let (_, shell, _, _) = typed(&mut d, b"\n");
    assert_eq!(shell, b"ab\n");
}

#[test]
fn history_down_from_fresh_bells() {
    // Down with no browse in progress refuses; the line stays.
    let mut d = Discipline::new();
    typed(&mut d, b"one\n");
    typed(&mut d, b"tw");
    let (client, shell, _, _) = typed(&mut d, b"\x1b[B");
    assert_eq!(client, BELL);
    assert!(shell.is_empty());
    let (_, shell, _, _) = typed(&mut d, b"\n");
    assert_eq!(shell, b"tw\n");
}

#[test]
fn ctrl_d_at_end_of_line_is_silent() {
    // Past the last cell there is nothing to delete: no bell, no
    // redraw, no shell bytes.
    let mut d = Discipline::new();
    typed(&mut d, b"ab");
    let (client, shell, _, eof) = typed(&mut d, b"\x04");
    assert!(!eof);
    assert!(client.is_empty());
    assert!(shell.is_empty());
}

#[test]
fn empty_erases_bell() {
    // Ctrl-U and Ctrl-W on an empty line refuse with a bell each.
    let mut d = Discipline::new();
    for b in [0x15u8, 0x17] {
        let (client, shell, _, _) = typed(&mut d, &[b]);
        assert_eq!(client, BELL, "byte {b:#x}");
        assert!(shell.is_empty(), "byte {b:#x}");
    }
}

#[test]
fn midline_ctrl_w_erases_word_and_redraws() {
    // "foo bar", two lefts, Ctrl-W: erases back over the word
    // fragment before the cursor ("b"), leaves "foo ar", and only a
    // redraw puts the shifted tail right.
    let mut d = Discipline::new();
    typed(&mut d, b"foo bar\x1b[D\x1b[D");
    let (client, shell, _, _) = typed(&mut d, b"\x17\n");
    assert!(has(&client, b"\x1b[K"));
    assert_eq!(shell, b"foo ar\n");
}

#[test]
fn history_evicts_past_the_cap() {
    // Past `HISTORY_CAP` lines the oldest leaves: Up from a full
    // history reaches back exactly the cap, then bells.
    let mut d = Discipline::new();
    for i in 0..HISTORY_CAP + 5 {
        let line = format!("cmd{i}\n");
        typed(&mut d, line.as_bytes());
    }
    assert_eq!(d.history().len(), HISTORY_CAP);
    assert_eq!(d.history()[0], b"cmd5");
    for _ in 0..HISTORY_CAP {
        let (client, _, _, _) = typed(&mut d, b"\x1b[A");
        assert_ne!(client, BELL);
    }
    let (client, _, _, _) = typed(&mut d, b"\x1b[A");
    assert_eq!(client, BELL);
}

#[test]
fn history_down_past_newest_restores_the_parked_line() {
    // Type a partial line, browse up, browse back past the newest:
    // the uncommitted line set aside on first browse returns, and
    // Enter runs it instead of the recalled entry.
    let mut d = Discipline::new();
    typed(&mut d, b"one\n");
    typed(&mut d, b"tw");
    let (client, _, _, _) = typed(&mut d, b"\x1b[A");
    assert!(has(&client, b"one"));
    let (client, shell, _, _) = typed(&mut d, b"\x1b[B");
    assert!(has(&client, b"tw"));
    assert!(shell.is_empty());
    let (_, shell, _, _) = typed(&mut d, b"\n");
    assert_eq!(shell, b"tw\n");
}

#[test]
fn arrows_at_either_bound_bell() {
    // Left at column zero and Right at end of line refuse with a
    // bell each; the line underneath is untouched.
    let mut d = Discipline::new();
    let (client, shell, _, _) = typed(&mut d, b"\x1b[D");
    assert_eq!(client, BELL);
    assert!(shell.is_empty());
    let (client, shell, _, _) = typed(&mut d, b"a\x1b[C");
    assert_eq!(client, [b"a".to_vec(), BELL.to_vec()].concat());
    assert!(shell.is_empty());
    let (_, shell, _, _) = typed(&mut d, b"\n");
    assert_eq!(shell, b"a\n");
}

#[test]
fn ctrl_w_eats_blanks_then_the_word() {
    // Trailing blanks go first, then the word itself: the whole
    // five cells rub out and the submit that follows is empty.
    let mut d = Discipline::new();
    typed(&mut d, b"foo  ");
    let (client, shell, _, _) = typed(&mut d, b"\x17");
    assert_eq!(client, Discipline::rubout(5));
    assert!(shell.is_empty());
    let (_, shell, _, _) = typed(&mut d, b"\n");
    assert_eq!(shell, b"\n");
}
