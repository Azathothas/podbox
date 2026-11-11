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

/// Quote one word for `/bin/sh`.
pub fn sh_quote(w: &str) -> String {
    format!("'{}'", w.replace('\'', "'\\''"))
}

/// The init script for `argv` with `env` (`NAME=VALUE`) in `workdir`.
pub fn script(nonce: &str, argv: &[String], env: &[String], workdir: &str) -> String {
    let mut s = String::from("#!/bin/sh\n");
    s.push_str("mount -t proc proc /proc 2>/dev/null\n");
    s.push_str("mount -t sysfs sysfs /sys 2>/dev/null\n");
    s.push_str("mount -t devtmpfs devtmpfs /dev 2>/dev/null\n");
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
    let words: Vec<String> = argv.iter().map(|a| sh_quote(a)).collect();
    s.push_str(&format!("set -- {}\n", words.join(" ")));
    s.push_str("\"$@\"\nrc=$?\n");
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
