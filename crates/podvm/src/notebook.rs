//! `podvm notebook`: cycles of a notebook that keep one directory
//! (TODO/cli.md T-1660, issue 174).
//!
//! Each cycle is a new container of `podvm run --rm` with a read-only
//! root, an empty `/tmp`, a `/workspace` that holds only the notebook, and
//! the kept directory at `/persist`. Each `--restart-kernel-after CELL`
//! cuts the notebook after that cell, and the next part runs in a new
//! container: a new kernel, the variables gone, `/persist` kept. A `.ipynb`
//! runs through `jupyter nbconvert --execute`; a `.py` (marimo) through
//! `marimo export html`.
//!
//! Each output line prints as `[N] text`, N the cycle. The exit is 1 when a
//! cell raised, 143 when `--kill-after` ended a cycle (each process of the
//! cycle gets SIGTERM, then SIGKILL), else the exit of the run.
//!
//! ⚠ On a rootfs tier the kernel's ports are on the host's loopback: two
//! notebooks pick their own ports, and the ports are not private.

mod cells;
#[cfg(test)]
mod tests;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use podvm_image::error::{EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};

const USAGE: &str = "\
usage: podvm notebook NB --persist DIR [--cycles N] [--kill-after S]
                      [--restart-kernel-after CELL]... [--image IMAGE]

Runs the notebook NB (.ipynb, or a marimo .py) N times (1 by default),
each time in a new container with a read-only root; only DIR survives a
cycle, at /persist. Each output line prints as `[N] text`. Exit 1 when a
cell raised, 143 when --kill-after S ended a cycle (TODO/cli.md T-1660).";

/// The image of a Jupyter notebook, and of a marimo file.
pub const JUPYTER_IMAGE: &str = "quay.io/jupyter/base-notebook:latest";
pub const MARIMO_IMAGE: &str = "ghcr.io/marimo-team/marimo:latest";
/// The exit of a cycle that `--kill-after` ended: 128 + SIGTERM.
pub use podvm_probe::exit::EXIT_KILLED;
/// The time from SIGTERM to SIGKILL.
const GRACE: Duration = Duration::from_secs(3);

/// The arguments of `podvm notebook`.
#[derive(Debug, Default, PartialEq)]
pub struct Args {
    pub nb: PathBuf,
    pub persist: PathBuf,
    pub cycles: u32,
    pub kill_after: Option<u64>,
    pub restart_after: Vec<usize>,
    pub image: Option<String>,
}

/// Parse the arguments. Err is the message of a usage error.
pub fn parse(args: &[String]) -> Result<Args, String> {
    let mut a = Args {
        cycles: 1,
        ..Args::default()
    };
    let mut nb = None;
    let mut persist = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        let mut value = |name: &str| -> Result<String, String> {
            inline
                .clone()
                .or_else(|| it.next().cloned())
                .ok_or(format!("{name} needs a value"))
        };
        let number = |name: &str, v: String| -> Result<u64, String> {
            v.parse::<u64>()
                .ok()
                .filter(|n| *n > 0)
                .ok_or(format!("{name} takes a whole number above 0, not {v:?}"))
        };
        match flag {
            "--persist" => persist = Some(PathBuf::from(value("--persist")?)),
            "--cycles" => a.cycles = number("--cycles", value("--cycles")?)?.min(1000) as u32,
            "--kill-after" => a.kill_after = Some(number("--kill-after", value("--kill-after")?)?),
            "--restart-kernel-after" => a
                .restart_after
                .push(number("--restart-kernel-after", value("--restart-kernel-after")?)? as usize),
            "--image" => a.image = Some(value("--image")?),
            f if f.starts_with('-') => return Err(format!("unknown option {f:?}")),
            _ if nb.is_none() => nb = Some(PathBuf::from(arg)),
            _ => return Err(format!("one notebook only, and {arg:?} is a second")),
        }
    }
    a.nb = nb.ok_or("the notebook NB is missing")?;
    a.persist = persist.ok_or("--persist DIR is missing: the kept directory")?;
    Ok(a)
}

