use super::*;

#[derive(Debug, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

fn parse_json(text: &str) -> Option<(JsonValue, usize)> {
    let bytes = text.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos >= bytes.len() {
        return None;
    }
    match bytes[pos] {
        b'n' if text[pos..].starts_with("null") => Some((JsonValue::Null, pos + 4)),
        b't' if text[pos..].starts_with("true") => Some((JsonValue::Bool(true), pos + 4)),
        b'f' if text[pos..].starts_with("false") => Some((JsonValue::Bool(false), pos + 5)),
        b'"' => {
            let mut out = String::new();
            let mut cursor = pos + 1;
            while cursor < bytes.len() {
                match bytes[cursor] {
                    b'"' => {
                        return Some((JsonValue::Text(out), cursor + 1));
                    }
                    b'\\' if cursor + 1 < bytes.len() => {
                        let esc = bytes[cursor + 1];
                        match esc {
                            b'"' => out.push('"'),
                            b'\\' => out.push('\\'),
                            b'/' => out.push('/'),
                            b'n' => out.push('\n'),
                            b't' => out.push('\t'),
                            b'r' => out.push('\r'),
                            _ => {
                                out.push('\\');
                                out.push(esc as char);
                            }
                        }
                        cursor += 2;
                    }
                    _ => {
                        out.push(bytes[cursor] as char);
                        cursor += 1;
                    }
                }
            }
            None
        }
        b'[' => {
            let mut items = Vec::new();
            let mut cursor = pos + 1;
            loop {
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && bytes[cursor] == b']' {
                    return Some((JsonValue::Array(items), cursor + 1));
                }
                let (value, next) = parse_json(&text[cursor..])?;
                items.push(value);
                cursor += next;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && bytes[cursor] == b',' {
                    cursor += 1;
                    continue;
                }
                if cursor < bytes.len() && bytes[cursor] == b']' {
                    return Some((JsonValue::Array(items), cursor + 1));
                }
                return None;
            }
        }
        b'{' => {
            let mut fields = Vec::new();
            let mut cursor = pos + 1;
            loop {
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && bytes[cursor] == b'}' {
                    return Some((JsonValue::Object(fields), cursor + 1));
                }
                let (key, next) = parse_json(&text[cursor..])?;
                let JsonValue::Text(name) = key else {
                    return None;
                };
                cursor += next;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor >= bytes.len() || bytes[cursor] != b':' {
                    return None;
                }
                cursor += 1;
                let (value, next) = parse_json(&text[cursor..])?;
                fields.push((name, value));
                cursor += next;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && bytes[cursor] == b',' {
                    cursor += 1;
                    continue;
                }
                if cursor < bytes.len() && bytes[cursor] == b'}' {
                    return Some((JsonValue::Object(fields), cursor + 1));
                }
                return None;
            }
        }
        _ => {
            let mut cursor = pos;
            while cursor < bytes.len()
                && !bytes[cursor].is_ascii_whitespace()
                && !matches!(bytes[cursor], b',' | b']' | b'}')
            {
                cursor += 1;
            }
            if cursor == pos {
                return None;
            }
            Some((JsonValue::Number(text[pos..cursor].to_string()), cursor))
        }
    }
}

fn json_field<'a>(fields: &'a [(String, JsonValue)], name: &str) -> Option<&'a JsonValue> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

