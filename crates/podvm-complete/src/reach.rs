//! Does this mirror speak HTTPS? `pod-rootfs` asks it once per host
//! (`pod_rootfs::reach::Probe`), and this module answers it over podvm's
//! transport ([`podvm_image::net`]: `pod-proxy`, the proxy of the
//! environment, podvm's roots).
//!
//! ⛔ Bounded, always. [`TODO/RULES.md`](../../../TODO/RULES.md) section 8: a
//! runtime whose audience is automated may not wait unbounded, and this probe
//! runs on the path to starting a container.
//!
//! ⚠ **Any HTTP response is a yes**, 404 included. The question is whether the
//! host terminates TLS on 443, not whether it serves `/`. A connect failure, a
//! TLS failure or a timeout is a no, and the reason travels with it so the
//! banner can say which.

use std::time::Duration;

use podvm_image::net::{Limits, Net, Refusal};

pub use pod_rootfs::reach::Answer;

/// ⚠ Two seconds to connect and two to read: the alternative -- a
/// black-holed tcp/80 inside the payload -- costs the package manager's own
/// timeout, which is minutes.
const CONNECT: Duration = Duration::from_secs(2);
const READ: Duration = Duration::from_secs(2);

/// One bounded `HEAD https://HOST/`.
pub fn probe(host: &str) -> Answer {
    let (config, _roots) = podvm_image::tls::client_config();
    // ⚠ The environment's proxy where it sets one: an intercepting proxy is
    // the normal case in the environments podvm is built for.
    let mut net = match Net::from_environment(config) {
        Ok(n) => n,
        Err(why) => return Answer::No(why),
    };
    let limits = Limits {
        head: CONNECT.saturating_add(READ),
        idle: READ,
    };
    match net.send("HEAD", &format!("https://{host}/"), &[], true, limits) {
        Ok(_) => Answer::Https,
        // ⭐ A status is an answer: the host terminated TLS and replied.
        Err(Refusal::Status(_)) => Answer::Https,
        Err(e) => Answer::No(e.to_string()),
    }
}

/// TODO/image.md T-1762: how long a verdict holds. A yes holds a day; a
/// no holds ten minutes, so a mirror that comes back is asked again soon.
const FRESH_YES: u64 = 24 * 3600;
const FRESH_NO: u64 = 600;

/// One verdict of the cache: `HOST TAB https|no TAB SECONDS TAB REASON`.
fn parse_line(line: &str) -> Option<(&str, Answer, u64)> {
    let mut f = line.splitn(4, '\t');
    let host = f.next()?;
    let word = f.next()?;
    let at: u64 = f.next()?.parse().ok()?;
    let answer = match word {
        "https" => Answer::Https,
        "no" => Answer::No(format!("{} (cached)", f.next().unwrap_or(""))),
        _ => return None,
    };
    Some((host, answer, at))
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The cached verdict of `host` in `text`, where it is fresh at `now`.
pub fn lookup(text: &str, host: &str, now: u64) -> Option<Answer> {
    text.lines().filter_map(parse_line).find_map(|(h, a, at)| {
        let fresh = if a.ok() { FRESH_YES } else { FRESH_NO };
        (h == host && now.saturating_sub(at) < fresh).then_some(a)
    })
}

/// `text` with the verdict of `host` at `now` in place of its old line.
pub fn record(text: &str, host: &str, answer: &Answer, now: u64) -> String {
    let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
    let mut out: String = text
        .lines()
        .filter(|l| parse_line(l).is_some_and(|(h, _, _)| h != host))
        .map(|l| format!("{l}\n"))
        .collect();
    let (word, why) = match answer {
        Answer::Https => ("https", String::new()),
        Answer::No(why) => ("no", clean(why)),
    };
    out.push_str(&format!("{}\t{word}\t{now}\t{why}\n", clean(host)));
    out
}

/// [`probe`] behind a cache file of the store: a fresh verdict of an
/// earlier run answers with no request (TODO/image.md T-1762). Before,
/// each `run` of ubuntu paid about 2 s for four probes. With no path, or a
/// file that does not read or write, each host is probed as before.
pub fn cached(path: Option<std::path::PathBuf>) -> impl FnMut(&str) -> Answer {
    move |host| {
        let Some(path) = &path else {
            return probe(host);
        };
        let text = std::fs::read_to_string(path).unwrap_or_default();
        if let Some(a) = lookup(&text, host, now()) {
            return a;
        }
        let a = probe(host);
        let next = record(&text, host, &a, now());
        // Write beside, then rename: a reader never sees half a file.
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        if std::fs::write(&tmp, next).is_ok() && std::fs::rename(&tmp, path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_verdict_is_cached_with_its_age_bound() {
        let t = record("", "a.example", &Answer::Https, 1000);
        let t = record(
            &t,
            "b.example",
            &Answer::No("connect\ttimed out".into()),
            1000,
        );
        assert_eq!(
            lookup(&t, "a.example", 1000 + FRESH_YES - 1),
            Some(Answer::Https)
        );
        assert_eq!(lookup(&t, "a.example", 1000 + FRESH_YES), None);
        let no = lookup(&t, "b.example", 1001).expect("a fresh no");
        assert_eq!(no.why(), "connect timed out (cached)");
        assert_eq!(lookup(&t, "b.example", 1000 + FRESH_NO), None);
        // A new verdict replaces the old line of its host.
        let t = record(&t, "b.example", &Answer::Https, 2000);
        assert_eq!(t.lines().count(), 2);
        assert_eq!(lookup(&t, "b.example", 2001), Some(Answer::Https));
        assert_eq!(lookup("garbage\nx\ty\n", "x", 0), None);
    }

    /// ⛔ The three states stay three. A `No` carries its reason so the banner
    /// can say why a file was left alone.
    #[test]
    fn a_refusal_carries_its_reason() {
        let a = Answer::No("connect timed out".into());
        assert!(!a.ok());
        assert_eq!(a.why(), "connect timed out");
    }
}
