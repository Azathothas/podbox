//! `podvm guest-net`: the network of a guest, from inside it,
//! TODO/machine.md T-1656.
//!
//! The pinned kernel keeps its NIC driver in modules. The run archive
//! carries podvm itself (`/podvm/podvm`) and the three modules of the pin
//! (`/podvm/modules/*.ko`), so a guest needs no tool of the image: this
//! loads the modules with `finit_module(2)`, brings up `lo` and `eth0`
//! with the address of QEMU's user-mode network (10.0.2.15/24, the gateway
//! 10.0.2.2, which reaches the host's loopback), and writes
//! `/etc/resolv.conf` with QEMU's resolver 10.0.2.3. Each step that fails
//! is one line on the console; the command runs either way.

use std::os::fd::AsRawFd;

/// The modules, in the order that their symbols need.
pub const MODULES: [&str; 3] = ["failover", "net_failover", "virtio_net"];
pub const DIR: &str = "/podvm/modules";
pub const ADDR: [u8; 4] = [10, 0, 2, 15];
pub const MASK: [u8; 4] = [255, 255, 255, 0];
pub const GATEWAY: [u8; 4] = [10, 0, 2, 2];
pub const RESOLVER: &str = "10.0.2.3";

unsafe extern "C" {
    fn syscall(n: i64, ...) -> i64;
    fn socket(domain: i32, ty: i32, proto: i32) -> i32;
    fn ioctl(fd: i32, req: u64, ...) -> i32;
    fn close(fd: i32) -> i32;
}

const SYS_FINIT_MODULE: i64 = 313;
const SIOCGIFFLAGS: u64 = 0x8913;
const SIOCSIFFLAGS: u64 = 0x8914;
const SIOCSIFADDR: u64 = 0x8916;
const SIOCSIFNETMASK: u64 = 0x891c;
const SIOCADDRT: u64 = 0x890b;
const IFF_UP: i16 = 1;
const RTF_UP: u16 = 1;
const RTF_GATEWAY: u16 = 2;
const EEXIST: i32 = 17;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SockaddrIn {
    family: u16,
    port: u16,
    addr: [u8; 4],
    zero: [u8; 8],
}

pub fn inet(addr: [u8; 4]) -> SockaddrIn {
    SockaddrIn {
        family: 2,
        port: 0,
        addr,
        zero: [0; 8],
    }
}

/// `struct ifreq` with an address or the flags in its union.
#[repr(C)]
pub struct Ifreq {
    name: [u8; 16],
    data: [u8; 24],
}

pub fn ifreq(name: &str) -> Ifreq {
    let mut r = Ifreq {
        name: [0; 16],
        data: [0; 24],
    };
    r.name[..name.len().min(15)].copy_from_slice(&name.as_bytes()[..name.len().min(15)]);
    r
}

/// `struct rtentry` of x86_64.
#[repr(C)]
pub struct Rtentry {
    pad1: u64,
    dst: SockaddrIn,
    gateway: SockaddrIn,
    genmask: SockaddrIn,
    flags: u16,
    pad2: i16,
    pad3: u64,
    pad4: *mut u8,
    metric: i16,
    dev: *mut u8,
    mtu: u64,
    window: u64,
    irtt: u16,
}

fn err() -> std::io::Error {
    std::io::Error::last_os_error()
}

pub(crate) fn load(name: &str) -> Result<(), String> {
    let path = format!("{DIR}/{name}.ko");
    let f = std::fs::File::open(&path).map_err(|e| format!("{path}: {e}"))?;
    // SAFETY: finit_module with an open descriptor and an empty string.
    let r = unsafe { syscall(SYS_FINIT_MODULE, f.as_raw_fd() as i64, c"".as_ptr(), 0i64) };
    if r != 0 {
        let e = err();
        if e.raw_os_error() != Some(EEXIST) {
            return Err(format!("finit_module {path}: {e}"));
        }
    }
    Ok(())
}