// Render a ledger item the way the reference output renders one.
fn render_json_item(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "None".to_string(),
        JsonValue::Bool(true) => "True".to_string(),
        JsonValue::Bool(false) => "False".to_string(),
        JsonValue::Number(raw) => raw.clone(),
        JsonValue::Text(text) => text.clone(),
        JsonValue::Array(items) => {
            let inner: Vec<String> = items.iter().map(render_json_item).collect();
            format!("[{}]", inner.join(", "))
        }
        JsonValue::Object(fields) => {
            let inner: Vec<String> = fields
                .iter()
                .map(|(key, val)| format!("'{}': {}", key, render_json_item(val)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}

pub(super) fn check_post_task_cleanup(gate: &mut Gate, root: &Path) {
    let rules = root.join("TODO").join("RULES.md");
    let Ok(rules_text) = read_text(&rules) else {
        gate.err(
            "TODO/RULES.md",
            "does not exist, and check 29 holds the \
             post-task cleanup procedure it must carry"
                .to_string(),
        );
        return;
    };
    for anchor in RULE_ANCHORS {
        gate.bump("post_task_cleanup");
        if !rules_text.contains(anchor) {
            gate.err(
                "TODO/RULES.md",
                format!(
                    "names no post-task cleanup procedure: {} is gone, and \
                     storage on the lane host is a fixed allowance",
                    quote_repr(anchor)
                ),
            );
        }
    }
    let Some(tool) = which("wsl-toolkit") else {
        return;
    };
    let tool_str = tool.to_string_lossy().into_owned();
    let ran = run_with_timeout(
        &tool_str,
        &["--instance", "podvm", "gc", "--json"],
        root,
        Duration::from_millis(180000),
    );
    let (code, stdout, stderr) = match ran {
        Err(RunError::Spawn) => {
            gate.err(
                "wsl-toolkit",
                "the lane-job ledger could not be read: could not run the ledger command"
                    .to_string(),
            );
            return;
        }
        Err(RunError::Timeout) => {
            gate.err(
                "wsl-toolkit",
                "the lane-job ledger could not be read: the ledger command timed out".to_string(),
            );
            return;
        }
        Ok(done) => done,
    };
    if code != Some(0) {
        let detail: String = stderr.trim().chars().take(120).collect();
        gate.err(
            "wsl-toolkit",
            format!(
                "the lane-job ledger could not be read: exit {}: {detail}",
                code.unwrap_or(-1)
            ),
        );
        return;
    }
    let report_text = stdout;
    let Some((report, _)) = parse_json(&report_text) else {
        gate.err(
            "wsl-toolkit",
            "the lane-job ledger is not JSON: no JSON value found".to_string(),
        );
        return;
    };
    let JsonValue::Object(fields) = report else {
        gate.err(
            "wsl-toolkit",
            "the lane-job ledger has an unknown schema or is not a dry run".to_string(),
        );
        return;
    };
    let schema_ok = matches!(
        json_field(&fields, "schema"),
        Some(JsonValue::Text(text)) if text == "wsl-toolkit-cleanup/1"
    );
    let dry_ok = matches!(json_field(&fields, "dry_run"), Some(JsonValue::Bool(true)));
    if !schema_ok || !dry_ok {
        gate.err(
            "wsl-toolkit",
            "the lane-job ledger has an unknown schema or is not a dry run".to_string(),
        );
        return;
    }
    for field in CLEANUP_FIELDS {
        let items = json_field(&fields, field);
        gate.bump("post_task_cleanup");
        let Some(items) = items else {
            continue;
        };
        if *items == JsonValue::Null {
            continue;
        }
        let JsonValue::Array(items) = items else {
            gate.err(
                "wsl-toolkit",
                format!("the lane-job ledger field {field} is not a list"),
            );
            continue;
        };
        for item in items {
            gate.err(
                "wsl-toolkit",
                format!(
                    "lane job still kept: {} -- remove it \
                     with `wsl-toolkit --instance podvm gc \
                     --job <id> --apply` before the next job",
                    render_json_item(item)
                ),
            );
        }
    }
}
pub(super) fn check_perf_budget(gate: &mut Gate, root: &Path) {
    let Ok(text) = read_text(&root.join(PERF_CEILINGS)) else {
        gate.err(
            PERF_CEILINGS,
            "does not exist, and check 30 holds the \
             perf budgets against the committed readings. \
             TODO/gate.md T-1338."
                .to_string(),
        );
        return;
    };
    let mut ceilings: BTreeMap<String, (String, f64, f64)> = BTreeMap::new();
    for (index, line) in text.split('\n').enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            gate.err(
                &format!("{PERF_CEILINGS}:{}", index + 1),
                format!("row does not parse: {}", line.trim()),
            );
            continue;
        }
        let metric = parts[0].trim().to_string();
        let unit = parts[1].trim().to_string();
        let ceiling: Option<f64> = parts[2].trim().parse().ok();
        let tolerance: Option<f64> = parts[3].trim().parse().ok();
        match (ceiling, tolerance) {
            (Some(ceiling), Some(tolerance)) => {
                ceilings.insert(metric, (unit, ceiling, tolerance));
            }
            _ => {
                gate.err(
                    &format!("{PERF_CEILINGS}:{}", index + 1),
                    format!("ceiling is not a number: {}", line.trim()),
                );
            }
        }
    }
    if ceilings.is_empty() {
        gate.err(
            PERF_CEILINGS,
            "carries no budgets, and an empty budget file \
             holds every reading trivially. TODO/gate.md T-1338."
                .to_string(),
        );
        return;
    }
    let mut rows = Vec::new();
    for rel in PERF_RESULTS {
        let Ok(body) = read_text(&root.join(rel)) else {
            if rel.ends_with("perf-lane.txt") || rel.ends_with("perf-seeds.tsv") {
                gate.err(
                    rel,
                    "does not exist, and check 30 compares the \
                     committed perf readings. TODO/gate.md T-1338."
                        .to_string(),
                );
            }
            continue;
        };
        for (index, line) in body.split('\n').enumerate() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 10 {
                continue;
            }
            rows.push((
                rel.to_string(),
                index + 1,
                parts[4].to_string(),
                parts[5].to_string(),
                parts[7].to_string(),
                parts[8].to_string(),
                parts[9].to_string(),
            ));
        }
    }
    for metric in ceilings.keys() {
        if !rows
            .iter()
            .any(|(_, _, _, row_metric, _, _, _)| row_metric == metric)
        {
            gate.err(
                PERF_CEILINGS,
                format!(
                    "budget `{metric}` names no committed reading, \
                     and a budget nothing measures holds vacuously. \
                     TODO/gate.md T-1338."
                ),
            );
        }
    }
    for (rel, line_no, shape, metric, value, unit, state) in rows {
        if state != "ok" {
            continue;
        }
        let Some((_, ceiling, tolerance)) = ceilings.get(&metric) else {
            continue;
        };
        let Ok(number): Result<f64, _> = value.parse() else {
            continue;
        };
        gate.bump("perf_budget");
        if number > ceiling * (1.0 + tolerance) {
            gate.err(
                &format!("{rel}:{line_no}"),
                format!(
                    "perf regression: `{metric}` reads {value}{unit} \
                     against a {} ceiling on shape {shape}. \
                     TODO/gate.md T-1338.",
                    format_ceiling(*ceiling)
                ),
            );
        }
    }
}
