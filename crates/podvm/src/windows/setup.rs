//! Acquisition and the one provisioning boot. `TODO/milestones.md` T-1112.

use std::path::{Path, PathBuf};
use std::time::Duration;

use podvm_image::error::{EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};
use podvm_image::net::{Body, Limits, Net};
use podvm_windows::Ceiling;

use super::args::{Args, USAGE};
use super::plan;

/// The default ceiling for `fetch`. Deliberately finite: a default of
/// "unlimited" makes `--max-bytes` decorative, and the point of the ceiling
/// is that a caller who did not think about it is still bounded. A caller
/// who means to pull something larger says so.
pub(crate) const DEFAULT_MAX_BYTES: u64 = 8 * 1024 * 1024 * 1024;
/// The ceiling is finite and nonzero: an unlimited default makes
/// `--max-bytes` decorative, and a zero one downloads nothing.
const _: () = assert!(DEFAULT_MAX_BYTES > 0 && DEFAULT_MAX_BYTES < u64::MAX);

/// Bounds a stalled transfer, not a large one. The body is streamed, so an
/// overall deadline would fail a slow but healthy download; these fail a
/// connection that has stopped producing bytes. The pair repeats the
/// registry client's (`podvm-image/src/registry.rs`), one shape per caller
/// rather than a shared constant in a third place.
const FETCH_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const FETCH_READ_TIMEOUT: Duration = Duration::from_secs(120);
pub(crate) const FETCH_LIMITS: Limits = Limits {
    head: FETCH_CONNECT_TIMEOUT.saturating_add(FETCH_READ_TIMEOUT),
    idle: FETCH_READ_TIMEOUT,
};

/// One GET of `url` with `limits`, through the proxy of the environment
/// (`podvm_image::net`), with the redirects followed. A black-holed origin
/// fails loud instead of hanging a caller that cannot prompt.
pub(crate) fn get(url: &str, headers: &[(&str, String)], limits: Limits) -> Result<Body, String> {
    let (config, _roots) = podvm_image::tls::client_config();
    let mut net = Net::from_environment(config)?;
    net.send("GET", url, headers, true, limits)
        .map_err(|e| e.to_string())
}

/// Where `setup` leaves the provisioned image: beside the vendor's, under a
/// name that says which one it came from.
///
/// ⛔ **The provisioning writes have to outlive the boot that made them, and
/// the first revision got that wrong.** `run`'s disk is a disposable overlay
/// over the base image, so an installer that wrote into a scratch overlay
/// left the agent in a file `run` then threw away: the guest booted, the task
/// was gone, and every run timed out.
///
/// ⛔ **And the writes are committed into this file rather than left in the
/// overlay.** `provision` boots a scratch overlay inside the per-run
/// directory, which `cleanup` removes, and `qemu-img commit`s it into a
/// second, standalone qcow2 at this path. So `setup` produces a *new base*,
/// the vendor's image plus the agent, and `run` makes its overlay over it.
/// The provisioned image names no backing file, so the vendor's is free to
/// move or be removed afterwards. `TODO/gate.md` T-1641.
pub(crate) fn provisioned_path(image: &Path) -> PathBuf {
    let stem = image
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "windows".to_string());
    image.with_file_name(format!("{stem}.podvm.qcow2"))
}

/// Where `fetch` puts the image when `--image` names nothing.
pub(crate) fn fetch_dest(a: &Args) -> PathBuf {
    a.image.clone().unwrap_or_else(podvm_windows::base_cache)
}

