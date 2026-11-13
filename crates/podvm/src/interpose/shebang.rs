//! The interpreter of a `#!` script, TODO/cli.md T-1757.
//!
//! A script is no ELF, so the interposer cannot be classified on it; the
//! loader that runs it is its interpreter, and `LD_PRELOAD` passes to that
//! interpreter through the exec. `docker-entrypoint.sh` of the node image
//! is such a script: without this, `-u` did not reach the payload.

use super::GuestRoot;

/// The guest path of the interpreter of the script at the host path
/// `path`, where the file starts with `#!`; `None` for any other file.
pub fn interpreter(path: &str) -> Option<String> {
    use std::io::Read;
    let mut head = [0u8; 256];
    let n = std::fs::File::open(path).ok()?.read(&mut head).ok()?;
    let line = head[..n].strip_prefix(b"#!")?;
    let end = line.iter().position(|b| *b == b'\n').unwrap_or(line.len());
    let text = std::str::from_utf8(&line[..end]).ok()?;
    let first = text.split_whitespace().next()?;
    first.starts_with('/').then(|| first.to_string())
}

/// The host path of the program that the loader runs for the host path
/// `path`: the interpreter of a `#!` script, resolved in the image, or
/// `path` itself.
pub fn effective<G: GuestRoot + ?Sized>(root: &G, path: String) -> String {
    match interpreter(&path).and_then(|i| podvm_enter::abi::resolve_in(root, &i).ok()) {
        Some(p) => p.display().to_string(),
        None => path,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_interpreter_of_a_script_is_its_first_word() {
        let dir = std::env::temp_dir().join(format!("podvm-shebang-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let s = dir.join("s");
        std::fs::write(&s, "#!/bin/sh -e\necho\n").unwrap();
        assert_eq!(
            super::interpreter(s.to_str().unwrap()).as_deref(),
            Some("/bin/sh")
        );
        std::fs::write(&s, "\x7fELF....").unwrap();
        assert_eq!(super::interpreter(s.to_str().unwrap()), None);
        std::fs::write(&s, "#!relative\n").unwrap();
        assert_eq!(super::interpreter(s.to_str().unwrap()), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
