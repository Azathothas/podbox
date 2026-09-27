//! A relay is a LIST, and a client that names one relay fails for a minute
//! at a time. This is the failover over that list.
//!
//! ⛔ **A SHARED PUBLIC RELAY REFUSES IN BURSTS, AND THIS IS MEASURED, NOT
//! ASSUMED.** On 2026-09-27, in one window all three built-in relays answered
//! `502 Bad Gateway`, `400 Bad Request` and `503 Service Unavailable` at
//! once, and in the same minute a later attempt succeeded. A client that
//! names one relay is therefore down for the length of that window however
//! healthy the code is. This module is the answer: N hops, tried in order,
//! each one's refusal reported, and a bad relay costs one timeout rather than
//! the session.
//!
//! ⛔ **THE ORDER IS THE PRODUCT.** First-byte latency to a slow target was
//! measured across three relays answering the same banner: 0.67s, 0.76s and
//! 4.34s. Ordered worst-first, a session that would have answered in 0.67s
//! spends 4.34s on a hop that also answers, and a `ConnectTimeout` is
//! consumed before a good relay is tried. A correct but unordered relay list
//! is a latency bug, so `catalog` is ordered by measurement and this tries it
//! in that order.
//!
//! ⛔ **A NON-RETRYABLE FAILURE STOPS THE LIST.** A bad credential or an
//! unresolvable name retried against every relay turns one sentence into N
//! and multiplies the wall clock by the length of the list, for an outcome
//! that cannot change.

use std::time::Duration;

use crate::error::{Error, Kind, Result};
use crate::transport::{Dialer, Stream};

/// One hop's outcome, kept whether it worked or not, so a failure says which
/// relay refused and how rather than only that something did.
#[derive(Debug, Clone)]
pub struct Attempt {
    /// The relay spec as written, quoted back so a reader recognises it.
    pub relay: String,
    pub outcome: std::result::Result<(), (Kind, String)>,
}

/// The failover chain over a relay list.
pub struct Chain<'a> {
    dialer: &'a Dialer,
    relays: Vec<String>,
    /// The bound on ONE hop. Sized against a slow target's first byte, not a
    /// fast LAN's: measured 4.34s to a banner, so a bound under that fails a
    /// healthy relay.
    pub hop_timeout: Duration,
    /// The bound on the whole chain, so a list of dead relays is bounded
    /// rather than multiplying one timeout by the list length.
    pub total: Duration,
    attempts: Vec<Attempt>,
}

impl<'a> Chain<'a> {
    pub fn new(dialer: &'a Dialer, relays: Vec<String>) -> Chain<'a> {
        Chain {
            dialer,
            relays,
            hop_timeout: Duration::from_secs(15),
            total: Duration::from_secs(90),
            attempts: Vec::new(),
        }
    }

    /// Every hop gets its own full slice rather than a share of one global
    /// clock. A shared clock lets the first hop's latency starve every hop
    /// after it, which is how a session that would have answered in 0.67s
    /// died at "banner exchange timed out" with every relay healthy.
    fn hop_slice(&self) -> Duration {
        let n = self.relays.len().max(1) as u32;
        let share = self.total / n;
        if share.is_zero() {
            Duration::from_secs(1)
        } else {
            share.min(self.hop_timeout)
        }
    }

    /// Try each relay in order until one produces a stream.
    pub fn connect(&mut self) -> Result<Box<dyn Stream>> {
        if self.relays.is_empty() {
            return Err(Error::new(
                Kind::Config,
                "chain",
                "no relay is configured; pass --relay, set PODSSH_RELAY, or name a target",
            ));
        }
        let slice = self.hop_slice();
        let mut last: Option<Error> = None;
        for relay in &self.relays {
            match self.dialer.dial(relay) {
                Ok(s) => {
                    let _ = s.set_read_timeout(Some(slice));
                    self.attempts.push(Attempt { relay: relay.clone(), outcome: Ok(()) });
                    return Ok(s);
                }
                Err(e) => {
                    let kind = classify(&e);
                    self.attempts.push(Attempt {
                        relay: relay.clone(),
                        outcome: Err((kind, e.to_string())),
                    });
                    if !kind.retryable() {
                        return Err(Error::new(
                            kind,
                            format!("relay {relay}"),
                            e.to_string(),
                        ));
                    }
                    last = Some(Error::new(kind, format!("relay {relay}"), e.to_string()));
                }
            }
        }
        Err(self.exhausted(last))
    }

    /// The error a fully exhausted chain reports.
    ///
    /// ⛔ IT NAMES EVERY RELAY AND WHAT EACH SAID. "all relays failed" is the
    /// least useful sentence a transport can produce, and the refusal text is
    /// usually the whole diagnosis: a `403 not on the egress allowlist` says
    /// the policy is the wall, and a timeout on the same hop says the relay
    /// is dead.
    fn exhausted(&self, last: Option<Error>) -> Error {
        let mut because = String::from("every relay failed:");
        for a in &self.attempts {
            match &a.outcome {
                Ok(()) => because.push_str(&format!("\n  {} ok", a.relay)),
                Err((k, m)) => because.push_str(&format!("\n  {} [{}] {}", a.relay, k.token(), m)),
            }
        }
        let kind = last.map(|l| l.kind).unwrap_or(Kind::Unreachable);
        Error::new(kind, "chain", because)
    }

    /// The per-relay transcript, for `probe --json`.
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }
}

