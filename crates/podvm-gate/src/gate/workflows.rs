use super::*;

pub(super) fn bootstrap_components(body: &str) -> BTreeSet<String> {
    let mut have = BTreeSet::new();
    let mut at = 0;
    while let Some(found) = body[at..].find("bootstrap-env.sh") {
        let pos = at + found + "bootstrap-env.sh".len();
        let tail: String = body[pos..]
            .chars()
            .take_while(|c| !matches!(c, '\n' | '#' | '|' | '&' | ';'))
            .collect();
        for token in tail.split_whitespace() {
            if !token.starts_with('-') {
                have.insert(token.to_string());
            }
        }
        at = pos;
    }
    have
}

pub(super) struct WorkflowJob {
    pub(super) name: String,
    pub(super) body: String,
}

pub(super) fn workflow_jobs(text: &str) -> Vec<WorkflowJob> {
    let mut jobs: Vec<WorkflowJob> = Vec::new();
    let mut name: Option<String> = None;
    let mut buf: Vec<String> = Vec::new();
    let mut in_jobs = false;
    for line in text.split('\n') {
        let stripped: String = line.trim_end_matches([' ', '\t', '\r', '\n']).to_string();
        if stripped == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        if line.starts_with("  ") && !line[2..].starts_with(' ') && !line[2..].starts_with('\t') {
            let rest = line[2..].trim_end();
            if let Some(colon) = rest.find(':') {
                let candidate = &rest[..colon];
                if !candidate.is_empty()
                    && candidate
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                    && rest[colon + 1..].trim().is_empty()
                {
                    if let Some(active) = name.take() {
                        jobs.push(WorkflowJob {
                            name: active,
                            body: buf.join("\n"),
                        });
                    }
                    name = Some(candidate.to_string());
                    buf = Vec::new();
                    continue;
                }
            }
        }
        if name.is_some() {
            buf.push(line.to_string());
        }
    }
    if let Some(active) = name {
        jobs.push(WorkflowJob {
            name: active,
            body: buf.join("\n"),
        });
    }
    jobs
}

pub(super) fn uncommented(body: &str) -> String {
    body.split('\n')
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

// A docker exit-code declaration, not a use of one and not a bare number
// in a match arm or a message.
pub(super) fn find_exit_decls(text: &str) -> Vec<(String, String, usize)> {
    let mut out = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("pub const ") else {
            continue;
        };
        if !rest.starts_with("EXIT_") {
            continue;
        }
        let mut pos = "EXIT_".len();
        let bytes = rest.as_bytes();
        while pos < bytes.len() && (bytes[pos].is_ascii_uppercase() || bytes[pos] == b'_') {
            pos += 1;
        }
        if pos == "EXIT_".len() {
            continue;
        }
        let name = rest[..pos].to_string();
        let mut cursor = pos;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b':' {
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if !rest[cursor..].starts_with("i32") {
            continue;
        }
        cursor += "i32".len();
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
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == start {
            continue;
        }
        let value = rest[start..cursor].to_string();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b';' {
            continue;
        }
        out.push((name, value, index + 1));
    }
    out
}

fn match_unqualified_at(line: &str, at: usize) -> Option<(String, usize)> {
    let bytes = line.as_bytes();
    if at > 0 {
        let prev = bytes[at - 1];
        if is_word_byte(prev) || matches!(prev, b'/' | b'.' | b':' | b'-') {
            return None;
        }
    }
    for image in UNQUALIFIED_IMAGES {
        let (head, tail) = if image.ends_with('/') || image.ends_with(':') {
            let mut run = at + image.len();
            if image.ends_with('/') {
                let start = run;
                while run < bytes.len() {
                    let b = bytes[run];
                    if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-') {
                        run += 1;
                    } else {
                        break;
                    }
                }
                if run == start {
                    continue;
                }
            } else {
                let start = run;
                while run < bytes.len() {
                    let b = bytes[run];
                    if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-') {
                        run += 1;
                    } else {
                        break;
                    }
                }
                if run == start {
                    continue;
                }
            }
            (image, run)
        } else {
            if !line[at..].starts_with(image) {
                continue;
            }
            (image, at + image.len())
        };
        if !line[at..].starts_with(head) {
            continue;
        }
        let mut end = tail;
        if end < bytes.len() && bytes[end] == b':' {
            let mut tag = end + 1;
            while tag < bytes.len() {
                let b = bytes[tag];
                if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-') {
                    tag += 1;
                } else {
                    break;
                }
            }
            if tag > end + 1 {
                end = tag;
            }
        }
        if end < bytes.len() {
            let next = bytes[end];
            if is_word_byte(next) || matches!(next, b'/' | b'.' | b':' | b'-') {
                continue;
            }
        }
        return Some((line[at..end].to_string(), end));
    }
    None
}

