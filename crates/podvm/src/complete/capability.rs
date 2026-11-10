//! TODO/cli.md T-1650. The capability gaps of the loader family.
//!
//! A gap is a thing that a payload on the userland rung cannot do and the
//! chroot rung can. Each gap names the entry that closes it, and leaves
//! [`LOADER_GAPS`] in the change that closes it: a test reads
//! `TODO/INDEX.md` and fails while a listed entry is done. `--strict`
//! refuses a gap where podvm can tell that the payload hits it, and warns
//! on each other gap, because podvm cannot tell what an arbitrary program
//! starts.
//!
//! Not in the list, because their entries closed them: a `dlopen` of a
//! guest path (T-1645) and a relative path (T-1643). A static payload
//! refuses before the exec without `--unsafe-host-paths` (T-1648), so it
//! is no gap of the loader family; a static child is.

use std::io::Read;

/// What podvm reads to tell that a payload hits a gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Probe {
    /// Nothing tells it: the gap is a warning on each run.
    Unknown,
    /// A command of the payload names a statically linked program of the
    /// image.
    StaticChild,
    /// A command of the payload runs on node.
    Libuv,
}

/// One gap of the loader family, and the entry that closes it.
pub struct GapDef {
    /// The entry that closes the gap.
    pub entry: &'static str,
    /// The sentence that a `--strict` line prints.
    pub text: &'static str,
    probe: Probe,
}

/// The open gaps of the loader family, as of the entries they name.
pub const LOADER_GAPS: &[GapDef] = &[
    GapDef {
        entry: "T-1646",
        text: "a child that the payload starts through fexecve, through execveat \
               with flags, or through system() or popen() inside libc, or a child \
               ELF of another machine, does not enter through the image's loader: \
               the kernel reads its loader or its #! interpreter on the host root \
               (TODO/interpose.md T-1646)",
        probe: Probe::Unknown,
    },
    GapDef {
        entry: "T-1652",
        text: "a statically linked child keeps the direct exec: its raw syscalls \
               pass the interposer, so its absolute paths resolve on the host and \
               a write lands on the host tree; T-1648 refuses only a static \
               top-level payload (TODO/enter.md T-1652, T-1651)",
        probe: Probe::StaticChild,
    },
    GapDef {
        entry: "T-1697",
        text: "libuv calls statx as a raw syscall, which passes the interposer: \
               node, npm and the other tools on node stat guest paths on the \
               host, so module resolution fails or reads host files \
               (TODO/interpose.md T-1697)",
        probe: Probe::Libuv,
    },
];

/// A gap of the selected family for one payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub entry: &'static str,
    pub text: &'static str,
    /// Why podvm can tell that this payload hits the gap. `None` where it
    /// cannot tell, which `--strict` warns on.
    pub hit: Option<String>,
}

/// The gaps of the loader family for this payload. `argv` is the argv of
/// the caller; `routed` tells that it named a `#!` script, which runs its
/// arguments as a command (an entrypoint script ends in `exec "$@"`).
pub fn loader_gaps(rootfs: &str, argv: &[String], routed: bool, path_dirs: &[String]) -> Vec<Gap> {
    let cmds = commands(argv, routed);
    LOADER_GAPS
        .iter()
        .map(|g| Gap {
            entry: g.entry,
            text: g.text,
            hit: match g.probe {
                Probe::Unknown => None,
                Probe::StaticChild => static_hit(rootfs, &cmds, path_dirs),
                Probe::Libuv => libuv_hit(&cmds),
            },
        })
        .collect()
}

/// At most this many commands are read from one payload.
const MAX_COMMANDS: usize = 64;

/// The shells whose `-c` string podvm reads.
const SHELLS: &[&str] = &["sh", "ash", "bash", "dash", "zsh", "ksh", "mksh"];

/// Words that come before the program name of a simple command.
const PREFIXES: &[&str] = &[
    "exec", "command", "env", "nohup", "nice", "time", "then", "do", "else", "elif", "if", "while",
    "until", "!", "{",
];

/// The tools that run on node, from their name.
const NODE_TOOLS: &[&str] = &["npm", "npx", "yarn", "pnpm", "corepack"];

fn basename(w: &str) -> &str {
    w.rsplit('/').next().unwrap_or(w)
}

/// The commands that a payload runs, as podvm can read them from its argv:
/// the argv itself; each simple command of a `-c` string of a shell; and,
/// for a followed `#!` script, its arguments. Bounded by [`MAX_COMMANDS`].
pub(crate) fn commands(argv: &[String], routed: bool) -> Vec<Vec<String>> {
    let mut out = vec![argv.to_vec()];
    if routed && argv.len() > 1 {
        out.push(argv[1..].to_vec());
    }
    let mut i = 0;
    while i < out.len() && out.len() < MAX_COMMANDS {
        if let Some(s) = shell_c(&out[i]) {
            let more = split_commands(&s);
            out.extend(more);
        }
        i += 1;
    }
    out.truncate(MAX_COMMANDS);
    out
}

/// The `-c` string where `cmd` is a shell given `-c`.
fn shell_c(cmd: &[String]) -> Option<String> {
    let words = program_words(cmd);
    let mut it = words.iter().map(String::as_str);
    let name = basename(it.next()?);
    let name = if name == "busybox" { it.next()? } else { name };
    if !SHELLS.contains(&name) {
        return None;
    }
    let mut seen_c = false;
    for w in it {
        if w.starts_with("--") {
            continue;
        }
        if w.starts_with('-') && w.len() > 1 {
            seen_c |= w.contains('c');
            continue;
        }
        return seen_c.then(|| w.to_string());
    }
    None
}

