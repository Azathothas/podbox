//! The `/podvm-init` of a guest and the reading of its console,
//! TODO/machine.md T-1656.
//!
//! The kernel runs `/podvm-init` (`rdinit=`) with the console on the first
//! serial port. The script mounts `/proc`, `/sys` and `/dev`, prints the
//! ready marker, runs the command with the image's environment in the
//! working directory, prints the exit marker with the code, and resets the
//! machine; `-no-reboot` makes the emulator exit then. Each marker carries
//! a nonce of 128 bits of this run, as T-1304 rules: the payload can print
//! anything, so a marker without a nonce is a collision waiting to happen.
//! stdout and stderr share the serial line.

/// The channel of the init: the guest's port of `--podvm-channel`, and
/// whether the run is sealed (T-1751).
pub struct Chan {
    pub port: Option<u16>,
    pub sealed: bool,
}

/// Quote one word for `/bin/sh`.
pub fn sh_quote(w: &str) -> String {
    format!("'{}'", w.replace('\'', "'\\''"))
}

/// The init script for `argv` with `env` (`NAME=VALUE`) in `workdir`.
/// `net` runs `podvm guest-net` first, which loads the NIC driver and
/// configures the network; a failure of it is a line of output. `vols`
/// runs `podvm guest-9p` before the `cd`, so the working directory can be
/// in a volume (TODO/machine.md T-1723); where a mount fails, the run ends
/// with 125 and the reason, and the command does not run. `tty` (`-t`, the
/// size `ROWSxCOLS` or empty) runs the command through `podvm guest-tty` on
/// the virtio console (TODO/machine.md T-1723) in place of the FIFOs.
/// `channel` starts `podvm guest-channel` and waits until it listens
/// (TODO/machine.md T-1751); a sealed run ([`super::sealed`]) fetches the
/// bundle first and sources its fragment after the image's command.
#[allow(clippy::too_many_arguments)]
pub fn script(
    nonce: &str,
    argv: &[String],
    env: &[String],
    workdir: &str,
    net: bool,
    vols: &[super::vol::Vol],
    tty: Option<&str>,
    channel: Option<Chan>,
) -> String {
    let mut s = String::from("#!/bin/sh\n");
    s.push_str("mount -t proc proc /proc 2>/dev/null\n");
    s.push_str("mount -t sysfs sysfs /sys 2>/dev/null\n");
    s.push_str("mount -t devtmpfs devtmpfs /dev 2>/dev/null\n");
    if !vols.is_empty() {
        let args: Vec<String> = vols.iter().map(|v| sh_quote(&super::vol::arg(v))).collect();
        s.push_str(&format!(
            "/podvm/podvm guest-9p {} 2>/podvm-9p.err || {{ echo PODVM-READY-{nonce}; \
             cat /podvm-9p.err; echo; echo PODVM-EXIT-{nonce} 125; sync; reboot -f; \
             echo b > /proc/sysrq-trigger; }}\n",
            args.join(" ")
        ));
    }
    for e in env {
        if let Some((k, v)) = e.split_once('=')
            && !k.is_empty()
            && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            s.push_str(&format!("export {k}={}\n", sh_quote(v)));
        }
    }
    s.push_str(&format!("cd {} || exit 125\n", sh_quote(workdir)));
    s.push_str(&format!("echo PODVM-READY-{nonce}\n"));
    if net {
        s.push_str("/podvm/podvm guest-net\n");
    }
    // TODO/machine.md T-1751: the channel listens before the command runs.
    if let Some(c) = &channel {
        let args = match (c.sealed, c.port) {
            (true, Some(p)) => format!("--sealed {p}"),
            (true, None) => "--sealed".to_string(),
            (false, Some(p)) => p.to_string(),
            (false, None) => String::new(),
        };
        // A bundle of 64 MiB takes longer than 10 s under TCG.
        let tries = if c.sealed { 1200 } else { 100 };
        s.push_str(&format!(
            "/podvm/podvm guest-channel {args} & i=0\nwhile [ ! -e {r} ] && [ $i -lt {tries} ]; do sleep 0.1; i=$((i+1)); done\n",
            r = super::channel::READY
        ));
        if c.sealed {
            s.push_str(&format!(
                "[ -e {r} ] || {{ echo 'podvm: the run bundle did not reach the guest'; echo; \
                 echo PODVM-EXIT-{nonce} 125; sync; reboot -f; echo b > /proc/sysrq-trigger; }}\n",
                r = super::channel::READY
            ));
        }
    }
    // stdin: the bytes of podvm's stdin where there were some, else none.
    s.push_str("if [ -f /podvm/stdin ]; then exec 0</podvm/stdin; else exec 0</dev/null; fi\n");
    let words: Vec<String> = argv.iter().map(|a| sh_quote(a)).collect();
    s.push_str(&format!("set -- {}\n", words.join(" ")));
    if channel.as_ref().is_some_and(|c| c.sealed) {
        s.push_str(&format!(". {} || exit 125\n", super::sealed::FRAGMENT));
    }
    if let Some(size) = tty {
        let size = if size.is_empty() {
            String::new()
        } else {
            format!("{} ", sh_quote(size))
        };
        s.push_str(&format!(
            "/podvm/podvm guest-tty {size}-- \"$@\"\nrc=$?\necho; echo PODVM-EXIT-{nonce} $rc\n"
        ));
        s.push_str("sync\nreboot -f 2>/dev/null\necho b > /proc/sysrq-trigger\n");
        return s;
    }
    // TODO/machine.md T-1728. The command's stdout and stderr are FIFOs, as
    // docker's are without -t: a serial port is a terminal, so `ls` coloured
    // its output and `[ -t 1 ]` was true. One `cat` copies each FIFO to its
    // port: stdout to the console, stderr to the second port (T-1656) where
    // the node is there.
    s.push_str("mkfifo /podvm-out /podvm-err\n");
    s.push_str("cat /podvm-out & o=$!\n");
    s.push_str(
        "if [ -c /dev/ttyS1 ]; then cat /podvm-err >/dev/ttyS1 & else cat /podvm-err >&2 & fi\ne=$!\n",
    );
    s.push_str("\"$@\" >/podvm-out 2>/podvm-err\nrc=$?\nwait $o $e\n");
    s.push_str(&format!("echo; echo PODVM-EXIT-{nonce} $rc\n"));
    s.push_str("sync\nreboot -f 2>/dev/null\necho b > /proc/sysrq-trigger\n");
    s
}

