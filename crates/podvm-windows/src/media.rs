//! TODO/machine.md T-1866: the USB stick of an unattended install. Setup
//! of Windows reads `autounattend.xml` at the root of a removable drive.
//! The FAT image comes from `fat16` of `pod-guest`, which writes the long
//! name since podcrates `v0.45.0` (podcrates@T-610).
//!
//! ⛔ The answer file holds the password of the guest. It is written into
//! the image file only (mode 0600); no argv names more than its path.

use std::path::{Path, PathBuf};

use crate::answer::{self, Answer};
use crate::plan::{Medium, MediumKind};

/// The FAT image of the stick: one file, `autounattend.xml`.
pub fn image(a: &Answer) -> crate::Result<Vec<u8>> {
    let mut fat = crate::Fat16::with_oem("PODVM")?;
    fat.put(answer::FILE, answer::xml(a).as_bytes())?;
    Ok(fat.into_image())
}

/// Write the stick to `path` with mode 0600, and give its medium: a USB
/// stick on the xHCI bus, read-only, not the boot medium.
pub fn write(a: &Answer, path: &Path) -> std::io::Result<Medium> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let bytes = image(a).map_err(|e| std::io::Error::other(e.to_string()))?;
    let _ = std::fs::remove_file(path);
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(&bytes)?;
    f.sync_all()?;
    Ok(medium(path.to_path_buf()))
}

/// The medium of a stick at `path`.
pub fn medium(path: PathBuf) -> Medium {
    Medium {
        kind: MediumKind::Usb,
        path,
        readonly: true,
        boot: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer::Target;

    fn an_answer() -> Answer {
        Answer {
            target: Target::Win11,
            locale: "en-US".into(),
            user: "podvm".into(),
            password: "Zz9!secret-of-test".into(),
            computer: "podvm-win".into(),
        }
    }

    /// The image reads back through the reader of `fat16`, with the long
    /// name, and holds the answer file byte for byte.
    #[test]
    fn answer_file_media_round_trips_through_fat16() {
        let a = an_answer();
        let img = image(&a).unwrap();
        let fat = crate::Fat16::from_image(img).expect("a FAT image");
        assert!(
            fat.list().iter().any(|n| n == answer::FILE),
            "{:?}",
            fat.list()
        );
        assert_eq!(fat.get(answer::FILE).unwrap(), answer::xml(&a).into_bytes());
    }

    /// The emulator line takes the stick as a read-only USB drive, and no
    /// argument holds the password.
    #[test]
    fn answer_file_media_is_one_usb_storage_line_with_no_password() {
        let dir = std::env::temp_dir().join(format!("podvm-media-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = an_answer();
        let path = dir.join("answer.img");
        let m = write(&a, &path).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let mut options = crate::plan::Options::default();
        options.media.push(m);
        let plan = crate::Plan {
            emulator: "qemu-system-x86_64".into(),
            accel: crate::Accel::Tcg,
            share: "/usr/share/qemu".into(),
            firmware_code: "/code.fd".into(),
            firmware_vars: dir.join("vars.fd"),
            root: dir.join("run.qcow2"),
            mailbox: dir.join("mailbox.img"),
            serial: dir.join("serial.log"),
            monitor: dir.join("qmp.sock"),
            memory_mib: 4096,
            cpus: 2,
            emu_args: Vec::new(),
            options,
        };
        let argv = crate::argv(&plan);
        let usb: Vec<&String> = argv.iter().filter(|a| a.contains("usb-storage")).collect();
        assert_eq!(usb.len(), 1, "{argv:?}");
        assert!(
            usb[0].contains("bus=xhci.0") && usb[0].contains("removable=on"),
            "{usb:?}"
        );
        assert!(
            argv.iter()
                .any(|a| a.contains("readonly=on") && a.contains("answer.img")),
            "{argv:?}"
        );
        assert!(
            !argv.iter().any(|a| a.contains(&a_password())),
            "the password is in argv"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn a_password() -> String {
        an_answer().password
    }
}

/// TODO/machine.md T-1866: the answer file of each target is pinned by a
/// golden file. `PODVM_BLESS=1` writes the files again after a change
/// that a reviewer read.
#[cfg(test)]
mod golden {
    use crate::answer::{xml, Answer, Target};

    fn case(target: Target) -> Answer {
        Answer {
            target,
            locale: "en-US".into(),
            user: "podvm".into(),
            password: "Golden-Pass-1!".into(),
            computer: "podvm-golden".into(),
        }
    }

    #[test]
    fn answer_file_matches_the_golden_file_of_each_target() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden");
        for (target, file) in [
            (Target::Win11, "win11.xml"),
            (Target::Server2025, "server2025.xml"),
            (Target::Win10, "win10.xml"),
        ] {
            let got = xml(&case(target));
            let path = dir.join(file);
            if std::env::var_os("PODVM_BLESS").is_some() {
                std::fs::write(&path, &got).unwrap();
            }
            let want = std::fs::read_to_string(&path).unwrap_or_default();
            assert_eq!(
                got, want,
                "{file} differs; read the change, then PODVM_BLESS=1"
            );
        }
    }
}
