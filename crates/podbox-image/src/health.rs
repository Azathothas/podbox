//! TODO/image.md T-1321. Store health and pull provenance.
//!
//! Digests are enforced per read, but nothing answered "is the store
//! healthy, and how did the bytes get there". `verify_blobs` sweeps
//! indexed blobs against their digests (corruption, partial writes and
//! bit-rot get a one-command check), and `note_pull`/`read_provenance`
//! keep one JSON line per pulled manifest beside the index. Verify
//! reports; it never refetches, and no new authority is introduced.

use std::io::Read;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::digest::Digest;
use crate::error::{Error, Result};
use crate::store::Store;

/// One line of the provenance sidecar: how one manifest's bytes got here.
/// Written by `pull`, read by `verify`. Six fields, every one recorded at
/// pull time rather than derived later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub registry: String,
    pub repository: String,
    pub tag: Option<String>,
    pub manifest_digest: String,
    pub pulled_at: String,
    /// `CARGO_PKG_VERSION`: the crate that pulls is versioned with the
    /// workspace, and no build script carries the commit into this crate,
    /// so the release line is what the sidecar can honestly name.
    pub podbox_version: String,
}

/// One blob the sweep refused: what the index names and what the file
/// hashes to, or the absence where no file is there at all.
#[derive(Debug, Clone)]
pub struct BlobMismatch {
    pub want: String,
    pub got: String,
}

const PROVENANCE_FILE: &str = "provenance.jsonl";

impl Store {
    fn provenance_path(&self) -> PathBuf {
        self.root().join(PROVENANCE_FILE)
    }

    /// Record one pull's provenance as one JSON line, under the
    /// store-wide lock so two pulls cannot interleave mid-line.
    pub fn note_pull(&self, provenance: &Provenance) -> Result<()> {
        let _guard = self.lock()?;
        let mut line = serde_json::to_string(provenance)
            .map_err(|e| Error::Oci(format!("the provenance line does not render: {e}")))?;
        line.push('\n');
        append_bytes(&self.provenance_path(), line.as_bytes())
    }

    /// Every provenance line, oldest first. A corrupt line refuses with
    /// its number: a health sidecar that silently skips what it cannot
    /// read is the defect this entry exists to close.
    pub fn read_provenance(&self) -> Result<Vec<Provenance>> {
        let path = self.provenance_path();
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(Error::io(path.display().to_string(), e)),
        };
        let text = String::from_utf8(bytes)
            .map_err(|e| Error::Oci(format!("{} is not UTF-8 JSON lines: {e}", path.display())))?;
        let mut out = Vec::new();
        for (n, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            out.push(serde_json::from_str(line).map_err(|e| {
                Error::Oci(format!(
                    "{} line {} does not parse: {e}",
                    path.display(),
                    n + 1
                ))
            })?);
        }
        Ok(out)
    }

    /// Hash every named blob against its digest, streaming so a large
    /// store never sits fully in memory. One entry per blob that fails,
    /// silent where all hold.
    pub fn verify_blobs(&self, digests: &[String]) -> Result<Vec<BlobMismatch>> {
        let mut mismatches = Vec::new();
        for want in digests {
            let digest = Digest::parse(want)?;
            let path = self.blob_path(&digest);
            let got = match stream_digest(&path) {
                Ok(got) => got,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => "absent".to_string(),
                Err(e) => return Err(Error::io(path.display().to_string(), e)),
            };
            if got != *want {
                mismatches.push(BlobMismatch {
                    want: want.clone(),
                    got,
                });
            }
        }
        Ok(mismatches)
    }
}

fn append_bytes(path: &PathBuf, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| Error::io(path.display().to_string(), e))?;
    f.write_all(bytes)
        .map_err(|e| Error::io(path.display().to_string(), e))?;
    Ok(())
}

fn stream_digest(path: &PathBuf) -> std::io::Result<String> {
    use sha2::Digest as _;
    let mut f = std::fs::File::open(path)?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!(
        "sha256:{}",
        crate::digest::hex_of(&hasher.finalize())
    ))
}
