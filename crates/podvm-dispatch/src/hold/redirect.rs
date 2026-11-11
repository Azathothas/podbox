//! A payload that ignores the proxy variables reaches the broker
//! (TODO/net.md T-1665). podvm writes each allowed name into the
//! container's `/etc/hosts` with a fake address `198.18.0.N` (N from 1).
//! A `connect` to `198.18.0.N:PORT` becomes a connect to the broker on
//! the loopback, then the handler sends `CONNECT NAME:PORT` with the
//! run's credential and reads the answer, before the call returns. The
//! payload then holds a tunnel to NAME:PORT, and the broker applies its
//! policy as for each other client.
//!
//! The handler runs in the SIGSYS handler: no allocation, no lock. The
//! table is written once, before the payload starts.

use core::cell::UnsafeCell;

use crate::sys::sc;

/// The variable that carries `TOKEN:NAME,NAME...` to each process of the
/// payload. podvm sets it; it passes each `execve` unchanged.
pub const VAR: &str = crate::BROKER_VAR;
/// The first three bytes of each fake address (in 198.18.0.0/15, RFC 2544).
pub const NET: [u8; 3] = crate::FAKE_NET;
/// The most names: N runs from 1 to 254.
pub const MAX: usize = 254;

const READ: i64 = 0;
const WRITE: i64 = 1;
const CONNECT: i64 = 42;
const FCNTL: i64 = 72;
const F_GETFL: i64 = 3;
const F_SETFL: i64 = 4;
const O_NONBLOCK: i64 = 0o4000;
const EACCES: i64 = 13;
const ECONNREFUSED: i64 = 111;
const EINTR: i64 = 4;

struct Table {
    token: [u8; 64],
    token_len: usize,
    buf: [u8; 4096],
    names: [(u16, u16); MAX],
    count: usize,
}

struct Cell(UnsafeCell<Table>);
// SAFETY: written once by `set` before the payload runs; read after.
unsafe impl Sync for Cell {}

static TABLE: Cell = Cell(UnsafeCell::new(Table {
    token: [0; 64],
    token_len: 0,
    buf: [0; 4096],
    names: [(0, 0); MAX],
    count: 0,
}));

fn table() -> &'static Table {
    // SAFETY: see `Cell`.
    unsafe { &*TABLE.0.get() }
}

/// Take `TOKEN:NAME,NAME...`. False where it does not parse or does not
/// fit. Call it once, before the payload starts.
pub fn set(spec: &[u8]) -> bool {
    let Some(colon) = spec.iter().position(|&c| c == b':') else {
        return false;
    };
    let (token, names) = (&spec[..colon], &spec[colon + 1..]);
    if token.is_empty() || token.len() > 64 || !token.iter().all(u8::is_ascii_alphanumeric) {
        return false;
    }
    // SAFETY: see `Cell`; nothing reads the table before this returns.
    let t = unsafe { &mut *TABLE.0.get() };
    t.token[..token.len()].copy_from_slice(token);
    t.token_len = token.len();
    t.count = 0;
    let mut at = 0usize;
    for name in names.split(|&c| c == b',').filter(|n| !n.is_empty()) {
        let ok = name
            .iter()
            .all(|&c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-');
        if !ok || t.count == MAX || at + name.len() > t.buf.len() {
            return false;
        }
        t.buf[at..at + name.len()].copy_from_slice(name);
        t.names[t.count] = (at as u16, name.len() as u16);
        t.count += 1;
        at += name.len();
    }
    true
}

/// The name of the fake address in a `sockaddr_in`, with its port.
pub fn target(sa: &[u8]) -> Option<(&'static [u8], u16)> {
    if sa.len() < 8 || u16::from_ne_bytes([sa[0], sa[1]]) != 2 || sa[4..7] != NET {
        return None;
    }
    let n = sa[7] as usize;
    let t = table();
    if n == 0 || n > t.count {
        return None;
    }
    let (at, len) = t.names[n - 1];
    let name = &t.buf[at as usize..at as usize + len as usize];
    Some((name, u16::from_be_bytes([sa[2], sa[3]])))
}

/// Connect `fd` to the broker on `127.0.0.1:broker` and open a tunnel to
/// `name:port`. 0, or a negative errno: EACCES where the broker refuses
/// (403, 407), ECONNREFUSED for each other failure.
pub fn tunnel(fd: i64, broker: u16, name: &[u8], port: u16) -> i64 {
    // SAFETY: plain system calls on the payload's descriptor.
    let flags = unsafe { sc(FCNTL, fd, F_GETFL, 0) };
    if flags < 0 {
        return flags;
    }
    if flags & O_NONBLOCK != 0 {
        unsafe { sc(FCNTL, fd, F_SETFL, flags & !O_NONBLOCK) };
    }
    let r = handshake(fd, broker, name, port);
    if flags & O_NONBLOCK != 0 {
        unsafe { sc(FCNTL, fd, F_SETFL, flags) };
    }
    r
}

fn handshake(fd: i64, broker: u16, name: &[u8], port: u16) -> i64 {
    let mut sa = [0u8; 16];
    sa[..2].copy_from_slice(&2u16.to_ne_bytes());
    sa[2..4].copy_from_slice(&broker.to_be_bytes());
    sa[4..8].copy_from_slice(&[127, 0, 0, 1]);
    let r = loop {
        // SAFETY: sa is a sockaddr_in of 16 bytes.
        let r = unsafe { sc(CONNECT, fd, sa.as_ptr() as i64, 16) };
        if r != -EINTR {
            break r;
        }
    };
    if r < 0 {
        return r;
    }
    let mut req = [0u8; 1024];
    let Some(n) = request(&mut req, name, port, table()) else {
        return -ECONNREFUSED;
    };
    if !write_all(fd, &req[..n]) {
        return -ECONNREFUSED;
    }
    // One byte at a time: the payload reads each byte after the head.
    let mut head = [0u8; 1024];
    let mut got = 0usize;
    while got < head.len() {
        // SAFETY: one byte into head.
        let r = unsafe { sc(READ, fd, head[got..].as_mut_ptr() as i64, 1) };
        if r == -EINTR {
            continue;
        }
        if r <= 0 {
            return -ECONNREFUSED;
        }
        got += 1;
        if head[..got].ends_with(b"\r\n\r\n") {
            return status(&head[..got]);
        }
    }
    -ECONNREFUSED
}

/// The answer of an HTTP head: 0 for 200, EACCES for 403 and 407.
pub fn status(head: &[u8]) -> i64 {
    if head.len() < 12 || !head.starts_with(b"HTTP/1.") {
        return -ECONNREFUSED;
    }
    match &head[9..12] {
        b"200" => 0,
        b"403" | b"407" => -EACCES,
        _ => -ECONNREFUSED,
    }
}

fn write_all(fd: i64, mut b: &[u8]) -> bool {
    while !b.is_empty() {
        // SAFETY: b is readable.
        let r = unsafe { sc(WRITE, fd, b.as_ptr() as i64, b.len() as i64) };
        if r == -EINTR {
            continue;
        }
        if r <= 0 {
            return false;
        }
        b = &b[r as usize..];
    }
    true
}

/// `CONNECT name:port` with the Basic credential `run:TOKEN`, into out.
fn request(out: &mut [u8], name: &[u8], port: u16, t: &Table) -> Option<usize> {
    let mut w = Writer { out, at: 0 };
    let mut p = [0u8; 5];
    let port = digits(port, &mut p);
    let mut cred = [0u8; 4 + 64];
    cred[..4].copy_from_slice(b"run:");
    cred[4..4 + t.token_len].copy_from_slice(&t.token[..t.token_len]);
    let mut b64 = [0u8; 92];
    let b64 = base64(&cred[..4 + t.token_len], &mut b64);
    for part in [
        b"CONNECT ".as_slice(),
        name,
        b":",
        port,
        b" HTTP/1.1\r\nHost: ",
        name,
        b":",
        port,
        b"\r\nProxy-Authorization: Basic ",
        b64,
        b"\r\n\r\n",
    ] {
        w.put(part)?;
    }
    Some(w.at)
}

struct Writer<'a> {
    out: &'a mut [u8],
    at: usize,
}

