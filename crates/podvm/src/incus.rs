//! The incus back end, TODO/machine.md T-1718, in the form of
//! `docs/incus.md`: podvm drives an incusd over its REST API on a unix
//! socket. `--podvm-tier=incus` selects it for `run`.

pub(crate) mod config;
pub(crate) mod image;
pub(crate) mod inspect;
pub(crate) mod run;
pub(crate) mod verbs;

#[cfg(test)]
mod tests;

/// The verbs that `--podvm-tier=incus` serves on an instance that stays
/// (TODO/machine.md T-1722): their exit, or `None` for another verb.
pub fn intercept(verb: Option<&str>, rest: &[String]) -> Option<i32> {
    let a = verbs::wants(rest)?;
    match verb? {
        v @ ("ps" | "start" | "stop" | "rm" | "logs") => Some(verbs::dispatch(v, &a)),
        "inspect" => Some(inspect::inspect(&a)),
        _ => None,
    }
}

/// The command of an instance that stays (TODO/machine.md T-1722), as
/// `run -d` and `create` push it to [`CMD`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cmd {
    pub argv: Vec<String>,
    pub env: Vec<String>,
    pub cwd: String,
}

impl Cmd {
    /// The JSON of the file.
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::json!({"argv": self.argv, "env": self.env, "cwd": self.cwd})
            .to_string()
            .into_bytes()
    }

    /// The command of the JSON of the file.
    pub fn parse(b: &[u8]) -> Result<Cmd, String> {
        let v: serde_json::Value = serde_json::from_slice(b).map_err(|e| e.to_string())?;
        let list = |k: &str| -> Vec<String> {
            v.get(k)
                .and_then(serde_json::Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default()
        };
        Ok(Cmd {
            argv: list("argv"),
            env: list("env"),
            cwd: v
                .get("cwd")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("/")
                .to_string(),
        })
    }
}

/// The files of the command in the instance.
pub const CMD: &str = "/.podvm/cmd";
pub const LOG: &str = "/.podvm/log";
pub const EXIT: &str = "/.podvm/exit";

/// The exit code of a wait status.
fn code_of(status: i32) -> i32 {
    if status & 0x7f == 0 {
        (status >> 8) & 0xff
    } else {
        128 + (status & 0x7f)
    }
}

/// `podvm guest-hold [--run]`: the init of a podvm instance. As PID 1 it
/// reaps each orphan, and it ends on SIGTERM, SIGINT or SIGPWR (incus stops
/// an instance with SIGPWR); a PID 1 with no handler would ignore each.
///
/// With `--run` (TODO/machine.md T-1722) it first starts the command of
/// [`CMD`] with stdout and stderr appended to [`LOG`], passes a stop signal
/// to it as SIGTERM, and ends with the command's exit code, which it writes
/// to [`EXIT`]: the instance stops when its command ends, as a docker
/// container does.
pub fn hold(args: &[String]) -> i32 {
    unsafe extern "C" {
        fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
        fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
        fn pause() -> i32;
        fn kill(pid: i32, sig: i32) -> i32;
        fn _exit(code: i32) -> !;
    }
    static CHILD: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
    extern "C" fn end(_: i32) {
        let child = CHILD.load(std::sync::atomic::Ordering::Relaxed);
        if child > 0 {
            // SAFETY: kill is async-signal-safe; the loop reaps the child.
            unsafe { kill(child, 15) };
            return;
        }
        // SAFETY: _exit is async-signal-safe.
        unsafe { _exit(0) }
    }
    if args.first().is_some_and(|a| a == "--run") {
        match start_cmd() {
            Ok(pid) => CHILD.store(pid, std::sync::atomic::Ordering::Relaxed),
            Err(e) => {
                let _ = std::fs::write(LOG, format!("podvm guest-hold: {e}\n"));
                let _ = std::fs::write(EXIT, "127\n");
                return 127;
            }
        }
    }
    // SAFETY: plain system calls; the handler only signals or exits.
    unsafe {
        for sig in [2, 15, 30] {
            signal(sig, end);
        }
        loop {
            let mut status = 0;
            let pid = waitpid(-1, &mut status, 0);
            if pid < 0 {
                pause();
                continue;
            }
            if pid == CHILD.load(std::sync::atomic::Ordering::Relaxed) {
                let code = code_of(status);
                let _ = std::fs::write(EXIT, format!("{code}\n"));
                return code;
            }
        }
    }
}

/// Start the command of [`CMD`]; its pid.
fn start_cmd() -> Result<i32, String> {
    let text = std::fs::read(CMD).map_err(|e| format!("{CMD}: {e}"))?;
    let cmd = Cmd::parse(&text).map_err(|e| format!("{CMD}: {e}"))?;
    let (prog, rest) = cmd.argv.split_first().ok_or("the command is empty")?;
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG)
        .map_err(|e| format!("{LOG}: {e}"))?;
    let err = log.try_clone().map_err(|e| e.to_string())?;
    let child = std::process::Command::new(prog)
        .args(rest)
        .env_clear()
        .envs(cmd.env.iter().filter_map(|e| e.split_once('=')))
        .current_dir(if cmd.cwd.is_empty() { "/" } else { &cmd.cwd })
        .stdin(std::process::Stdio::null())
        .stdout(log)
        .stderr(err)
        .spawn()
        .map_err(|e| format!("{prog}: {e}"))?;
    Ok(child.id() as i32)
}
