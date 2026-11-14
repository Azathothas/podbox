//! The pull of a name of the catalog (TODO/machine.md T-1861): the download
//! of its pinned URL, bounded by its size and checked against its sha256,
//! into the store of the catalog, and the base image that it gives.

use std::io::Read;
use std::path::{Path, PathBuf};

use podvm_image::error::EXIT_RUNTIME_ERROR;
use podvm_windows::catalog::{Entry, Kind};

/// The member of the FreeDOS zip that is the disk, and its size
/// (`experiments/369-windows-tcg-dos.sh`).
const DOS_IMAGE: &str = "FD14LITE.img";
const DOS_IMAGE_BYTES: u64 = 33_554_432;

/// The download of `e` in the store of the catalog: fetched and checked
/// where it is not there.
pub(crate) fn fetched(verb: &str, e: &Entry) -> Result<PathBuf, i32> {
    let dir = podvm_windows::catalog::store_dir();
    let dest = dir.join(e.file);
    if dest.is_file() {
        return Ok(dest);
    }
    if let Err(err) = std::fs::create_dir_all(&dir) {
        eprintln!("podvm {verb}: {}: {err}", dir.display());
        return Err(EXIT_RUNTIME_ERROR);
    }
    eprintln!(
        "podvm {verb}: windows:{}: fetching {} ({} bytes) into {}",
        e.name,
        e.url,
        e.size,
        dest.display()
    );
    let ceiling = podvm_windows::Ceiling {
        max_bytes: e.ceiling(),
        fsize: super::plan::ceiling()?,
    };
    let response = super::setup::get(e.url, super::setup::FETCH_LIMITS).map_err(|err| {
        eprintln!("podvm {verb}: {}: {err}", e.url);
        EXIT_RUNTIME_ERROR
    })?;
    let declared = response
        .header("Content-Length")
        .and_then(|v| v.trim().parse::<u64>().ok());
    match podvm_windows::fetch::store(response, declared, &dest, Some(e.sha256), ceiling) {
        Ok(_) => Ok(dest),
        Err(err) => {
            eprintln!("podvm {verb}: windows:{}: {err}", e.name);
            Err(EXIT_RUNTIME_ERROR)
        }
    }
}

/// The DOS base of `windows:dos`: the disk of the pinned FreeDOS zip, at the
/// base cache of the DOS flavor, made where it is not there.
pub(crate) fn dos_base(verb: &str) -> Result<PathBuf, i32> {
    let base = podvm_windows::dos::base_cache();
    if base.is_file() {
        return Ok(base);
    }
    let e = podvm_windows::catalog::resolve("dos").map_err(|_| EXIT_RUNTIME_ERROR)?;
    debug_assert_eq!(e.kind, Kind::DosZip);
    let zip = fetched(verb, e)?;
    let bytes = std::fs::read(&zip).map_err(|err| {
        eprintln!("podvm {verb}: {}: {err}", zip.display());
        EXIT_RUNTIME_ERROR
    })?;
    let disk = zip_member(&bytes, DOS_IMAGE, DOS_IMAGE_BYTES).map_err(|why| {
        eprintln!("podvm {verb}: {}: {why}", zip.display());
        EXIT_RUNTIME_ERROR
    })?;
    write_new(&base, &disk).map_err(|err| {
        eprintln!("podvm {verb}: {}: {err}", base.display());
        EXIT_RUNTIME_ERROR
    })?;
    eprintln!(
        "podvm {verb}: windows:dos: the DOS base is {}",
        base.display()
    );
    Ok(base)
}