pub(super) fn search_unqualified(line: &str, from: usize) -> Option<(String, usize, usize)> {
    let mut at = from;
    while at < line.len() {
        if let Some((matched, end)) = match_unqualified_at(line, at) {
            return Some((matched, at, end));
        }
        at = step_at(line, at);
    }
    None
}

pub(super) fn search_docker_io(line: &str) -> Option<String> {
    if let Some(found) = line.find("docker.io/") {
        let mut end = found + "docker.io/".len();
        let bytes = line.as_bytes();
        while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
            end += 1;
        }
        return Some(line[found..end].to_string());
    }
    None
}
pub(super) struct ParityRow {
    pub(super) verb: String,
    pub(super) spellings: Option<String>,
    status: String,
}

pub(super) struct ParityRowMatch {
    pub(super) row: ParityRow,
    pub(super) line: usize,
    pub(super) offset: usize,
}

pub(super) fn find_parity_rows(text: &str) -> Vec<ParityRowMatch> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = text[at..].find("Row") {
        let mut pos = at + found + "Row".len();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b'{' {
            at = pos;
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if !text[pos..].starts_with("verb:") {
            at = pos;
            continue;
        }
        pos += "verb:".len();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b'"' {
            at = pos;
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
        let verb = text[verb_start..pos].to_string();
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b',' {
            at = pos;
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if !text[pos..].starts_with("flag:") {
            at = pos;
            continue;
        }
        pos += "flag:".len();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let spellings;
        if text[pos..].starts_with("Option::None") {
            spellings = None;
            pos += "Option::None".len();
        } else if text[pos..].starts_with("Some(\"") {
            pos += "Some(\"".len();
            let list_start = pos;
            while pos < bytes.len() && bytes[pos] != b'"' {
                pos += 1;
            }
            if pos >= bytes.len() {
                break;
            }
            spellings = Some(text[list_start..pos].to_string());
            pos += 1;
            if !text[pos..].starts_with(')') {
                at = pos;
                continue;
            }
            pos += 1;
        } else {
            at = pos;
            continue;
        }
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b',' {
            at = pos;
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if !text[pos..].starts_with("status:") {
            at = pos;
            continue;
        }
        pos += "status:".len();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let status_start = pos;
        while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
            pos += 1;
        }
        if pos == status_start {
            at = pos;
            continue;
        }
        let status = text[status_start..pos].to_string();
        let line = text[..at + found].matches('\n').count() + 1;
        out.push(ParityRowMatch {
            row: ParityRow {
                verb,
                spellings,
                status,
            },
            line,
            offset: at + found,
        });
        at = pos;
    }
    out
}

pub(super) struct ParityAdmission {
    pub(super) admitted: BTreeMap<String, BTreeSet<String>>,
    pub(super) refused_verbs: BTreeSet<String>,
    pub(super) spellings: BTreeMap<String, BTreeSet<String>>,
}

pub(super) fn parity_admission(text: &str) -> ParityAdmission {
    let mut admitted: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut refused_verbs = BTreeSet::new();
    let mut spellings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row_match in find_parity_rows(text) {
        let row = row_match.row;
        if let Some(list) = row.spellings {
            for spelling in list.split(',') {
                spellings
                    .entry(rows_of(&row.verb).to_string())
                    .or_default()
                    .insert(spelling.trim().to_string());
            }
            if row.status == "NoneStatus" {
                continue;
            }
            for spelling in list.split(',') {
                admitted
                    .entry(rows_of(&row.verb).to_string())
                    .or_default()
                    .insert(spelling.trim().to_string());
            }
        } else if row.status == "NoneStatus" {
            refused_verbs.insert(row.verb.clone());
        }
    }
    ParityAdmission {
        admitted,
        refused_verbs,
        spellings,
    }
}
