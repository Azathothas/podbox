//! Acquisition and the one provisioning boot. `TODO/milestones.md` T-1112.

use std::path::{Path, PathBuf};
use std::time::Duration;

use podbox_image::error::{EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};
use podbox_windows::Ceiling;

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
/// connection that has stopped producing bytes. The triple repeats the
/// registry client's (`podbox-image/src/registry.rs`), one shape per caller
/// rather than a shared constant in a third place.
const FETCH_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const FETCH_READ_TIMEOUT: Duration = Duration::from_secs(120);
const FETCH_WRITE_TIMEOUT: Duration = Duration::from_secs(60);

/// The HTTP caller for `fetch`, with the timeouts above rather than the
/// library default, so a black-holed origin fails loud instead of hanging
/// a caller that cannot prompt.
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(FETCH_CONNECT_TIMEOUT)
        .timeout_read(FETCH_READ_TIMEOUT)
        .timeout_write(FETCH_WRITE_TIMEOUT)
        .build()
}

/// Where `setup` leaves the provisioned image: beside the vendor's, under a
/// name that says which one it came from.
///
/// ⛔ **The provisioning writes have to outlive the boot that made them, and
/// the first revision got that wrong.** `run`'s disk is a disposable overlay
/// over the base image, so an installer that wrote into a scratch overlay
/// left the agent in a file `run` then threw away: the guest booted, the task
/// was gone, and every run timed out. `setup` therefore produces a *new base*
///, the vendor's image plus the agent, and `run` makes its overlay over it.
pub(crate) fn provisioned_path(image: &Path) -> PathBuf {
    let stem = image
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "windows".to_string());
    image.with_file_name(format!("{stem}.podbox.qcow2"))
}

/// Where `fetch` puts the image when `--image` names nothing.
pub(crate) fn fetch_dest(a: &Args) -> PathBuf {
    a.image.clone().unwrap_or_else(podbox_windows::base_cache)
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
/// ⚠ **The policy is in `podbox-windows`, this is the socket.** The crate
/// owns the ceiling, the hash and the cleanup and is tested without a
/// network; what is left here is one HTTP call and the reporting.
pub(crate) fn fetch(a: &Args) -> i32 {
    let Some(url) = a.url.clone() else {
        eprintln!("podbox windows fetch: --url is required\n{USAGE}");
        return EXIT_FLAG_ERROR;
    };
    let dest = fetch_dest(a);
    if dest.exists() {
        eprintln!(
            "podbox windows fetch: {} already exists; remove it to fetch again",
            dest.display()
        );
        return EXIT_FLAG_ERROR;
    }
    if let Some(dir) = dest.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("podbox windows fetch: {}: {e}", dir.display());
            return EXIT_RUNTIME_ERROR;
        }
    }
    let fsize = match plan::ceiling() {
        Ok(c) => c,
        Err(c) => return c,
    };
    let ceiling = Ceiling {
        max_bytes: a.max_bytes.unwrap_or(DEFAULT_MAX_BYTES),
        fsize,
    };
    let response = match agent().get(&url).call() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("podbox windows fetch: {url}: {e}");
            return EXIT_RUNTIME_ERROR;
        }
    };
    let declared = response
        .header("Content-Length")
        .and_then(|v| v.trim().parse::<u64>().ok());
    match podbox_windows::fetch::store(
        response.into_reader(),
        declared,
        &dest,
        a.sha256.as_deref(),
        ceiling,
    ) {
        Ok(got) => {
            println!("{got}  {}", dest.display());
            if a.sha256.is_none() {
                eprintln!(
                    "podbox windows fetch: no --sha256 was pinned, so {} is \
                     bounded but unverified. Pin the digest for anything you \
                     will run again",
                    dest.display()
                );
            }
            0
        }
        Err(e) => {
            eprintln!("podbox windows fetch: {e}");
            EXIT_RUNTIME_ERROR
        }
    }
}

/// `setup`: provision the image once, into [`provisioned_path`].
pub(crate) fn setup(a: &Args) -> i32 {
    let Some(image) = a.image.clone() else {
        eprintln!("podbox windows setup: --image is required\n{USAGE}");
        return EXIT_FLAG_ERROR;
    };
    if !image.is_file() {
        eprintln!(
            "podbox windows setup: {} is not a file. Fetch one with `podbox \
             windows fetch` or pass the vendor image",
            image.display()
        );
        return EXIT_RUNTIME_ERROR;
    }
    let out = provisioned_path(&image);
    if out.exists() {
        eprintln!(
            "podbox windows setup: {} already exists; remove it to provision again",
            out.display()
        );
        return EXIT_FLAG_ERROR;
    }
    let plan = match plan::build(a) {
        Ok(p) => p,
        Err(c) => return c,
    };
    // ⚠ The provisioned image IS this boot's disk, not a disposable overlay
    // over it: what the installer writes here has to survive.
    let plan = podbox_windows::Plan {
        root: out.clone(),
        ..plan
    };
    match podbox_windows::provision(
        &plan::qemu_img(),
        &image,
        &plan,
        Duration::from_secs(a.timeout.unwrap_or(240)),
    ) {
        Ok(report) => {
            println!("{}", String::from_utf8_lossy(&report));
            println!("provisioned {}", out.display());
            // ⚠ `out` is beside the vendor image and is the whole product of
            // this step, so only the scratch directory goes.
            podbox_windows::cleanup(&plan);
            0
        }
        Err(e) => {
            eprintln!("podbox windows setup: {e}");
            eprintln!(
                "podbox windows setup: nothing was left at {}",
                out.display()
            );
            let _ = std::fs::remove_file(&out);
            podbox_windows::cleanup(&plan);
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
            PathBuf::from("/images/ValidationOS.podbox.qcow2")
        );
        assert_eq!(
            provisioned_path(Path::new("freedos.img")),
            PathBuf::from("freedos.podbox.qcow2")
        );
    }

    /// A hung origin fails on the read timeout instead of hanging the
    /// caller: the defect was `ureq::get` with the library default and no
    /// bound at all. The server accepts and then never answers; the agent
    /// carries a 200 ms read timeout for the test, so the error must arrive
    /// long before the server's 10 s hold ends.
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
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_millis(200))
            .timeout_write(Duration::from_secs(5))
            .build();
        let err = agent
            .get(&format!("http://127.0.0.1:{port}/hangs"))
            .call()
            .unwrap_err();
        assert!(
            matches!(err, ureq::Error::Transport(_)),
            "a hung read is a transport error, not {err:?}"
        );
        assert!(
            t0.elapsed() < Duration::from_secs(8),
            "the timeout fired; the server never answered"
        );
    }
}
