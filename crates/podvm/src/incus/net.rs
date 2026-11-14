//! `--podvm-net none` and `--podvm-channel` on the incus tier,
//! TODO/machine.md T-1779 (PodUtils/podrunner#88): the forms of the
//! machine tier (T-1751) for an instance.
//!
//! * `--podvm-net none`: the instance has no NIC. The default profile gives
//!   a `nic` device (named `eth0` by convention); a device of the instance
//!   with the same name and type `none` masks it. pod-incus has no update
//!   of an instance, so [`create`] reads the expanded devices after the
//!   create and, where a NIC of another name stays, makes the instance
//!   again with that name masked too.
//! * `--podvm-channel HOST_SOCKET:127.0.0.1:PORT`: a `disk` device mounts
//!   the host socket at [`SOCK`] in the instance, and podvm's init
//!   (`guest-hold`, [`listen`]) listens on `127.0.0.1:PORT` and relays each
//!   connection to it. A `proxy` device did this first; its `forkproxy`
//!   failed to start on the packaged incus 6.0.0 of Ubuntu 24.04 ("Failed
//!   to safely open namespace file descriptor based on pidfd",
//!   PodUtils/podrunner#90).

use std::collections::BTreeMap;
use std::path::PathBuf;

use pod_incus::{Client, Devices, NewInstance};

/// The name of the device of the channel.
pub const CHANNEL: &str = "podvm-channel";
/// The host socket in the instance.
pub const SOCK: &str = "/.podvm/channel.sock";
/// The port of the channel, which podvm pushes before the start.
pub const PORT_FILE: &str = "/.podvm/channel.port";
/// The file that `guest-hold` writes once it listens.
pub const READY: &str = "/.podvm/channel.ready";

/// The devices of the two flags: a mask of type `none` for each name of
/// `nics`, and the `disk` device that mounts the socket of the channel.
pub fn devices(net_none: bool, nics: &[String], channel: Option<&(PathBuf, u16)>) -> Devices {
    let mut d = Devices::new();
    if net_none {
        for n in nics {
            let mut m = BTreeMap::new();
            m.insert("type".to_string(), "none".to_string());
            d.insert(n.clone(), m);
        }
    }
    if let Some((host, _)) = channel {
        let mut p = BTreeMap::new();
        p.insert("type".to_string(), "disk".to_string());
        p.insert("source".to_string(), host.display().to_string());
        p.insert("path".to_string(), SOCK.to_string());
        d.insert(CHANNEL.to_string(), p);
    }
    d
}

/// The names of the `nic` devices of the expanded devices.
pub fn nics(expanded: &Devices) -> Vec<String> {
    expanded
        .iter()
        .filter(|(_, d)| d.get("type").is_some_and(|t| t == "nic"))
        .map(|(n, _)| n.clone())
        .collect()
}

/// Create `new` (its devices already hold those of [`devices`]); with
/// `net_none`, make sure that no NIC stays.
pub async fn create(
    c: &Client,
    new: &NewInstance,
    net_none: bool,
    channel: Option<&(PathBuf, u16)>,
) -> Result<(), String> {
    c.create_instance(new).await.map_err(|e| e.to_string())?;
    if !net_none {
        return Ok(());
    }
    let inst = c.instance(&new.name).await.map_err(|e| e.to_string())?;
    let left = nics(&inst.expanded_devices);
    if left.is_empty() {
        return Ok(());
    }
    let _ = c.delete_instance(&new.name).await;
    let mut again = new.clone();
    let mut masks: Vec<String> = new
        .devices
        .iter()
        .filter(|(_, d)| d.get("type").is_some_and(|t| t == "none"))
        .map(|(n, _)| n.clone())
        .collect();
    masks.extend(left);
    again.devices.extend(devices(true, &masks, channel));
    c.create_instance(&again).await.map_err(|e| e.to_string())?;
    let inst = c.instance(&again.name).await.map_err(|e| e.to_string())?;
    let left = nics(&inst.expanded_devices);
    if !left.is_empty() {
        let _ = c.delete_instance(&again.name).await;
        return Err(format!(
            "--podvm-net none: the instance keeps the NIC {} (TODO/machine.md T-1779)",
            left.join(", ")
        ));
    }
    Ok(())
}

