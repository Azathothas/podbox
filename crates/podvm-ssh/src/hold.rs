//! Hold the client's first bytes until the server proves it is up.
//!
//! A server behind a slow transport (a guest that opens its serial line
//! after the client already speaks) loses whatever arrives before its
//! first open: the bytes sit in a queue the open then discards, and the
//! server reads a truncated first line and exits. The SSH client always
//! speaks first, so the machine arm cannot fix this by waiting: any wait
//! that reads the banner consumes it, and the client behind it then
//! starves on the one banner it will never see.
//!
//! This hold is the one place that waits without consuming. It buffers
//! both directions until a full server version line arrives, then
//! replays each buffer in order and returns, and the caller pumps from
//! there. Every bound is explicit: the buffers are capped, the wait has
//! a deadline, and every exit past success is loud.

use std::time::{Duration, Instant};

use crate::error::{Error, Kind};
use crate::transport::Stream;

use super::pump::is_would_block;
use super::pump::READ_TIMEOUT;

/// How much each direction may buffer while the banner is outstanding.
/// A banner arrives in hundreds of bytes; a megabyte is headroom, not a
/// second channel. Past it something is wrong, and the hold fails loud
/// rather than growing.
const HOLD_CAP: usize = 1024 * 1024;

/// Hold client bytes until the server sends its version line, then replay
/// both buffers in order. `a` is the client, `b` is the server. Returns
/// once the replay is flushed; the caller owns the session from there.
///
/// Loud exits: the deadline passes with no banner, the server closes
/// first, or either buffer outgrows its cap. A client departure
/// observed before the banner is a clean close: there is nobody left
/// to answer. Past the banner the replay can still fail loud writing
/// to a client that left meanwhile; peer timing decides which arm
/// fires, and both arms are terminal, so the hold never hangs between
/// them.
pub fn hold_until_banner(
    a: &mut dyn Stream,
    b: &mut dyn Stream,
    hold_secs: u64,
) -> Result<(), Error> {
    a.set_read_timeout(Some(READ_TIMEOUT))
        .map_err(Error::pump)?;
    b.set_read_timeout(Some(READ_TIMEOUT))
        .map_err(Error::pump)?;
    // Checked: `Instant + Duration` panics past what the clock can
    // represent, and the seconds arrive as untrusted CLI input. An
    // unschedulable deadline is a usage refusal, typed here where the
    // overflow would happen rather than behind a second constant.
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(hold_secs))
        .ok_or_else(|| {
            Error::new(
                Kind::Usage,
                "hold for banner",
                "--hold-for-banner SECS is too large to schedule",
            )
        })?;
    let mut from_a: Vec<u8> = Vec::new();
    let mut from_b: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 65536];
    loop {
        if Instant::now() >= deadline {
            return Err(Error::new(
                Kind::Dial,
                "hold for banner",
                format!("no server version line within {hold_secs}s"),
            ));
        }
        match b.read(&mut tmp) {
            Ok(0) => {
                return Err(Error::new(
                    Kind::Pump,
                    "hold for banner",
                    "the server closed before its version line",
                ));
            }
            Ok(n) => {
                if from_b.len() + n > HOLD_CAP {
                    return Err(Error::new(
                        Kind::Pump,
                        "hold for banner",
                        "the server talked past the hold cap with no version line",
                    ));
                }
                from_b.extend_from_slice(&tmp[..n]);
                if banner_seen(&from_b) {
                    drain(a, &mut from_a)?;
                    replay(a, b, &from_b, &from_a)?;
                    return Ok(());
                }
            }
            Err(e) if is_would_block(&e) => {}
            Err(e) => {
                return Err(Error::pump(e));
            }
        }
        match a.read(&mut tmp) {
            Ok(0) => {
                return Ok(());
            }
            Ok(n) => {
                if from_a.len() + n > HOLD_CAP {
                    return Err(Error::new(
                        Kind::Pump,
                        "hold for banner",
                        "the client talked past the hold cap before any banner",
                    ));
                }
                from_a.extend_from_slice(&tmp[..n]);
            }
            Err(e) if is_would_block(&e) => {}
            Err(e) => {
                return Err(Error::pump(e));
            }
        }
    }
}

/// True once a full version line has arrived: a newline-terminated line
/// starting with `SSH-`. Earlier chatter lines stay buffered and ride
/// along in order on the replay.
fn banner_seen(buf: &[u8]) -> bool {
    let mut start = 0;
    while let Some(rel) = buf[start..].iter().position(|&c| c == b'\n') {
        let line = &buf[start..start + rel];
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.starts_with(b"SSH-") {
            return true;
        }
        start += rel + 1;
    }
    false
}