fn up(fd: i32, name: &str, addr: Option<([u8; 4], [u8; 4])>) -> Result<(), String> {
    let call = |req: u64, r: &mut Ifreq, what: &str| {
        // SAFETY: an ioctl of the interface family with a full ifreq.
        if unsafe { ioctl(fd, req, r as *mut Ifreq) } < 0 {
            Err(format!("{what} {name}: {}", err()))
        } else {
            Ok(())
        }
    };
    if let Some((a, m)) = addr {
        for (req, v, what) in [
            (SIOCSIFADDR, a, "SIOCSIFADDR"),
            (SIOCSIFNETMASK, m, "SIOCSIFNETMASK"),
        ] {
            let mut r = ifreq(name);
            let s = inet(v);
            // SAFETY: a sockaddr_in is 16 bytes, inside the 24 of the union.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &s as *const SockaddrIn as *const u8,
                    r.data.as_mut_ptr(),
                    16,
                )
            };
            call(req, &mut r, what)?;
        }
    }
    let mut r = ifreq(name);
    call(SIOCGIFFLAGS, &mut r, "SIOCGIFFLAGS")?;
    let flags = i16::from_ne_bytes([r.data[0], r.data[1]]) | IFF_UP;
    r.data[..2].copy_from_slice(&flags.to_ne_bytes());
    call(SIOCSIFFLAGS, &mut r, "SIOCSIFFLAGS")
}

fn route(fd: i32) -> Result<(), String> {
    let mut rt = Rtentry {
        pad1: 0,
        dst: inet([0; 4]),
        gateway: inet(GATEWAY),
        genmask: inet([0; 4]),
        flags: RTF_UP | RTF_GATEWAY,
        pad2: 0,
        pad3: 0,
        pad4: std::ptr::null_mut(),
        metric: 0,
        dev: std::ptr::null_mut(),
        mtu: 0,
        window: 0,
        irtt: 0,
    };
    // SAFETY: SIOCADDRT with a full rtentry.
    if unsafe { ioctl(fd, SIOCADDRT, &mut rt as *mut Rtentry) } < 0 {
        return Err(format!("SIOCADDRT default via 10.0.2.2: {}", err()));
    }
    Ok(())
}

/// The subcommand. Exit 0 where the network is up, 1 where a step failed.
/// Bring `lo` up, for a guest with no NIC (`--podvm-net none`, T-1751).
pub fn lo_up() -> Result<(), String> {
    // SAFETY: a datagram socket of the inet family, for the ioctls.
    let fd = unsafe { socket(2, 2, 0) };
    if fd < 0 {
        return Err(format!("socket: {}", err()));
    }
    let r = up(fd, "lo", None);
    // SAFETY: the socket of this function.
    unsafe { close(fd) };
    r
}

pub fn main() -> i32 {
    let mut failed = Vec::new();
    for m in MODULES {
        if let Err(e) = load(m) {
            failed.push(e);
        }
    }
    let present = || std::path::Path::new("/sys/class/net/eth0").exists();
    for _ in 0..60 {
        if present() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // SAFETY: a datagram socket of the inet family, for the ioctls.
    let fd = unsafe { socket(2, 2, 0) };
    if fd < 0 {
        failed.push(format!("socket: {}", err()));
    } else {
        if let Err(e) = up(fd, "lo", None) {
            failed.push(e);
        }
        match up(fd, "eth0", Some((ADDR, MASK))).and_then(|()| route(fd)) {
            Ok(()) => {}
            Err(e) => failed.push(e),
        }
        // SAFETY: the socket of this function.
        unsafe { close(fd) };
    }
    if let Err(e) = std::fs::write("/etc/resolv.conf", format!("nameserver {RESOLVER}\n")) {
        failed.push(format!("/etc/resolv.conf: {e}"));
    }
    for f in &failed {
        eprintln!("podvm guest-net: {f}");
    }
    i32::from(!failed.is_empty())
}
