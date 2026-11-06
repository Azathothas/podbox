use super::*;

pub(super) fn parity_cluster_values(text: &str) -> BTreeSet<(String, char)> {
    let mut out = BTreeSet::new();
    let Some(decl) = text.find("pub const CLUSTER_VALUES") else {
        return out;
    };
    let Some(close) = text[decl..].find("];") else {
        return out;
    };
    let slice = &text[decl..decl + close];
    let bytes = slice.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'(' {
            at += 1;
            continue;
        }
        let mut pos = at + 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b'"' {
            at += 1;
            continue;
        }
        pos += 1;
        let verb_start = pos;
        while pos < bytes.len() && bytes[pos] != b'"' {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let verb = slice[verb_start..pos].to_string();
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b',' {
            at += 1;
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos + 2 >= bytes.len()
            || bytes[pos] != b'\''
            || !bytes[pos + 1].is_ascii_alphanumeric()
            || bytes[pos + 2] != b'\''
        {
            at += 1;
            continue;
        }
        let member = bytes[pos + 1] as char;
        pos += 3;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b')' {
            at += 1;
            continue;
        }
        out.insert((verb, member));
        at = pos + 1;
    }
    out
}

pub(super) fn parity_cluster_boundary(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(decl) = text.find("const CLUSTER_BOUNDARY") else {
        return out;
    };
    let Some(close) = text[decl..].find("];") else {
        return out;
    };
    let slice = &text[decl..decl + close];
    let bytes = slice.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'"' {
            at += 1;
            continue;
        }
        let mut pos = at + 1;
        while pos < bytes.len() && bytes[pos].is_ascii_lowercase() {
            pos += 1;
        }
        if pos > at + 1 && pos < bytes.len() && bytes[pos] == b'"' {
            out.insert(slice[at + 1..pos].to_string());
            at = pos + 1;
        } else {
            at += 1;
        }
    }
    out
}

// Tokenise a shell command the way the reference port does: quoting keeps
// a template one value, and a line that does not parse that way falls back
// to a plain split, which still keeps every dash-token intact.
fn shlex_split(cmd: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut started = false;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for ch in cmd.chars() {
        if escaped {
            if ch != '\n' {
                cur.push(ch);
                started = true;
            }
            escaped = false;
            continue;
        }
        if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                cur.push(ch);
                started = true;
            }
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
            } else if ch == '\\' {
                escaped = true;
            } else {
                cur.push(ch);
                started = true;
            }
            continue;
        }
        match ch {
            '\\' => {
                escaped = true;
            }
            '\'' => {
                in_single = true;
                started = true;
            }
            '"' => {
                in_double = true;
                started = true;
            }
            ' ' | '\t' | '\r' | '\n' => {
                if started {
                    tokens.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            _ => {
                cur.push(ch);
                started = true;
            }
        }
    }
    if escaped || in_single || in_double {
        return None;
    }
    if started {
        tokens.push(cur);
    }
    Some(tokens)
}

pub(super) struct ProveCommand {
    pub(super) verb: String,
    pub(super) flags: Vec<String>,
    pub(super) bad: Option<String>,
}

fn match_prove_inv(line: &str, at: usize) -> Option<(String, usize)> {
    let bytes = line.as_bytes();
    if !line[at..].starts_with("podvm") {
        return None;
    }
    let mut pos = at + "podvm".len();
    let mut spaces = 0;
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t' | b'\r' | b'\n') {
        pos += 1;
        spaces += 1;
    }
    if spaces == 0 {
        return None;
    }
    for head in ["image", "system"] {
        if line[pos..].starts_with(head) {
            let mut sub = pos + head.len();
            let mut sub_spaces = 0;
            while sub < bytes.len() && matches!(bytes[sub], b' ' | b'\t' | b'\r' | b'\n') {
                sub += 1;
                sub_spaces += 1;
            }
            if sub_spaces > 0 && sub < bytes.len() && bytes[sub].is_ascii_alphabetic() {
                let mut end = sub + 1;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric()
                        || bytes[end] == b'_'
                        || bytes[end] == b'-')
                {
                    end += 1;
                }
                return Some((line[pos..end].to_string(), end));
            }
        }
    }
    if pos < bytes.len() && bytes[pos].is_ascii_alphabetic() {
        let mut end = pos + 1;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
        {
            end += 1;
        }
        return Some((line[pos..end].to_string(), end));
    }
    None
}