/// The words of a simple command from its program name on: assignments
/// and the words of [`PREFIXES`], with their options, come off the front.
fn program_words(cmd: &[String]) -> Vec<String> {
    let mut at = 0;
    while let Some(w) = cmd.get(at) {
        let assignment = w
            .split_once('=')
            .is_some_and(|(k, _)| !k.is_empty() && !k.contains('/') && !k.starts_with('-'));
        let prefix = PREFIXES.contains(&w.as_str());
        let option = at > 0 && w.starts_with('-') && PREFIXES.contains(&cmd[at - 1].as_str());
        if !(assignment || prefix || option) {
            break;
        }
        at += 1;
    }
    cmd[at.min(cmd.len())..].to_vec()
}

/// Split a shell string into its simple commands, as words. Quotes are
/// taken off and a backslash escapes one character; `;`, `&`, `|`, `(`,
/// `)`, a backquote and a newline end a command; `<` and `>` end a word.
/// This reads the common shapes, not the whole grammar: a shape that it
/// reads wrong only loses a hit, and the gap stays a warning.
fn split_commands(s: &str) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut cmd: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut chars = s.chars();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                word.push(c);
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                quote = Some(c);
                in_word = true;
            }
            '\\' => {
                word.extend(chars.next());
                in_word = true;
            }
            ' ' | '\t' | '<' | '>' | ';' | '&' | '|' | '(' | ')' | '`' | '\n' => {
                if in_word {
                    cmd.push(std::mem::take(&mut word));
                    in_word = false;
                }
                if !matches!(c, ' ' | '\t' | '<' | '>') && !cmd.is_empty() {
                    out.push(std::mem::take(&mut cmd));
                }
            }
            _ => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        cmd.push(word);
    }
    if !cmd.is_empty() {
        out.push(cmd);
    }
    out
}

/// A command that runs on node: a tool of [`NODE_TOOLS`], or `node` given
/// a script. `node -e`, `-p`, `--version` and `--help` read no module
/// path, so podvm cannot tell that they hit the gap.
fn libuv_hit(cmds: &[Vec<String>]) -> Option<String> {
    for cmd in cmds {
        let words = program_words(cmd);
        let Some(first) = words.first() else {
            continue;
        };
        let name = basename(first);
        if NODE_TOOLS.contains(&name) {
            return Some(format!(
                "`{name}` runs on node, whose module loader stats each path"
            ));
        }
        if name != "node" && name != "nodejs" {
            continue;
        }
        let args = &words[1..];
        let inline = [
            "-e",
            "--eval",
            "-p",
            "--print",
            "-v",
            "--version",
            "-h",
            "--help",
        ];
        if args.iter().any(|a| inline.contains(&a.as_str())) {
            continue;
        }
        if let Some(script) = args.iter().find(|a| !a.starts_with('-')) {
            return Some(format!(
                "`{name}` runs the script {script}, and its module loader stats each path"
            ));
        }
    }
    None
}

/// A command after the first that names a statically linked program of
/// the image. The first command is the payload, which the entry decided.
fn static_hit(rootfs: &str, cmds: &[Vec<String>], path_dirs: &[String]) -> Option<String> {
    for cmd in cmds.iter().skip(1) {
        let words = program_words(cmd);
        let Some(name) = words.first() else {
            continue;
        };
        if name.contains('$') {
            continue;
        }
        let Ok(path) = podvm_enter::ladder::resolve_payload(rootfs, name, path_dirs) else {
            continue;
        };
        if static_file(&path) == Some(true) {
            return Some(format!(
                "`{name}` is a statically linked program of the image"
            ));
        }
    }
    None
}

/// The head of a file that [`static_head`] reads.
const STATIC_HEAD: u64 = 64 * 1024;

fn static_file(path: &str) -> Option<bool> {
    let mut head = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(STATIC_HEAD)
        .read_to_end(&mut head)
        .ok()?;
    static_head(&head)
}

/// Whether the head of a 64-bit little-endian ELF executable holds no
/// `PT_INTERP`. `None` where the head cannot tell: not such an ELF file,
/// or a program header table past the head.
///
/// ⚠ Not `podvm_enter::abi::interp_of`: that reader answers "static" where
/// an offset falls past the bytes, which is right for a whole file and
/// wrong for a head. This one reads the program header types only, and
/// only where the whole table is inside the head.
pub(crate) fn static_head(h: &[u8]) -> Option<bool> {
    if h.len() < 64 || &h[..4] != b"\x7fELF" || h[4] != 2 || h[5] != 1 {
        return None;
    }
    let u16_at = |at: usize| u16::from_le_bytes([h[at], h[at + 1]]) as usize;
    if !matches!(u16_at(16), 2 | 3) || u16_at(54) != 56 {
        return None;
    }
    let mut off = [0u8; 8];
    off.copy_from_slice(&h[32..40]);
    let phoff = usize::try_from(u64::from_le_bytes(off)).ok()?;
    let end = phoff.checked_add(u16_at(56).checked_mul(56)?)?;
    if end > h.len() {
        return None;
    }
    let interp = (0..u16_at(56)).any(|i| {
        let at = phoff + i * 56;
        u32::from_le_bytes([h[at], h[at + 1], h[at + 2], h[at + 3]]) == 3
    });
    Some(!interp)
}

#[cfg(test)]
mod tests;