/// `podvm notebook ...`.
pub fn main(args: &[String]) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        println!("{USAGE}");
        return 0;
    }
    let a = match parse(args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("podvm notebook: {e}\n{USAGE}");
            return EXIT_FLAG_ERROR;
        }
    };
    match run(&a) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("podvm notebook: {e} (TODO/cli.md T-1660)");
            EXIT_RUNTIME_ERROR
        }
    }
}

fn run(a: &Args) -> Result<i32, String> {
    let marimo = a.nb.extension().is_some_and(|e| e == "py");
    let name =
        a.nb.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("{}: not a file name", a.nb.display()))?
            .to_string();
    let text = std::fs::read(&a.nb).map_err(|e| format!("{}: {e}", a.nb.display()))?;
    let parts: Vec<Vec<u8>> = if marimo {
        if !a.restart_after.is_empty() {
            return Err("--restart-kernel-after needs a .ipynb: a marimo export is one run".into());
        }
        vec![text]
    } else {
        let nb: serde_json::Value = serde_json::from_slice(&text)
            .map_err(|e| format!("{}: not a notebook: {e}", a.nb.display()))?;
        cells::segments(&nb, &a.restart_after)?
            .iter()
            .map(|s| serde_json::to_vec(s).unwrap_or_default())
            .collect()
    };
    std::fs::create_dir_all(&a.persist).map_err(|e| format!("{}: {e}", a.persist.display()))?;
    let persist = std::fs::canonicalize(&a.persist).map_err(|e| e.to_string())?;
    let image = a
        .image
        .clone()
        .unwrap_or_else(|| (if marimo { MARIMO_IMAGE } else { JUPYTER_IMAGE }).to_string());
    eprintln!(
        "podvm notebook: {} cycle(s) of {name}, {} kernel(s) each, from {image}; each cycle \
         is a new container with a read-only root, and only {} (at /persist) is kept. The \
         kernel's ports are on the host's loopback: not private (TODO/cli.md T-1660)",
        a.cycles,
        parts.len(),
        persist.display()
    );
    let mut raised = false;
    for cycle in 1..=a.cycles {
        for (k, part) in parts.iter().enumerate() {
            let ws = workspace(cycle, k)?;
            std::fs::write(ws.join(&name), part).map_err(|e| e.to_string())?;
            let argv = run_argv(&ws, &persist, &image, &name, marimo, a.kill_after);
            let r = cycle_run(&argv, a.kill_after);
            let html = std::fs::read(ws.join("out.html"));
            let _ = std::fs::remove_dir_all(&ws);
            let (code, out, err) = r?;
            if code == EXIT_KILLED {
                println!("[{cycle}] killed after {} s", a.kill_after.unwrap_or(0));
                return Ok(EXIT_KILLED);
            }
            if marimo {
                if code != 0 {
                    eprint!("{err}");
                    return Ok(code);
                }
                let html = html.map_err(|e| format!("marimo wrote no HTML: {e}"))?;
                let to = a.nb.with_extension("html");
                std::fs::write(&to, &html).map_err(|e| format!("{}: {e}", to.display()))?;
                println!("[{cycle}] html: {} ({} bytes)", to.display(), html.len());
                continue;
            }
            let nb: serde_json::Value = match serde_json::from_str(&out) {
                Ok(v) if code == 0 => v,
                _ => {
                    eprint!("{err}");
                    return Ok(if code == 0 { EXIT_RUNTIME_ERROR } else { code });
                }
            };
            let o = cells::outcome(&nb, cycle);
            for l in &o.lines {
                println!("{l}");
            }
            raised |= o.raised;
        }
    }
    Ok(if raised { 1 } else { 0 })
}