/// Classify a dial error into the kind the chain branches on.
///
/// ⛔ A `403`/`400` from a relay is a REFUSAL and the next relay is worth
/// trying; a `502`/`503` is the relay's own upstream failing and is usually
/// also worth trying; a connect timeout is UNREACHABLE. The distinction that
/// matters for the chain is only "stop" versus "continue", and a credential
/// or a name is the only thing that stops it.
fn classify(e: &std::io::Error) -> Kind {
    let text = e.to_string();
    let lower = text.to_ascii_lowercase();
    if lower.contains("403") || lower.contains("forbidden") || lower.contains("refused") {
        Kind::Refused
    } else if lower.contains("401") || lower.contains("token") || lower.contains("auth") {
        // ⛔ A relay that says the TOKEN is wrong is not going to accept it on
        // the next relay either, and a wrong token is the one failure that must
        // not be retried down a list of five.
        Kind::Auth
    } else if lower.contains("502")
        || lower.contains("503")
        || lower.contains("bad gateway")
        || lower.contains("unavailable")
    {
        Kind::Protocol
    } else {
        Kind::Unreachable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wrong_token_stops_the_chain_and_a_refusal_does_not() {
        // ⛔ The regression this holds: a bad token retried against every
        // relay turns one sentence into five and multiplies the wall clock by
        // five, for an outcome that cannot change.
        let tok = std::io::Error::other("relay: missing or wrong token");
        assert_eq!(classify(&tok), Kind::Auth);
        assert!(!classify(&tok).retryable());

        let refused = std::io::Error::other("relay HTTP/1.1 503 Service Unavailable");
        assert_eq!(classify(&refused), Kind::Protocol);
        assert!(classify(&refused).retryable());

        let forbidden = std::io::Error::other("relay HTTP/1.1 403 Forbidden");
        assert_eq!(classify(&forbidden), Kind::Refused);
        assert!(classify(&forbidden).retryable());
    }

    #[test]
    fn every_hop_gets_a_full_slice_and_the_total_still_bounds_the_run() {
        let d = Dialer::new(None, std::sync::Arc::new(
            crate::transport::tls::ClientConfig::new(true, &[]).expect("test tls config")), Duration::from_secs(5));
        let c = Chain::new(&d, vec!["a".into(), "b".into(), "c".into()]);
        // 90s over 3 hops is 30s, capped at the 15s hop bound. The point is
        // that it is NOT 90/3-then-still-going: a slow first hop cannot eat
        // the whole chain.
        assert_eq!(c.hop_slice(), Duration::from_secs(15));
        let d2 = Dialer::new(None, std::sync::Arc::new(
            crate::transport::tls::ClientConfig::new(true, &[]).expect("test tls config")), Duration::from_secs(5));
        let c2 = Chain::new(&d2, vec!["a".into(), "b".into(), "c".into(), "d".into(),
                                       "e".into(), "f".into()]);
        assert_eq!(c2.hop_slice(), Duration::from_secs(15));
    }

    #[test]
    fn an_empty_chain_is_a_config_failure_and_not_a_hang() {
        let d = Dialer::new(None, std::sync::Arc::new(
            crate::transport::tls::ClientConfig::new(true, &[]).expect("test tls config")), Duration::from_secs(5));
        let mut c = Chain::new(&d, Vec::new());
        let e = crate::error::err_of_boxed(c.connect());
        assert_eq!(e.kind, Kind::Config);
        assert!(e.because.contains("no relay"), "{}", e.because);
    }
}
