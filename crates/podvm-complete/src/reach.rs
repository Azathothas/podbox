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

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔ The three states stay three. A `No` carries its reason so the banner
    /// can say why a file was left alone.
    #[test]
    fn a_refusal_carries_its_reason() {
        let a = Answer::No("connect timed out".into());
        assert!(!a.ok());
        assert_eq!(a.why(), "connect timed out");
    }
}
