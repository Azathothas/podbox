//! The rendezvous relay. A relay is a dumb ciphertext pipe: it pairs one
//! `Node` with one `Client` by name and then moves bytes until either side
//! closes. It cannot read the ssh session and can at worst deny service.
//!
//! ⛔ It is a separate process (`podssh relay`), not a library the agent
//! embeds, because it has to live on a host the agent can dial. The sandbox
//! never listens on anything: TCP `bind(2)` is denied in the cages podbox
//! targets, so the conversation is always two outbound dials.
//!
//! A relay can be reached directly (`tls://`, `wss://`) or through the one
//! egress hop a sealed cage leaves open (`--proxy http-connect://...`).

use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::protocol::{self, Role};
use crate::transport::{self, Dialer, Stream};

/// How long a registered node is held before its dial is dropped. Finite so a
/// relay never accumulates abandoned sockets.
const MAX_IDLE: Duration = Duration::from_secs(900);
/// How long a peer has to state its handshake.
const HANDSHAKE: Duration = Duration::from_secs(30);
/// The per-direction read timeout the pump runs with.
pub const PUMP_TIMEOUT: Duration = Duration::from_millis(120);

pub struct Config {
    pub listen: String,
    pub key: String,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub ws: bool,
    pub once: bool,
}

struct SlotState {
    stream: Option<Box<dyn Stream>>,
    paired: bool,
}

struct Slot {
    state: Mutex<SlotState>,
    cv: Condvar,
}

impl Slot {
    fn new(stream: Box<dyn Stream>) -> Self {
        Slot {
            state: Mutex::new(SlotState {
                stream: Some(stream),
                paired: false,
            }),
            cv: Condvar::new(),
        }
    }
}

type Waiting = Arc<Mutex<HashMap<String, VecDeque<Arc<Slot>>>>>;

pub fn run(cfg: Config) -> io::Result<()> {
    let listener = Listener::bind(&cfg.listen)?;
    let tls = match (&cfg.tls_cert, &cfg.tls_key) {
        (Some(c), Some(k)) => Some(transport::tls::server_config(c, k)?),
        (None, None) => None,
        _ => {
            return Err(io::Error::other(
                "relay needs both --tls-cert and --tls-key, or neither",
            ))
        }
    };
    let waiting: Waiting = Arc::new(Mutex::new(HashMap::new()));
    let done = Arc::new(AtomicBool::new(false));
    let key = Arc::new(cfg.key.clone());
    eprintln!(
        "podssh relay: listening on {} ({}{}{})",
        cfg.listen,
        if tls.is_some() { "tls+" } else { "" },
        if cfg.ws { "ws" } else { "raw" },
        if cfg.key.is_empty() {
            ", NO AUTH KEY"
        } else {
            ", shared key required"
        }
    );
    while !done.load(Ordering::SeqCst) {
        let stream = listener.accept()?;
        let tls = tls.clone();
        let ws = cfg.ws;
        let key = key.clone();
        let waiting = waiting.clone();
        let done = done.clone();
        let once = cfg.once;
        std::thread::spawn(move || {
            if let Err(e) = handle(stream, tls, ws, key, waiting, once, done.clone()) {
                eprintln!("podssh relay: connection ended: {e}");
            }
        });
    }
    Ok(())
}

