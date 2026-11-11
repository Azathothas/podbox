//! TODO/enter.md T-1651: the emulate tier's emulator, by architecture and
//! on PATH.

use crate::lifecycle::emulate::{find_qemu, qemu_arch};

#[test]
fn each_oci_architecture_names_its_qemu_and_its_elf_machine() {
    assert_eq!(qemu_arch("amd64"), Some(("x86_64", 62)));
    assert_eq!(qemu_arch("arm64"), Some(("aarch64", 183)));
    assert_eq!(qemu_arch("riscv64"), Some(("riscv64", 243)));
    assert_eq!(qemu_arch("mips"), None);
}

#[test]
fn the_emulator_is_a_static_program_on_path() {
    let d = std::env::temp_dir().join(format!("podvm-t1651-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("a")).unwrap();
    std::fs::create_dir_all(d.join("b")).unwrap();
    // A dynamic stand-in first on PATH, then a static one (this test
    // binary, which the workspace links static).
    std::fs::write(d.join("a/qemu-aarch64-static"), b"#!/bin/sh\n").unwrap();
    std::fs::copy(std::env::current_exe().unwrap(), d.join("b/qemu-aarch64")).unwrap();
    let path = format!("{}:{}", d.join("a").display(), d.join("b").display());
    if podvm_dispatch::available().is_ok() {
        assert_eq!(find_qemu("arm64", &path).unwrap(), d.join("b/qemu-aarch64"));
    }
    let none = find_qemu("arm64", &d.join("a").display().to_string()).unwrap_err();
    assert!(none.contains("qemu-aarch64-static"), "{none}");
    let e = find_qemu("riscv64", &path).unwrap_err();
    assert!(
        e.contains("no qemu-riscv64-static or qemu-riscv64 on PATH"),
        "{e}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
