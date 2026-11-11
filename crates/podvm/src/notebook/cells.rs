//! The cells of a notebook: the segments between kernel restarts, and the
//! `[N] text` lines of an executed notebook. TODO/cli.md T-1660.

use serde_json::Value;

/// The notebook `nb` cut after each cell of `after` (1-based cell numbers,
/// over all cells): one notebook for each kernel. A number past the last
/// cell cuts nothing.
pub fn segments(nb: &Value, after: &[usize]) -> Result<Vec<Value>, String> {
    let cells = nb
        .get("cells")
        .and_then(Value::as_array)
        .ok_or("the notebook has no `cells` array")?;
    let mut cuts: Vec<usize> = after
        .iter()
        .copied()
        .filter(|&n| n >= 1 && n < cells.len())
        .collect();
    cuts.sort_unstable();
    cuts.dedup();
    let mut out = Vec::new();
    let mut from = 0;
    for cut in cuts.into_iter().chain(std::iter::once(cells.len())) {
        let mut seg = nb.clone();
        seg["cells"] = Value::Array(cells[from..cut].to_vec());
        out.push(seg);
        from = cut;
    }
    Ok(out)
}

/// What an executed notebook printed, and whether a cell raised.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    pub lines: Vec<String>,
    pub raised: bool,
}

/// The text of a notebook field: a string, or an array of strings.
fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
        _ => String::new(),
    }
}

/// Each output of the executed notebook `nb` as lines `[cycle] text`: the
/// streams, the plain text of a result or a display, and `ENAME: EVALUE`
/// of an error. A failed `!` line is a stream, not an error, so it does
/// not count as a raise.
pub fn outcome(nb: &Value, cycle: u32) -> Outcome {
    let mut o = Outcome::default();
    let cells = nb.get("cells").and_then(Value::as_array);
    for cell in cells.into_iter().flatten() {
        let outputs = cell.get("outputs").and_then(Value::as_array);
        for out in outputs.into_iter().flatten() {
            let body = match out.get("output_type").and_then(Value::as_str) {
                Some("stream") => out.get("text").map(text).unwrap_or_default(),
                Some("execute_result" | "display_data") => out
                    .get("data")
                    .and_then(|d| d.get("text/plain"))
                    .map(text)
                    .unwrap_or_default(),
                Some("error") => {
                    o.raised = true;
                    format!(
                        "{}: {}",
                        out.get("ename").and_then(Value::as_str).unwrap_or("Error"),
                        out.get("evalue").and_then(Value::as_str).unwrap_or("")
                    )
                }
                _ => String::new(),
            };
            o.lines
                .extend(body.lines().map(|l| format!("[{cycle}] {l}")));
        }
    }
    o
}
