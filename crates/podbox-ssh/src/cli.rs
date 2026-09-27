//! The command line. One parser, one place that knows the flag names, so a
//! new subcommand cannot quietly accept a flag nothing reads.

use std::path::PathBuf;
use std::time::Duration;

use crate::protocol::Protocol;
use crate::sshserver::ServerSpec;
use crate::transport::Dialer;

const USAGE: &str = "\
usage: podssh <command> [options]

  probe     measure this machine's egress and verify the candidate relays
  relay     run a rendezvous relay on a host this machine can dial
  serve     agent side: run an ssh server here and register with a relay
  connect   operator side: an ssh ProxyCommand that reaches a serve
  forward   reach one fixed host:port through the egress proxy, no relay
  config    print an ~/.ssh/config snippet for a name
  selftest  run the in-process protocol and framing checks
  version   print the version

  Common: --relay <url> (repeatable), --name <id>, --auth <secret>,
          --protocol podssh1|sandssh1, --proxy <url>, --tls-ca <file>,
          --insecure, --help
  Relay:  --listen <addr|unix:/path>, --key <secret>, --tls-cert, --tls-key, --ws, --once
  Serve:  --server <cmd>, --forward <host:port>, --host-key <file>,
          --authorized-keys <file>
  Forward: --target <host:port>, --expect-banner <prefix>

  A relay URL is a transport: tcp://, tls://, ws://, wss://, unix://,
  or exec://<command>. `podssh probe --json` prints everything as one
  JSON document. Nothing here needs a listening socket on the agent side.
";

#[derive(Default)]
struct Opts {
    relays: Vec<String>,
    name: Option<String>,
    auth: Option<String>,
    protocol: Option<String>,
    proxy: Option<String>,
    insecure: bool,
    tls_ca: Vec<String>,
    server: Option<String>,
    forward: Option<String>,
    host_key: Option<String>,
    authorized_keys: Option<String>,
    listen: Option<String>,
    key: Option<String>,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    ws: bool,
    once: bool,
    json: bool,
    target: Option<String>,
    expect_banner: Option<String>,
    wait: Option<u64>,
    handshake_timeout: Option<u64>,
    user: Option<String>,
    positional: Vec<String>,
}

pub fn main(argv: Vec<String>) -> i32 {
    let mut args = argv.into_iter().skip(1).peekable();
    let Some(command) = args.next() else {
        print!("{USAGE}");
        return 0;
    };
    let rest: Vec<String> = args.collect();
    match command.as_str() {
        "probe" => cmd_probe(&rest),
        "relay" => cmd_relay(&rest),
        "serve" => cmd_serve(&rest),
        "connect" => cmd_connect(&rest),
        "forward" => cmd_forward(&rest),
        "config" => cmd_config(&rest),
        "selftest" => cmd_selftest(&rest),
        "version" | "--version" | "-v" => {
            println!("podssh {}", env!("CARGO_PKG_VERSION"));
            0
        }
        "-h" | "--help" | "help" => {
            print!("{USAGE}");
            0
        }
        other => {
            eprintln!("podssh: unknown command {other:?}");
            eprint!("{USAGE}");
            2
        }
    }
}

fn parse(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        match flag.as_str() {
            "--relay" => o.relays.push(value(args, &mut i, inline.clone(), &flag)?),
            "--name" => o.name = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--auth" => o.auth = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--protocol" => o.protocol = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--proxy" | "--via" => o.proxy = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--tls-ca" => o.tls_ca.push(value(args, &mut i, inline.clone(), &flag)?),
            "--server" => o.server = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--forward" => o.forward = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--host-key" => o.host_key = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--authorized-keys" => {
                o.authorized_keys = Some(value(args, &mut i, inline.clone(), &flag)?)
            }
            "--listen" => o.listen = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--key" => o.key = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--tls-cert" => o.tls_cert = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--tls-key" => o.tls_key = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--target" => o.target = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--expect-banner" => {
                o.expect_banner = Some(value(args, &mut i, inline.clone(), &flag)?)
            }
            "--user" => o.user = Some(value(args, &mut i, inline.clone(), &flag)?),
            "--wait" => {
                o.wait = Some(
                    value(args, &mut i, inline.clone(), &flag)?
                        .parse()
                        .map_err(|_| "--wait takes seconds".to_string())?,
                )
            }
            "--handshake-timeout" => {
                o.handshake_timeout = Some(
                    value(args, &mut i, inline.clone(), &flag)?
                        .parse()
                        .map_err(|_| "--handshake-timeout takes seconds".to_string())?,
                )
            }
            "--insecure" => o.insecure = true,
            "--ws" => o.ws = true,
            "--once" => o.once = true,
            "--json" => o.json = true,
            "-h" | "--help" => return Err("__help__".into()),
            other if other.starts_with('-') => {
                return Err(format!("unknown option {other:?}"));
            }
            _ => o.positional.push(arg.clone()),
        }
        i += 1;
    }
    Ok(o)
}