/// A new, empty directory for the `/workspace` of one kernel.
fn workspace(cycle: u32, k: usize) -> Result<PathBuf, String> {
    let d = std::env::temp_dir().join(format!("podvm-notebook-{}-{cycle}-{k}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    Ok(d)
}

/// The `podvm run` of one kernel.
pub fn run_argv(
    ws: &Path,
    persist: &Path,
    image: &str,
    name: &str,
    marimo: bool,
    kill_after: Option<u64>,
) -> Vec<String> {
    let s = |x: &str| x.to_string();
    let mut v = vec![s("run"), s("--rm"), s("--read-only"), s("--tmpfs=/tmp")];
    for e in [
        "HOME=/tmp",
        "JUPYTER_RUNTIME_DIR=/tmp/jupyter-runtime",
        "JUPYTER_DATA_DIR=/tmp/jupyter-data",
        "IPYTHONDIR=/tmp/ipython",
        "MPLCONFIGDIR=/tmp/mpl",
        "XDG_CACHE_HOME=/tmp/cache",
        "PYTHONDONTWRITEBYTECODE=1",
    ] {
        v.push(s("-e"));
        v.push(s(e));
    }
    v.push(s("-v"));
    v.push(format!("{}:/workspace", ws.display()));
    v.push(s("-v"));
    v.push(format!("{}:/persist", persist.display()));
    v.push(s("--workdir=/workspace"));
    // The bound of the run backs the bound of the verb.
    if let Some(k) = kill_after {
        v.push(format!("--timeout={}", k + 2 * GRACE.as_secs() + 5));
    }
    v.push(s("--entrypoint="));
    v.push(s(image));
    if marimo {
        v.extend(
            [
                "marimo",
                "export",
                "html",
                "--no-include-code",
                name,
                "-o",
                "out.html",
            ]
            .map(s),
        );
    } else {
        v.extend(
            [
                "jupyter",
                "nbconvert",
                "--to",
                "notebook",
                "--execute",
                "--allow-errors",
                "--stdout",
                "--log-level=ERROR",
                name,
            ]
            .map(s),
        );
    }
    v
}

/// Run one kernel: its exit code, its stdout and its stderr. At the bound,
/// SIGTERM to the run and each process under it, then SIGKILL, and
/// [`EXIT_KILLED`].
fn cycle_run(argv: &[String], kill_after: Option<u64>) -> Result<(i32, String, String), String> {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().map_err(|e| format!("the podvm binary: {e}"))?;
    let mut child = std::process::Command::new(exe)
        .args(argv)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("podvm run: {e}"))?;
    let mut so = child.stdout.take().ok_or("no stdout")?;
    let mut se = child.stderr.take().ok_or("no stderr")?;
    let t_out = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = so.read_to_string(&mut s);
        s
    });
    let t_err = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = se.read_to_string(&mut s);
        s
    });
    let deadline = kill_after.map(|k| Instant::now() + Duration::from_secs(k));
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            break Some(st);
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            break None;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let code = match status {
        Some(st) => st.code().unwrap_or_else(|| 128 + st.signal().unwrap_or(0)),
        None => {
            end_tree(&mut child);
            // A run killed during the copy of its tree leaves a partial
            // tree; its process is gone now, so the sweep removes it.
            if let Ok(store) = podvm_image::open_store() {
                podvm_extract::tree::sweep(&store);
            }
            EXIT_KILLED
        }
    };
    let out = t_out.join().unwrap_or_default();
    let err = t_err.join().unwrap_or_default();
    Ok((code, out, err))
}

/// SIGTERM to the run and each process under it, SIGKILL to each that
/// stays after [`GRACE`], and the wait of the run.
fn end_tree(child: &mut std::process::Child) {
    let pid = i64::from(child.id());
    let tree = |pid| {
        let mut t = podvm_supervise::tree::descendants(Path::new("/proc"), pid).unwrap_or_default();
        t.push(pid);
        t
    };
    let first = tree(pid);
    for p in &first {
        let _ = podvm_probe::sys::kill(*p, 15);
    }
    let until = Instant::now() + GRACE;
    while Instant::now() < until {
        if matches!(child.try_wait(), Ok(Some(_))) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // The run may be gone and its payload not: a process that the run's
    // end gave to another parent is in the tree taken before the SIGTERM.
    for p in first.into_iter().chain(tree(pid)) {
        let _ = podvm_probe::sys::kill(p, 9);
    }
    let _ = child.wait();
}
