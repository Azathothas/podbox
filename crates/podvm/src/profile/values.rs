//! The readers of the values of a profile: each error names the key and
//! the type it found. TODO/cli.md T-1654.

use toml::{Table, Value};

/// The type of `v` with its article, as an error names it.
pub(super) fn article(v: &Value) -> String {
    let t = v.type_str();
    if t.starts_with(['a', 'e', 'i', 'o', 'u']) {
        format!("an {t}")
    } else {
        format!("a {t}")
    }
}

pub(super) fn want_table<'a>(t: &'a Table, key: &str) -> Result<Option<&'a Table>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(Value::Table(x)) => Ok(Some(x)),
        Some(other) => Err(format!("`{key}` must be a table, not {}", article(other))),
    }
}

pub(super) fn only(t: &Table, prefix: &str, allowed: &[&str]) -> Result<(), String> {
    for k in t.keys() {
        if !allowed.contains(&k.as_str()) {
            let full = if prefix.is_empty() {
                k.clone()
            } else {
                format!("{prefix}.{k}")
            };
            let scope = if prefix.is_empty() {
                "a profile".to_string()
            } else {
                format!("`{prefix}`")
            };
            return Err(format!(
                "unknown key `{full}`: {scope} takes {}. A misspelt key that is silently \
                 ignored leaves a cage looser than the recipe says",
                allowed.join(", ")
            ));
        }
    }
    Ok(())
}

pub(super) fn string(t: &Table, table: &str, key: &str) -> Result<Option<String>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(format!(
            "`{table}.{key}` must be a string, not {}",
            article(other)
        )),
    }
}

pub(super) fn boolean(t: &Table, table: &str, key: &str) -> Result<Option<bool>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(Value::Boolean(b)) => Ok(Some(*b)),
        Some(other) => Err(format!(
            "`{table}.{key}` must be a boolean, not {}",
            article(other)
        )),
    }
}

pub(super) fn strings(t: &Table, table: &str, key: &str) -> Result<Vec<String>, String> {
    match t.get(key) {
        None => Ok(Vec::new()),
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.clone()),
                other => Err(format!(
                    "`{table}.{key}` must be an array of strings; it holds {}",
                    article(other)
                )),
            })
            .collect(),
        Some(other) => Err(format!(
            "`{table}.{key}` must be an array of strings, not {}",
            article(other)
        )),
    }
}

pub(super) fn integer(
    t: &Table,
    table: &str,
    key: &str,
    min: i64,
    max: i64,
) -> Result<Option<i64>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(Value::Integer(n)) if (min..=max).contains(n) => Ok(Some(*n)),
        Some(Value::Integer(n)) => Err(format!(
            "`{table}.{key}` must be from {min} to {max}, not {n}"
        )),
        Some(other) => Err(format!(
            "`{table}.{key}` must be an integer, not {}",
            article(other)
        )),
    }
}

/// A size: an integer of bytes, or a string with a K, M, G or T suffix.
pub(super) fn size(t: &Table, table: &str, key: &str) -> Result<Option<u64>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(Value::Integer(n)) if *n > 0 => Ok(Some(*n as u64)),
        Some(Value::String(s)) => crate::tier::parse_mem(s).map(Some).ok_or_else(|| {
            format!("`{table}.{key}` takes bytes with an optional K/M/G/T suffix, not {s:?}")
        }),
        Some(other) => Err(format!(
            "`{table}.{key}` must be a size (an integer of bytes or a string as \"1g\"), not {}",
            other.to_string().trim()
        )),
    }
}
