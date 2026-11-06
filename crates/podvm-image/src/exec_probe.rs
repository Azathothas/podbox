//! `TODO/image.md`: can a file created under the store execute (`noexec`).
//!
//! `pod_hostprobe::exec` makes the probe. This module gives it podvm's
//! settings, so a denial names `$PODVM_STORE`, and turns its failures into
//! this crate's errors.

use std::path::Path;

pub use pod_hostprobe::exec::{ExecProbe, ExecVerdict};

use crate::error::Result;

/// Probe whether files created under `dir` execute. The probe file is
/// removed on every path.
pub fn probe_store_exec(dir: &Path) -> Result<ExecProbe> {
    pod_hostprobe::exec::probe_dir_exec(&podvm_probe::settings(), dir)
        .map_err(crate::space::host_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_directory_that_cannot_be_written_is_an_io_error_with_its_path() {
        let d = std::env::temp_dir()
            .join(format!("podvm-execprobe-{}", std::process::id()))
            .join("does-not-exist");
        let err = probe_store_exec(&d).unwrap_err();
        assert!(
            matches!(&err, crate::Error::Io { path, source }
                if path.starts_with(&*d.to_string_lossy())
                    && source.kind() == std::io::ErrorKind::NotFound),
            "{err:?}"
        );
    }
}
