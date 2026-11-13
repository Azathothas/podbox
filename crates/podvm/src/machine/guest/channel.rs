//! `--podvm-channel HOST_SOCKET:127.0.0.1:PORT`: one channel from a TCP
//! port of a guest to a Unix socket of the host, TODO/machine.md T-1751
//! (PodUtils/podrunner#83).
//!
//! ⭐ The guest needs no NIC for it (`--podvm-net none`). QEMU gives the
//! guest one `virtio-serial` port, `podvm.ch`, on a chardev socket of the
//! run. In the guest, `podvm guest-channel PORT` listens on
//! `127.0.0.1:PORT` and opens one stream of `pod-link` over the port for
//! each connection; on the host, podvm accepts each stream and dials
//! `HOST_SOCKET` for it. The caller's socket decides what the guest can
//! reach: podrunner puts its egress module there.
//!
//! ⚠ The port is a character device, and a `tokio::fs::File` runs one
//! operation at a time: a read that waits for the host would stop each
//! write. The guest reads one descriptor of the port and writes a
//! duplicate of it.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use podvm_image::error::EXIT_FLAG_ERROR;

/// The name of the port in the guest.
pub const PORT_NAME: &str = "podvm.ch";
/// The file that `guest-channel` writes once it listens.
pub const READY: &str = "/podvm-ch.ready";

/// `HOST_SOCKET:127.0.0.1:PORT`. The host socket is an absolute path; the
/// address in the guest is the loopback.
pub fn parse(verb: &str, a: &str) -> Result<(PathBuf, u16), i32> {
    let bad = |why: &str| {
        eprintln!(
            "podvm {verb}: --podvm-channel takes HOST_SOCKET:127.0.0.1:PORT, not {a:?}: {why} \
             (TODO/machine.md T-1751)"
        );
        EXIT_FLAG_ERROR
    };
    let mut it = a.rsplitn(3, ':');
    let (Some(port), Some(addr), Some(host)) = (it.next(), it.next(), it.next()) else {
        return Err(bad("three parts"));
    };
    if addr != "127.0.0.1" {
        return Err(bad("the address in the guest is 127.0.0.1"));
    }
    let port: u16 = match port.parse() {
        Ok(p) if p > 0 => p,
        _ => return Err(bad("the port is 1 to 65535")),
    };
    if !host.starts_with('/') {
        return Err(bad("the host socket is an absolute path"));
    }
    Ok((PathBuf::from(host), port))
}

/// The chardev socket of the run, beside the stderr socket.
pub fn socket_path(err_sock: &Path) -> PathBuf {
    err_sock.with_extension("ch")
}

/// The emulator's arguments: a second `virtio-serial` bus with one port.
pub fn argv(sock: &Path) -> Vec<String> {
    vec![
        "-device".into(),
        "virtio-serial-pci,id=podvmch".into(),
        "-chardev".into(),
        format!(
            "socket,id=podvmchd,path={},server=on,wait=off",
            sock.display()
        ),
        "-device".into(),
        format!("virtserialport,bus=podvmch.0,chardev=podvmchd,name={PORT_NAME}"),
    ]
}

fn runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

/// The host side: connect to the chardev socket, then dial `host` for each
/// stream that the guest opens, until the emulator ends.
pub fn serve(sock: PathBuf, host: PathBuf, deadline: Instant) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let Ok(rt) = runtime() else { return };
        rt.block_on(async move {
            let carrier = loop {
                match tokio::net::UnixStream::connect(&sock).await {
                    Ok(s) => break s,
                    Err(_) if Instant::now() < deadline => {
                        tokio::time::sleep(Duration::from_millis(50)).await
                    }
                    Err(_) => return,
                }
            };
            let Ok(link) =
                pod_link::Link::new(carrier, pod_link::Role::Listener, Default::default())
            else {
                return;
            };
            while let Ok(mut stream) = link.accept().await {
                let host = host.clone();
                tokio::spawn(async move {
                    match tokio::net::UnixStream::connect(&host).await {
                        Ok(mut h) => {
                            let _ = tokio::io::copy_bidirectional(&mut stream, &mut h).await;
                        }
                        Err(e) => eprintln!(
                            "podvm: machine tier: --podvm-channel: {}: {e}",
                            host.display()
                        ),
                    }
                });
            }
        });
    })
}

/// The device of the port named [`PORT_NAME`], from sysfs.
fn port_device() -> Option<PathBuf> {
    for e in std::fs::read_dir("/sys/class/virtio-ports").ok()?.flatten() {
        let name = std::fs::read_to_string(e.path().join("name")).unwrap_or_default();
        if name.trim() == PORT_NAME {
            return Some(Path::new("/dev").join(e.file_name()));
        }
    }
    None
}

/// `podvm guest-channel PORT`, in a guest: listen on `127.0.0.1:PORT` and
/// carry each connection over the port. It writes [`READY`] once it
/// listens, and runs until the guest ends.
pub fn guest_main(args: &[String]) -> i32 {
    let Some(port) = args.first().and_then(|p| p.parse::<u16>().ok()) else {
        eprintln!("podvm guest-channel: name the port");
        return EXIT_FLAG_ERROR;
    };
    if let Err(e) = super::net::lo_up() {
        eprintln!("podvm guest-channel: {e}");
    }
    let mut dev = port_device();
    for _ in 0..100 {
        if dev.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
        dev = port_device();
    }
    let Some(dev) = dev else {
        eprintln!("podvm guest-channel: no virtio port named {PORT_NAME}");
        return 1;
    };
    let opened = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&dev)
        .and_then(|f| Ok((f.try_clone()?, f)));
    let (r, w) = match opened {
        Ok(p) => p,
        Err(e) => {
            eprintln!("podvm guest-channel: {}: {e}", dev.display());
            return 1;
        }
    };
    let Ok(rt) = runtime() else { return 1 };
    rt.block_on(async move {
        let carrier = tokio::io::join(tokio::fs::File::from_std(r), tokio::fs::File::from_std(w));
        let link = match pod_link::Link::new(carrier, pod_link::Role::Dialer, Default::default()) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("podvm guest-channel: {e}");
                return 1;
            }
        };
        let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("podvm guest-channel: 127.0.0.1:{port}: {e}");
                return 1;
            }
        };
        let _ = std::fs::write(READY, b"");
        while let Ok((mut tcp, _)) = listener.accept().await {
            match link.open() {
                Ok(mut s) => {
                    tokio::spawn(async move {
                        let _ = tokio::io::copy_bidirectional(&mut tcp, &mut s).await;
                    });
                }
                Err(e) => {
                    eprintln!("podvm guest-channel: {e}");
                    return 1;
                }
            }
        }
        0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_names_a_host_socket_and_a_port_of_the_loopback() {
        assert_eq!(
            parse("run", "/run/egress.sock:127.0.0.1:8080").unwrap(),
            (PathBuf::from("/run/egress.sock"), 8080)
        );
        for bad in [
            "egress.sock:127.0.0.1:80",
            "/s:0.0.0.0:80",
            "/s:127.0.0.1:0",
            "/s:80",
            "/s:127.0.0.1:x",
        ] {
            assert!(parse("run", bad).is_err(), "{bad}");
        }
        let a = argv(Path::new("/r/x.ch"));
        assert!(a
            .iter()
            .any(|w| w == "virtserialport,bus=podvmch.0,chardev=podvmchd,name=podvm.ch"));
    }
}
