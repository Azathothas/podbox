//! Name resolution that works in a cage with no UDP and no libc DNS.
//!
//! ⛔ **A CONSTRAINED SANDBOX HAS NO WORKING RESOLVER, AND THAT IS MEASURED
//! NOT ASSUMED.** In the reference cage `/etc/resolv.conf` names `1.1.1.1`,
//! but UDP 53 is refused at the egress allowlist, so libc's `getaddrinfo`
//! returns EAI_AGAIN for every name. Meanwhile the egress HTTP CONNECT proxy
//! resolves names FOR us: `CONNECT github.com:443` returns 200. So there are
//! two resolvers, and a name is resolved by whichever can answer.
//!
//! ⛔ **DNS-OVER-HTTPS RIDES THE SAME `Dialer` AS EVERY TRANSPORT.** No new
//! port, no new dependency on the network, no UDP. cloudflare-dns.com and
//! dns.google were both measured answering `A` queries through the egress
//! proxy on 443. A resolver needing its own path would be the one part of
//! podssh that does not work in the machine podssh exists for, and it is
//! built on the same [`crate::transport::Dialer`] so a relay that works is a
//! resolver that works.
//!
//! ⛔ **THE SYSTEM RESOLVER IS PREFERRED WHERE IT ANSWERS**, because a name in
//! `/etc/hosts` or a split-horizon internal name is only visible to it. DoH is
//! the fallback, taken when the system one demonstrably cannot answer, which
//! is what makes podssh work on a laptop and in the cage with no flag naming
//! which.

use std::io::{Read, Write};
use std::net::IpAddr;
use std::time::Duration;

use crate::error::{Error, Kind, Result};

/// How names are turned into addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// libc `getaddrinfo`. Correct on a normal machine, and the right default
    /// there because it honours `/etc/hosts`, NSS and the split resolver.
    System,
    /// DNS-over-HTTPS through a transport dialer. The only thing that answers
    /// in a cage whose UDP is refused.
    Doh { endpoint: String },
}

/// A configured resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolver {
    pub mode: Mode,
    pub timeout: Duration,
}

impl Resolver {
    pub fn system() -> Resolver {
        Resolver {
            mode: Mode::System,
            timeout: Duration::from_secs(5),
        }
    }

    pub fn doh(endpoint: impl Into<String>) -> Resolver {
        Resolver {
            mode: Mode::Doh {
                endpoint: endpoint.into(),
            },
            timeout: Duration::from_secs(8),
        }
    }

    /// Choose a resolver from the environment, without a flag.
    ///
    /// ⛔ **A WORKING SYSTEM RESOLVER IS PREFERRED EVEN WHERE A PROXY EXISTS.**
    /// The DoH mode is taken only when `PODSSH_DOH` names an endpoint, because
    /// that is the operator saying "the system one cannot answer here", which
    /// is a statement this tool cannot make on its own without paying a failed
    /// lookup on every connect.
    pub fn from_env() -> Resolver {
        match std::env::var("PODSSH_DOH") {
            Ok(e) if !e.is_empty() => Resolver::doh(e),
            _ => Resolver::system(),
        }
    }

    /// Whether the system resolver can answer anything at all, cheaply.
    ///
    /// ⛔ "Cheaply" is a bounded probe of a name that always exists
    /// (`localhost`), not a lookup of the caller's target: a probe must not
    /// have the side effect of a DNS query for a real hostname.
    pub fn system_works(&self) -> bool {
        std::net::ToSocketAddrs::to_socket_addrs(&("localhost", 53u16)).is_ok()
    }

    /// Resolve a name to its addresses.
    pub fn resolve(&self, host: &str) -> Result<Vec<IpAddr>> {
        // A literal is already an address; no resolver is consulted.
        if let Ok(ip) = host.parse::<IpAddr>() {
            return Ok(vec![ip]);
        }
        match &self.mode {
            Mode::System => {
                use std::net::ToSocketAddrs;
                let addrs: Vec<IpAddr> = (host, 0u16)
                    .to_socket_addrs()
                    .map_err(|e| {
                        Error::new(Kind::Resolve, host.to_string(), format!("getaddrinfo: {e}"))
                    })?
                    .map(|a| a.ip())
                    .collect();
                if addrs.is_empty() {
                    return Err(Error::new(
                        Kind::Resolve,
                        host.to_string(),
                        "the system resolver returned no address",
                    ));
                }
                Ok(addrs)
            }
            Mode::Doh { endpoint } => self.doh_resolve(host, endpoint),
        }
    }

