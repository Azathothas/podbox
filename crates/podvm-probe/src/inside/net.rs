//! The network sections of the probe inside the payload. Each check has a
//! bound; a bound that runs out is TIMEOUT (a connect) or DROPPED (a
//! datagram with no answer), never a guess.

use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::mpsc;
use std::time::Duration;

use super::checks::errno_name;
use super::{Record, Verdict};

/// The bound of one connect or one answer.
pub const BOUND: Duration = Duration::from_secs(3);
/// The name that the dns and tcp-direct sections resolve.
pub const NAME: &str = "example.com:443";

fn io_record(section: &str, target: &str, e: &std::io::Error) -> Record {
    if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock {
        return Record::new(section, target, Verdict::Timeout, "no answer in the bound");
    }
    match e.raw_os_error() {
        Some(n) => Record::new(section, target, Verdict::of_errno(n), &errno_name(n)),
        None => Record::new(section, target, Verdict::Unknown, &e.to_string()),
    }
}

/// Resolve `name` with a bound: the first address, or the record of why
/// not.
fn resolve(name: &str) -> Result<SocketAddr, Record> {
    let (tx, rx) = mpsc::channel();
    let n = name.to_string();
    std::thread::spawn(move || {
        let _ = tx.send(n.to_socket_addrs().map(|mut a| a.next()));
    });
    match rx.recv_timeout(BOUND) {
        Ok(Ok(Some(a))) => Ok(a),
        Ok(Ok(None)) => Err(Record::new("dns", name, Verdict::Unresolved, "no address")),
        Ok(Err(e)) => Err(Record::new(
            "dns",
            name,
            Verdict::Unresolved,
            &e.to_string(),
        )),
        Err(_) => Err(Record::new(
            "dns",
            name,
            Verdict::Timeout,
            "no answer in the bound",
        )),
    }
}

fn connect(section: &str, target: &str, addr: SocketAddr) -> Record {
    match TcpStream::connect_timeout(&addr, BOUND) {
        Ok(_) => Record::new(section, target, Verdict::Allow, &addr.to_string()),
        Err(e) => io_record(section, target, &e),
    }
}

/// A DNS query for `example.com` (type A) to `addr`, and the wait for an
/// answer.
fn udp(section: &str, target: &str, addr: &str) -> Record {
    let q: [u8; 29] = [
        0x13, 0x37, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3,
        b'c', b'o', b'm', 0, 0, 1, 0, 1,
    ];
    let sock = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => return io_record(section, target, &e),
    };
    let _ = sock.set_read_timeout(Some(BOUND));
    if let Err(e) = sock.send_to(&q, addr) {
        return io_record(section, target, &e);
    }
    let mut buf = [0u8; 512];
    match sock.recv_from(&mut buf) {
        Ok((n, from)) => Record::new(
            section,
            target,
            Verdict::Allow,
            &format!("{n} bytes from {from}"),
        ),
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) =>
        {
            Record::new(section, target, Verdict::Dropped, "no answer in the bound")
        }
        Err(e) => io_record(section, target, &e),
    }
}

/// Each network section.
pub fn sections() -> Vec<Record> {
    let mut v = Vec::new();
    let resolved = resolve(NAME);
    v.push(match &resolved {
        Ok(a) => Record::new("dns", NAME, Verdict::Allow, &a.ip().to_string()),
        Err(r) => r.clone(),
    });
    v.push(match &resolved {
        Ok(a) => connect("tcp-direct", NAME, *a),
        Err(_) => Record::new(
            "tcp-direct",
            NAME,
            Verdict::Unresolved,
            "the name did not resolve",
        ),
    });
    v.push(connect(
        "tcp-direct-ip",
        "1.1.1.1:443",
        "1.1.1.1:443".parse().expect("an address"),
    ));
    v.push(udp("udp-direct", "1.1.1.1:53", "1.1.1.1:53"));
    // The listener lives until the connect is answered.
    v.push(match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => match l.local_addr() {
            Ok(a) => connect("loopback", "127.0.0.1", a),
            Err(e) => io_record("loopback", "127.0.0.1", &e),
        },
        Err(e) => io_record("loopback", "127.0.0.1", &e),
    });
    v.push(connect(
        "metadata",
        "169.254.169.254:80",
        "169.254.169.254:80".parse().expect("an address"),
    ));
    v
}
