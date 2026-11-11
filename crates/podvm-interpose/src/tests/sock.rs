//! TODO/interpose.md T-1710: a Unix socket path takes the path table.
//!
//! The test binary defines the interposed entry points, so each file call
//! of the test goes through the table while it is set: a test reads its
//! host files after the drop of the table. The suite runs with one thread
//! (gate.yml).

use crate::sock_calls::{self, SUN_PATH};
use core::ffi::{c_int, c_void};

unsafe extern "C" {
    fn socket(domain: c_int, ty: c_int, proto: c_int) -> c_int;
    fn listen(fd: c_int, backlog: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

const AF_UNIX: c_int = 1;
const SOCK_STREAM: c_int = 1;

/// A `sockaddr_un` of `path`, and its length.
fn sun(path: &[u8]) -> ([u8; 110], u32) {
    let mut a = [0u8; 110];
    a[..2].copy_from_slice(&(AF_UNIX as u16).to_ne_bytes());
    a[2..2 + path.len()].copy_from_slice(path);
    (a, (2 + path.len() + 1) as u32)
}

struct Table;

fn set(root: &std::path::Path, ro: Option<&str>) -> Table {
    unsafe {
        std::env::set_var("PODVM_MAPS", format!("/:{}", root.display()));
        match ro {
            Some(v) => std::env::set_var("PODVM_RO", v),
            None => std::env::remove_var("PODVM_RO"),
        }
    }
    Table
}

impl Drop for Table {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("PODVM_MAPS");
            std::env::remove_var("PODVM_RO");
        }
    }
}

fn unix() -> c_int {
    let fd = unsafe { socket(AF_UNIX, SOCK_STREAM, 0) };
    assert!(fd >= 0);
    fd
}

/// Bind `guest` under the table, listen, connect a second socket by the
/// same guest name, and read the name back. The bind result, the connect
/// result, and the name that `getsockname` answers.
fn round_trip(guest: &[u8]) -> (c_int, c_int, Vec<u8>) {
    let (a, n) = sun(guest);
    let s = unix();
    let b = unsafe { sock_calls::bind(s, a.as_ptr() as *const c_void, n) };
    unsafe { listen(s, 1) };
    let c = unix();
    let k = unsafe { sock_calls::connect(c, a.as_ptr() as *const c_void, n) };
    let mut back = [0u8; 110];
    let mut len = back.len() as u32;
    unsafe { sock_calls::getsockname(s, back.as_mut_ptr() as *mut c_void, &mut len) };
    let name = back[2..(len as usize).saturating_sub(1).max(2)].to_vec();
    unsafe {
        close(c);
        close(s);
    }
    (b, k, name)
}

#[test]
fn a_socket_path_binds_in_the_rootfs_and_reads_back_its_guest_name() {
    let root = std::env::temp_dir().join(format!("pv1710-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("run")).unwrap();
    let guest = format!("/run/t1710-{}.sock", std::process::id());
    let table = set(&root, None);
    let (b, k, name) = round_trip(guest.as_bytes());
    drop(table);
    assert_eq!((b, k), (0, 0), "bind and connect by the guest name");
    assert_eq!(
        String::from_utf8_lossy(&name),
        guest,
        "getsockname answers the guest name"
    );
    assert!(
        root.join(&guest[1..]).exists(),
        "the socket is inside the rootfs"
    );
    assert!(
        !std::path::Path::new(&guest).exists(),
        "no socket at the host path"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A host path longer than `sun_path` takes `/proc/self/fd/N/NAME`.
#[test]
fn a_long_host_path_binds_through_the_parent_descriptor() {
    let deep = "d".repeat(SUN_PATH);
    let root = std::env::temp_dir().join(format!("pv1710-long-{}/{deep}", std::process::id()));
    std::fs::create_dir_all(root.join("run")).unwrap();
    assert!(root.join("run/s.sock").as_os_str().len() >= SUN_PATH);
    let table = set(&root, None);
    let (b, k, name) = round_trip(b"/run/s.sock");
    drop(table);
    assert_eq!((b, k), (0, 0), "bind and connect through the long form");
    assert_eq!(
        name, b"/run/s.sock",
        "getsockname answers the guest name of a long-form bind"
    );
    assert!(
        root.join("run/s.sock").exists(),
        "the socket is inside the rootfs"
    );
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}

/// An abstract name passes unchanged; a bind under a read-only prefix
/// answers `EROFS`, as a write does.
#[test]
fn an_abstract_name_passes_and_a_read_only_bind_answers_erofs() {
    let root = std::env::temp_dir().join(format!("pv1710-ro-{}", std::process::id()));
    std::fs::create_dir_all(root.join("run")).unwrap();
    let table = set(&root, Some("ro:/"));
    let (a, n) = sun(b"\0podvm-t1710-abstract");
    let pass = unsafe { sock_calls::rewrite(b"bind", a.as_ptr() as *const c_void, n, true) };
    assert!(matches!(pass, Ok(None)));
    let (a, n) = sun(b"/run/ro.sock");
    let s = unix();
    let r = unsafe { sock_calls::bind(s, a.as_ptr() as *const c_void, n) };
    let e = crate::errno();
    unsafe { close(s) };
    drop(table);
    assert_eq!((r, e), (-1, 30));
    let _ = std::fs::remove_dir_all(&root);
}
