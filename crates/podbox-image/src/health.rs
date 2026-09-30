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
use crate::store::{Record, Store};

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

    /// Every blob one record needs, on disk and hashing to its name.
    ///
    /// ⭐ TODO/image.md T-1419. Presence means blobs on disk, not a row in
    /// the store: the CLI `acquire` path (`--pull missing`) calls this
    /// before trusting a record, and a missing blob is a named refusal,
    /// never a raw path. A blob with no file reports `got: "absent"`;
    /// `--pull always` stays the recovery path.
    ///
    /// SEAM for the CLI agent: `Store::verify_record_blobs(&self, record:
    /// &Record) -> Result<Vec<BlobMismatch>>`. An empty vector means the
    /// record is runnable; a non-empty one names each missing or corrupt
    /// blob by digest, and the caller refuses by image name with the remedy.
    pub fn verify_record_blobs(&self, record: &Record) -> Result<Vec<BlobMismatch>> {
        let digests: Vec<String> = record.blobs().iter().map(|s| s.to_string()).collect();
        self.verify_blobs(&digests)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A store only this test names: the suite runs threads in one process,
    /// and two tests sharing a directory share a store.
    fn scratch(name: &str) -> Store {
        let d = std::env::temp_dir().join(format!("podbox-health-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        Store::open(d).unwrap()
    }

    fn plant(store: &Store, bytes: &[u8]) -> String {
        let want = Digest::of(bytes);
        store
            .put_bytes(bytes, &want, "health test plants a blob")
            .unwrap();
        want.to_string()
    }

    fn record_with(layer: &str, config: &str) -> Record {
        Record {
            repository: "example.test/library/alpine".to_string(),
            tag: Some("3.20".to_string()),
            digest: config.to_string(),
            digest_media_type: "application/vnd.oci.image.index.v1+json".to_string(),
            manifest_digest: config.to_string(),
            config_digest: config.to_string(),
            platform: "linux/amd64".to_string(),
            layers: vec![layer.to_string()],
            stored_bytes: 0,
            architecture: "amd64".to_string(),
            os: "linux".to_string(),
            created: None,
            pulled_at: "2026-09-30T00:00:00Z".to_string(),
        }
    }

    /// The happy path the CLI trusts: every blob present and whole verifies
    /// silent.
    #[test]
    fn verify_blobs_is_silent_where_every_blob_holds() {
        let s = scratch("all-hold");
        let a = plant(&s, b"layer-a");
        let b = plant(&s, b"config-b");
        assert!(s.verify_blobs(&[a, b]).unwrap().is_empty());
    }

    /// TODO/image.md T-1419. A record whose blob is gone counts as present
    /// nowhere here: the deleted blob reports as absent by digest.
    #[test]
    fn verify_blobs_names_a_deleted_blob_as_absent() {
        let s = scratch("deleted-absent");
        let a = plant(&s, b"layer-a");
        let b = plant(&s, b"config-b");
        let digest = Digest::parse(&b).unwrap();
        std::fs::remove_file(s.blob_path(&digest)).unwrap();
        let mismatches = s.verify_blobs(&[a, b.clone()]).unwrap();
        assert_eq!(mismatches.len(), 1, "only the deleted blob reports");
        assert_eq!(mismatches[0].want, b);
        assert_eq!(mismatches[0].got, "absent");
    }

    /// Corruption is the same shape with the actual digest: the report names
    /// what the file hashes to, not just that it differs.
    #[test]
    fn verify_blobs_names_a_corrupted_blob_by_its_actual_digest() {
        let s = scratch("corrupted-actual");
        let a = plant(&s, b"layer-a");
        let digest = Digest::parse(&a).unwrap();
        std::fs::write(s.blob_path(&digest), b"something else entirely").unwrap();
        let mismatches = s.verify_blobs(std::slice::from_ref(&a)).unwrap();
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].want, a);
        assert_eq!(
            mismatches[0].got,
            Digest::of(b"something else entirely").to_string()
        );
    }

    /// TODO/image.md T-1419, the seam the CLI `acquire` path calls: one call
    /// over the record checks every blob the record names, and an empty
    /// vector means the record is runnable.
    #[test]
    fn verify_record_blobs_checks_every_blob_the_record_names() {
        let s = scratch("record-seam");
        let layer = plant(&s, b"layer-a");
        let config = plant(&s, b"config-b");
        let record = record_with(&layer, &config);
        assert!(s.verify_record_blobs(&record).unwrap().is_empty());
        let digest = Digest::parse(&layer).unwrap();
        std::fs::remove_file(s.blob_path(&digest)).unwrap();
        let mismatches = s.verify_record_blobs(&record).unwrap();
        assert_eq!(mismatches.len(), 1, "the deleted layer reports");
        assert_eq!(mismatches[0].want, layer);
        assert_eq!(mismatches[0].got, "absent");
    }
}