fn prove_term_at(tail: &str) -> usize {
    let bytes = tail.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() {
        match bytes[pos] {
            b'|' | b';' | b'`' | b'&' => return pos,
            b'$' if pos + 1 < bytes.len() && bytes[pos + 1] == b'(' => {
                return pos;
            }
            _ => pos += 1,
        }
    }
    bytes.len()
}

pub(super) fn prove_commands(
    block: &[String],
    spellings: &BTreeMap<String, BTreeSet<String>>,
    cluster_values: &BTreeSet<(String, char)>,
    boundary: &BTreeSet<String>,
) -> Vec<ProveCommand> {
    let mut out = Vec::new();
    for line in block {
        let mut at = 0;
        while at < line.len() {
            let Some(found) = line[at..].find("podvm") else {
                break;
            };
            let pos = at + found;
            let Some((verb, end)) = match_prove_inv(line, pos) else {
                at = pos + 1;
                continue;
            };
            let before = line[..pos].trim_end();
            if before.ends_with('!') {
                at = end;
                continue;
            }
            let under = rows_of(&verb).to_string();
            let bounded = boundary.contains(&under);
            let tail = &line[end..];
            let term = prove_term_at(tail);
            let cmd = &tail[..term];
            let tokens = shlex_split(cmd)
                .unwrap_or_else(|| cmd.split_whitespace().map(str::to_string).collect());
            let mut flags = Vec::new();
            let mut bad = None;
            let mut prev_dash = !bounded;
            for tok in &tokens {
                if tok.starts_with('-') && tok.chars().count() > 1 {
                    let body: String = tok.chars().skip(1).collect();
                    let first = body.chars().next().unwrap_or('\0');
                    if tok.chars().count() > 2
                        && !tok.starts_with("--")
                        && first.is_alphanumeric()
                        && body.is_ascii()
                    {
                        for member in body.chars() {
                            let one = format!("-{member}");
                            if cluster_values.contains(&(under.clone(), member)) {
                                flags.push(one);
                                break;
                            }
                            let named = spellings.get(&under).is_some_and(|set| set.contains(&one));
                            if !member.is_alphanumeric() || !named {
                                bad = Some(one);
                                break;
                            }
                            flags.push(one);
                        }
                        if bad.is_some() {
                            break;
                        }
                        prev_dash = false;
                        continue;
                    }
                    let flag = tok.split('=').next().unwrap_or(tok).to_string();
                    flags.push(flag);
                    prev_dash = true;
                } else if prev_dash {
                    prev_dash = false;
                } else if bounded {
                    break;
                }
            }
            out.push(ProveCommand { verb, flags, bad });
            at = end;
        }
    }
    out
}
pub(super) enum RunError {
    Spawn,
    Timeout,
}

pub(super) fn run_with_timeout(
    program: &str,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> Result<(Option<i32>, String, String), RunError> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| RunError::Spawn)?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code();
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stdout = String::from_utf8_lossy(&buf).into_owned();
                }
                if let Some(mut pipe) = child.stderr.take() {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf);
                    stderr = String::from_utf8_lossy(&buf).into_owned();
                }
                return Ok((code, stdout, stderr));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(RunError::Timeout);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return Err(RunError::Spawn),
        }
    }
}

pub(super) fn which(program: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        for extension in ["exe", "bat", "cmd"] {
            let with_ext = dir.join(format!("{program}.{extension}"));
            if with_ext.is_file() {
                return Some(with_ext);
            }
        }
    }
    None
}

pub(super) fn tracked_files(root: &Path) -> Option<BTreeSet<String>> {
    let output = Command::new("git")
        .args(["-C", &root.to_string_lossy(), "ls-files", "-z"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut files = BTreeSet::new();
    for part in output.stdout.split(|b| *b == 0) {
        if part.is_empty() {
            continue;
        }
        files.insert(String::from_utf8_lossy(part).into_owned());
    }
    Some(files)
}

pub(super) fn is_ours(rel: &str) -> bool {
    if rel.starts_with(CORPUS_PREFIX) {
        return false;
    }
    for prefix in VERBATIM_PREFIXES {
        if rel.starts_with(prefix) {
            return false;
        }
    }
    true
}

pub(super) fn has_source_suffix(rel: &str) -> bool {
    SOURCE_SUFFIXES.iter().any(|suffix| rel.ends_with(suffix))
}

// The first word of a status value, stars stripped, exactly as the
// reference reader takes it. A value with no word is invalid rather than
// a crash.
pub(super) fn status_word(value: &str) -> Option<String> {
    let stripped = value.trim_matches('*');
    stripped.split_whitespace().next().map(str::to_string)
}