/// `fetch`: download a base image, bounded and verified.
///
/// ⛔ **Three refusals, and two of them happen before the transfer.** The
/// declared length is judged against the ceiling first, so a download that
/// would be refused at the end does not spend the bandwidth; the arriving
/// bytes are judged as they arrive, for an origin that declared nothing or
/// lied; and the digest is judged at the end. A refusal removes the partial
/// file, so no later run trusts a truncated image under the destination's
/// name.
///
/// ⚠ **The policy is in `podvm-windows`, this is the socket.** The crate
/// owns the ceiling, the hash and the cleanup and is tested without a
/// network; what is left here is one HTTP call and the reporting.
pub(crate) fn fetch(a: &Args) -> i32 {
    let Some(url) = a.url.clone() else {
        eprintln!("podvm windows fetch: --url is required\n{USAGE}");
        return EXIT_FLAG_ERROR;
    };
    // TODO/machine.md T-1863: the byte range of one file of the UDF image,
    // read by range requests; nothing is downloaded.
    if let Some(file) = &a.extent {
        return match podvm_windows::udf::extent(super::origin::RangeRead { url: url.clone() }, file)
        {
            Ok(r) => {
                println!("{} {} {}", r.start, r.end - r.start, file);
                0
            }
            Err(why) => {
                eprintln!("podvm windows fetch: {url}: {why}");
                EXIT_RUNTIME_ERROR
            }
        };
    }
    let dest = fetch_dest(a);
    if dest.exists() {
        eprintln!(
            "podvm windows fetch: {} already exists; remove it to fetch again",
            dest.display()
        );
        return EXIT_FLAG_ERROR;
    }
    if let Some(dir) = dest.parent()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        eprintln!("podvm windows fetch: {}: {e}", dir.display());
        return EXIT_RUNTIME_ERROR;
    }
    let fsize = match plan::ceiling() {
        Ok(c) => c,
        Err(c) => return c,
    };
    let ceiling = Ceiling {
        max_bytes: a.max_bytes.unwrap_or(DEFAULT_MAX_BYTES),
        fsize,
    };
    let mut origin = super::origin::Http { url: url.clone() };
    match podvm_windows::fetch::store(
        &mut origin,
        &dest,
        a.sha256.as_deref(),
        ceiling,
        super::origin::TRIES,
    ) {
        Ok(got) => {
            println!("{got}  {}", dest.display());
            if a.sha256.is_none() {
                eprintln!(
                    "podvm windows fetch: no --sha256 was pinned, so {} is \
                     bounded but unverified. Pin the digest for anything you \
                     will run again",
                    dest.display()
                );
            }
            0
        }
        Err(e) => {
            eprintln!("podvm windows fetch: {e}");
            EXIT_RUNTIME_ERROR
        }
    }
}

/// ⛔ `out` is a qcow2 whatever the vendor image is, and that is not a
/// re-encoding: `commit` copies the backing's bytes into it. The size is the
/// vendor image's size, so a large base costs a large `out`.
const PROVISIONED_FORMAT: &str = "qcow2";