fn handle(
    mut stream: Box<dyn Stream>,
    tls: Option<Arc<rustls::ServerConfig>>,
    ws: bool,
    key: Arc<String>,
    waiting: Waiting,
    once: bool,
    done: Arc<AtomicBool>,
) -> io::Result<()> {
    if let Some(cfg) = tls {
        stream = Box::new(transport::tls::ServerTls::accept(stream, cfg)?);
    }
    if ws {
        stream = Box::new(transport::ws::Ws::server(stream)?);
    }
    stream.set_read_timeout(Some(HANDSHAKE))?;
    let (proto, role, name, auth) = match protocol::accept(&mut *stream, HANDSHAKE) {
        Ok(v) => v,
        Err(e) => {
            let _ = stream.write_all(b"ERR bad handshake\n");
            let _ = stream.flush();
            return Err(e);
        }
    };
    let _ = proto;
    if !protocol::is_valid_name(&name) {
        let _ = stream.write_all(b"ERR bad name\n");
        let _ = stream.flush();
        return Err(io::Error::other("invalid node name"));
    }
    if !key.is_empty() && auth != *key {
        let _ = stream.write_all(b"ERR bad key\n");
        let _ = stream.flush();
        return Err(io::Error::other("bad key"));
    }

    match role {
        Role::Probe => {
            stream.write_all(b"OK\n")?;
            stream.flush()?;
            Ok(())
        }
        Role::Node => {
            let slot = Arc::new(Slot::new(stream));
            {
                let mut map = waiting.lock().unwrap();
                map.entry(name.clone()).or_default().push_back(slot.clone());
            }
            eprintln!("podssh relay: node {name} waiting");
            let mut st = slot.state.lock().unwrap();
            loop {
                if st.paired {
                    break;
                }
                let (g, t) = slot.cv.wait_timeout(st, MAX_IDLE).unwrap();
                st = g;
                if t.timed_out() && !st.paired {
                    drop(st);
                    let mut map = waiting.lock().unwrap();
                    if let Some(q) = map.get_mut(&name) {
                        q.retain(|s| !Arc::ptr_eq(s, &slot));
                        if q.is_empty() {
                            map.remove(&name);
                        }
                    }
                    return Ok(());
                }
            }
            // From here the client thread owns this socket and writes the
            // verdict to the node; this thread is done with it.
            Ok(())
        }
        Role::Client => {
            let slot = {
                let mut map = waiting.lock().unwrap();
                match map.get_mut(&name).and_then(|q| q.pop_front()) {
                    Some(s) => s,
                    None => {
                        let _ = stream.write_all(b"ERR node not connected; retry\n");
                        let _ = stream.flush();
                        return Err(io::Error::other("no node waiting"));
                    }
                }
            };
            let node_stream = {
                let mut st = slot.state.lock().unwrap();
                let s = st.stream.take();
                st.paired = true;
                slot.cv.notify_all();
                s
            };
            let Some(mut node) = node_stream else {
                let _ = stream.write_all(b"ERR node vanished\n");
                let _ = stream.flush();
                return Err(io::Error::other("node stream already taken"));
            };
            node.write_all(b"OK\n")?;
            node.flush()?;
            stream.write_all(b"OK\n")?;
            stream.flush()?;
            eprintln!("podssh relay: paired client with node {name}");
            node.set_read_timeout(Some(PUMP_TIMEOUT))?;
            stream.set_read_timeout(Some(PUMP_TIMEOUT))?;
            let r = crate::util::pump(&mut *node, &mut *stream);
            if once {
                done.store(true, Ordering::SeqCst);
            }
            r
        }
    }
}

/// A TCP or unix listener behind one type, because a relay is deployed with
/// either and the rest of the code should not care.
enum Listener {
    Tcp(std::net::TcpListener),
    Unix(std::os::unix::net::UnixListener),
}

impl Listener {
    fn bind(addr: &str) -> io::Result<Self> {
        let path = addr
            .strip_prefix("unix://")
            .or_else(|| addr.strip_prefix("unix:"))
            .or_else(|| addr.starts_with('/').then_some(addr));
        if let Some(path) = path {
            let _ = std::fs::remove_file(path);
            return Ok(Listener::Unix(std::os::unix::net::UnixListener::bind(
                path,
            )?));
        }
        Ok(Listener::Tcp(std::net::TcpListener::bind(addr)?))
    }

    fn accept(&self) -> io::Result<Box<dyn Stream>> {
        match self {
            Listener::Tcp(l) => {
                let (s, _) = l.accept()?;
                let _ = s.set_nodelay(true);
                Ok(Box::new(transport::Tcp(s)))
            }
            Listener::Unix(l) => {
                let (s, _) = l.accept()?;
                Ok(Box::new(transport::Unix(s)))
            }
        }
    }
}

/// Build a [`Dialer`] for a peer from the common CLI flags.
pub fn dialer_for(
    proxy: Option<&str>,
    insecure: bool,
    tls_ca: &[PathBuf],
    timeout: Duration,
) -> io::Result<Dialer> {
    let proxy = match proxy {
        Some(p) => Some(transport::proxy::Proxy::parse(p)?),
        None => None,
    };
    let tls = Arc::new(transport::tls::ClientConfig::new(insecure, tls_ca)?);
    Ok(Dialer::new(proxy, tls, timeout))
}
