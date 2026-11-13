//! Link the object again when its version script changes, T-1743.
//!
//! `podvm-interpose-build` gives `interpose.map` to the linker as a flag. A
//! change of the content of the file is no change that cargo sees, so an
//! object kept the export list of the map before the change: the musl object
//! of T-1743 hid the four fortified opens and failed the export check. This
//! script names the file, so cargo builds the crate again when it changes.

fn main() {
    println!("cargo:rerun-if-changed=interpose.map");
}
