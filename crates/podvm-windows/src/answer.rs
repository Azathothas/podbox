//! The answer file of an unattended Windows install (TODO/machine.md T-1866):
//! `autounattend.xml` for a setup ISO of the catalog, as a pure function.
//!
//! The file installs on a GPT disk for UEFI, takes the image of the ISO by
//! its index, sets the locale, makes one local administrator `podvm` with
//! one automatic logon, skips the out-of-box screens, turns on Remote
//! Desktop and its firewall group, turns off sleep, and ends its first logon
//! with a `READY` marker on the first serial port and a shutdown. Windows 11
//! gets the LabConfig keys that let Setup run with no TPM, no Secure Boot
//! and a small memory (the keys of dockur/windows `assets/win11x64.xml`;
//! T-1867 proves them in a guest).
//!
//! ⛔ The password is random for each machine. It goes into this file and
//! into the machine record (mode 0600), never into an argv or a log.

/// The setup ISOs that the file serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Win11,
    Server2025,
    Win10,
}

impl Target {
    /// The target of a name of the catalog, or `None` for a name with no
    /// setup ISO.
    pub fn of(name: &str) -> Option<Target> {
        match name {
            "11" => Some(Target::Win11),
            "2025" => Some(Target::Server2025),
            "10" => Some(Target::Win10),
            _ => None,
        }
    }

    /// The index of the image in `install.wim` of the evaluation ISO: the
    /// one image of the client ISOs, and the Standard edition with the
    /// Desktop Experience of the server ISO (an estimate until T-1867 reads
    /// the WIM).
    pub fn image_index(self) -> u32 {
        match self {
            Target::Win11 | Target::Win10 => 1,
            Target::Server2025 => 2,
        }
    }
}

/// The values of one answer file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub target: Target,
    /// A language tag such as `en-US`.
    pub locale: String,
    pub user: String,
    pub password: String,
    pub computer: String,
}

/// The marker that the first logon writes on COM1 when the install ended.
pub const READY: &str = "PODVM-WINDOWS-READY";

/// The name of the file that Setup reads at the root of a removable drive.
pub const FILE: &str = "autounattend.xml";

/// A password of 20 characters from `/dev/urandom`: letters of both cases,
/// digits and one sign, which the complexity rule of Windows takes.
pub fn password() -> std::io::Result<String> {
    use std::io::Read;
    const SET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let mut raw = [0u8; 17];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut raw)?;
    let body: String = raw
        .iter()
        .map(|b| SET[*b as usize % SET.len()] as char)
        .collect();
    Ok(format!("Pv{body}!"))
}

/// XML text with `&`, `<`, `>` and `"` escaped.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const ARCH: &str = r#"processorArchitecture="amd64" publicKeyToken="31bf3856ad364e35" language="neutral" versionScope="nonSxS""#;

