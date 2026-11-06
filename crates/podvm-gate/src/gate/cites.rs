use super::*;

pub(super) fn find_cites(line: &str) -> Vec<Cite> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < line.len() {
        if let Some((cite, end)) = match_cite_at(line, at) {
            out.push(cite);
            at = end.max(at + 1);
        } else {
            at = step_at(line, at);
        }
    }
    out
}

// A path named in backticks with no line number. The shape the first
// defect took, and a path-and-line matcher cannot see it.
pub(super) fn find_bares(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'`' {
            at = step_at(line, at);
            continue;
        }
        let mut found = None;
        for prefix in CITE_PREFIXES {
            let head = format!("`{prefix}/");
            if line[at..].starts_with(&head) {
                let mut run = at + head.len();
                while run < bytes.len() {
                    let b = bytes[run];
                    if b.is_ascii_alphanumeric()
                        || matches!(b, b'.' | b'_' | b'+' | b'@' | b'/' | b'-')
                    {
                        run += 1;
                    } else {
                        break;
                    }
                }
                if run < bytes.len() && bytes[run] == b'`' {
                    found = Some((line[at + 1..run].to_string(), run + 1));
                }
                break;
            }
        }
        if let Some((cited, next)) = found {
            out.push(cited);
            at = next;
        } else {
            at = step_at(line, at);
        }
    }
    out
}

pub(super) struct Link {
    pub(super) href: String,
}

pub(super) fn find_links(line: &str) -> Vec<Link> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'[' {
            at = step_at(line, at);
            continue;
        }
        let Some(close) = line[at..].find(']') else {
            break;
        };
        let paren = at + close + 1;
        if paren >= bytes.len() || bytes[paren] != b'(' {
            at = step_at(line, at);
            continue;
        }
        let rest = &line[paren + 1..];
        let Some(end) = rest.find(')') else {
            break;
        };
        let segment = &rest[..end];
        if segment.is_empty() || segment.starts_with(')') || segment.starts_with('#') {
            at = paren + 1;
            continue;
        }
        let href = segment.split('#').next().unwrap_or("").trim();
        if href.is_empty() {
            at = paren + 1;
            continue;
        }
        out.push(Link {
            href: href.to_string(),
        });
        at = paren + 1 + end + 1;
    }
    out
}

// Nothing under these may be tracked. The results directory is the
// exception and is the evidence, so it is deliberately not a scratch
// prefix: every scratch directory is a dot directory under experiments.
pub(super) fn is_scratch(rel: &str) -> bool {
    if let Some(rest) = rel.strip_prefix("experiments/.") {
        return !rest.is_empty() && !rest.starts_with('/') && rest.contains('/');
    }
    if rel.starts_with("target/") {
        return true;
    }
    if let Some(rest) = rel.strip_prefix("crates/") {
        if let Some(slash) = rest.find('/') {
            if &rest[slash..] == "/target/" {
                return true;
            }
            if rest[slash..].starts_with("/target/") {
                return true;
            }
        }
        return false;
    }
    rel.contains("/__pycache__/")
}

// Split text into lines the way the reference reader splits them: a
// trailing newline ends the last line rather than starting a new one.
pub(super) fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut parts: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') && parts.last().is_some_and(|last| last.is_empty()) {
        parts.pop();
    }
    parts
}

// A whole-line declaration of the form NAME digits, blank-separated.
fn match_spaced_decl(line: &str, name: &str) -> Option<String> {
    let rest = line.strip_prefix(name)?.strip_prefix(' ')?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(rest.to_string())
}

// A whole-line declaration of the form NAME=digits, multiline search.
fn match_decl_line(line: &str, name: &str) -> Option<String> {
    let rest = line.strip_prefix(name)?.strip_prefix('=')?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(rest.to_string())
}

pub(super) fn search_decl(text: &str, name: &str) -> Option<String> {
    split_lines(text)
        .iter()
        .find_map(|line| match_decl_line(line, name))
}

pub(super) fn search_spaced_decl(text: &str, name: &str) -> Option<String> {
    split_lines(text)
        .iter()
        .find_map(|line| match_spaced_decl(line, name))
}

pub(super) struct Experiment {
    pub(super) num: String,
    pub(super) name: String,
}

fn match_experiment_at(text: &str, at: usize) -> Option<(Experiment, usize)> {
    let rest = text.get(at..)?;
    let after = rest.strip_prefix("experiments/")?;
    let mut digits = 0;
    while after
        .as_bytes()
        .get(digits)
        .is_some_and(|b| b.is_ascii_digit())
    {
        digits += 1;
    }
    if digits == 0 || after.as_bytes().get(digits) != Some(&b'-') {
        return None;
    }
    let num = after[..digits].to_string();
    let name_start = digits + 1;
    let name_bytes = after.as_bytes();
    let mut run = name_start;
    while run < name_bytes.len() {
        let b = name_bytes[run];
        if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'+' | b'-') {
            run += 1;
        } else {
            break;
        }
    }
    let mut split = run;
    while split > name_start {
        if after[split..].starts_with(".sh") {
            let name = after[name_start..split].to_string();
            if !name.is_empty() {
                let end = at + "experiments/".len() + split + ".sh".len();
                return Some((Experiment { num, name }, end));
            }
            break;
        }
        split -= 1;
    }
    None
}

