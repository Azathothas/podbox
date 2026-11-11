//! The hold of the syscall tier on the network (TODO/net.md T-1665): with
//! an egress broker on `127.0.0.1:PORT`, the payload reaches that address
//! and Unix sockets, and nothing else. `connect` to another address, a
//! datagram with an address (`sendto`, `sendmsg`, `sendmmsg`) and
//! `io_uring_setup` (whose operations pass the handler) fail with EACCES.
//! A seccomp filter cannot read a sockaddr; the handler can.

use core::sync::atomic::{AtomicI64, Ordering};

const CONNECT: i64 = 42;
const SENDTO: i64 = 44;
const SENDMSG: i64 = 46;
const SENDMMSG: i64 = 307;
const IO_URING_SETUP: i64 = 425;
const EACCES: i64 = 13;

static PORT: AtomicI64 = AtomicI64::new(0);

/// Hold the payload to the broker on `127.0.0.1:port` (0: no hold).
pub fn set_port(port: i64) {
    PORT.store(port, Ordering::Relaxed);
}

/// The port of the broker, 0 where no hold is on.
pub fn port() -> i64 {
    PORT.load(Ordering::Relaxed)
}

/// True where a socket address of `len` bytes at `addr` is one that the
/// payload may reach: a Unix socket, `AF_UNSPEC` (the end of a datagram
/// association), or the broker.
pub fn allowed(sa: &[u8], broker: u16) -> bool {
    if sa.len() < 2 {
        return false;
    }
    match u16::from_ne_bytes([sa[0], sa[1]]) {
        0 | 1 => true,
        2 if sa.len() >= 8 => {
            let port = u16::from_be_bytes([sa[2], sa[3]]);
            sa[4..8] == [127, 0, 0, 1] && port == broker
        }
        _ => false,
    }
}

fn reachable(addr: i64, len: i64) -> bool {
    if addr == 0 || !(2..=128).contains(&len) {
        return false;
    }
    // SAFETY: the payload gave `len` bytes of a socket address at `addr`.
    let sa = unsafe { core::slice::from_raw_parts(addr as *const u8, len as usize) };
    allowed(sa, port() as u16)
}

/// A call that the hold refuses, or None.
pub fn call(n: i64, a: [i64; 6]) -> Option<i64> {
    if port() == 0 {
        return None;
    }
    let refuse = |ok: bool| (!ok).then_some(-EACCES);
    match n {
        CONNECT => refuse(reachable(a[1], a[2])),
        SENDTO if a[4] != 0 => refuse(reachable(a[4], a[5])),
        SENDMSG => refuse(msg_ok(a[1])),
        SENDMMSG => {
            let ok = (0..a[2].clamp(0, 1024)).all(|i| msg_ok(a[1] + i * 64));
            refuse(ok)
        }
        IO_URING_SETUP => Some(-EACCES),
        _ => None,
    }
}

/// The `msg_name` of a `struct msghdr` at `m`: none, or one to reach.
fn msg_ok(m: i64) -> bool {
    if m == 0 {
        return true;
    }
    // SAFETY: the payload gave a struct msghdr at m (msg_name, msg_namelen).
    let (name, len) = unsafe { (*(m as *const i64), *((m + 8) as *const u32) as i64) };
    name == 0 || reachable(name, len)
}