/// The `autounattend.xml` of `a`.
pub fn xml(a: &Answer) -> String {
    let (loc, user, pass, host) = (
        esc(&a.locale),
        esc(&a.user),
        esc(&a.password),
        esc(&a.computer),
    );
    let index = a.target.image_index();
    let labconfig = if a.target == Target::Win11 {
        ["BypassTPMCheck", "BypassSecureBootCheck", "BypassRAMCheck"]
            .iter()
            .enumerate()
            .map(|(i, k)| {
                format!(
                    "        <RunSynchronousCommand wcm:action=\"add\"><Order>{}</Order><Path>reg add HKLM\\SYSTEM\\Setup\\LabConfig /v {k} /t REG_DWORD /d 1 /f</Path></RunSynchronousCommand>\n",
                    i + 1
                )
            })
            .collect::<String>()
    } else {
        String::new()
    };
    let first = [
        r#"reg add "HKLM\SYSTEM\CurrentControlSet\Control\Terminal Server" /v fDenyTSConnections /t REG_DWORD /d 0 /f"#,
        r#"netsh advfirewall firewall set rule group="Remote Desktop" new enable=Yes"#,
        "powercfg /change standby-timeout-ac 0",
        "powercfg /change monitor-timeout-ac 0",
        &format!("cmd /c echo {READY}> COM1"),
        "shutdown /s /t 5",
    ]
    .iter()
    .enumerate()
    .map(|(i, c)| {
        format!(
            "        <SynchronousCommand wcm:action=\"add\"><Order>{}</Order><CommandLine>{}</CommandLine></SynchronousCommand>\n",
            i + 1,
            esc(c)
        )
    })
    .collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!-- podvm: the answer file of an unattended install (TODO/machine.md T-1866) -->
<unattend xmlns="urn:schemas-microsoft-com:unattend" xmlns:wcm="http://schemas.microsoft.com/WMIConfig/2002/State">
  <settings pass="windowsPE">
    <component name="Microsoft-Windows-International-Core-WinPE" {ARCH}>
      <SetupUILanguage><UILanguage>{loc}</UILanguage></SetupUILanguage>
      <InputLocale>{loc}</InputLocale><SystemLocale>{loc}</SystemLocale><UILanguage>{loc}</UILanguage><UserLocale>{loc}</UserLocale>
    </component>
    <component name="Microsoft-Windows-Setup" {ARCH}>
      <RunSynchronous>
{labconfig}      </RunSynchronous>
      <DiskConfiguration>
        <Disk wcm:action="add">
          <DiskID>0</DiskID><WillWipeDisk>true</WillWipeDisk>
          <CreatePartitions>
            <CreatePartition wcm:action="add"><Order>1</Order><Type>EFI</Type><Size>300</Size></CreatePartition>
            <CreatePartition wcm:action="add"><Order>2</Order><Type>MSR</Type><Size>16</Size></CreatePartition>
            <CreatePartition wcm:action="add"><Order>3</Order><Type>Primary</Type><Extend>true</Extend></CreatePartition>
          </CreatePartitions>
          <ModifyPartitions>
            <ModifyPartition wcm:action="add"><Order>1</Order><PartitionID>1</PartitionID><Format>FAT32</Format><Label>System</Label></ModifyPartition>
            <ModifyPartition wcm:action="add"><Order>2</Order><PartitionID>3</PartitionID><Format>NTFS</Format><Label>Windows</Label><Letter>C</Letter></ModifyPartition>
          </ModifyPartitions>
        </Disk>
      </DiskConfiguration>
      <ImageInstall>
        <OSImage>
          <InstallFrom><MetaData wcm:action="add"><Key>/IMAGE/INDEX</Key><Value>{index}</Value></MetaData></InstallFrom>
          <InstallTo><DiskID>0</DiskID><PartitionID>3</PartitionID></InstallTo>
        </OSImage>
      </ImageInstall>
      <UserData><AcceptEula>true</AcceptEula><ProductKey><Key></Key><WillShowUI>Never</WillShowUI></ProductKey></UserData>
    </component>
  </settings>
  <settings pass="specialize">
    <component name="Microsoft-Windows-Shell-Setup" {ARCH}>
      <ComputerName>{host}</ComputerName>
    </component>
  </settings>
  <settings pass="oobeSystem">
    <component name="Microsoft-Windows-International-Core" {ARCH}>
      <InputLocale>{loc}</InputLocale><SystemLocale>{loc}</SystemLocale><UILanguage>{loc}</UILanguage><UserLocale>{loc}</UserLocale>
    </component>
    <component name="Microsoft-Windows-Shell-Setup" {ARCH}>
      <OOBE>
        <HideEULAPage>true</HideEULAPage><HideOnlineAccountScreens>true</HideOnlineAccountScreens>
        <HideWirelessSetupInOOBE>true</HideWirelessSetupInOOBE><ProtectYourPC>3</ProtectYourPC>
      </OOBE>
      <UserAccounts>
        <LocalAccounts>
          <LocalAccount wcm:action="add">
            <Name>{user}</Name><Group>Administrators</Group>
            <Password><Value>{pass}</Value><PlainText>true</PlainText></Password>
          </LocalAccount>
        </LocalAccounts>
      </UserAccounts>
      <AutoLogon>
        <Enabled>true</Enabled><LogonCount>1</LogonCount><Username>{user}</Username>
        <Password><Value>{pass}</Value><PlainText>true</PlainText></Password>
      </AutoLogon>
      <FirstLogonCommands>
{first}      </FirstLogonCommands>
    </component>
  </settings>
</unattend>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(target: Target) -> Answer {
        Answer {
            target,
            locale: "en-US".into(),
            user: "podvm".into(),
            password: "Pv-canary-<&>\"".into(),
            computer: "PODVM-WIN".into(),
        }
    }

    /// Each element that opens is closed, in order: a check of
    /// well-formedness that needs no XML crate.
    fn well_formed(x: &str) -> Result<(), String> {
        let mut stack: Vec<String> = Vec::new();
        let mut rest = x;
        while let Some(i) = rest.find('<') {
            rest = &rest[i + 1..];
            let j = rest.find('>').ok_or("an open tag")?;
            let tag = &rest[..j];
            rest = &rest[j + 1..];
            if tag.starts_with('?') || tag.starts_with('!') || tag.ends_with('/') {
                continue;
            }
            let name = tag.split_whitespace().next().unwrap_or("");
            if let Some(close) = name.strip_prefix('/') {
                match stack.pop() {
                    Some(open) if open == close => {}
                    other => return Err(format!("</{close}> closes {other:?}")),
                }
            } else {
                stack.push(name.to_string());
            }
        }
        if stack.is_empty() {
            Ok(())
        } else {
            Err(format!("open: {stack:?}"))
        }
    }

    #[test]
    fn answer_file_is_well_formed_for_each_target() {
        for t in [Target::Win11, Target::Server2025, Target::Win10] {
            let x = xml(&answer(t));
            well_formed(&x).unwrap_or_else(|e| panic!("{t:?}: {e}"));
            assert!(x.contains("<Key>/IMAGE/INDEX</Key>"), "{t:?}");
            assert!(x.contains(READY) && x.contains("shutdown /s"), "{t:?}");
            assert!(x.contains("fDenyTSConnections"), "{t:?}");
            assert_eq!(x.contains("BypassTPMCheck"), t == Target::Win11, "{t:?}");
            // The password is escaped, and appears only in its two places.
            assert!(x.contains("Pv-canary-&lt;&amp;&gt;&quot;"), "{t:?}");
            assert_eq!(x.matches("Pv-canary").count(), 2, "{t:?}");
        }
        assert_eq!(Target::of("11"), Some(Target::Win11));
        assert_eq!(Target::of("dos"), None);
    }

    #[test]
    fn answer_file_password_is_random_and_complex() {
        let (a, b) = (password().unwrap(), password().unwrap());
        assert_ne!(a, b);
        assert_eq!(a.len(), 20);
        assert!(a.chars().any(|c| c.is_ascii_uppercase()));
        assert!(a.chars().any(|c| c.is_ascii_lowercase()));
        assert!(a.chars().any(|c| c.is_ascii_digit()) || a.ends_with('!'));
    }
}