impl Writer<'_> {
    fn put(&mut self, b: &[u8]) -> Option<()> {
        let end = self
            .at
            .checked_add(b.len())
            .filter(|&e| e <= self.out.len())?;
        self.out[self.at..end].copy_from_slice(b);
        self.at = end;
        Some(())
    }
}

fn digits(mut v: u16, out: &mut [u8; 5]) -> &[u8] {
    let mut at = out.len();
    loop {
        at -= 1;
        out[at] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    &out[at..]
}

fn base64<'a>(input: &[u8], out: &'a mut [u8; 92]) -> &'a [u8] {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut n = 0;
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let v = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            out[n] = if i <= chunk.len() {
                A[((v >> shift) & 63) as usize]
            } else {
                b'='
            };
            n += 1;
        }
    }
    &out[..n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_vectors() {
        let mut o = [0u8; 92];
        assert_eq!(base64(b"", &mut o), b"");
        assert_eq!(base64(b"f", &mut o), b"Zg==");
        assert_eq!(base64(b"fo", &mut o), b"Zm8=");
        assert_eq!(base64(b"foo", &mut o), b"Zm9v");
        assert_eq!(base64(b"foobar", &mut o), b"Zm9vYmFy");
    }

    #[test]
    fn the_head_of_the_broker_gives_the_errno() {
        assert_eq!(status(b"HTTP/1.1 200 Connection established\r\n\r\n"), 0);
        assert_eq!(status(b"HTTP/1.1 403 Forbidden\r\n\r\n"), -EACCES);
        assert_eq!(
            status(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n"),
            -EACCES
        );
        assert_eq!(status(b"HTTP/1.1 502 Bad Gateway\r\n\r\n"), -ECONNREFUSED);
        assert_eq!(status(b"garbage"), -ECONNREFUSED);
    }

    #[test]
    fn a_fake_address_names_its_host_and_the_request_carries_the_credential() {
        assert!(set(b"abc123:example.com,api.example.org"));
        let mut sa = [0u8; 16];
        sa[..2].copy_from_slice(&2u16.to_ne_bytes());
        sa[2..4].copy_from_slice(&443u16.to_be_bytes());
        sa[4..8].copy_from_slice(&[198, 18, 0, 2]);
        assert_eq!(target(&sa), Some((&b"api.example.org"[..], 443)));
        sa[7] = 3;
        assert_eq!(target(&sa), None, "no third name");
        sa[4..8].copy_from_slice(&[93, 184, 216, 34]);
        assert_eq!(target(&sa), None, "a real address");
        let mut out = [0u8; 1024];
        let n = request(&mut out, b"example.com", 443, table()).unwrap();
        let text = std::str::from_utf8(&out[..n]).unwrap();
        assert!(
            text.starts_with("CONNECT example.com:443 HTTP/1.1\r\n"),
            "{text}"
        );
        // base64("run:abc123")
        assert!(
            text.contains("Proxy-Authorization: Basic cnVuOmFiYzEyMw==\r\n\r\n"),
            "{text}"
        );
        assert!(!set(b"no-colon"));
        assert!(!set(b"tok:bad name"));
    }
}
