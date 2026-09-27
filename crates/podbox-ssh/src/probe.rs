//! `podssh probe`: measure what this machine permits, verify the candidate
//! relays, and say which shape will work. Every number is produced here, by
//! this process, on this host.
//!
//! ⛔ The probe never trusts the catalog. It re-dials every entry and reports
//! what happened now, because an egress relay that answered yesterday can be
//! dead today and a cage only has one open port to spend.

use std::io::Write;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::catalog;
use crate::protocol::{self, Role};
use crate::transport::proxy::Proxy;
use crate::transport::{connect_tcp, Dialer, Stream};

pub struct Config {
    pub dialer: Dialer,
    pub relays: Vec<String>,
    pub proxies: Vec<String>,
    pub target: String,
}

pub fn run(cfg: &Config) -> i32 {
    let mut out = Value::Object(Default::default());
    out["local"] = local_egress();
    out["proxies"] = Value::Array(
        cfg.proxies
            .iter()
            .map(|p| probe_proxy(p, &cfg.target))
            .collect(),
    );
    out["relays"] = Value::Array(cfg.relays.iter().map(|r| probe_relay(&cfg.dialer, r)).collect());
    out["catalog"] = json!({
        "egress": catalog::EGRESS.iter().map(|e| json!({
            "url": e.url, "verified": e.verified, "method": e.method, "note": e.note,
        })).collect::<Vec<_>>(),
        "turn": catalog::TURN.iter().map(|t| json!({
            "url": t.url, "verified": t.verified, "note": t.note,
        })).collect::<Vec<_>>(),
    });

    let recommend = recommend(&out);
    out["recommended"] = json!(recommend);

    print!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    println!();

    let any_relay = out["relays"]
        .as_array()
        .map(|a| a.iter().any(|r| r["status"] == "ok"))
        .unwrap_or(false);
    let any_proxy = out["proxies"]
        .as_array()
        .map(|a| a.iter().any(|r| r["status"] == "ssh-banner"))
        .unwrap_or(false);
    if any_relay || any_proxy {
        0
    } else {
        1
    }
}

fn local_egress() -> Value {
    let direct = |host: &str, port: u16| -> String {
        match connect_tcp(host, port, Duration::from_secs(4)) {
            Ok(_s) => "open".to_string(),
            Err(e) => format!("{}: {e}", e.kind()),
        }
    };
    let bind = std::net::TcpListener::bind("127.0.0.1:0")
        .map(|_| "allowed".to_string())
        .unwrap_or_else(|e| format!("denied: {e}"));
    let unix = std::os::unix::net::UnixStream::pair()
        .map(|_| "allowed".to_string())
        .unwrap_or_else(|e| format!("denied: {e}"));
    let udp = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:53")?;
            s.send(&[0u8; 12])?;
            Ok(())
        })
        .map(|_| "send-ok".to_string())
        .unwrap_or_else(|e| format!("denied: {e}"));
    json!({
        "tcp_443_1.1.1.1": direct("1.1.1.1", 443),
        "tcp_80_1.1.1.1": direct("1.1.1.1", 80),
        "tcp_22_1.1.1.1": direct("1.1.1.1", 22),
        "tcp_bind_loopback": bind,
        "unix_socketpair": unix,
        "udp_send_8.8.8.8_53": udp,
    })
}

fn probe_proxy(url: &str, target: &str) -> Value {
    let started = Instant::now();
    let (host, port) = match split(target) {
        Ok(v) => v,
        Err(e) => return json!({"url": url, "status": "bad-target", "error": e.to_string()}),
    };
    let proxy = match Proxy::parse(url) {
        Ok(p) => p,
        Err(e) => return json!({"url": url, "status": "bad-url", "error": e.to_string()}),
    };
    match proxy.connect(&host, port, Duration::from_secs(8)) {
        Err(e) => json!({"url": url, "target": target, "status": "connect-failed",
            "error": e.to_string(), "ms": started.elapsed().as_millis()}),
        Ok(mut s) => match read_banner(&mut *s, Duration::from_secs(8)) {
            Ok(b) => json!({"url": url, "target": target, "status": "ssh-banner",
                "banner": String::from_utf8_lossy(&b).trim_end(),
                "ms": started.elapsed().as_millis()}),
            Err(e) => json!({"url": url, "target": target, "status": "connect-ok-no-banner",
                "error": e.to_string(), "ms": started.elapsed().as_millis()}),
        },
    }
}

fn probe_relay(dialer: &Dialer, url: &str) -> Value {
    let started = Instant::now();
    let mut s = match dialer.dial(url) {
        Ok(s) => s,
        Err(e) => return json!({"url": url, "status": "dial-failed", "error": e.to_string()}),
    };
    let auth = catalog::auth(None);
    match protocol::greet(&mut *s, protocol::Protocol::Podssh1, Role::Probe, "probe", &auth,
        Duration::from_secs(10))
    {
        Ok(()) => json!({"url": url, "status": "ok", "ms": started.elapsed().as_millis()}),
        Err(e) => json!({"url": url, "status": "handshake-failed", "error": e.to_string(),
            "ms": started.elapsed().as_millis()}),
    }
}

fn recommend(out: &Value) -> String {
    let any_relay = out["relays"]
        .as_array()
        .map(|a| a.iter().any(|r| r["status"] == "ok"))
        .unwrap_or(false);
    let any_proxy = out["proxies"]
        .as_array()
        .map(|a| a.iter().any(|r| r["status"] == "ssh-banner"))
        .unwrap_or(false);
    if any_relay {
        "rendezvous: at least one relay paired. `podssh serve` and `podssh connect` are ready"
            .to_string()
    } else if any_proxy {
        "egress: a proxy reaches a real ssh server. Use it as --proxy for the rendezvous, \
         or `podssh forward --target <host:port> --proxy <url>` with no relay at all"
            .to_string()
    } else {
        "no route found: no relay paired and no proxy reached an ssh banner. Run `podssh relay` \
         on a host this cage can reach and pass its URL, or add the host to the egress allowlist"
            .to_string()
    }
}

fn read_banner(s: &mut dyn Stream, timeout: Duration) -> std::io::Result<Vec<u8>> {
    s.set_read_timeout(Some(timeout))?;
    let mut out = Vec::new();
    let mut b = [0u8; 1];
    while out.len() < 255 {
        match s.read(&mut b) {
            Ok(0) => break,
            Ok(_) => {
                out.push(b[0]);
                if out.ends_with(b"\r\n") || out.ends_with(b"\n") {
                    return Ok(out);
                }
            }
            Err(e) => return Err(e),
        }
    }
    if out.is_empty() {
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "no banner",
        ))
    } else {
        Ok(out)
    }
}

fn split(addr: &str) -> std::io::Result<(String, u16)> {
    let (h, p) = addr
        .rsplit_once(':')
        .ok_or_else(|| std::io::Error::other("not host:port"))?;
    Ok((
        h.to_string(),
        p.parse::<u16>()
            .map_err(|_| std::io::Error::other("bad port"))?,
    ))
}

#[allow(dead_code)]
fn silence(mut s: std::net::TcpStream) {
    let _ = s.write_all(b"");
}