fn value(
    args: &[String],
    i: &mut usize,
    inline: Option<String>,
    flag: &str,
) -> Result<String, String> {
    if let Some(v) = inline {
        return Ok(v);
    }
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))
}

fn parse_or_help(args: &[String]) -> Result<Opts, i32> {
    match parse(args) {
        Ok(o) => Ok(o),
        Err(e) if e == "__help__" => {
            print!("{USAGE}");
            Err(0)
        }
        Err(e) => {
            eprintln!("podssh: {e}");
            Err(2)
        }
    }
}

fn require(name: &str, v: Option<String>) -> Result<String, i32> {
    v.ok_or_else(|| {
        eprintln!("podssh: {name} is required");
        2
    })
}

fn protocol_of(o: &Opts) -> Result<Protocol, i32> {
    match o.protocol.as_deref() {
        None => Ok(Protocol::Podssh1),
        Some(s) => Protocol::parse(s).ok_or_else(|| {
            eprintln!("podssh: --protocol takes podssh1 or sandssh1");
            2
        }),
    }
}

fn dialer_of(o: &Opts) -> Result<Dialer, i32> {
    let tls_ca: Vec<PathBuf> = o.tls_ca.iter().map(PathBuf::from).collect();
    let proxy = crate::catalog::proxy(o.proxy.as_deref());
    crate::relay::dialer_for(proxy.as_deref(), o.insecure, &tls_ca, Duration::from_secs(15))
        .map_err(|e| {
            eprintln!("podssh: {e}");
            1
        })
}

fn cmd_probe(args: &[String]) -> i32 {
    let o = match parse_or_help(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let dialer = match dialer_of(&o) {
        Ok(d) => d,
        Err(c) => return c,
    };
    let mut proxies = Vec::new();
    if let Some(p) = crate::catalog::proxy(o.proxy.as_deref()) {
        proxies.push(p);
    }
    for e in crate::catalog::EGRESS {
        proxies.push(e.url.to_string());
    }
    let mut relays = crate::catalog::relay_candidates(&o.relays);
    if relays.is_empty() {
        relays = o.positional.clone();
    }
    let cfg = crate::probe::Config {
        dialer,
        relays,
        proxies,
        target: o.target.clone().unwrap_or_else(|| "railway.new:22".into()),
    };
    crate::probe::run(&cfg)
}

fn cmd_relay(args: &[String]) -> i32 {
    let o = match parse_or_help(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let listen = require("--listen", o.listen.clone()).unwrap_or_else(|c| std::process::exit(c));
    let key = crate::catalog::auth(o.key.as_deref());
    let cfg = crate::relay::Config {
        listen,
        key,
        tls_cert: o.tls_cert.clone().map(PathBuf::from),
        tls_key: o.tls_key.clone().map(PathBuf::from),
        ws: o.ws,
        once: o.once,
    };
    match crate::relay::run(cfg) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("podssh relay: {e}");
            1
        }
    }
}

fn cmd_serve(args: &[String]) -> i32 {
    let o = match parse_or_help(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let name = match require("--name", o.name.clone()) {
        Ok(n) => n,
        Err(c) => return c,
    };
    let protocol = match protocol_of(&o) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let dialer = match dialer_of(&o) {
        Ok(d) => d,
        Err(c) => return c,
    };
    if let Some(k) = &o.host_key {
        std::env::set_var("PODSSH_HOST_KEY", k);
    }
    if let Some(k) = &o.authorized_keys {
        std::env::set_var("PODSSH_AUTHORIZED_KEYS", k);
    }
    let server = match (&o.server, &o.forward) {
        (Some(_), Some(_)) => {
            eprintln!("podssh: --server and --forward are mutually exclusive");
            return 2;
        }
        (Some(c), None) => ServerSpec::Command(c.clone()),
        (None, Some(a)) => ServerSpec::Forward(a.clone()),
        (None, None) => ServerSpec::Auto,
    };
    let mut cfg = crate::serve::Config::new(crate::catalog::relay_candidates(&o.relays), name, dialer, server);
    cfg.auth = crate::catalog::auth(o.auth.as_deref());
    cfg.protocol = protocol;
    cfg.once = o.once;
    cfg.handshake_timeout = Duration::from_secs(o.handshake_timeout.unwrap_or(900));
    match crate::serve::run(&cfg) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("podssh serve: {e}");
            1
        }
    }
}

