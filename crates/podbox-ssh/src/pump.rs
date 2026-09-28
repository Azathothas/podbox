//! The one bidirectional pump every spliced link runs.
//!
//! ⛔ A single-threaded pump is deliberate. One thread owns both directions,
//! so no lock and no clone can deadlock the moment one side goes quiet. The
//! streams carry short read timeouts, so an idle direction costs one timeout
//! per turn rather than blocking the other forever.
//!
//! ⛔ Write errors fail loud. A pump that cannot write has nowhere to put the
//! bytes, so it returns an error rather than dropping them. A read error
//! closes that direction and half-closes the peer, so the loop always
//! terminates: a peer that sees EOF finishes, and the pump returns.

use std::io;
use std::time::Duration;

use crate::error::Error;
use crate::transport::Stream;

/// How long one pump turn waits for a byte before trying the other side.
pub const READ_TIMEOUT: Duration = Duration::from_millis(50);

/// True for the errors a read reports as "nothing yet". The pump treats
/// these as "try the other direction", never as a close.
pub fn is_would_block(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}

/// Move bytes in both directions until either side closes.
pub fn pump(a: &mut dyn Stream, b: &mut dyn Stream) -> Result<(), Error> {
    // ⛔ The pump owns the timeout invariant. A caller that forgot it would
    // hand the loop a stream that blocks forever, so the pump sets it
    // itself. A stream that cannot take a timeout still works: its reads
    // report `WouldBlock` through the drain thread.
    a.set_read_timeout(Some(READ_TIMEOUT))
        .map_err(Error::pump)?;
    b.set_read_timeout(Some(READ_TIMEOUT))
        .map_err(Error::pump)?;
    let mut buf = [0u8; 65536];
    let mut a_open = true;
    let mut b_open = true;
    while a_open || b_open {
        let mut moved = false;
        if a_open {
            match a.read(&mut buf) {
                Ok(0) => {
                    a_open = false;
                    // ⛔ Best-effort half-close. The peer is already gone or
                    // going; a failure here carries no bytes to report.
                    let _ = b.shutdown_write();
                }
                Ok(n) => {
                    b.write_all(&buf[..n]).map_err(Error::pump)?;
                    b.flush().map_err(Error::pump)?;
                    moved = true;
                }
                Err(e) if is_would_block(&e) => {}
                Err(_) => {
                    a_open = false;
                    let _ = b.shutdown_write();
                }
            }
        }
        if b_open {
            match b.read(&mut buf) {
                Ok(0) => {
                    b_open = false;
                    let _ = a.shutdown_write();
                }
                Ok(n) => {
                    a.write_all(&buf[..n]).map_err(Error::pump)?;
                    a.flush().map_err(Error::pump)?;
                    moved = true;
                }
                Err(e) if is_would_block(&e) => {}
                Err(_) => {
                    b_open = false;
                    let _ = a.shutdown_write();
                }
            }
        }
        // ⛔ An idle turn waits before polling again. Two streams with
        // nothing to say would otherwise spin the loop at full CPU.
        if !moved {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Instant;

    fn pair() -> (UnixStream, UnixStream) {
        let (a, b) = UnixStream::pair().unwrap();
        a.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        b.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        (a, b)
    }

    fn splice(a: crate::transport::Unix, b: crate::transport::Unix) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut a = a;
            let mut b = b;
            pump(&mut a, &mut b).unwrap();
        })
    }

    fn read_exact(s: &mut UnixStream, buf: &mut [u8]) {
        let mut got = 0;
        let deadline = Instant::now() + Duration::from_secs(5);
        while got < buf.len() {
            assert!(Instant::now() < deadline, "timed out waiting for bytes");
            let n = s.read(&mut buf[got..]).unwrap();
            assert!(n > 0, "unexpected close");
            got += n;
        }
    }

    #[test]
    fn pump_carries_bytes_both_directions_until_close() {
        let (mut ca, ra) = pair();
        let (mut cb, rb) = pair();
        let handle = splice(crate::transport::Unix(ra), crate::transport::Unix(rb));
        ca.write_all(b"from-a").unwrap();
        let mut buf = [0u8; 6];
        read_exact(&mut cb, &mut buf);
        assert_eq!(&buf, b"from-a");
        cb.write_all(b"from-b").unwrap();
        read_exact(&mut ca, &mut buf);
        assert_eq!(&buf, b"from-b");
        drop(ca);
        drop(cb);
        handle.join().unwrap();
    }

    #[test]
    fn half_close_a_to_b_delivers_eof() {
        // ⛔ The direction this pins: a shutdown on one write side arrives
        // as EOF on the other read side, and the pump returns instead of
        // waiting for a close that never comes.
        let (ca, ra) = pair();
        let (mut cb, rb) = pair();
        let handle = splice(crate::transport::Unix(ra), crate::transport::Unix(rb));
        ca.shutdown(std::net::Shutdown::Write).unwrap();
        let mut buf = [0u8; 1];
        cb.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        assert_eq!(cb.read(&mut buf).unwrap(), 0);
        drop(ca);
        drop(cb);
        handle.join().unwrap();
    }

    #[test]
    fn half_close_b_to_a_delivers_eof() {
        let (mut ca, ra) = pair();
        let (cb, rb) = pair();
        let handle = splice(crate::transport::Unix(ra), crate::transport::Unix(rb));
        cb.shutdown(std::net::Shutdown::Write).unwrap();
        let mut buf = [0u8; 1];
        ca.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        assert_eq!(ca.read(&mut buf).unwrap(), 0);
        drop(ca);
        drop(cb);
        handle.join().unwrap();
    }

    #[test]
    fn would_block_means_nothing_yet() {
        let e = io::Error::new(io::ErrorKind::WouldBlock, "x");
        assert!(is_would_block(&e));
        let e = io::Error::new(io::ErrorKind::TimedOut, "x");
        assert!(is_would_block(&e));
        let e = io::Error::new(io::ErrorKind::Interrupted, "x");
        assert!(is_would_block(&e));
        let e = io::Error::new(io::ErrorKind::BrokenPipe, "x");
        assert!(!is_would_block(&e));
    }

    /// A stream whose writes always fail. The pump must report that as
    /// an error rather than drop the bytes.
    struct FailWrite;
    impl Read for FailWrite {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Ok(0)
        }
    }
    impl Write for FailWrite {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "no reader"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl crate::transport::Stream for FailWrite {
        fn set_read_timeout(&self, _d: Option<Duration>) -> io::Result<()> {
            Ok(())
        }
        fn shutdown_write(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn write_failure_fails_loud() {
        use crate::error::Kind;
        // ⛔ The guarantee this pins: bytes the pump cannot deliver are an
        // error, not a silent drop.
        let (mut fa, ra) = pair();
        fa.write_all(b"data-for-nowhere").unwrap();
        let mut a = crate::transport::Unix(ra);
        let mut b = FailWrite;
        let e = pump(&mut a, &mut b).unwrap_err();
        assert_eq!(e.kind(), Kind::Pump);
        assert!(e.to_string().contains("pump"), "{e}");
    }
}
