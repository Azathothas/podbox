//! The catalog of the Windows guest: `podvm run windows:NAME` (TODO/machine.md
//! T-1861). The owner accepts the license terms and the terms of service of
//! Microsoft and of each other vendor (`docs/decisions.md`, 2026-11-14,
//! "Windows media"), so podvm fetches, verifies and keeps the media itself.
//!
//! Each entry pins the vendor's URL, its size and its sha256. The pins of the
//! evaluation ISOs of Microsoft are the ones of dockur/windows `src/define.sh`
//! (each URL answered `HEAD` with its pinned size on 2026-11-14); the pin of
//! FreeDOS is the one of `experiments/369-windows-tcg-dos.sh`; the pin of
//! Validation OS was measured by podvm on 2026-11-14. Each digest is checked
//! against the bytes at the download, and a mismatch removes the file.

/// What the download is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A setup ISO of Windows: it installs into a disk (TODO/machine.md T-1867).
    Iso,
    /// The ISO of Validation OS, whose VHDX is the disk (T-1863).
    ValidationOs,
    /// A zip that holds a raw disk image of DOS.
    DosZip,
}

/// One name of the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The name after `windows:`.
    pub name: &'static str,
    /// Other spellings of the name.
    pub aliases: &'static [&'static str],
    /// What the name is, for a person.
    pub what: &'static str,
    pub kind: Kind,
    pub url: &'static str,
    /// The size of the download in bytes.
    pub size: u64,
    /// The sha256 of the download, lower-case hex.
    pub sha256: &'static str,
    /// The file name in the store.
    pub file: &'static str,
}

impl Entry {
    /// The most bytes that the download may take: its size and 1 percent,
    /// for an origin that sends a little more than it declares.
    pub fn ceiling(&self) -> u64 {
        self.size + self.size / 100
    }
}

/// TODO/machine.md T-1863: the sha256 of `ValidationOS.vhdx` in the ISO
/// of the catalog (build 26100.9818: bytes 2 035 712 to 945 754 112),
/// measured by podvm's fetch and by curl alike (experiment 449,
/// 2026-11-14).
pub const VALIDATIONOS_VHDX_SHA256: Option<&str> =
    Some("a0f7900be9fe1172411ecd48b9598ee4b387853f9d6eb12fe5e85f7602a63000");

/// The names, in the order of `podvm windows images`.
pub const CATALOG: &[Entry] = &[
    Entry {
        name: "dos",
        aliases: &["freedos"],
        what: "FreeDOS 1.4 LiteUSB, a tiny DOS guest that runs under TCG",
        kind: Kind::DosZip,
        url: "https://download.freedos.org/1.4/FD14-LiteUSB.zip",
        size: 17_671_175,
        sha256: "857dcd2ebf9d3d094320154db5fb5b830acba6fb98f981a95a0ca7ab3350338b",
        file: "FD14-LiteUSB.zip",
    },
    Entry {
        name: "validationos",
        aliases: &["vos"],
        what: "Windows Validation OS, build 26100.9818: commands and exit codes, no desktop",
        kind: Kind::ValidationOs,
        url: "https://software-static.download.prss.microsoft.com/dbazure/26100.9818.261025-0959.ge_release_svc_prod3_amd64fre_en-us_VALIDATIONOS.iso",
        size: 2_509_025_280,
        sha256: VALIDATIONOS_SHA256,
        file: "26100.9818_VALIDATIONOS.iso",
    },
    Entry {
        name: "11",
        aliases: &["win11", "11e"],
        what: "Windows 11 Enterprise Evaluation, en-US",
        kind: Kind::Iso,
        url: "https://software-static.download.prss.microsoft.com/dbazure/888969d5-f34g-4e03-ac9d-1f9786c66749/26200.6584.250915-1905.25h2_ge_release_svc_refresh_CLIENTENTERPRISEEVAL_OEMRET_x64FRE_en-us.iso",
        size: 7_092_807_680,
        sha256: "a61adeab895ef5a4db436e0a7011c92a2ff17bb0357f58b13bbc4062e535e7b9",
        file: "win11x64-enterprise-eval.iso",
    },
    Entry {
        name: "2025",
        aliases: &["win2025"],
        what: "Windows Server 2025 Evaluation, en-US",
        kind: Kind::Iso,
        url: "https://software-static.download.prss.microsoft.com/dbazure/998969d5-f34g-4e03-ac9d-1f9786c66749/26100.32230.260111-0550.lt_release_svc_refresh_SERVER_EVAL_x64FRE_en-us.iso",
        size: 8_152_356_864,
        sha256: "7b052573ba7894c9924e3e87ba732ccd354d18cb75a883efa9b900ea125bfd51",
        file: "win2025-eval.iso",
    },
    Entry {
        name: "2022",
        aliases: &["win2022"],
        what: "Windows Server 2022 Evaluation, en-US",
        kind: Kind::Iso,
        url: "https://software-static.download.prss.microsoft.com/sg/download/888969d5-f34g-4e03-ac9d-1f9786c66749/SERVER_EVAL_x64FRE_en-us.iso",
        size: 5_044_094_976,
        sha256: "3e4fa6d8507b554856fc9ca6079cc402df11a8b79344871669f0251535255325",
        file: "win2022-eval.iso",
    },
    Entry {
        name: "2019",
        aliases: &["win2019"],
        what: "Windows Server 2019 Evaluation, en-US",
        kind: Kind::Iso,
        url: "https://software-download.microsoft.com/download/pr/17763.737.190906-2324.rs5_release_svc_refresh_SERVER_EVAL_x64FRE_en-us_1.iso",
        size: 5_296_713_728,
        sha256: "549bca46c055157291be6c22a3aaaed8330e78ef4382c99ee82c896426a1cee1",
        file: "win2019-eval.iso",
    },
    Entry {
        name: "10",
        aliases: &["win10"],
        what: "Windows 10 Enterprise Evaluation 22H2, en-US",
        kind: Kind::Iso,
        url: "https://software-static.download.prss.microsoft.com/dbazure/988969d5-f34g-4e03-ac9d-1f9786c66750/19045.2006.220908-0225.22h2_release_svc_refresh_CLIENTENTERPRISEEVAL_OEMRET_x64FRE_en-us.iso",
        size: 5_550_497_792,
        sha256: "ef7312733a9f5d7d51cfa04ac497671995674ca5e1058d5164d6028f0938d668",
        file: "win10x64-enterprise-eval.iso",
    },
];

