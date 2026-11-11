//! The guest's view of a loopback proxy (TODO/net.md T-1665, issue 184).
//!
//! On the machine tier, `127.0.0.1` in the guest is the guest (F29). QEMU's
//! user-mode network gives the host's loopback at the gateway 10.0.2.2, so
//! each proxy variable of the payload whose host is `127.0.0.1`,
//! `localhost` or `::1` is rewritten to 10.0.2.2. The scheme, the
//! credentials, the port and the rest stay byte for byte: a password with
//! `@` survives, and a percent escape is not decoded.
//!
//! ⛔ An error never holds the value: a proxy URL can hold a credential.

/// The variables that name a proxy.
pub const VARS: &[&str] = &[
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "FTP_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "ftp_proxy",
];

/// The host loopback, as the guest of QEMU's user-mode network sees it.
pub const GATEWAY: &str = "10.0.2.2";

/// The URL with a loopback host rewritten to [`GATEWAY`], the URL unchanged
/// where its host is another, or the reason why it cannot be read (no
/// scheme, no port). The reason never holds the URL.
pub fn rewrite(url: &str) -> Result<String, &'static str> {
    let Some(at) = url.find("://") else {
        return Err("it has no scheme (as http://HOST:PORT)");
    };
    let (scheme, rest) = (&url[..at + 3], &url[at + 3..]);
    if at == 0 {
        return Err("it has no scheme (as http://HOST:PORT)");
    }
    // The authority ends at the first `/`, `?` or `#`; the credentials end
    // at its last `@`, so an `@` of a password stays in the password.
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    let (userinfo, hostport) = match authority.rfind('@') {
        Some(i) => (&authority[..=i], &authority[i + 1..]),
        None => ("", authority),
    };
    let (host, port) = if let Some(v6) = hostport.strip_prefix('[') {
        let close = v6.find(']').ok_or("its IPv6 host has no closing ]")?;
        let host = &hostport[..close + 2];
        let port = v6[close + 1..].strip_prefix(':').unwrap_or("");
        (host, port)
    } else {
        match hostport.rsplit_once(':') {
            Some((h, p)) => (h, p),
            None => (hostport, ""),
        }
    };
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return Err("it has no port (as http://HOST:PORT)");
    }
    if host.is_empty() {
        return Err("it has no host");
    }
    let loopback = matches!(
        host.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "[::1]"
    ) || host.starts_with("127.");
    if !loopback {
        return Ok(url.to_string());
    }
    Ok(format!("{scheme}{userinfo}{GATEWAY}:{port}{tail}"))
}

/// The environment of a guest with each proxy variable rewritten, or the
/// refusal that names the variable (never its value).
pub fn rewrite_env(env: &[String]) -> Result<Vec<String>, String> {
    env.iter()
        .map(|e| match e.split_once('=') {
            Some((k, v)) if VARS.contains(&k) && !v.is_empty() => {
                rewrite(v).map(|u| format!("{k}={u}")).map_err(|why| {
                    format!(
                        "{k} cannot be given to the guest: {why}. podvm rewrites a loopback \
                         proxy to the gateway {GATEWAY} and needs the scheme and the port \
                         for it (TODO/net.md T-1665)"
                    )
                })
            }
            _ => Ok(e.clone()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loopback_proxy_moves_to_the_gateway_byte_for_byte() {
        assert_eq!(
            rewrite("http://127.0.0.1:3128").unwrap(),
            "http://10.0.2.2:3128"
        );
        assert_eq!(
            rewrite("http://localhost:8080/").unwrap(),
            "http://10.0.2.2:8080/"
        );
        assert_eq!(
            rewrite("socks5h://[::1]:1080").unwrap(),
            "socks5h://10.0.2.2:1080"
        );
        // A password with `@` survives, at the last `@` of the authority.
        assert_eq!(
            rewrite("http://user:p@ss@w0rd@127.0.0.1:3128").unwrap(),
            "http://user:p@ss@w0rd@10.0.2.2:3128"
        );
        // A percent escape is not decoded.
        assert_eq!(
            rewrite("http://us%40er:p%3Ass%25@127.0.0.1:3128").unwrap(),
            "http://us%40er:p%3Ass%25@10.0.2.2:3128"
        );
        // Another host stays as it is.
        assert_eq!(
            rewrite("http://proxy.example:3128").unwrap(),
            "http://proxy.example:3128"
        );
    }

    #[test]
    fn a_url_with_no_port_or_no_scheme_is_refused_without_its_value() {
        assert!(rewrite("127.0.0.1:3128").unwrap_err().contains("no scheme"));
        assert!(rewrite("http://127.0.0.1").unwrap_err().contains("no port"));
        assert!(rewrite("http://user:secret@127.0.0.1/")
            .unwrap_err()
            .contains("no port"));
        let e = rewrite_env(&["HTTPS_PROXY=http://user:secret@127.0.0.1".to_string()]).unwrap_err();
        assert!(e.starts_with("HTTPS_PROXY cannot be given"), "{e}");
        assert!(
            !e.contains("secret"),
            "the refusal holds the credential: {e}"
        );
        // A variable that is not a proxy stays as it is.
        let env = rewrite_env(&["NO_PROXY=localhost".into(), "A=127.0.0.1".into()]).unwrap();
        assert_eq!(env, ["NO_PROXY=localhost", "A=127.0.0.1"]);
    }
}
