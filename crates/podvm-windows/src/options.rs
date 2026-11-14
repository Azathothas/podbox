//! TODO/machine.md T-1865: the options of the boot line of each kind of
//! Windows guest, on the plan options of `pod-guest` (podcrates
//! `v0.45.0`, podcrates@T-593).
//!
//! ⛔ ValidationOS has no network driver, and its run is the mailbox run:
//! `-nic none` and no other device. A NIC there is a defect, as the check
//! of winquick says.

use crate::plan::{Forward, Nic, NicModel, Options, UserNet, Virtio};
use crate::Accel;

/// The mailbox run: ValidationOS and DOS. No NIC, no media, no display.
pub fn mailbox() -> Options {
    Options::default()
}

/// A desktop guest (11, Server, 10): a user-mode NIC that Windows drives
/// with its inbox driver, `forwards` to it on the loopback of the host, a
/// USB tablet for a VNC client, a virtio RNG, and the Hyper-V flags where
/// the accelerator is KVM.
pub fn desktop(accel: Accel, forwards: Vec<Forward>) -> Options {
    Options {
        nic: Nic::User(UserNet {
            model: NicModel::E1000e,
            forwards,
            restrict: false,
        }),
        virtio: vec![Virtio::Rng],
        usb_tablet: true,
        hyperv: accel == Accel::Kvm,
        ..Options::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Proto;

    fn plan_with(options: Options, accel: Accel) -> Vec<String> {
        let d = std::path::PathBuf::from("/run/test/podvm-plan");
        crate::argv(&crate::Plan {
            emulator: "qemu-system-x86_64".into(),
            accel,
            share: "/usr/share/qemu".into(),
            firmware_code: "/code.fd".into(),
            firmware_vars: d.join("vars.fd"),
            root: d.join("run.qcow2"),
            mailbox: d.join("mailbox.img"),
            serial: d.join("serial.log"),
            monitor: d.join("qmp.sock"),
            memory_mib: 4096,
            cpus: 2,
            emu_args: Vec::new(),
            options,
        })
    }

    /// The check of winquick: a ValidationOS run has `-nic none` and no NIC
    /// device.
    #[test]
    fn plan_options_of_validationos_have_no_nic() {
        let argv = plan_with(mailbox(), Accel::Kvm);
        let line = argv.join(" ");
        assert!(line.contains("-nic none"), "{line}");
        for nic in ["e1000", "virtio-net", "netdev", "hostfwd"] {
            assert!(
                !line.contains(nic),
                "a NIC in a ValidationOS run: {nic} in {line}"
            );
        }
    }

    /// A desktop has the NIC with its forward on the loopback, the tablet,
    /// the RNG, and Hyper-V only under KVM.
    #[test]
    fn plan_options_of_a_desktop_name_each_device() {
        let rdp = Forward::local(Proto::Tcp, 33890, 3389);
        let kvm = plan_with(desktop(Accel::Kvm, vec![rdp]), Accel::Kvm).join(" ");
        assert!(kvm.contains("e1000e"), "{kvm}");
        assert!(kvm.contains("hostfwd=tcp:127.0.0.1:33890-:3389"), "{kvm}");
        assert!(kvm.contains("usb-tablet"), "{kvm}");
        assert!(kvm.contains("virtio-rng-pci"), "{kvm}");
        assert!(kvm.contains("hv_"), "no Hyper-V flag under KVM: {kvm}");
        assert!(!kvm.contains("-nic none"), "{kvm}");
        let tcg = plan_with(desktop(Accel::Tcg, Vec::new()), Accel::Tcg).join(" ");
        assert!(!tcg.contains("hv_"), "a Hyper-V flag under TCG: {tcg}");
    }
}