/// Read whatever the client already sent before the banner landed, so
/// the replay carries every pre-banner byte in order rather than
/// leaving the tail for the pump. WouldBlock ends it: a client that
/// pauses is the normal case. EOF ends it too, with a clean close for
/// a client that already left. Only a client that streams past the cap
/// without pausing fails loud.
fn drain(a: &mut dyn Stream, from_a: &mut Vec<u8>) -> Result<(), Error> {
    let mut tmp = [0u8; 65536];
    loop {
        match a.read(&mut tmp) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                if from_a.len() + n > HOLD_CAP {
                    return Err(Error::new(
                        Kind::Pump,
                        "hold for banner",
                        "the client talked past the hold cap before any banner",
                    ));
                }
                from_a.extend_from_slice(&tmp[..n]);
            }
            Err(e) if is_would_block(&e) => return Ok(()),
            Err(e) => return Err(Error::pump(e)),
        }
    }
}

/// Flush the hold: server bytes to the client first, then client bytes
/// to the server. Each direction keeps its own arrival order, and the
/// banner precedes everything the client sent, the way a live server
/// would have ordered them.
fn replay(
    a: &mut dyn Stream,
    b: &mut dyn Stream,
    from_b: &[u8],
    from_a: &[u8],
) -> Result<(), Error> {
    a.write_all(from_b).map_err(Error::pump)?;
    a.flush().map_err(Error::pump)?;
    b.write_all(from_a).map_err(Error::pump)?;
    b.flush().map_err(Error::pump)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Unix;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::thread;
    use std::time::Duration;

    const BANNER: &[u8] = b"SSH-2.0-holdtest_1\r\n";
    const IDENT: &[u8] = b"SSH-2.0-holdclient_1\r\n";
    const KEX: &[u8] = b"fake-kex-bytes";

    /// Read one end to EOF with a bounded wait. Threads use this so a
    /// test that fails still joins instead of hanging the suite.
    fn read_all(mut s: UnixStream) -> Vec<u8> {
        let mut got = Vec::new();
        let mut tmp = [0u8; 65536];
        s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        loop {
            match s.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => got.extend_from_slice(&tmp[..n]),
                Err(_) => break,
            }
        }
        got
    }

    struct Script {
        hold_secs: u64,
        client_writes: Vec<Vec<u8>>,
        server_writes: Vec<Vec<u8>>,
        server_delay_ms: u64,
        server_close_first: bool,
    }

    /// Run the real hold between two scripted peers. Returns the hold's
    /// own result plus what each peer received.
    ///
    /// `client_close_first` drops the client end before the hold starts,
    /// exercising the clean-close arm; otherwise the client thread writes
    /// its script then reads to EOF.
    fn run_hold(script: Script, client_close_first: bool) -> (Result<(), Error>, Vec<u8>, Vec<u8>) {
        let (ca, sa) = UnixStream::pair().unwrap();
        let (cb, sb) = UnixStream::pair().unwrap();
        let client_got = thread::spawn(move || {
            if client_close_first {
                drop(ca);
                return Vec::new();
            }
            let mut ca = ca;
            for w in &script.client_writes {
                let _ = ca.write_all(w);
            }
            read_all(ca)
        });
        let server_got = thread::spawn(move || {
            let mut cb = cb;
            thread::sleep(Duration::from_millis(script.server_delay_ms));
            if script.server_close_first {
                drop(cb);
                return Vec::new();
            }
            for w in &script.server_writes {
                if cb.write_all(w).is_err() {
                    break;
                }
            }
            read_all(cb)
        });
        let mut a = Unix(sa);
        let mut b = Unix(sb);
        let code = hold_until_banner(&mut a, &mut b, script.hold_secs);
        drop(a);
        drop(b);
        let got_client = client_got.join().unwrap();
        let got_server = server_got.join().unwrap();
        (code, got_client, got_server)
    }

    fn live() -> Script {
        Script {
            hold_secs: 5,
            client_writes: vec![IDENT.to_vec()],
            server_writes: vec![BANNER.to_vec()],
            server_delay_ms: 0,
            server_close_first: false,
        }
    }

    #[test]
    fn banner_seen_matches_a_version_line() {
        assert!(banner_seen(b"SSH-2.0-thing_1.0\r\n"));
        assert!(banner_seen(b"noise\r\nSSH-2.0-thing_1.0\r\n"));
        assert!(!banner_seen(b"noise\r\n"));
        assert!(!banner_seen(b"SSH-2.0-partial"));
        assert!(!banner_seen(b""));
    }

    #[test]
    fn hold_replays_both_directions_in_order() {
        let (code, got_client, got_server) = run_hold(live(), false);
        assert!(code.is_ok(), "hold failed: {code:?}");
        assert_eq!(got_client, BANNER);
        assert_eq!(got_server, IDENT);
    }

    #[test]
    fn hold_tolerates_a_banner_split_across_reads() {
        let mut script = live();
        script.server_writes = vec![b"SSH-2.".to_vec(), b"0-holdtest_1\r\n".to_vec()];
        let (code, got_client, got_server) = run_hold(script, false);
        assert!(code.is_ok(), "hold failed: {code:?}");
        assert_eq!(got_client, BANNER);
        assert_eq!(got_server, IDENT);
    }

    #[test]
    fn hold_carries_chatter_ahead_of_the_banner() {
        let mut script = live();
        script.server_writes = vec![b"noise\r\n".to_vec(), BANNER.to_vec()];
        let (code, got_client, _) = run_hold(script, false);
        assert!(code.is_ok(), "hold failed: {code:?}");
        assert_eq!(got_client, b"noise\r\nSSH-2.0-holdtest_1\r\n");
    }

    #[test]
    fn hold_buffers_client_bytes_sent_before_the_banner() {
        let mut script = live();
        script.client_writes = vec![IDENT.to_vec(), KEX.to_vec()];
        script.server_delay_ms = 200;
        let (code, got_client, got_server) = run_hold(script, false);
        assert!(code.is_ok(), "hold failed: {code:?}");
        assert_eq!(got_client, BANNER);
        assert_eq!(got_server, [IDENT, KEX].concat());
    }

    #[test]
    fn hold_fails_loud_where_no_banner_comes() {
        let mut script = live();
        script.hold_secs = 1;
        script.server_writes = vec![];
        let (code, _, _) = run_hold(script, false);
        let e = code.unwrap_err();
        assert_eq!(e.kind(), Kind::Dial, "{e}");
    }

    #[test]
    fn hold_fails_loud_where_the_server_closes_first() {
        let mut script = live();
        script.server_close_first = true;
        let (code, _, _) = run_hold(script, false);
        // The server end drops before any banner, which the hold reads
        // as EOF.
        let e = code.unwrap_err();
        assert_eq!(e.kind(), Kind::Pump, "{e}");
    }

    #[test]
    fn hold_fails_loud_past_the_buffer_cap() {
        let mut script = live();
        script.server_writes = vec![vec![b'x'; HOLD_CAP + 1]];
        let (code, _, _) = run_hold(script, false);
        let e = code.unwrap_err();
        assert_eq!(e.kind(), Kind::Pump, "{e}");
    }

    #[test]
    fn hold_refuses_a_deadline_it_cannot_schedule() {
        let mut script = live();
        script.hold_secs = u64::MAX;
        script.server_writes = vec![];
        let (code, _, _) = run_hold(script, false);
        // No thread timing involved: the deadline check runs before the
        // first read, so even an idle server answers at once.
        let e = code.unwrap_err();
        assert_eq!(e.kind(), Kind::Usage, "{e}");
    }

    #[test]
    fn hold_closes_clean_where_the_client_goes_first() {
        let mut script = live();
        // The server stays quiet long enough that the client's
        // departure is what the hold observes; a banner already in
        // hand that cannot be delivered fails loud instead, by design.
        script.server_delay_ms = 500;
        let (code, _, _) = run_hold(script, true);
        assert!(code.is_ok(), "hold failed: {code:?}");
    }

    #[test]
    fn hold_fails_loud_where_the_client_outruns_the_cap() {
        // The client-side cap mirrors the server-side one: a client
        // streaming past a megabyte with no banner in sight fails the
        // same loud way. The server stays quiet past the failure so
        // the cap arm, not the deadline or the EOF arm, answers.
        let mut script = live();
        script.client_writes = vec![vec![b'y'; HOLD_CAP + 1]];
        script.server_delay_ms = 1000;
        let (code, _, _) = run_hold(script, false);
        let e = code.unwrap_err();
        assert_eq!(e.kind(), Kind::Pump, "{e}");
    }

    #[test]
    fn replay_fails_loud_where_the_client_left_at_replay() {
        // The peer race the module docs own (a client leaving between
        // the banner and the replay) cannot be scheduled deterministically
        // through run_hold, so this pins the arm directly: the banner is
        // already in hand, the client end is already gone, and the replay
        // must fail loud rather than report success.
        let (ca, sa) = UnixStream::pair().unwrap();
        drop(ca);
        let (_cb, sb) = UnixStream::pair().unwrap();
        let mut a = Unix(sa);
        let mut b = Unix(sb);
        let e = replay(&mut a, &mut b, BANNER, IDENT).unwrap_err();
        assert_eq!(e.kind(), Kind::Pump, "{e}");
    }
}