pub(super) fn is_experiment_file(rel: &str) -> Option<Experiment> {
    let (exp, end) = match_experiment_at(rel, 0)?;
    if end == rel.len() {
        Some(exp)
    } else {
        None
    }
}

pub(super) fn find_experiments(text: &str) -> Vec<(Experiment, usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < text.len() {
        if let Some((exp, end)) = match_experiment_at(text, at) {
            let line = text[..at].matches('\n').count() + 1;
            out.push((exp, line, at));
            at = end.max(at + 1);
        } else {
            at = step_at(text, at);
        }
    }
    out
}

// The toolchain a cargo build needs, derived in two hops and hard-coded
// in neither. The cargo config names a program; that program says which
// bootstrap component installs it.
fn toolchain_wrappers(config: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in config.split('\n') {
        let mut pos = 0;
        let bytes = line.as_bytes();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let mut matched = None;
        for key in ["CC", "AR", "LD", "CXX"] {
            if line[pos..].starts_with(key) && line[pos + key.len()..].starts_with('_') {
                matched = Some(key.len());
                break;
            }
        }
        let Some(key_len) = matched else {
            continue;
        };
        let mut cursor = pos + key_len + 1;
        while cursor < bytes.len()
            && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
        {
            cursor += 1;
        }
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'{' {
            continue;
        }
        let after = &line[cursor..];
        let Some(value_at) = after.find("value") else {
            continue;
        };
        if after[..value_at].contains('}') {
            continue;
        }
        let mut val = cursor + value_at + "value".len();
        while val < bytes.len() && bytes[val].is_ascii_whitespace() {
            val += 1;
        }
        if val >= bytes.len() || bytes[val] != b'=' {
            continue;
        }
        val += 1;
        while val < bytes.len() && bytes[val].is_ascii_whitespace() {
            val += 1;
        }
        if val >= bytes.len() || bytes[val] != b'"' {
            continue;
        }
        val += 1;
        let start = val;
        while val < bytes.len() && bytes[val] != b'"' {
            val += 1;
        }
        if val >= bytes.len() {
            continue;
        }
        out.push(line[start..val].to_string());
    }
    out
}

fn wrapper_components(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = body[at..].find("bootstrap-env.sh") {
        let mut pos = at + found + "bootstrap-env.sh".len();
        let bytes = body.as_bytes();
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
                out.push(body[start..pos].to_string());
            }
        }
        at = at + found + 1;
    }
    out
}

pub(super) fn cargo_toolchain_components(root: &Path) -> BTreeMap<String, String> {
    let mut comps = BTreeMap::new();
    let Ok(config) = read_text(&root.join(CARGO_CONFIG)) else {
        return comps;
    };
    for wrapper in toolchain_wrappers(&config) {
        let Ok(body) = read_text(&root.join(&wrapper)) else {
            continue;
        };
        for component in wrapper_components(&body) {
            comps.entry(component).or_insert(wrapper.clone());
        }
    }
    comps
}

pub(super) fn is_cargo_call(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(found) = text[at..].find("cargo") {
        let pos = at + found;
        let before_ok = pos == 0
            || (!is_word_byte(bytes[pos - 1]) && !matches!(bytes[pos - 1], b'/' | b'.' | b'-'));
        if before_ok {
            let mut cursor = pos + "cargo".len();
            let mut spaces = 0;
            while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n') {
                cursor += 1;
                spaces += 1;
            }
            if spaces > 0 && cursor < bytes.len() && bytes[cursor].is_ascii_lowercase() {
                return true;
            }
        }
        at = pos + 1;
    }
    false
}

pub(super) fn find_repo_scripts(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at + 1 < bytes.len() {
        if bytes[at] == b'.' && bytes[at + 1] == b'/' {
            let mut matched = None;
            for prefix in ["scripts/", "experiments/"] {
                if text[at + 2..].starts_with(prefix) {
                    matched = Some(prefix.len());
                    break;
                }
            }
            if let Some(prefix_len) = matched {
                let script_start = at + 2;
                let mut run = script_start + prefix_len;
                while run < bytes.len() {
                    let b = bytes[run];
                    if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'/' | b'-') {
                        run += 1;
                    } else {
                        break;
                    }
                }
                let mut split = run;
                let mut done = false;
                while split > script_start + prefix_len {
                    if text[split..].starts_with(".sh") {
                        out.push(text[script_start..split + 3].to_string());
                        at = split + 3;
                        done = true;
                        break;
                    }
                    split -= 1;
                }
                if done {
                    continue;
                }
            }
        }
        at = step_at(text, at);
    }
    out.sort();
    out.dedup();
    out
}
