use super::*;

pub(super) fn git(root: &Path, args: &[&str]) -> Result<ChildOut, String> {
    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run_bounded("git", &owned, root, &[], 120)
}

pub(super) struct Plant {
    pub(super) root: PathBuf,
    pub(super) gate: PathBuf,
    pub(super) backup: PathBuf,
    pub(super) base_out: String,
    pub(super) plants_caught: u32,
    pub(super) plants_missed: u32,
    pub(super) controls_quiet: u32,
    pub(super) controls_fired: u32,
    pub(super) transcript: Option<PathBuf>,
}

impl Plant {
    pub(super) fn backup_all(&self) -> Result<(), String> {
        for file in FILES {
            let dest = self.backup.join(file);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
            }
            fs::copy(self.root.join(file), &dest)
                .map_err(|e| format!("could not back up {file}: {e}"))?;
        }
        Ok(())
    }

    // Restore undoes every kind of plant, not only an edit to a file in the
    // list. A case that stages a scratch file must unstage it too, or the
    // leftover makes every later case measure a dirty tree. The duplicate
    // experiment script is found by name rather than passed in: only the
    // plant creates that name, so finding it here cannot touch real work,
    // and a run killed mid-case still scrubs on the next restore.
    pub(super) fn restore_all(&self, scratch: &str) {
        for file in FILES {
            let _ = fs::copy(self.backup.join(file), self.root.join(file));
        }
        let _ = git(
            &self.root,
            &[
                "rm",
                "-q",
                "--cached",
                "-f",
                "--ignore-unmatch",
                "-r",
                scratch,
            ],
        );
        let _ = fs::remove_dir_all(self.root.join(scratch));
        if let Ok(dir) = fs::read_dir(self.root.join("experiments")) {
            for entry in dir.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with("-plantdup.sh") {
                    let rel = format!("experiments/{name}");
                    let _ = git(
                        &self.root,
                        &["rm", "-q", "--cached", "-f", "--ignore-unmatch", &rel],
                    );
                    let _ = fs::remove_file(self.root.join(&rel));
                }
            }
        }
    }

    // Hash the whole working state, not just the listed files. A plant that
    // creates a new file changes nothing in the list, so a list-only hash
    // would report "did not land" on a plant that did.
    pub(super) fn hashes(&self) -> String {
        let mut out = String::new();
        for file in FILES {
            match git(&self.root, &["hash-object", file]) {
                Ok(child) => out.push_str(child.combined.trim_end()),
                Err(_) => out.push_str("missing"),
            }
            out.push('\n');
        }
        match git(&self.root, &["status", "--porcelain"]) {
            Ok(child) => out.push_str(&child.combined),
            Err(_) => out.push_str("no-status"),
        }
        out
    }

    pub(super) fn run_gate(&self, extra_env: &[(String, String)]) -> Result<ChildOut, String> {
        let gate = self.gate.to_string_lossy().into_owned();
        run_bounded(&gate, &[], &self.root, extra_env, GATE_TIMEOUT_SECS)
    }

    // A plant case: the expected substring is the planted defect's own
    // message. A case that matched only the exit code would pass on any
    // unrelated failure.
    pub(super) fn case_plant(
        &mut self,
        name: &str,
        want: &str,
        scratch: &str,
        mutate: &dyn Fn() -> bool,
    ) {
        let before = self.hashes();
        mutate();
        let after = self.hashes();
        // Guard 1: did the mutation land?
        if before == after {
            println!("  MISS {name:<34} the mutation did not land; the files are byte-identical");
            self.plants_missed += 1;
            self.restore_all(scratch);
            return;
        }
        let (rc, out) = match self.run_gate(&[]) {
            Ok(child) => (child.code.unwrap_or(1), child.combined),
            Err(e) => {
                println!("  MISS {name:<34} the gate could not run: {e}");
                self.plants_missed += 1;
                self.restore_all(scratch);
                return;
            }
        };
        self.restore_all(scratch);
        if rc == 0 {
            println!("  MISS {name:<34} the gate stayed green");
            self.plants_missed += 1;
            return;
        }
        // The finding, not the exit code: this message, and not on the clean
        // tree.
        if out.contains(want) {
            if self.base_out.contains(want) {
                println!("  MISS {name:<34} that message is already on the clean tree");
                self.plants_missed += 1;
            } else {
                println!("  ok   {name:<34} red, and it named it");
                self.plants_caught += 1;
            }
        } else {
            println!("  MISS {name:<34} red for another reason, not this one");
            for line in out.lines().skip(1).take(3) {
                println!("         {line}");
            }
            self.plants_missed += 1;
        }
    }

    // A control changes something the gate must not object to. It proves the
    // gate is discriminating rather than merely noisy.
    pub(super) fn case_control(&mut self, name: &str, scratch: &str, mutate: &dyn Fn()) {
        mutate();
        let (rc, out) = match self.run_gate(&[]) {
            Ok(child) => (child.code.unwrap_or(1), child.combined),
            Err(e) => {
                println!("  FIRE {name:<34} the gate could not run: {e}");
                self.controls_fired += 1;
                self.restore_all(scratch);
                return;
            }
        };
        self.restore_all(scratch);
        if rc == 0 {
            println!("  ok   {name:<34} quiet, as a control must be");
            self.controls_quiet += 1;
        } else {
            println!("  FIRE {name:<34} the gate objected to a legitimate edit");
            for line in out.lines().skip(1).take(3) {
                println!("         {line}");
            }
            self.controls_fired += 1;
        }
    }
}