/// Write `bytes` to `path` through a file beside it and a rename, so that no
/// reader sees half a base.
fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = path.with_extension(format!("{}.part", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// The bytes of the member `name` of the zip `zip`, which must have `size`
/// bytes when it is out: read through the central directory, stored or
/// deflated.
pub(crate) fn zip_member(zip: &[u8], name: &str, size: u64) -> Result<Vec<u8>, String> {
    let u16_at = |o: usize| {
        zip.get(o..o + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
    };
    let u32_at = |o: usize| {
        zip.get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    // The end of the central directory: its signature, from the end.
    let eocd = (0..zip.len().saturating_sub(21))
        .rev()
        .find(|&o| zip[o..].starts_with(b"PK\x05\x06"))
        .ok_or("no end of the central directory: not a zip")?;
    let count = u16_at(eocd + 10).ok_or("a short zip")?;
    let mut at = u32_at(eocd + 16).ok_or("a short zip")?;
    for _ in 0..count {
        if !zip.get(at..).is_some_and(|z| z.starts_with(b"PK\x01\x02")) {
            return Err("a damaged central directory".to_string());
        }
        let method = u16_at(at + 10).ok_or("a short entry")?;
        let packed = u32_at(at + 20).ok_or("a short entry")?;
        let n = u16_at(at + 28).ok_or("a short entry")?;
        let extra = u16_at(at + 30).ok_or("a short entry")?;
        let note = u16_at(at + 32).ok_or("a short entry")?;
        let local = u32_at(at + 42).ok_or("a short entry")?;
        let this = zip.get(at + 46..at + 46 + n).ok_or("a short entry")?;
        at += 46 + n + extra + note;
        if this != name.as_bytes() {
            continue;
        }
        let ln = u16_at(local + 26).ok_or("a short local header")?;
        let lx = u16_at(local + 28).ok_or("a short local header")?;
        let data = zip
            .get(local + 30 + ln + lx..local + 30 + ln + lx + packed)
            .ok_or("a short member")?;
        let out = match method {
            0 => data.to_vec(),
            8 => {
                let mut v = Vec::with_capacity(size as usize);
                flate2::read::DeflateDecoder::new(data)
                    .take(size + 1)
                    .read_to_end(&mut v)
                    .map_err(|e| format!("{name}: {e}"))?;
                v
            }
            m => {
                return Err(format!(
                    "{name}: the method {m} of the zip is not read here"
                ))
            }
        };
        if out.len() as u64 != size {
            return Err(format!("{name} has {} bytes, not {size}", out.len()));
        }
        return Ok(out);
    }
    Err(format!("{name} is not in the zip"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zip of one stored member, as `zip -0` writes it.
    fn stored_zip(name: &str, body: &[u8]) -> Vec<u8> {
        let mut z = Vec::new();
        let n = name.len() as u16;
        let len = body.len() as u32;
        z.extend_from_slice(b"PK\x03\x04");
        z.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&0u16.to_le_bytes());
        z.extend_from_slice(name.as_bytes());
        z.extend_from_slice(body);
        let cd = z.len() as u32;
        z.extend_from_slice(b"PK\x01\x02");
        z.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&[0; 12]);
        z.extend_from_slice(&0u32.to_le_bytes());
        z.extend_from_slice(name.as_bytes());
        let cd_len = z.len() as u32 - cd;
        z.extend_from_slice(b"PK\x05\x06");
        z.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
        z.extend_from_slice(&cd_len.to_le_bytes());
        z.extend_from_slice(&cd.to_le_bytes());
        z.extend_from_slice(&[0, 0]);
        z
    }

    #[test]
    fn catalog_zip_member_reads_a_stored_member_and_refuses_the_rest() {
        let z = stored_zip("FD14LITE.img", b"disk-bytes");
        assert_eq!(zip_member(&z, "FD14LITE.img", 10).unwrap(), b"disk-bytes");
        assert!(zip_member(&z, "other", 10)
            .unwrap_err()
            .contains("not in the zip"));
        assert!(zip_member(&z, "FD14LITE.img", 11)
            .unwrap_err()
            .contains("10 bytes"));
        assert!(zip_member(b"no zip", "x", 1).is_err());
    }

    /// TODO/machine.md T-1861: `run windows:NAME` takes the door of the
    /// Windows driver with no tier and no platform; `create` and `-d` stay
    /// refused, and an image that is no Windows name keeps its own path.
    #[test]
    fn windows_name_takes_the_driver_door_with_no_flag() {
        use super::super::should_drive;
        assert!(should_drive(
            "run",
            false,
            false,
            "linux",
            Some("windows:dos")
        ));
        assert!(should_drive(
            "run",
            false,
            false,
            "linux",
            Some("windows:11")
        ));
        assert!(!should_drive(
            "create",
            false,
            false,
            "linux",
            Some("windows:dos")
        ));
        assert!(!should_drive(
            "run",
            true,
            false,
            "linux",
            Some("windows:dos")
        ));
        assert!(!should_drive(
            "run",
            false,
            false,
            "linux",
            Some("alpine:3.20")
        ));
        assert!(should_drive("run", false, true, "windows", None));
    }
}
