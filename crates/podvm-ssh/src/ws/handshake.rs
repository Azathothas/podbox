use super::*;

pub(super) fn split_close(payload: &[u8]) -> (u16, Vec<u8>) {
    if payload.len() >= 2 {
        (
            u16::from_be_bytes([payload[0], payload[1]]),
            payload[2..].to_vec(),
        )
    } else {
        (1005, Vec::new())
    }
}

/// Read one HTTP head, one byte at a time so the byte after the blank line
/// (the first frame) is never consumed by the header read. Returns the first
/// line and the `name: value` pairs below it.
pub(super) fn read_head(s: &mut dyn Stream) -> io::Result<(String, Vec<(String, String)>)> {
    s.set_read_timeout(Some(Duration::from_secs(15)))?;
    let mut raw = Vec::new();
    let mut b = [0u8; 1];
    loop {
        if raw.len() >= MAX_HEAD {
            return Err(io::Error::other("http head exceeds 16 KiB"));
        }
        match s.read(&mut b) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "peer closed during the websocket upgrade",
                ))
            }
            Ok(_) => {
                raw.push(b[0]);
                if raw.len() >= 4 && raw[raw.len() - 4..] == *b"\r\n\r\n" {
                    break;
                }
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut
                    || e.kind() == io::ErrorKind::Interrupted =>
            {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "websocket upgrade read timed out",
                ))
            }
            Err(e) => return Err(e),
        }
    }
    let text = String::from_utf8_lossy(&raw).to_string();
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("").to_string();
    let mut headers = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    Ok((first, headers))
}

/// Fill `n` bytes from the kernel CSPRNG, in a loop: the kernel may return
/// short, and the loop is the contract, not a precaution. Mirrors the reader
/// in `podvm-complete`'s device shim, which is the tree's other caller of
/// this syscall.
pub(super) fn random_bytes(n: usize) -> io::Result<Vec<u8>> {
    use podvm_probe::sys;
    let mut out = vec![0u8; n];
    let mut off = 0;
    while off < n {
        match sys::getrandom(&mut out[off..]) {
            Ok(0) => break,
            Ok(k) => off += k as usize,
            Err(e) if e == sys::EINTR => continue,
            Err(e) => {
                return Err(io::Error::other(format!(
                    "getrandom(2) for the websocket mask: {}",
                    e.name()
                )))
            }
        }
    }
    if off < n {
        return Err(io::Error::other("getrandom(2) returned no bytes"));
    }
    Ok(out)
}

/// SHA-1 (RFC 3174), for the accept value only: not for signatures, not for
/// passwords, not for anything adversarial.
pub(super) fn sha1(data: &[u8]) -> [u8; 20] {
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
    for chunk in msg.as_chunks::<64>().0 {
        for (i, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*word);
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