/// Push the port of the channel, before the start.
pub async fn push_port(
    c: &Client,
    name: &str,
    channel: Option<&(PathBuf, u16)>,
) -> Result<(), String> {
    let Some((_, port)) = channel else {
        return Ok(());
    };
    let mut w =
        pod_incus::FileWrite::file(pod_incus::Payload::bytes(port.to_string().into_bytes()));
    w.uid = Some(0);
    w.gid = Some(0);
    w.mode = Some(0o644);
    c.put_file(name, PORT_FILE, w)
        .await
        .map_err(|e| e.to_string())
}

/// Wait until `guest-hold` listens (10 s at most), so that a command of
/// `exec` finds the port.
pub async fn wait_ready(
    c: &Client,
    name: &str,
    channel: Option<&(PathBuf, u16)>,
) -> Result<(), String> {
    if channel.is_none() {
        return Ok(());
    }
    for _ in 0..100 {
        if c.get_file(name, READY).await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    Err(format!(
        "--podvm-channel: podvm's init did not listen within 10 s (no {READY}) \
         (TODO/machine.md T-1779)"
    ))
}

/// In `guest-hold`: where [`PORT_FILE`] names a port, listen on
/// `127.0.0.1:PORT` before any command starts, write [`READY`], and relay
/// each connection to [`SOCK`] on a thread.
pub fn listen() {
    let Ok(text) = std::fs::read_to_string(PORT_FILE) else {
        return;
    };
    let Ok(port) = text.trim().parse::<u16>() else {
        eprintln!("podvm guest-hold: {PORT_FILE} names no port");
        return;
    };
    let l = match std::net::TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("podvm guest-hold: 127.0.0.1:{port}: {e}");
            return;
        }
    };
    let _ = std::fs::write(READY, b"");
    std::thread::spawn(move || {
        for tcp in l.incoming().flatten() {
            std::thread::spawn(move || relay(tcp));
        }
    });
}

/// Copy one connection both ways to [`SOCK`].
fn relay(tcp: std::net::TcpStream) {
    use std::net::Shutdown;
    let Ok(unix) = std::os::unix::net::UnixStream::connect(SOCK) else {
        return;
    };
    let (Ok(mut tcp_r), Ok(mut unix_w)) = (tcp.try_clone(), unix.try_clone()) else {
        return;
    };
    let up = std::thread::spawn(move || {
        let _ = std::io::copy(&mut tcp_r, &mut unix_w);
        let _ = unix_w.shutdown(Shutdown::Write);
    });
    let (mut unix_r, mut tcp_w) = (unix, tcp);
    let _ = std::io::copy(&mut unix_r, &mut tcp_w);
    let _ = tcp_w.shutdown(Shutdown::Write);
    let _ = up.join();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_nic_masks_the_profile_and_the_channel_is_a_socket_device() {
        let ch = (PathBuf::from("/run/egress.sock"), 3128);
        let d = devices(true, &["eth0".to_string()], Some(&ch));
        assert_eq!(d["eth0"]["type"], "none");
        let p = &d[CHANNEL];
        assert_eq!(p["type"], "disk");
        assert_eq!(p["source"], "/run/egress.sock");
        assert_eq!(p["path"], SOCK);
        assert!(devices(false, &["eth0".to_string()], None).is_empty());
        let mut e = Devices::new();
        e.insert(
            "eth0".into(),
            BTreeMap::from([("type".into(), "nic".into())]),
        );
        e.insert(
            "root".into(),
            BTreeMap::from([("type".into(), "disk".into())]),
        );
        assert_eq!(nics(&e), ["eth0"]);
    }
}
