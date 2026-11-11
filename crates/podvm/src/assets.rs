//! The rule for each file that podvm takes from outside (TODO/cli.md
//! T-1667, issue 188): a pin where podvm knows the SHA-256, else a hash
//! recorded at the first use in the store, and a refusal when the file
//! changes after that.
//!
//! A checksum proves integrity, not who built the file, and [`Proof::words`]
//! says which of the two a check gives.

use std::path::{Path, PathBuf};

/// What a check of an asset proves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proof {
    /// The SHA-256 is podvm's pin.
    Pinned,
    /// The first use: the SHA-256 is recorded now.
    FirstUse,
    /// The SHA-256 is the one recorded at the first use.
    Known,
}

impl Proof {
    pub fn words(self) -> &'static str {
        match self {
            Proof::Pinned => "integrity against podvm's pin (SHA-256), not who built the file",
            Proof::FirstUse => {
                "first use: its SHA-256 is recorded now, and a change refuses; this proves \
                 neither integrity nor who built the file"
            }
            Proof::Known => {
                "the SHA-256 of its first use: integrity since then, not who built the file"
            }
        }
    }
}

/// The SHA-256 of a file, in hex.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    use sha2::Digest;
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut h = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// The record of the asset `name` at `path`: one file under
/// `<store>/assets/`, named by the SHA-256 of the name and the path, so a
/// path of another asset is another record.
pub fn record_path(store_root: &Path, name: &str, path: &Path) -> PathBuf {
    use sha2::Digest;
    let key = sha2::Sha256::digest(format!("{name}\0{}", path.display()).as_bytes());
    let hex: String = key.iter().take(8).map(|b| format!("{b:02x}")).collect();
    store_root
        .join("assets")
        .join(format!("{name}-{hex}.sha256"))
}

/// Check the asset `name` at `path`: against `pin` where podvm has one,
/// else against the record of its first use. Err names both hashes and the
/// record to remove to accept a new file.
pub fn verify(
    store_root: &Path,
    name: &str,
    path: &Path,
    pin: Option<&str>,
) -> Result<Proof, String> {
    let got = sha256_file(path)?;
    if let Some(want) = pin {
        return if got == want {
            Ok(Proof::Pinned)
        } else {
            Err(format!(
                "{name} {} has SHA-256 {got}, not podvm's pin {want}",
                path.display()
            ))
        };
    }
    let rec = record_path(store_root, name, path);
    match std::fs::read_to_string(&rec) {
        Ok(text) => {
            let want = text.split_whitespace().next().unwrap_or("");
            if want == got {
                Ok(Proof::Known)
            } else {
                Err(format!(
                    "{name} {} changed since its first use: its SHA-256 was {want} and is \
                     {got} now. podvm refuses a changed file; remove {} to accept the new \
                     one (TODO/cli.md T-1667)",
                    path.display(),
                    rec.display()
                ))
            }
        }
        Err(_) => {
            if let Some(dir) = rec.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            std::fs::write(&rec, format!("{got}  {}\n", path.display()))
                .map_err(|e| format!("{}: {e}", rec.display()))?;
            Ok(Proof::FirstUse)
        }
    }
}

/// The space floor before a download of `need` bytes to `dir`: Err names
/// the path, the free and the needed space.
pub fn space_floor(dir: &Path, what: &str, need: u64) -> Result<(), String> {
    podvm_image::space::require(&dir.to_string_lossy(), what, need, 8, 0)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("podvm-assets-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The Prove: a planted change of an asset after its first use is
    /// refused, and the message names the record to remove.
    #[test]
    fn a_planted_changed_asset_is_refused() {
        let d = tmp("change");
        let f = d.join("vmlinuz");
        std::fs::write(&f, b"kernel one").unwrap();
        assert_eq!(verify(&d, "machine-kernel", &f, None), Ok(Proof::FirstUse));
        assert_eq!(verify(&d, "machine-kernel", &f, None), Ok(Proof::Known));
        std::fs::write(&f, b"kernel two").unwrap();
        let e = verify(&d, "machine-kernel", &f, None).unwrap_err();
        assert!(e.contains("changed since its first use"), "{e}");
        assert!(
            e.contains(&record_path(&d, "machine-kernel", &f).display().to_string()),
            "{e}"
        );
        // Another path is another asset.
        let g = d.join("other");
        std::fs::write(&g, b"kernel two").unwrap();
        assert_eq!(verify(&d, "machine-kernel", &g, None), Ok(Proof::FirstUse));
        // A pin wins over a record and refuses a mismatch.
        let pin = sha256_file(&f).unwrap();
        assert_eq!(
            verify(&d, "machine-kernel", &f, Some(&pin)),
            Ok(Proof::Pinned)
        );
        assert!(verify(&d, "x", &f, Some("00"))
            .unwrap_err()
            .contains("not podvm's pin"));
        assert!(Proof::Pinned.words().contains("not who built"));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// The Prove: a download below the space floor refuses and names the
    /// path.
    #[test]
    fn a_run_below_the_space_floor_refuses_and_names_the_path() {
        let d = tmp("floor");
        let e = space_floor(&d, "the guest kernel archive", u64::MAX / 4).unwrap_err();
        assert!(e.contains(&d.display().to_string()), "{e}");
        assert!(e.contains("the guest kernel archive"), "{e}");
        assert!(space_floor(&d, "a small file", 1).is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }
}
