//! The egress hop. A sealed cage usually allows one destination port and
//! nothing else, so every transport first asks a proxy to CONNECT to the real
//! target and then runs over the answer.
//!
//! ⛔ The proxy is dialed directly, with `connect_tcp`, never through itself:
//! a proxy chain is not implemented because a second hop of the same kind has
//! never been the thing that unblocked a cage.

use std::io;
use std::time::Duration;

use super::{connect_tcp, read_headers, Stream, Tcp};
use crate::url::{self, Spec};
use crate::util::b64_encode;

#[derive(Debug, Clone)]
pub enum Proxy {
    HttpConnect(Spec),
    Socks5(Spec),
}

impl Proxy {
    pub fn parse(raw: &str) -> io::Result<Self> {
        let spec = url::parse(raw)?;
        match spec.scheme.as_str() {
            "http-connect" | "https-connect" | "http" | "https" => Ok(Proxy::HttpConnect(spec)),
            "socks5" | "socks5h" => Ok(Proxy::Socks5(spec)),
            other => Err(io::Error::other(format!(
                "unknown proxy scheme {other:?}; podssh knows http-connect and socks5"
            ))),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Proxy::HttpConnect(s) => format!("http-connect {}", s.raw),
            Proxy::Socks5(s) => format!("socks5 {}", s.raw),
        }
    }

    pub fn connect(
        &self,
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> io::Result<Box<dyn Stream>> {
        let spec = match self {
            Proxy::HttpConnect(s) | Proxy::Socks5(s) => s,
        };
        let phost = spec
            .host
            .clone()
            .ok_or_else(|| io::Error::other(format!("{} needs a host", spec.raw)))?;
        let pport = spec.port_or_default().unwrap_or(443);
        let mut s: Box<dyn Stream> = Box::new(Tcp(connect_tcp(&phost, pport, timeout)?));
        match self {
            Proxy::HttpConnect(_) => http_connect(&mut *s, spec, host, port, timeout)?,
            Proxy::Socks5(_) => socks5(&mut *s, spec, host, port, timeout)?,
        }
        Ok(s)
    }
}

fn http_connect(
    s: &mut dyn Stream,
    spec: &Spec,
    host: &str,
    port: u16,
    timeout: Duration,
) -> io::Result<()> {
    let mut req = format!(
        "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\
         Proxy-Connection: Keep-Alive\r\n"
    );
    if let (Some(u), Some(p)) = (&spec.user, &spec.password) {
        req.push_str(&format!(
            "Proxy-Authorization: Basic {}\r\n",
            b64_encode(format!("{u}:{p}").as_bytes())
        ));
    } else if let Some(u) = &spec.user {
        req.push_str(&format!(
            "Proxy-Authorization: Basic {}\r\n",
            b64_encode(u.as_bytes())
        ));
    }
    req.push_str("\r\n");
    s.write_all(req.as_bytes())?;
    s.flush()?;
    let head = read_headers(s, 8 * 1024, timeout)?;
    let text = String::from_utf8_lossy(&head);
    let status = text.lines().next().unwrap_or("");
    if !status.contains(" 2") {
        return Err(io::Error::other(format!("proxy refused CONNECT: {status}")));
    }
    Ok(())
}

fn socks5(
    s: &mut dyn Stream,
    spec: &Spec,
    host: &str,
    port: u16,
    timeout: Duration,
) -> io::Result<()> {
    let method = if spec.user.is_some() { 0x02 } else { 0x00 };
    s.write_all(&[0x05, 0x01, method])?;
    s.flush()?;
    let mut reply = [0u8; 2];
    read_exact(s, &mut reply, timeout)?;
    if reply[0] != 0x05 || reply[1] == 0xFF {
        return Err(io::Error::other("socks5: no acceptable auth method"));
    }
    if reply[1] == 0x02 {
        let user = spec.user.clone().unwrap_or_default();
        let pass = spec.password.clone().unwrap_or_default();
        if user.len() > 255 || pass.len() > 255 {
            return Err(io::Error::other("socks5: credential longer than 255 bytes"));
        }
        let mut auth = vec![0x01, user.len() as u8];
        auth.extend_from_slice(user.as_bytes());
        auth.push(pass.len() as u8);
        auth.extend_from_slice(pass.as_bytes());
        s.write_all(&auth)?;
        s.flush()?;
        let mut a = [0u8; 2];
        read_exact(s, &mut a, timeout)?;
        if a[1] != 0x00 {
            return Err(io::Error::other("socks5: authentication failed"));
        }
    }
    let mut req = vec![0x05, 0x01, 0x00, 0x03, host.len().min(255) as u8];
    req.extend_from_slice(&host.as_bytes()[..host.len().min(255)]);
    req.extend_from_slice(&port.to_be_bytes());
    s.write_all(&req)?;
    s.flush()?;
    let mut head = [0u8; 4];
    read_exact(s, &mut head, timeout)?;
    if head[1] != 0x00 {
        return Err(io::Error::other(format!(
            "socks5: connect failed with code {:#x}",
            head[1]
        )));
    }
    let skip = match head[3] {
        0x01 => 4,
        0x04 => 16,
        0x03 => {
            let mut l = [0u8; 1];
            read_exact(s, &mut l, timeout)?;
            l[0] as usize
        }
        other => {
            return Err(io::Error::other(format!("socks5: bad address type {other:#x}")))
        }
    };
    let mut tail = vec![0u8; skip + 2];
    read_exact(s, &mut tail, timeout)?;
    Ok(())
}

/// Read exactly `buf.len()` bytes, one read at a time. The bytes after a
/// CONNECT reply belong to TLS or the relay, so nothing may be over-read.
pub fn read_exact(s: &mut dyn Stream, buf: &mut [u8], timeout: Duration) -> io::Result<()> {
    s.set_read_timeout(Some(timeout))?;
    let mut got = 0;
    while got < buf.len() {
        match s.read(&mut buf[got..]) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "peer closed")),
            Ok(n) => got += n,
            Err(e) if crate::util::is_would_block(&e) => {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "read timed out"))
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn parse_accepts_both_proxy_schemes() {
        assert!(matches!(
            Proxy::parse("http-connect://proxy:443").unwrap(),
            Proxy::HttpConnect(_)
        ));
        assert!(matches!(
            Proxy::parse("socks5://user:pass@proxy:1080").unwrap(),
            Proxy::Socks5(_)
        ));
        assert!(Proxy::parse("ftp://proxy:21").is_err());
    }

    #[test]
    fn http_connect_round_trip_over_unix() {
        let (mut server, mut client) = std::os::unix::net::UnixStream::pair().unwrap();
        let h = std::thread::spawn(move || {
            let mut s: Box<dyn Stream> = Box::new(super::super::Unix(server.try_clone().unwrap()));
            let head = read_headers(&mut *s, 4096, Duration::from_secs(2)).unwrap();
            assert!(String::from_utf8_lossy(&head).contains("CONNECT example.com:22"));
            s.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                .unwrap();
            s.flush().unwrap();
            // The byte after the reply belongs to the caller.
            let mut b = [0u8; 4];
            s.read_exact(&mut b).unwrap();
            assert_eq!(&b, b"ssh!");
            let _ = server.write_all(b"");
        });
        let spec = url::parse("http-connect://p:443").unwrap();
        http_connect(
            &mut super::super::Unix(client.try_clone().unwrap()),
            &spec,
            "example.com",
            22,
            Duration::from_secs(2),
        )
        .unwrap();
        client.write_all(b"ssh!").unwrap();
        client.flush().unwrap();
        h.join().unwrap();
    }
}