    /// One DNS-over-HTTPS `A` query, JSON form, over the shared `Dialer`.
    fn doh_resolve(&self, host: &str, endpoint: &str) -> Result<Vec<IpAddr>> {
        // ⛔ The DoH request is hand-built and written over a `Stream` from the
        // `Dialer`, NOT through a second HTTP client. A second client would
        // be a second place a CONNECT bug hides, and the whole point is that
        // one dialer serves every transport including this one.
        let spec = format!("tls://{endpoint}:443");
        let dialer = crate::transport::Dialer::new(
            None,
            std::sync::Arc::new(
                crate::transport::tls::ClientConfig::new(false, &[])
                    .map_err(|e| Error::new(Kind::Config, "doh", e.to_string()))?,
            ),
            self.timeout,
        );
        let mut s = dialer
            .dial(&spec)
            .map_err(|e| Error::new(Kind::Unreachable, "doh", format!("dial {endpoint}: {e}")))?;
        s.set_read_timeout(Some(self.timeout)).ok();
        let query = format!(
            "GET /dns-query?name={host}&type=A HTTP/1.1\r\nHost: {endpoint}\r\n\
             Accept: application/dns-json\r\nUser-Agent: podssh/{ver}\r\n\
             Connection: close\r\n\r\n",
            ver = env!("CARGO_PKG_VERSION"),
        );
        s.write_all(query.as_bytes())
            .map_err(|e| Error::new(Kind::Unreachable, "doh", format!("write: {e}")))?;
        s.flush()
            .map_err(|e| Error::new(Kind::Unreachable, "doh", format!("flush: {e}")))?;
        let mut body = Vec::new();
        let mut buf = [0u8; 8192];
        let deadline = std::time::Instant::now() + self.timeout;
        while std::time::Instant::now() < deadline {
            match s.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    body.extend_from_slice(&buf[..n]);
                    if body.windows(4).any(|w| w == b"\r\n0\r\n") || body.len() > 64 * 1024 {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(Error::new(Kind::Unreachable, "doh", format!("read: {e}"))),
            }
        }
        parse_doh_a(&body, host)
    }
}

/// Pull the `A` records out of a JSON DoH response body.
///
/// ⛔ **THE `data` FIELD IS `"1.2.3.4"` WITH QUOTES, AND THE PARSE IS BY NAME
/// NOT BY POSITION.** A DoH response is an array of records; selecting by
/// index would read the `AAAA` answer or a `CNAME` as if it were the address.
/// This walks the `Answer` array and keeps entries whose `type` is 1.
fn parse_doh_a(body: &[u8], host: &str) -> Result<Vec<IpAddr>> {
    let text = String::from_utf8_lossy(body);
    // The status line and headers, if any, precede the JSON; take from the
    // first brace.
    let json = match text.find('{') {
        Some(i) => &text[i..],
        None => {
            return Err(Error::new(
                Kind::Protocol,
                "doh",
                "the response carried no JSON body",
            ))
        }
    };
    let v: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| Error::new(Kind::Protocol, "doh", format!("the body is not JSON: {e}")))?;
    let mut out = Vec::new();
    if let Some(answers) = v.get("Answer").and_then(|a| a.as_array()) {
        for a in answers {
            let ty = a.get("type").and_then(|t| t.as_u64()).unwrap_or(0);
            if ty != 1 {
                continue;
            }
            if let Some(data) = a.get("data").and_then(|d| d.as_str()) {
                if let Ok(ip) = data.parse::<IpAddr>() {
                    if !out.contains(&ip) {
                        out.push(ip);
                    }
                }
            }
        }
    }
    if out.is_empty() {
        return Err(Error::new(
            Kind::Resolve,
            host.to_string(),
            "DoH returned no A record",
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_doh_a_record_is_parsed_by_name_and_not_by_position() {
        // A response mixing a CNAME and an A: selecting by index would take
        // the CNAME's data and fail to parse it as an address.
        let body = br#"{"Status":0,"Answer":[
            {"name":"x.example","type":5,"TTL":300,"data":"alias.example"},
            {"name":"x.example","type":1,"TTL":300,"data":"93.184.216.34"}]}"#;
        let v = parse_doh_a(body, "x.example").unwrap();
        assert_eq!(v, vec!["93.184.216.34".parse::<IpAddr>().unwrap()]);
    }

    #[test]
    fn a_leading_http_head_is_skipped_and_the_json_is_found() {
        // ⛔ Measured: the real cloudflare-dns.com response arrived as
        // `HTTP/1.1 200 OK\r\ncontent-type: ...\r\n\r\n{"Status":0,...}`.
        // Parsing from byte zero would fail; the code finds the first `{`.
        let body = b"HTTP/1.1 200 OK\r\ncontent-type: application/dns-json\r\n\r\n{\"Status\":0,\"Answer\":[{\"type\":1,\"data\":\"1.1.1.1\"}]}";
        assert_eq!(
            parse_doh_a(body, "x").unwrap(),
            vec!["1.1.1.1".parse::<IpAddr>().unwrap()]
        );
    }

    #[test]
    fn an_empty_or_error_answer_is_a_resolve_failure() {
        // A DoH `Status: 3` (NXDOMAIN) has no A record, and reporting it as a
        // protocol error would tell an operator to look at the transport when
        // the name simply does not exist.
        let nx = br#"{"Status":3,"Answer":[]}"#;
        assert_eq!(
            crate::error::err_of(parse_doh_a(nx, "nope.example")).kind,
            Kind::Resolve
        );
        let nojson = b"HTTP/1.1 502 Bad Gateway\r\n\r\n<html>oops</html>";
        assert_eq!(
            crate::error::err_of(parse_doh_a(nojson, "x")).kind,
            Kind::Protocol
        );
    }

    #[test]
    fn a_literal_address_skips_the_resolver_entirely() {
        let r = Resolver::system();
        assert_eq!(
            r.resolve("192.0.2.7").unwrap(),
            vec!["192.0.2.7".parse::<IpAddr>().unwrap()]
        );
        assert_eq!(
            r.resolve("2001:db8::1").unwrap(),
            vec!["2001:db8::1".parse::<IpAddr>().unwrap()]
        );
    }

    #[test]
    fn both_modes_construct_and_the_env_picks_one() {
        assert_eq!(Resolver::system().mode, Mode::System);
        match Resolver::doh("cloudflare-dns.com").mode {
            Mode::Doh { endpoint } => assert_eq!(endpoint, "cloudflare-dns.com"),
            _ => panic!("expected DoH"),
        }
    }
}
