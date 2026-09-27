//! Small, dependency-free primitives podssh needs: SHA-1 and base64 for the
//! websocket handshake, random bytes, and the one bidirectional pump every
//! spliced link runs.
//!
//! ⛔ SHA-1 and base64 are written here rather than taken as crates. Both are
//! used for exactly one thing each, both are frozen by an RFC with published
//! test vectors, and neither is a security boundary: the websocket accept key
//! only proves the peer can parse HTTP. Every byte that matters is carried by
//! ssh above this layer.

use std::io::{self, Read};

/// SHA-1 (RFC 3174). Not for signatures, not for passwords: the two uses here
/// are the websocket `Sec-WebSocket-Accept` value and nothing else.
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 80];
    for chunk in msg.chunks_exact(64) {
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let tmp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding. Used for the websocket key and accept value.
pub fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Decode base64, tolerating missing padding. Returns `None` on a bad byte.
pub fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let mut table = [255u8; 256];
    for (i, &c) in B64.iter().enumerate() {
        table[c as usize] = i as u8;
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for c in s.bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = table[c as usize];
        if v == 255 {
            return None;
        }
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// Fill `n` bytes from the kernel CSPRNG. `/dev/urandom` is present wherever
/// this binary runs, including the cages podbox targets; there is no call to
/// `getrandom(2)` here so a seccomp profile cannot take it away.
pub fn random_bytes(n: usize) -> io::Result<Vec<u8>> {
    let mut f = std::fs::File::open("/dev/urandom")?;
    let mut buf = vec![0u8; n];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

/// True for the errors a non-blocking or timeout read reports as "nothing
/// yet". `pump` treats these as "try the other direction", never as a close.
pub fn is_would_block(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}

/// Move bytes in both directions until either side closes, one thread, no
/// clone. The streams carry short read timeouts, so an idle direction costs
/// one timeout per turn rather than blocking the other for ever.
///
/// ⛔ A single-threaded pump is deliberate. rustls and the websocket codec
/// both keep per-connection state that is not splittable, and a
/// `read`/`write` lock between two threads would deadlock the moment one side
/// went quiet.
pub fn pump<A: crate::transport::Stream + ?Sized, B: crate::transport::Stream + ?Sized>(
    a: &mut A,
    b: &mut B,
) -> io::Result<()> {
    let mut buf = [0u8; 65536];
    let mut a_open = true;
    let mut b_open = true;
    let dbg = std::env::var_os("PODSSH_DEBUG").is_some();
    while a_open || b_open {
        if a_open {
            match a.read(&mut buf) {
                Ok(0) => {
                    a_open = false;
                    if dbg {
                        eprintln!("[pump {} eof -> half-close {}]", a.describe(), b.describe());
                    }
                    let _ = b.shutdown_write();
                }
                Ok(n) => {
                    if dbg {
                        eprintln!("[pump {}->{} {n}]", a.describe(), b.describe());
                    }
                    b.write_all(&buf[..n])?;
                    b.flush()?;
                }
                Err(e) if is_would_block(&e) => {}
                Err(_) => a_open = false,
            }
        }
        if b_open {
            match b.read(&mut buf) {
                Ok(0) => {
                    b_open = false;
                    if dbg {
                        eprintln!("[pump {} eof -> half-close {}]", b.describe(), a.describe());
                    }
                    let _ = a.shutdown_write();
                }
                Ok(n) => {
                    if dbg {
                        eprintln!("[pump {}->{} {n}]", b.describe(), a.describe());
                    }
                    a.write_all(&buf[..n])?;
                    a.flush()?;
                }
                Err(e) if is_would_block(&e) => {}
                Err(_) => b_open = false,
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    #[test]
    fn sha1_rfc3174_vectors() {
        assert_eq!(
            sha1(b"abc").to_vec(),
            b"\xa9\x99\x3e\x36\x47\x06\x81\x6a\xba\x3e\x25\x71\x78\x50\xc2\x6c\x9c\xd0\xd8\x9d"
                .to_vec()
        );
        assert_eq!(
            sha1(b"").to_vec(),
            b"\xda\x39\xa3\xee\x5e\x6b\x4b\x0d\x32\x55\xbf\xef\x95\x60\x18\x90\xaf\xd8\x07\x09"
                .to_vec()
        );
        assert_eq!(
            sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").to_vec(),
            b"\x84\x98\x3e\x44\x1c\x3b\xd2\x6e\xba\xae\x4a\xa1\xf9\x51\x29\xe5\xe5\x46\x70\xf1"
                .to_vec()
        );
    }

    #[test]
    fn base64_round_trips_and_rfc_vectors() {
        assert_eq!(b64_encode(b""), "");
        assert_eq!(b64_encode(b"f"), "Zg==");
        assert_eq!(b64_encode(b"fo"), "Zm8=");
        assert_eq!(b64_encode(b"foo"), "Zm9v");
        assert_eq!(b64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(b64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(b64_encode(b"foobar"), "Zm9vYmFy");
        for n in 0..300 {
            let data: Vec<u8> = (0..n)
                .map(|i: usize| (i.wrapping_mul(31) & 0xff) as u8)
                .collect();
            assert_eq!(b64_decode(&b64_encode(&data)).unwrap(), data);
        }
        assert!(b64_decode("!!").is_none());
    }

    #[test]
    fn pump_carries_bytes_both_directions_until_close() {
        let (mut ca, ra) = UnixStream::pair().unwrap();
        let (mut cb, rb) = UnixStream::pair().unwrap();
        let mut ra = crate::transport::Unix(ra);
        let mut rb = crate::transport::Unix(rb);
        ra.0.set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        rb.0.set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();

        let relay = std::thread::spawn(move || {
            let r = pump(&mut ra, &mut rb);
            drop((ra, rb));
            r
        });

        ca.write_all(b"from-a").unwrap();
        let mut buf = [0u8; 6];
        read_exact(&mut cb, &mut buf);
        assert_eq!(&buf, b"from-a");

        cb.write_all(b"from-b").unwrap();
        read_exact(&mut ca, &mut buf);
        assert_eq!(&buf, b"from-b");

        drop(ca);
        drop(cb);
        relay.join().unwrap().unwrap();
    }

    fn read_exact(s: &mut UnixStream, buf: &mut [u8]) {
        use std::io::Read as _;
        s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut got = 0;
        while got < buf.len() {
            let n = s.read(&mut buf[got..]).unwrap();
            assert!(n > 0, "unexpected close");
            got += n;
        }
    }

    /// The defect of 2026-09-26, pinned, at the layer it was in.
    ///
    /// A `ProxyCommand`'s standard input is a socketpair ssh keeps open for
    /// the whole session, and `SO_RCVTIMEO` does not apply to it. The pump is
    /// single-threaded and polls both sides in turn, so a reader that can
    /// block there stops the whole session: ssh printed its version byte count
    /// and hung until `timeout` killed it, while the identical pipe case
    /// passed, because a closed pipe reaches EOF and a live socketpair does
    /// not.
    ///
    /// ⛔ The fix is not in `pump`, whose polling order was correct. It is
    /// that a source which cannot be timed out is never handed to the pump:
    /// `ChannelReader` drains such a descriptor on a helper thread and returns
    /// `WouldBlock` when nothing has arrived. `Stdio` reads through it, and
    /// this asserts the property the pump depends on, which the old direct
    /// `File` read did not have.
    #[test]
    fn a_source_the_pump_cannot_time_out_returns_would_block() {
        let (mut far, near) = UnixStream::pair().unwrap();
        far.set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        let mut reader = crate::transport::ChannelReader::from_read(near);

        // Nothing has been sent, and the pair is open: this must not block.
        let started = std::time::Instant::now();
        let mut buf = [0u8; 32];
        let n = reader.read(&mut buf);
        assert!(
            matches!(&n, Err(e) if e.kind() == io::ErrorKind::WouldBlock),
            "a live source with no bytes returned {n:?}, not WouldBlock"
        );
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "the read waited on a descriptor that cannot time out"
        );

        // Bytes that do arrive are carried, and the pump can then splice them.
        far.write_all(b"relay-first").unwrap();
        let mut got = 0;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while got < 11 && std::time::Instant::now() < deadline {
            match reader.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(k) => got += k,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(e) => panic!("read: {e}"),
            }
        }
        assert_eq!(&buf[..got], b"relay-first");
    }
}