// Small rewrites, each mirroring one shell substitution exactly: without a
// `g` flag the shell replaces the first match on every line, so these map
// over lines and replace once per line.
pub(super) fn sub_first_per_line(text: &str, find: &str, repl: &str) -> String {
    text.split('\n')
        .map(|line| {
            if line.contains(find) {
                line.replacen(find, repl, 1)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn read_file(root: &Path, file: &str) -> String {
    fs::read_to_string(root.join(file)).unwrap_or_default()
}

pub(super) fn write_file(root: &Path, file: &str, text: &str) {
    let _ = fs::write(root.join(file), text);
}

pub(super) fn append_file(root: &Path, file: &str, text: &str) {
    let mut old = read_file(root, file);
    old.push_str(text);
    write_file(root, file, &old);
}

// The first line starting with `prefix` gets that prefix swapped, once.
pub(super) fn swap_first_prefixed_line(root: &Path, file: &str, prefix: &str, replacement: &str) {
    let text = read_file(root, file);
    let mut done = false;
    let out: Vec<String> = text
        .split('\n')
        .map(|line| {
            if !done && line.starts_with(prefix) {
                done = true;
                format!("{replacement}{}", &line[prefix.len()..])
            } else {
                line.to_string()
            }
        })
        .collect();
    write_file(root, file, &out.join("\n"));
}

// A bare `name <digits>` reading line gets a new value, whole-line
// anchored like the shell substitution. A line carrying anything past the
// digits is left alone: planting into the wrong shape is the wrong defect.
pub(super) fn set_bare_reading(root: &Path, file: &str, name: &str, value: &str) {
    let text = read_file(root, file);
    let head = format!("{name} ");
    let out: Vec<String> = text
        .split('\n')
        .map(|line| {
            if let Some(rest) = line.strip_prefix(&head)
                && !rest.is_empty()
                && rest.bytes().all(|b| b.is_ascii_digit())
            {
                return format!("{head}{value}");
            }
            line.to_string()
        })
        .collect();
    write_file(root, file, &out.join("\n"));
}

// The planted citations are assembled at run time, never written out here.
// The gate reads this file like any other, so a literal bad citation in
// this source is a permanent gate failure rather than a plant. The same
// holds for the exit-code declaration below and for both ceiling numbers:
// each is read out of the file that owns it, at run time.
// The past-EOF citation names the gate's own source, read at run time.
// The retired script is gone, so citing it reports "does not exist"
// rather than the over-long citation these cases must plant.
pub(super) fn gate_source_name() -> String {
    ["crates/podvm-gate/src/main", ".rs"].concat()
}

pub(super) fn bad_bare_name() -> String {
    ["TODO/no-such-", "category.md"].concat()
}

pub(super) fn exit_declaration() -> String {
    ["pub", " const EXIT_", "RUNTIME_ERROR: i32 = 125;"].concat()
}

pub(super) fn ceiling_from_file(root: &Path, file: &str, name: &str) -> Option<String> {
    for line in read_file(root, file).split('\n') {
        if let Some(rest) = line.strip_prefix(name)
            && let Some(digits) = rest.strip_prefix('=')
            && !digits.is_empty()
            && digits.bytes().all(|b| b.is_ascii_digit())
        {
            return Some(digits.to_string());
        }
    }
    None
}

// The first numbered experiment script, alphabetically, like the shell glob.
pub(super) fn taken_experiment(root: &Path) -> Option<String> {
    let dir = fs::read_dir(root.join("experiments")).ok()?;
    let mut names: Vec<String> = Vec::new();
    for entry in dir.flatten() {
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    for name in names {
        if !name.ends_with(".sh") {
            continue;
        }
        let stem = &name[..name.len() - 3];
        if let Some(dash) = stem.find('-') {
            let (num, rest) = stem.split_at(dash);
            if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) && rest.len() > 1 {
                return Some(num.to_string());
            }
        }
    }
    None
}

// The cargo config names a wrapper program; that program says which
// bootstrap component installs it. Neither word is written out here, for the
// same reason the ceiling is not: naming it here would be a second
// declaration of the value the check exists to keep in one place.
pub(super) fn cc_component(root: &Path) -> Option<(String, String)> {
    let config = read_file(root, ".cargo/config.toml");
    let mut wrapper: Option<String> = None;
    for line in config.split('\n') {
        let trimmed = line.trim_start();
        if !(trimmed.starts_with("CC_")) {
            continue;
        }
        let Some(eq) = line.find('=') else {
            continue;
        };
        let Some(open) = line[eq..].find('{') else {
            continue;
        };
        if line[eq..eq + open].contains('}') {
            continue;
        }
        let after = &line[eq + open..];
        let Some(value_at) = after.find('"') else {
            continue;
        };
        let rest = &after[value_at + 1..];
        let Some(end) = rest.find('"') else {
            continue;
        };
        wrapper = Some(rest[..end].to_string());
        break;
    }
    let wrapper = wrapper?;
    let body = read_file(root, &wrapper);
    if body.is_empty() {
        return None;
    }
    let marker = "bootstrap-env.sh";
    let mut at = 0;
    while let Some(found) = body[at..].find(marker) {
        let mut pos = at + found + marker.len();
        let bytes = body.as_bytes();
        let base = at + found;
        if pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
            while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                pos += 1;
            }
            let start = pos;
            if pos < bytes.len() && bytes[pos].is_ascii_lowercase() {
                pos += 1;
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_lowercase()
                        || bytes[pos].is_ascii_digit()
                        || bytes[pos] == b'-')
                {
                    pos += 1;
                }
                return Some((wrapper, body[start..pos].to_string()));
            }
        }
        at = base + 1;
    }
    None
}

// Strip the component from every bootstrap line, the way the shell's
// unguided substitution does: the longest match wins, so the last ` comp`
// on the line goes.
pub(super) fn strip_component_line(line: &str, component: &str) -> String {
    let needle = format!(" {component}");
    if let Some(boot) = line.find("bootstrap-env.sh")
        && let Some(rel) = line[boot..].rfind(&needle)
    {
        let at = boot + rel;
        let mut out = line.to_string();
        out.replace_range(at..at + needle.len(), "");
        return out;
    }
    line.to_string()
}
