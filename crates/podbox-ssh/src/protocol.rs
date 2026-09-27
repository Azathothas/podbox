//! The rendezvous handshake, spoken over any byte stream.
//!
//! ```text
//! peer -> relay:  PODSSH1 <n|c|p> <name>\n<auth>\n
//! relay -> peer:  OK\n   or   ERR <reason>\n
//! then:           opaque bytes, both directions, until close
//! ```
//!
//! ⛔ `SANDSSH1` is accepted as an alias so an already-deployed sandssh relay
//! can be used without a fork of it. The wire form is otherwise identical,
//! which is the whole point of a magic string.
//!
//! ⛔ The handshake is the *last* thing podssh states in the clear. Everything
//! after is ssh, end to end, and a relay only ever sees ciphertext.

use std::io;
use std::time::Duration;

use crate::transport::Stream;

pub const MAGIC: &str = "PODSSH1";
pub const MAGIC_SANDSSH: &str = "SANDSSH1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Podssh1,
    Sandssh1,
}

impl Protocol {
    pub fn magic(self) -> &'static str {
        match self {
            Protocol::Podssh1 => MAGIC,
            Protocol::Sandssh1 => MAGIC_SANDSSH,
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "podssh1" | "podssh" => Some(Protocol::Podssh1),
            "sandssh1" | "sandssh" => Some(Protocol::Sandssh1),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Protocol::Podssh1 => "podssh1",
            Protocol::Sandssh1 => "sandssh1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The side that reaches the ssh server (the sandbox).
    Node,
    /// The side that speaks to an ssh client through stdio (the operator).
    Client,
    /// A health check: pair with nothing, answer OK, close.
    Probe,
}

impl Role {
    pub fn as_wire(self) -> &'static str {
        match self {
            Role::Node => "n",
            Role::Client => "c",
            Role::Probe => "p",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "n" => Some(Role::Node),
            "c" => Some(Role::Client),
            "p" => Some(Role::Probe),
            _ => None,
        }
    }
}

/// Read one `\n`-terminated line, one byte at a time so the byte after the
/// newline (the first ssh byte) is never consumed by the handshake.
pub fn read_line(s: &mut dyn Stream, limit: usize, timeout: Duration) -> io::Result<String> {
    s.set_read_timeout(Some(timeout))?;
    let mut out = Vec::new();
    let mut b = [0u8; 1];
    loop {
        if out.len() >= limit {
            return Err(io::Error::other("handshake line too long"));
        }
        match s.read(&mut b) {
            Ok(0) => {
                if out.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "relay closed during handshake",
                    ));
                }
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "relay closed mid-handshake",
                ));
            }
            Ok(_) => {
                if b[0] == b'\n' {
                    let mut line = String::from_utf8_lossy(&out).to_string();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    return Ok(line);
                }
                out.push(b[0]);
            }
            Err(e) if crate::util::is_would_block(&e) => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "handshake read timed out",
                ))
            }
            Err(e) => return Err(e),
        }
    }
}

/// Peer side: announce, then wait for the relay's verdict.
pub fn greet(
    s: &mut dyn Stream,
    proto: Protocol,
    role: Role,
    name: &str,
    auth: &str,
    timeout: Duration,
) -> io::Result<()> {
    if name.contains(|c: char| c == '\n' || c == '\r' || c == ' ') {
        return Err(io::Error::other("node name may not contain spaces or newlines"));
    }
    let line = format!("{} {} {}\n", proto.magic(), role.as_wire(), name);
    s.write_all(line.as_bytes())?;
    s.write_all(auth.as_bytes())?;
    s.write_all(b"\n")?;
    s.flush()?;
    let verdict = read_line(s, 512, timeout)?;
    if verdict == "OK" {
        Ok(())
    } else {
        Err(io::Error::other(format!("relay said: {verdict}")))
    }
}

/// Relay side: read the announcement. Returns `(protocol, role, name, auth)`.
pub fn accept(
    s: &mut dyn Stream,
    timeout: Duration,
) -> io::Result<(Protocol, Role, String, String)> {
    let line = read_line(s, 512, timeout)?;
    let mut parts = line.split(' ');
    let magic = parts.next().unwrap_or("");
    let role = parts.next().unwrap_or("");
    let name = parts.next().unwrap_or("");
    if parts.next().is_some() || name.is_empty() {
        return Err(io::Error::other("bad handshake line"));
    }
    let proto = match magic {
        MAGIC => Protocol::Podssh1,
        MAGIC_SANDSSH => Protocol::Sandssh1,
        other => {
            return Err(io::Error::other(format!(
                "unknown protocol magic {other:?}; podssh and sandssh are both accepted"
            )))
        }
    };
    let role = Role::parse(role).ok_or_else(|| io::Error::other("bad handshake role"))?;
    let auth = read_line(s, 1024, timeout)?;
    Ok((proto, role, name.to_string(), auth))
}

pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream;

    #[test]
    fn name_rules() {
        assert!(is_valid_name("agent-1"));
        assert!(is_valid_name("a.b_c"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("has space"));
        assert!(!is_valid_name(&"x".repeat(65)));
    }

    #[test]
    fn greet_and_accept_do_not_eat_the_payload() {
        let (a, b) = UnixStream::pair().unwrap();
        let mut a: Box<dyn Stream> = Box::new(crate::transport::Unix(a));
        let mut b: Box<dyn Stream> = Box::new(crate::transport::Unix(b));
        let h = std::thread::spawn(move || {
            let (proto, role, name, auth) = accept(&mut *a, Duration::from_secs(2)).unwrap();
            assert_eq!(proto, Protocol::Podssh1);
            assert_eq!(role, Role::Node);
            assert_eq!(name, "agent-1");
            assert_eq!(auth, "secret");
            a.write_all(b"OK\n").unwrap();
            a.flush().unwrap();
            // The first ssh byte must survive the handshake.
            let mut buf = [0u8; 4];
            a.read_exact(&mut buf).unwrap();
            assert_eq!(&buf, b"ssh!");
        });
        greet(
            &mut *b,
            Protocol::Podssh1,
            Role::Node,
            "agent-1",
            "secret",
            Duration::from_secs(2),
        )
        .unwrap();
        b.write_all(b"ssh!").unwrap();
        b.flush().unwrap();
        h.join().unwrap();
    }

    #[test]
    fn sandssh_magic_is_accepted() {
        let (a, b) = UnixStream::pair().unwrap();
        let mut a: Box<dyn Stream> = Box::new(crate::transport::Unix(a));
        let mut b: Box<dyn Stream> = Box::new(crate::transport::Unix(b));
        let h = std::thread::spawn(move || {
            let (proto, _, _, _) = accept(&mut *a, Duration::from_secs(2)).unwrap();
            assert_eq!(proto, Protocol::Sandssh1);
        });
        greet(
            &mut *b,
            Protocol::Sandssh1,
            Role::Client,
            "x",
            "",
            Duration::from_secs(2),
        )
        .unwrap_err();
        b.write_all(b"").unwrap();
        h.join().unwrap();
    }
}
