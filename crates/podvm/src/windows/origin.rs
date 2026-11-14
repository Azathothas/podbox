//! The origin of a Windows download: podvm's HTTP client for
//! `pod_guest::fetch::store` (podcrates `v0.45.0`, podcrates@T-590).
//!
//! TODO/machine.md T-1862: each GET after the first carries
//! `Range: bytes=N-`, so a download of gigabytes that breaks resumes where
//! it broke. `store` decides the tries, checks the `206`, and keeps the
//! partial file beside the destination.

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
    type Body = Body;

    fn get(&mut self, from: u64) -> std::io::Result<Reply<Body>> {
        let range = [("Range", format!("bytes={from}-"))];
        let headers: &[(&str, String)] = if from > 0 { &range } else { &[] };
        let body = super::setup::get(&self.url, headers, super::setup::FETCH_LIMITS)
            .map_err(std::io::Error::other)?;
        Ok(Reply {
            status: body.status(),
            length: body
                .header("Content-Length")
                .and_then(|v| v.trim().parse::<u64>().ok()),
            range: body.header("Content-Range").map(str::to_string),
            body,
        })
    }
}
