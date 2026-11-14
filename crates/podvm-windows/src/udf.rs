//! TODO/machine.md T-1863: the VHDX of Validation OS in its ISO, read
//! with `pod-udf` (podcrates `v0.45.0`, podcrates@T-591). The caller reads
//! the image by HTTP Range requests, so podvm fetches the directory blocks
//! and the extent of the VHDX, about 0.9 GB, and not the ISO of 2.5 GB.

use std::ops::Range;

pub use pod_udf::ReadAt;

/// The name of the disk in the ISO.
pub const VHDX: &str = "ValidationOS.vhdx";

/// The byte range of the image that holds `name`, which must be one
/// recorded extent. A file that the image lacks, or one in parts, is
/// refused by name.
pub fn extent<R: ReadAt>(reader: R, name: &str) -> Result<Range<u64>, String> {
    let e = pod_udf::find(reader, name)
        .map_err(|e| format!("the UDF image holds no {name}: {e} (TODO/machine.md T-1863)"))?;
    e.range().ok_or_else(|| {
        format!(
            "{name} is in {} parts in the UDF image, and podvm fetches one extent \
             (TODO/machine.md T-1863)",
            e.parts.len()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// A UDF image of `genisoimage -udf` with `ValidationOS.vhdx` (65 536
    /// bytes: `vhdxfile`, then `(i * 7) % 251`) and `sources/readme.txt`.
    fn image() -> Vec<u8> {
        let gz: &[u8] = include_bytes!("fixtures/udf.iso.gz");
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(gz)
            .read_to_end(&mut out)
            .unwrap();
        out
    }

    #[test]
    fn udf_finds_the_vhdx_extent_and_its_length() {
        let img = image();
        let r = extent(img.as_slice(), VHDX).unwrap();
        assert_eq!(r.end - r.start, 65_536);
        let bytes = &img[r.start as usize..r.end as usize];
        assert_eq!(&bytes[..8], b"vhdxfile");
        assert_eq!(bytes[8], 0);
        assert_eq!(bytes[9], 7);
    }

    #[test]
    fn udf_refuses_a_missing_file_by_name() {
        let e = extent(image().as_slice(), "NoSuch.vhdx").unwrap_err();
        assert!(e.contains("NoSuch.vhdx") && e.contains("T-1863"), "{e}");
    }
}