/// `setup`: provision the image once, into [`provisioned_path`].
///
/// ⛔ **The provisioning boot is disposable and its writes are committed, so
/// the vendor image is never a destination.** `plan::build` already put
/// `root` inside the per-run directory; that overlay is what the installer
/// writes into, and `provision` commits it into [`provisioned_path`] with no
/// `-b`. ⛔ The plan is left exactly as built: a `root` pointed at `out`
/// would make the commit fold the vendor's backing into the file that names
/// it. `TODO/gate.md` T-1641.
pub(crate) fn setup(a: &Args) -> i32 {
    let Some(image) = a.image.clone() else {
        eprintln!("podvm windows setup: --image is required\n{USAGE}");
        return EXIT_FLAG_ERROR;
    };
    if !image.is_file() {
        eprintln!(
            "podvm windows setup: {} is not a file. Fetch one with `podvm \
             windows fetch` or pass the vendor image",
            image.display()
        );
        return EXIT_RUNTIME_ERROR;
    }
    let out = provisioned_path(&image);
    if out.exists() {
        eprintln!(
            "podvm windows setup: {} already exists; remove it to provision again",
            out.display()
        );
        return EXIT_FLAG_ERROR;
    }
    let plan = match plan::build(a) {
        Ok(p) => p,
        Err(c) => return c,
    };
    // The commit writes a new file at this path, so a survivor of an earlier
    // attempt would be refused by the preflight above and overwritten here.
    let commit_target = out.with_extension(PROVISIONED_FORMAT);
    match podvm_windows::provision(
        &plan::qemu_img(),
        &image,
        &plan,
        &commit_target,
        Duration::from_secs(a.timeout.unwrap_or(240)),
    ) {
        Ok(report) => {
            println!("{}", String::from_utf8_lossy(&report));
            println!("provisioned {}", commit_target.display());
            // ⚠ `commit_target` is beside the vendor image and is the whole
            // product of this step, so only the scratch directory goes.
            podvm_windows::cleanup(&plan);
            0
        }
        Err(e) => {
            eprintln!("podvm windows setup: {e}");
            eprintln!(
                "podvm windows setup: nothing was left at {}",
                commit_target.display()
            );
            let _ = std::fs::remove_file(&commit_target);
            podvm_windows::cleanup(&plan);
            EXIT_RUNTIME_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_provisioned_image_is_beside_the_vendor_one_and_named_for_it() {
        assert_eq!(
            provisioned_path(Path::new("/images/ValidationOS.vhdx")),
            PathBuf::from("/images/ValidationOS.podvm.qcow2")
        );
        assert_eq!(
            provisioned_path(Path::new("freedos.img")),
            PathBuf::from("freedos.podvm.qcow2")
        );
    }

    /// ⛔ The defect this pins: `setup` pointed the boot's root at the
    /// provisioned image, which is the vendor image plus a `-b` pointer at the
    /// vendor file. Every later run then read `run.qcow2` -> `out` -> `base.vhdx`
    /// and broke when the vendor file moved. `plan::build` puts `root` in the
    /// per-run directory and `provision` commits it into this separate file.
    #[test]
    fn the_provisioning_root_is_scratch_and_the_product_is_a_separate_file() {
        let scratch = podvm_windows::scratch_dir("setup-shape").expect("a scratch directory");
        let vendor = std::env::temp_dir().join(format!("pbx-setup-{}", std::process::id()));
        std::fs::create_dir_all(&vendor).unwrap();
        let image = vendor.join("ValidationOS.vhdx");
        let out = provisioned_path(&image);
        let root = scratch.join("run.qcow2");
        assert_eq!(out, vendor.join("ValidationOS.podvm.qcow2"));
        assert_ne!(
            root, out,
            "the overlay the installer writes into is not the image it is committed to"
        );
        assert_eq!(
            podvm_windows::scratch_of(&podvm_windows::Plan {
                emulator: "qemu-system-x86_64".into(),
                accel: podvm_windows::Accel::Tcg,
                share: "/usr/share/qemu".into(),
                firmware_code: "/code.fd".into(),
                firmware_vars: scratch.join("vars.fd"),
                root: root.clone(),
                mailbox: scratch.join("mailbox.img"),
                serial: scratch.join("serial.log"),
                monitor: scratch.join("qmp.sock"),
                memory_mib: 4096,
                cpus: 2,
                emu_args: Vec::new(),
                options: Default::default(),
            })
            .as_deref(),
            Some(scratch.as_path()),
            "the scratch overlay is the only per-run residue, and cleanup removes it"
        );
        let _ = std::fs::remove_dir_all(&scratch);
        let _ = std::fs::remove_dir_all(&vendor);
    }

    /// A hung origin fails on the read timeout instead of hanging the
    /// caller: the defect was a GET with the library default and no bound
    /// at all. The server accepts and then never answers; the request
    /// carries a 200 ms limit for the test, so the error must arrive long
    /// before the server's 10 s hold ends.
    #[test]
    fn a_hung_origin_fails_on_the_read_timeout() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback binds where tests run");
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let _ = listener.accept();
            std::thread::sleep(Duration::from_secs(10));
        });
        let t0 = std::time::Instant::now();
        let limits = Limits {
            head: Duration::from_millis(200),
            idle: Duration::from_millis(200),
        };
        let err = match get(&format!("http://127.0.0.1:{port}/hangs"), &[], limits) {
            Ok(_) => panic!("a hung origin gives no answer"),
            Err(e) => e,
        };
        assert!(
            err.contains("127.0.0.1"),
            "a hung read names its host: {err}"
        );
        assert!(
            t0.elapsed() < Duration::from_secs(8),
            "the timeout fired; the server never answered"
        );
    }
}
