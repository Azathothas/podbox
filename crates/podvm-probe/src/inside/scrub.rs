//! Replace a value that looks like a secret with `REDACTED` before a
//! report holds it. A report is meant to be committed and compared, so it
//! must hold no token, key or password of the host or of the image.
//!
//! The rule is wide on purpose: a false `REDACTED` costs a detail, and a
//! missed token is a leak.

/// The marker that takes the place of a secret.
pub const REDACTED: &str = "REDACTED";

/// Prefixes of well-known token formats.
const PREFIXES: &[&str] = &[
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "github_pat_",
    "glpat-",
    "xoxb-",
    "xoxp-",
    "xoxa-",
    "sk-",
    "sk_live_",
    "pk_live_",
    "AKIA",
    "ASIA",
    "AIza",
    "ya29.",
    "npm_",
    "pypi-",
    "hf_",
];

/// Words of a variable name that holds a secret.
const NAMES: &[&str] = &[
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "KEY",
    "AUTH",
    "CREDENTIAL",
    "COOKIE",
    "SESSION",
];

/// True where the name of a variable says that its value is a secret.
pub fn secret_name(name: &str) -> bool {
    let up = name.to_ascii_uppercase();
    NAMES.iter().any(|w| up.contains(w))
}

/// True where one word looks like a secret: a known token prefix, a PEM
/// block, a JWT, or a long run of letters and digits with both cases or
/// digits mixed in.
fn secret_word(w: &str) -> bool {
    let w = w.trim_matches(|c: char| c == '"' || c == '\'' || c == ',' || c == ';');
    if PREFIXES
        .iter()
        .any(|p| w.starts_with(p) && w.len() >= p.len() + 8)
    {
        return true;
    }
    if w.contains("-----BEGIN") {
        return true;
    }
    if w.starts_with("eyJ") && w.matches('.').count() == 2 && w.len() > 30 {
        return true;
    }
    let run = w
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .count();
    if w.len() >= 32 && run == w.len() {
        let upper = w.chars().any(|c| c.is_ascii_uppercase());
        let lower = w.chars().any(|c| c.is_ascii_lowercase());
        let digit = w.chars().any(|c| c.is_ascii_digit());
        // A long hex digest is not a secret by itself: an image id, a
        // digest. Mixed case with digits is.
        return upper && lower && digit;
    }
    false
}

/// The password of a URL with user information (`scheme://user:pass@host`)
/// replaced.
fn scrub_url(w: &str) -> Option<String> {
    let (scheme, rest) = w.split_once("://")?;
    let (auth, tail) = rest.split_once('@')?;
    if auth.contains('/') {
        return None;
    }
    let user = auth.split(':').next().unwrap_or("");
    Some(format!("{scheme}://{user}:{REDACTED}@{tail}"))
}

/// `text` with each secret-shaped word replaced.
pub fn scrub(text: &str) -> String {
    if text.contains("-----BEGIN") {
        return REDACTED.to_string();
    }
    text.split(' ')
        .map(|w| {
            if let Some((k, v)) = w.split_once('=')
                && secret_name(k)
                && !v.is_empty()
            {
                return format!("{k}={REDACTED}");
            }
            if let Some(u) = scrub_url(w) {
                return u;
            }
            if secret_word(w) {
                return REDACTED.to_string();
            }
            w.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
