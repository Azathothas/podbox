//! The record of `podvm run --report` at layout [`crate::SCHEMA`]: the
//! fields that a caller decides on, read with their types; the whole
//! document stays in [`Report::raw`].

use crate::Error;
use serde_json::Value;

/// One record of `run --report`.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub schema: u64,
    /// The version of podvm that wrote the record.
    pub podvm: String,
    pub image: Option<String>,
    pub image_digest: Option<String>,
    /// The rung or tier that the run entered.
    pub rung: Option<String>,
    pub exit_code: Option<i64>,
    pub signal: Option<i64>,
    /// The exit code of a refusal of podvm; `None` where the payload ran.
    pub refused: Option<i64>,
    pub wall_ms: Option<u64>,
    pub user_ms: Option<u64>,
    pub system_ms: Option<u64>,
    pub max_rss_kib: Option<u64>,
    pub timed_out: bool,
    pub labels: Vec<String>,
    pub degradations: Vec<String>,
    /// The whole document.
    pub raw: Value,
}

fn bad(key: &str, want: &str) -> Error {
    Error::Record(format!("{key} is not {want}"))
}

fn opt_str(v: &Value, k: &str) -> Result<Option<String>, Error> {
    match v.get(k) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        _ => Err(bad(k, "a string or null")),
    }
}

fn opt_i64(v: &Value, k: &str) -> Result<Option<i64>, Error> {
    match v.get(k) {
        Some(Value::Null) => Ok(None),
        Some(n) => n
            .as_i64()
            .map(Some)
            .ok_or_else(|| bad(k, "an integer or null")),
        None => Err(bad(k, "an integer or null")),
    }
}

fn opt_u64(v: &Value, k: &str) -> Result<Option<u64>, Error> {
    match v.get(k) {
        Some(Value::Null) => Ok(None),
        Some(n) => n
            .as_u64()
            .map(Some)
            .ok_or_else(|| bad(k, "a whole number or null")),
        None => Err(bad(k, "a whole number or null")),
    }
}

fn strings(v: &Value, k: &str) -> Result<Vec<String>, Error> {
    v.get(k)
        .and_then(Value::as_array)
        .ok_or_else(|| bad(k, "an array"))?
        .iter()
        .map(|e| {
            e.as_str()
                .map(String::from)
                .ok_or_else(|| bad(k, "an array of strings"))
        })
        .collect()
}

impl Report {
    /// Read a record. A record of another layout version is refused: a
    /// caller must not read fields whose meaning it does not know.
    pub fn parse(text: &str) -> Result<Report, Error> {
        let v: Value = serde_json::from_str(text).map_err(|e| Error::Record(e.to_string()))?;
        let schema = v
            .get("schema")
            .and_then(Value::as_u64)
            .ok_or_else(|| bad("schema", "a whole number"))?;
        if schema != crate::SCHEMA {
            return Err(Error::Record(format!(
                "the layout version is {schema}, and this crate reads version {}",
                crate::SCHEMA
            )));
        }
        Ok(Report {
            schema,
            podvm: opt_str(&v, "podvm")?.ok_or_else(|| bad("podvm", "a string"))?,
            image: opt_str(&v, "image")?,
            image_digest: opt_str(&v, "image_digest")?,
            rung: opt_str(&v, "rung")?,
            exit_code: opt_i64(&v, "exit_code")?,
            signal: opt_i64(&v, "signal")?,
            refused: opt_i64(&v, "refused")?,
            wall_ms: opt_u64(&v, "wall_ms")?,
            user_ms: opt_u64(&v, "user_ms")?,
            system_ms: opt_u64(&v, "system_ms")?,
            max_rss_kib: opt_u64(&v, "max_rss_kib")?,
            timed_out: v
                .get("timed_out")
                .and_then(Value::as_bool)
                .ok_or_else(|| bad("timed_out", "a boolean"))?,
            labels: strings(&v, "labels")?,
            degradations: strings(&v, "degradations")?,
            raw: v,
        })
    }

    /// The payload ran, and its exit code was 0.
    pub fn succeeded(&self) -> bool {
        self.refused.is_none() && self.exit_code == Some(0)
    }
}