/// The sha256 of the ISO of Validation OS 26100.9818, measured by podvm on
/// 2026-11-14 over the bytes of its URL.
const VALIDATIONOS_SHA256: &str =
    "50f71af52a44be1b686f9c9dc2cf55fb3a30d791ee9b7a99213dfb2da0463ad1";

/// The prefix of a Windows name in `podvm run`.
pub const PREFIX: &str = "windows:";

/// The name after `windows:`, where `image` is one.
pub fn name_of(image: &str) -> Option<&str> {
    image.strip_prefix(PREFIX)
}

/// The entry of `name` (or of an alias of it), or a refusal that lists each
/// name.
pub fn resolve(name: &str) -> Result<&'static Entry, String> {
    let n = name.to_ascii_lowercase();
    CATALOG
        .iter()
        .find(|e| e.name == n || e.aliases.contains(&n.as_str()))
        .ok_or_else(|| {
            let names: Vec<String> = CATALOG.iter().map(|e| format!("windows:{}", e.name)).collect();
            format!(
                "windows:{name} is no name of the catalog; the names are {} (TODO/machine.md T-1861)",
                names.join(", ")
            )
        })
}

/// Where the downloads of the catalog live: beside the base caches.
pub fn store_dir() -> std::path::PathBuf {
    match crate::base_cache().parent() {
        Some(d) => d.join("catalog"),
        None => std::path::PathBuf::from("catalog"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_pins_parse_and_each_name_resolves() {
        for e in CATALOG {
            assert!(e.url.starts_with("https://"), "{}", e.name);
            assert!(e.size > 0 && e.ceiling() > e.size, "{}", e.name);
            assert!(
                e.sha256.len() == 64 && e.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
                "{}: {}",
                e.name,
                e.sha256
            );
            assert_eq!(resolve(e.name).unwrap().name, e.name);
            for a in e.aliases {
                assert_eq!(resolve(a).unwrap().name, e.name);
            }
        }
        assert_eq!(resolve("VOS").unwrap().name, "validationos");
    }

    #[test]
    fn catalog_refuses_an_unknown_name_with_the_list() {
        let e = resolve("95").unwrap_err();
        assert!(e.contains("windows:95 is no name"), "{e}");
        assert!(e.contains("windows:11") && e.contains("windows:dos"), "{e}");
        assert_eq!(name_of("windows:dos"), Some("dos"));
        assert_eq!(name_of("alpine:3.20"), None);
    }
}