fn cmd_connect(args: &[String]) -> i32 {
    let o = match parse_or_help(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let name = match require("--name", o.name.clone()) {
        Ok(n) => n,
        Err(c) => return c,
    };
    let protocol = match protocol_of(&o) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let dialer = match dialer_of(&o) {
        Ok(d) => d,
        Err(c) => return c,
    };
    let cfg = crate::connect::Config {
        relays: crate::catalog::relay_candidates(&o.relays),
        name,
        auth: crate::catalog::auth(o.auth.as_deref()),
        protocol,
        dialer,
        wait: Duration::from_secs(o.wait.unwrap_or(30)),
        handshake_timeout: Duration::from_secs(o.handshake_timeout.unwrap_or(900)),
    };
    match crate::connect::run(&cfg) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("podssh connect: {e}");
            1
        }
    }
}

fn cmd_forward(args: &[String]) -> i32 {
    let o = match parse_or_help(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let target = match require("--target", o.target.clone()) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let dialer = match dialer_of(&o) {
        Ok(d) => d,
        Err(c) => return c,
    };
    let cfg = crate::connect::ForwardConfig {
        target,
        expect_banner: o.expect_banner.clone(),
        dialer,
    };
    match crate::connect::forward(&cfg) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("podssh forward: {e}");
            1
        }
    }
}

fn cmd_config(args: &[String]) -> i32 {
    let o = match parse_or_help(args) {
        Ok(o) => o,
        Err(c) => return c,
    };
    let name = match require("--name", o.name.clone()) {
        Ok(n) => n,
        Err(c) => return c,
    };
    let relay = crate::catalog::relay_candidates(&o.relays)
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            eprintln!("podssh: config needs --relay (or PODSSH_RELAY)");
            std::process::exit(2);
        });
    let auth = crate::catalog::auth(o.auth.as_deref());
    let proxy = crate::catalog::proxy(o.proxy.as_deref());
    let user = o.user.clone().unwrap_or_else(|| "root".to_string());
    let mut proxy_cmd = format!("podssh connect --relay {} --name {}", relay, name);
    if !auth.is_empty() {
        proxy_cmd.push_str(&format!(" --auth {auth}"));
    }
    if let Some(p) = proxy {
        proxy_cmd.push_str(&format!(" --proxy {p}"));
    }
    println!(
        "# podssh -- add to ~/.ssh/config; then `ssh {name}` is a normal session\n\
         Host {name}\n\
         \x20   User {user}\n\
         \x20   ProxyCommand {proxy_cmd}\n\
         \x20   ServerAliveInterval 15\n\
         \x20   ServerAliveCountMax 4\n\
         # the ssh session is end to end; the relay only sees ciphertext"
    );
    0
}

fn cmd_selftest(_args: &[String]) -> i32 {
    let mut ok = true;
    let check = |name: &str, pass: bool, ok: &mut bool| {
        println!("  {:<46} {}", name, if pass { "ok" } else { "FAIL" });
        *ok = *ok && pass;
    };
    let vectors = crate::util::sha1(b"abc");
    check(
        "sha1 RFC 3174 vector",
        vectors[0] == 0xa9 && vectors[19] == 0x9d,
        &mut ok,
    );
    check(
        "base64 RFC 4648 vector",
        crate::util::b64_encode(b"foobar") == "Zm9vYmFy",
        &mut ok,
    );
    // protocol framing over a unix socketpair, the shape every relay uses.
    let (a, b) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut a: Box<dyn crate::transport::Stream> = Box::new(crate::transport::Unix(a));
    let mut b: Box<dyn crate::transport::Stream> = Box::new(crate::transport::Unix(b));
    let h = std::thread::spawn(move || {
        let r = crate::protocol::accept(&mut *a, Duration::from_secs(2));
        let _ = std::io::Write::write_all(&mut a, b"OK\n");
        r.is_ok()
    });
    let greeted = crate::protocol::greet(
        &mut *b,
        Protocol::Podssh1,
        crate::protocol::Role::Node,
        "selftest",
        "k",
        Duration::from_secs(2),
    )
    .is_ok();
    check("protocol greet/accept framing", greeted && h.join().unwrap(), &mut ok);
    // exec transport: any command is a transport.
    let exec_ok = crate::transport::exec::Exec::spawn("cat")
        .map(|mut s| {
            use std::io::{Read, Write};
            s.write_all(b"x").and_then(|_| s.flush()).and_then(|_| {
                let mut buf = [0u8; 1];
                s.read_exact(&mut buf).map(|_| buf == *b"x")
            })
        })
        .map(|r| r.is_ok())
        .unwrap_or(false);
    check("exec:// transport round trip", exec_ok, &mut ok);
    println!("selftest: {}", if ok { "PASS" } else { "FAIL" });
    if ok {
        0
    } else {
        1
    }
}