/// What one line of the console is.
#[derive(Debug, PartialEq, Eq)]
pub enum Line<'a> {
    /// Before the ready marker: the kernel and the image's start.
    Before,
    /// The ready marker.
    Ready,
    /// Output of the command.
    Out(&'a str),
    /// The exit marker, with the code and the output before it on the line.
    Exit(i32, &'a str),
}

/// The console reader of one run.
pub struct Console {
    nonce: String,
    ready: bool,
}

impl Console {
    pub fn new(nonce: &str) -> Self {
        Console {
            nonce: nonce.to_string(),
            ready: false,
        }
    }

    /// Read one line (without its newline; a `\r` is dropped).
    pub fn line<'a>(&mut self, raw: &'a str) -> Line<'a> {
        let raw = raw.trim_end_matches('\r');
        if !self.ready {
            if raw.trim() == format!("PODVM-READY-{}", self.nonce) {
                self.ready = true;
                return Line::Ready;
            }
            return Line::Before;
        }
        let mark = format!("PODVM-EXIT-{} ", self.nonce);
        if let Some(at) = raw.find(&mark) {
            let code = raw[at + mark.len()..].trim().parse::<i32>().unwrap_or(125);
            return Line::Exit(code, &raw[..at]);
        }
        Line::Out(raw)
    }
}

/// A nonce of 128 bits from the kernel's random source, as hex.
pub fn nonce() -> Result<String, String> {
    use std::io::Read;
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .map_err(|e| format!("/dev/urandom: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}
