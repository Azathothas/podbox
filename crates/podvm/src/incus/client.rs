//! A minimal client of the incus REST API on its unix socket, TODO/machine.md
//! T-1718.
//!
//! HTTP/1.1 with `Connection: close`, so each answer ends at the end of the
//! stream; a chunked body is decoded. incus answers each call with a JSON
//! envelope (`type` sync or async, `status_code`, `metadata`); an async call
//! names an operation, and [`Client::wait`] waits for it. This module goes
//! to a podcrates crate when podcrates tags one (COORD-034, issue 253).

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

/// The socket of an incusd: `PODVM_INCUS_SOCKET`, else `INCUS_DIR`'s, else
/// the system daemon's.
pub fn socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os("PODVM_INCUS_SOCKET") {
        return PathBuf::from(p);
    }
    if let Some(d) = std::env::var_os("INCUS_DIR") {
        return PathBuf::from(d).join("unix.socket");
    }
    PathBuf::from("/var/lib/incus/unix.socket")
}

pub struct Client {
    sock: PathBuf,
}

/// One answer: the HTTP status and the body.
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn json(&self) -> Result<Value, String> {
        serde_json::from_slice(&self.body).map_err(|e| format!("incus answered no JSON: {e}"))
    }
}

/// The `metadata` of a sync answer, or the error text of incus.
pub fn metadata(r: &Reply) -> Result<Value, String> {
    let v = r.json()?;
    if v.get("type").and_then(Value::as_str) == Some("error") || r.status >= 400 {
        let e = v
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("an error with no text");
        return Err(format!("incus: {e} (HTTP {})", r.status));
    }
    Ok(v.get("metadata").cloned().unwrap_or(Value::Null))
}

impl Client {
    pub fn new(sock: &Path) -> Client {
        Client {
            sock: sock.to_path_buf(),
        }
    }

    /// One call. `body` is the content type and the bytes; `headers` are
    /// more lines (`Name: value`).
    pub fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<(&str, &[u8])>,
        headers: &[String],
    ) -> Result<Reply, String> {
        let mut s =
            UnixStream::connect(&self.sock).map_err(|e| format!("{}: {e}", self.sock.display()))?;
        let _ = s.set_read_timeout(Some(Duration::from_secs(600)));
        let mut head = format!("{method} {path} HTTP/1.1\r\nHost: incus\r\nConnection: close\r\n");
        for h in headers {
            head.push_str(h);
            head.push_str("\r\n");
        }
        if let Some((ct, b)) = body {
            head.push_str(&format!(
                "Content-Type: {ct}\r\nContent-Length: {}\r\n",
                b.len()
            ));
        }
        head.push_str("\r\n");
        let io = |e: std::io::Error| format!("{}: {e}", self.sock.display());
        s.write_all(head.as_bytes()).map_err(io)?;
        if let Some((_, b)) = body {
            s.write_all(b).map_err(io)?;
        }
        let mut raw = Vec::new();
        s.read_to_end(&mut raw).map_err(io)?;
        parse(&raw)
    }

    /// A GET, and its metadata.
    pub fn get(&self, path: &str) -> Result<Value, String> {
        metadata(&self.call("GET", path, None, &[])?)
    }

    /// A call with a JSON body; for an async answer, the wait for its
    /// operation. The metadata of the end.
    pub fn send(&self, method: &str, path: &str, body: &Value) -> Result<Value, String> {
        let b = serde_json::to_vec(body).map_err(|e| e.to_string())?;
        let r = self.call(method, path, Some(("application/json", &b)), &[])?;
        self.finish(&r)
    }

    /// The end of an answer: its metadata, or, where it names an
    /// operation, the metadata of that operation when it ends.
    pub fn finish(&self, r: &Reply) -> Result<Value, String> {
        let v = r.json()?;
        let meta = metadata(r)?;
        if v.get("type").and_then(Value::as_str) == Some("async") {
            let op = v
                .get("operation")
                .and_then(Value::as_str)
                .ok_or("incus: an async answer names no operation")?;
            return self.wait(op);
        }
        Ok(meta)
    }

    /// Wait for the operation `op` (its path) and give its metadata.
    pub fn wait(&self, op: &str) -> Result<Value, String> {
        let m = self.get(&format!("{op}/wait?timeout=600"))?;
        let code = m.get("status_code").and_then(Value::as_u64).unwrap_or(0);
        if code != 200 {
            let e = m
                .get("err")
                .and_then(Value::as_str)
                .unwrap_or("the operation did not end");
            return Err(format!("incus: {e} (status {code})"));
        }
        Ok(m.get("metadata").cloned().unwrap_or(Value::Null))
    }
}

/// An HTTP/1.1 answer: the status, the headers and the body (plain or
/// chunked).
pub fn parse(raw: &[u8]) -> Result<Reply, String> {
    let end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("incus: an answer with no end of headers")?;
    let head = String::from_utf8_lossy(&raw[..end]);
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|l| l.split(' ').nth(1))
        .and_then(|c| c.parse::<u16>().ok())
        .ok_or("incus: an answer with no status")?;
    let chunked = lines.any(|l| {
        let l = l.to_ascii_lowercase();
        l.starts_with("transfer-encoding:") && l.contains("chunked")
    });
    let rest = &raw[end + 4..];
    let body = if chunked {
        dechunk(rest)?
    } else {
        rest.to_vec()
    };
    Ok(Reply { status, body })
}

fn dechunk(mut b: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let eol = b
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("incus: a chunk with no size line")?;
        let size_text = String::from_utf8_lossy(&b[..eol]);
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| "incus: a chunk size that does not parse".to_string())?;
        b = &b[eol + 2..];
        if size == 0 {
            return Ok(out);
        }
        let chunk = b.get(..size).ok_or("incus: a chunk is cut")?;
        out.extend_from_slice(chunk);
        b = b.get(size + 2..).unwrap_or(&[]);
    }
}
