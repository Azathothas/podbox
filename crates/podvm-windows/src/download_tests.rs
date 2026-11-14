//! TODO/machine.md T-1862: the download of a Windows medium through
//! `fetch::store` of `pod-guest` (podcrates `v0.45.0`, podcrates@T-590),
//! against an origin in memory that serves ranges and breaks once in the
//! middle, as a flaky CDN does. The check of `RLIMIT_FSIZE` before the
//! first byte is podvm's (`windows_fetch_refuses` in `windows/pull.rs`).

use crate::fetch::{store, Origin, Reply};
use crate::Ceiling;
use std::io::Read;

/// A body that ends with a reset after `stop` bytes, if set.
struct Cut {
    data: Vec<u8>,
    pos: usize,
    stop: Option<usize>,
}

impl Read for Cut {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.stop == Some(self.pos) {
            return Err(std::io::ErrorKind::ConnectionReset.into());
        }
        let end = self.stop.unwrap_or(self.data.len()).min(self.data.len());
        let n = buf.len().min(end - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// The origin: the first GET breaks at `break_at`; each later GET honours
/// the range.
struct Flaky {
    data: Vec<u8>,
    break_at: Option<usize>,
    froms: Vec<u64>,
}

impl Origin for Flaky {
    type Body = Cut;
    fn get(&mut self, from: u64) -> std::io::Result<Reply<Cut>> {
        self.froms.push(from);
        let len = self.data.len() as u64;
        let stop = self.break_at.take();
        if from == 0 {
            return Ok(Reply {
                status: 200,
                length: Some(len),
                range: None,
                body: Cut {
                    data: self.data.clone(),
                    pos: 0,
                    stop,
                },
            });
        }
        Ok(Reply {
            status: 206,
            length: Some(len - from),
            range: Some(format!("bytes {from}-{}/{len}", len - 1)),
            body: Cut {
                data: self.data[from as usize..].to_vec(),
                pos: 0,
                stop: None,
            },
        })
    }
}

fn scratch(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-dl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn data() -> Vec<u8> {
    (0..200_000u32).map(|i| (i % 251) as u8).collect()
}

const OPEN: Ceiling = Ceiling {
    max_bytes: u64::MAX,
    fsize: u64::MAX,
};

#[test]
fn download_resumes_after_a_break_with_the_right_digest() {
    let d = scratch("resume");
    let dest = d.join("medium.iso");
    let mut clean = Flaky {
        data: data(),
        break_at: None,
        froms: Vec::new(),
    };
    let want = store(&mut clean, &d.join("clean.iso"), None, OPEN, 1).unwrap();
    let mut flaky = Flaky {
        data: data(),
        break_at: Some(70_000),
        froms: Vec::new(),
    };
    let got = store(&mut flaky, &dest, Some(&want), OPEN, 3).unwrap();
    assert_eq!(got, want);
    assert_eq!(std::fs::read(&dest).unwrap(), data());
    assert_eq!(flaky.froms.first(), Some(&0));
    assert!(
        flaky.froms.get(1).is_some_and(|f| *f > 0),
        "no resume: {:?}",
        flaky.froms
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn download_with_a_wrong_digest_leaves_no_file() {
    let d = scratch("digest");
    let mut o = Flaky {
        data: data(),
        break_at: None,
        froms: Vec::new(),
    };
    let wrong = "0".repeat(64);
    assert!(store(&mut o, &d.join("medium.iso"), Some(&wrong), OPEN, 1).is_err());
    let left: Vec<_> = std::fs::read_dir(&d).unwrap().flatten().collect();
    assert!(left.is_empty(), "{left:?}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn download_over_the_ceiling_is_refused_with_no_file() {
    let d = scratch("ceiling");
    let mut o = Flaky {
        data: data(),
        break_at: None,
        froms: Vec::new(),
    };
    let low = Ceiling {
        max_bytes: 1000,
        fsize: u64::MAX,
    };
    assert!(store(&mut o, &d.join("medium.iso"), None, low, 1).is_err());
    let left: Vec<_> = std::fs::read_dir(&d).unwrap().flatten().collect();
    assert!(left.is_empty(), "{left:?}");
    let _ = std::fs::remove_dir_all(&d);
}
