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
//! * `--podvm-channel HOST_SOCKET:127.0.0.1:PORT`: a `proxy` device that
//!   listens on `tcp:127.0.0.1:PORT` in the instance (`bind=instance`) and
//!   connects to `unix:HOST_SOCKET` on the host, for each connection.

use std::collections::BTreeMap;
use std::path::PathBuf;

use pod_incus::{Client, Devices, NewInstance};

/// The name of the proxy device of the channel.
pub const CHANNEL: &str = "podvm-channel";

/// The devices of the two flags: a mask of type `none` for each name of
/// `nics`, and the proxy device of the channel.
pub fn devices(net_none: bool, nics: &[String], channel: Option<&(PathBuf, u16)>) -> Devices {
    let mut d = Devices::new();
    if net_none {
        for n in nics {
            let mut m = BTreeMap::new();
            m.insert("type".to_string(), "none".to_string());
            d.insert(n.clone(), m);
        }
    }
    if let Some((host, port)) = channel {
        let mut p = BTreeMap::new();
        p.insert("type".to_string(), "proxy".to_string());
        p.insert("listen".to_string(), format!("tcp:127.0.0.1:{port}"));
        p.insert("connect".to_string(), format!("unix:{}", host.display()));
        p.insert("bind".to_string(), "instance".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_nic_masks_the_profile_and_the_channel_is_a_proxy() {
        let ch = (PathBuf::from("/run/egress.sock"), 3128);
        let d = devices(true, &["eth0".to_string()], Some(&ch));
        assert_eq!(d["eth0"]["type"], "none");
        let p = &d[CHANNEL];
        assert_eq!(p["type"], "proxy");
        assert_eq!(p["listen"], "tcp:127.0.0.1:3128");
        assert_eq!(p["connect"], "unix:/run/egress.sock");
        assert_eq!(p["bind"], "instance");
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
