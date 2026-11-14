//! The origin of a Windows download: podvm's HTTP client for
//! `pod_guest::fetch::store` (podcrates `v0.45.0`, podcrates@T-590).
//!
//! TODO/machine.md T-1862: each GET after the first carries
//! `Range: bytes=N-`, so a download of gigabytes that breaks resumes where
//! it broke. `store` decides the tries, checks the `206`, and keeps the
//! partial file beside the destination. The body prints one progress line
//! on stderr for each twentieth of the object.

use std::io::Read;

use podvm_image::net::Body;
use podvm_windows::fetch::{Origin, Reply};

/// The tries of one download: the first GET and each resume.
pub(crate) const TRIES: u32 = 5;

/// One URL, through the proxy of the environment, with the redirects
/// followed (`super::setup::FETCH_LIMITS`).
pub(crate) struct Http {
    pub(crate) url: String,
}

impl Origin for Http {
    type Body = Progress<Body>;

    fn get(&mut self, from: u64) -> std::io::Result<Reply<Progress<Body>>> {
        let range = [("Range", format!("bytes={from}-"))];
        let headers: &[(&str, String)] = if from > 0 { &range } else { &[] };
        let body = super::setup::get(&self.url, headers, super::setup::FETCH_LIMITS)
            .map_err(std::io::Error::other)?;
        let length = body
            .header("Content-Length")
            .and_then(|v| v.trim().parse::<u64>().ok());
        let range = body.header("Content-Range").map(str::to_string);
        let status = body.status();
        // The size of the object: the part of a `206` starts at `from`.
        let total = length.map(|l| if status == 206 { from + l } else { l });
        Ok(Reply {
            status,
            length,
            range,
            body: Progress::new(body, if status == 206 { from } else { 0 }, total),
        })
    }
}

/// A body that counts the bytes of the whole object and prints a line at
/// each step.
pub(crate) struct Progress<R> {
    inner: R,
    seen: u64,
    total: Option<u64>,
    next: u64,
    step: u64,
}

impl<R> Progress<R> {
    pub(crate) fn new(inner: R, seen: u64, total: Option<u64>) -> Progress<R> {
        let step = total.map(|t| t / 20).unwrap_or(0).max(8 << 20);
        Progress {
            inner,
            seen,
            total,
            next: seen + step,
            step,
        }
    }
}

/// The line of a step, in MiB.
pub(crate) fn line(seen: u64, total: Option<u64>) -> String {
    let mib = |b: u64| b / (1 << 20);
    match total {
        Some(t) => format!("podvm windows: {} MiB of {} MiB", mib(seen), mib(t)),
        None => format!("podvm windows: {} MiB", mib(seen)),
    }
}

impl<R: Read> Read for Progress<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.seen += n as u64;
        if self.seen >= self.next {
            eprintln!("{}", line(self.seen, self.total));
            self.next = self.seen + self.step;
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    /// TODO/machine.md T-1862: the second GET of the HTTP origin carries
    /// the range, against a server on the loopback.
    #[test]
    fn windows_origin_sends_a_range_on_a_resume() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((s, _)) = l.accept() else { return };
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut head = Vec::new();
                loop {
                    let mut h = String::new();
                    if r.read_line(&mut h).unwrap_or(0) == 0 || h == "\r\n" {
                        break;
                    }
                    head.push(h.trim_end().to_string());
                }
                let range = head
                    .iter()
                    .any(|h| h.to_ascii_lowercase().starts_with("range: bytes=4-"));
                let _ = tx.send(range);
                let mut s = s;
                let reply = if range {
                    "HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 4-7/8\r\n\r\nefgh"
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\nabcdefgh"
                };
                let _ = s.write_all(reply.as_bytes());
            }
        });
        let mut o = Http {
            url: format!("http://127.0.0.1:{port}/medium.iso"),
        };
        let first = o.get(0).unwrap();
        assert_eq!(first.status, 200);
        assert!(!rx.recv().unwrap(), "the first GET carries a range");
        let mut second = o.get(4).unwrap();
        assert!(rx.recv().unwrap(), "the resume carries no range");
        assert_eq!(second.status, 206);
        let mut rest = String::new();
        second.body.read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "efgh");
    }

    #[test]
    fn windows_origin_progress_line_names_the_mib() {
        assert_eq!(
            line(3 << 20, Some(10 << 20)),
            "podvm windows: 3 MiB of 10 MiB"
        );
        assert_eq!(line(3 << 20, None), "podvm windows: 3 MiB");
    }
}
