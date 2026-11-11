//! The egress broker of `run --net broker` (TODO/net.md T-1665, issue 184).
//!
//! podvm hosts `pod-broker` for one run: one job, `run`, with a token of 32
//! random bytes, and a policy that allows the hosts of `--allow` on the
//! ports of `--ports` (443 where none is given). The broker listens on a
//! TCP port of the loopback and dials through the host's proxy where the
//! environment names one (the target's name goes to it), else direct. The
//! answers are the broker's: 407 without the token, 403 for a target off
//! the lists or a non-public address, 504 for a name that does not resolve,
//! 502 for a dial that fails.
//!
//! ⛔ The token never goes in argv or in a line that podvm prints: it is in
//! the payload's environment only (the password of the proxy URL).

use std::sync::Arc;
use std::time::Duration;

use pod_broker::policy::{Mode, Policy, Rule, SCHEMA};
use pod_broker::{Broker, Load, Settings, Token};

/// The job id of the run.
pub const JOB: &str = "run";
/// The address in the guest of the machine tier where the broker answers:
/// a forward of the restricted user-mode network.
pub const GUEST_FORWARD: (&str, u16) = ("10.0.2.100", 3128);
/// The hidden argument of podvm that relays one connection of a guest to
/// the broker: QEMU's `guestfwd ... cmd:` runs it for each connection, with
/// the connection on stdin and stdout.
pub const RELAY_ARG: &str = "--podvm-relay";

/// Relay stdin and stdout to `addr` until either side ends. The exit code.
pub fn relay(addr: &str) -> i32 {
    let Ok(stream) = std::net::TcpStream::connect(addr) else {
        return 1;
    };
    let Ok(mut up) = stream.try_clone() else {
        return 1;
    };
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut std::io::stdin().lock(), &mut up);
        let _ = up.shutdown(std::net::Shutdown::Write);
    });
    let mut down = stream;
    let _ = std::io::copy(&mut down, &mut std::io::stdout().lock());
    0
}

/// The ports where `--ports` gives none.
pub const DEFAULT_PORTS: &[u16] = &[443];

/// A broker that serves one run until the process ends.
pub struct Started {
    pub port: u16,
    token: String,
}

impl Started {
    /// The proxy URL of the run at `host:port` (the loopback on a rootfs
    /// tier, the forward of the guest on the machine tier).
    pub fn url(&self, host: &str, port: u16) -> String {
        format!("http://{JOB}:{}@{host}:{port}", self.token)
    }

    /// The proxy variables of the payload: each names the broker.
    pub fn env(&self, host: &str, port: u16) -> Vec<String> {
        let url = self.url(host, port);
        ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"]
            .iter()
            .map(|k| format!("{k}={url}"))
            .collect()
    }
}

/// The proxy variables of `env` replaced by `with`.
pub fn replace_proxy_env(env: &mut Vec<String>, with: Vec<String>) {
    env.retain(|e| {
        let k = e.split('=').next().unwrap_or("");
        !crate::machine::proxy::VARS.contains(&k) && k != "NO_PROXY" && k != "no_proxy"
    });
    env.extend(with);
}

/// The broker of a run on a rootfs tier, or None without `--network
/// broker`. Only the syscall tier holds the payload to it there: each other
/// rung refuses, because a payload that ignores the variables would reach
/// the network directly (TODO/net.md T-1665).
pub fn for_run(
    verb: &str,
    broker: bool,
    syscall: bool,
    allow: &[String],
    ports: &[u16],
) -> Result<Option<Started>, i32> {
    if !broker {
        return Ok(None);
    }
    if !syscall {
        eprintln!(
            "podvm {verb}: --network broker needs a mechanism that holds the payload to the \
             broker: --podvm-tier=syscall (a static program; its handler refuses each other \
             connect) or --podvm-tier=machine (a guest network with only the broker). On \
             this rung a payload that ignores the proxy variables reaches the network \
             directly, so podvm refuses (TODO/net.md T-1665)"
        );
        return Err(podvm_image::error::EXIT_RUNTIME_ERROR);
    }
    start(allow, ports).map(Some).map_err(|e| {
        eprintln!("podvm {verb}: --network broker: {e} (TODO/net.md T-1665)");
        podvm_image::error::EXIT_RUNTIME_ERROR
    })
}

/// The policy of `--allow` and `--ports`.
pub fn policy(allow: &[String], ports: &[u16]) -> Policy {
    let ports = if ports.is_empty() {
        DEFAULT_PORTS.to_vec()
    } else {
        ports.to_vec()
    };
    Policy {
        schema: SCHEMA,
        mode: Mode::Allow,
        allow: allow
            .iter()
            .map(|h| Rule {
                host: h.clone(),
                path: None,
                ports: ports.clone(),
            })
            .collect(),
        deny: Vec::new(),
    }
}

fn token() -> Result<String, String> {
    let mut b = [0u8; 32];
    std::io::Read::read_exact(
        &mut std::fs::File::open("/dev/urandom").map_err(|e| format!("/dev/urandom: {e}"))?,
        &mut b,
    )
    .map_err(|e| format!("/dev/urandom: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Start the broker of a run. Err names what failed, never the token.
pub fn start(allow: &[String], ports: &[u16]) -> Result<Started, String> {
    let token = token()?;
    let proxy = pod_proxy::env::proxy_from_vars("example.com", &|n| std::env::var_os(n))
        .map_err(|e| format!("the host's proxy variable: {e}"))?;
    let settings = Settings {
        // The host's proxy resolves the name; an allowlist of a proxy-only
        // host takes names, not addresses (pod-broker, `names_upstream`).
        names_upstream: proxy.is_some(),
        ..Settings::default()
    };
    let resolve: Arc<dyn pod_core::Resolve> = Arc::new(pod_proxy::resolve::System::new());
    let dial = pod_broker::upstream("podvm", proxy, resolve.clone());
    let broker = Arc::new(Broker::new(settings, resolve, dial));
    broker
        .load(Load {
            job: JOB.to_string(),
            token: Token::new(token.clone()),
            ttl: Duration::from_secs(24 * 3600),
            policy: policy(allow, ports),
        })
        .map_err(|e| format!("the policy of --allow and --ports: {e}"))?;
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| format!("127.0.0.1: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("127.0.0.1: {e}"))?
        .port();
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("127.0.0.1:{port}: {e}"))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("the runtime of the broker: {e}"))?;
    std::thread::Builder::new()
        .name("podvm-broker".into())
        .spawn(move || {
            runtime.block_on(async move {
                if let Ok(l) = tokio::net::TcpListener::from_std(listener) {
                    let _ = broker.serve_tcp(l).await;
                }
            })
        })
        .map_err(|e| format!("the thread of the broker: {e}"))?;
    Ok(Started { port, token })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_policy_allows_each_host_on_the_ports() {
        let p = policy(&["example.com".into(), "*.example.org".into()], &[]);
        assert_eq!(p.mode, Mode::Allow);
        assert_eq!(p.allow.len(), 2);
        assert_eq!(p.allow[0].ports, vec![443]);
        let p = policy(&["example.com".into()], &[80, 443]);
        assert_eq!(p.allow[0].ports, vec![80, 443]);
    }

    #[test]
    fn the_url_holds_the_token_only_as_the_password() {
        let s = Started {
            port: 1,
            token: "0123456789abcdef0123456789abcdef".into(),
        };
        assert_eq!(
            s.url("10.0.2.100", 3128),
            "http://run:0123456789abcdef0123456789abcdef@10.0.2.100:3128"
        );
        assert_eq!(s.env("127.0.0.1", 9).len(), 4);
    }
}
