//! TODO/machine.md T-1718: the HTTP parse, the image form, the command
//! form, and one `run` against a stand-in incusd on a unix socket.

use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::client::{metadata, parse};
use super::image::{alias, intended, unified_tarball, IMAGE_FORMAT};
use super::run::{command, env_map, Job};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-t1718-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_plain_and_a_chunked_answer_parse() {
    let r = parse(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi").unwrap();
    assert_eq!((r.status, r.body.as_slice()), (200, b"hi".as_slice()));
    let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n2;x=y\r\nde\r\n0\r\n\r\n";
    assert_eq!(parse(raw).unwrap().body, b"abcde");
    assert!(parse(b"HTTP/1.1 200 OK\r\n").is_err(), "no end of headers");
    let cut = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n9\r\nabc";
    assert!(parse(cut).is_err(), "a cut chunk");
}

#[test]
fn an_error_envelope_gives_the_text_of_incus() {
    let body = br#"{"type":"error","error":"not found","error_code":404}"#;
    let mut raw = b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec();
    raw.extend_from_slice(body);
    let e = metadata(&parse(&raw).unwrap()).unwrap_err();
    assert!(e.contains("not found") && e.contains("404"), "{e}");
    let ok = "HTTP/1.1 200 OK\r\n\r\n{\"type\":\"sync\",\"metadata\":{\"a\":1}}";
    assert_eq!(
        metadata(&parse(ok.as_bytes()).unwrap()).unwrap(),
        json!({"a": 1})
    );
}

#[test]
fn the_alias_names_the_digest_and_the_format() {
    let a = alias("sha256:0123456789abcdef0123");
    assert_eq!(a, format!("podvm-0123456789abcdef-v{IMAGE_FORMAT}"));
    assert_ne!(a, alias("sha256:0123456789abcdee0123"));
}

#[test]
fn the_command_and_the_environment_take_the_incus_form() {
    let argv = vec!["cat".to_string(), "a b".to_string()];
    assert_eq!(command(&argv, false), argv);
    let with = command(&argv, true);
    assert_eq!(with[0], "/bin/sh");
    assert_eq!(
        &with[4..],
        argv.as_slice(),
        "the argv stays apart, unquoted"
    );
    let env = env_map(&["A=1".into(), "B=x=y".into(), "NOEQ".into()]);
    assert_eq!(env, json!({"A": "1", "B": "x=y"}));
}

#[test]
fn the_sidecar_gives_the_intended_ids_and_a_later_line_wins() {
    let side = concat!(
        r#"{"path":"etc/shadow","uid":0,"gid":0,"mode":"0600","applied":{"uid":5,"gid":5}}"#,
        "\n",
        r#"{"path":"etc/shadow","uid":0,"gid":42,"mode":"0640","applied":{"uid":5,"gid":5},"reason":"x"}"#,
        "\nnot json\n",
    );
    let m = intended(side);
    assert_eq!(m.len(), 1);
    assert_eq!(m["etc/shadow"], (0, 42, 0o640));
}

#[test]
fn the_unified_tarball_carries_the_metadata_and_the_intended_owners() {
    let d = scratch("tar");
    std::fs::create_dir_all(d.join("img/rootfs/bin")).unwrap();
    std::fs::create_dir_all(d.join("img/rootfs/etc")).unwrap();
    std::fs::write(d.join("img/rootfs/bin/sh"), b"#!sh").unwrap();
    std::fs::write(d.join("img/rootfs/etc/shadow"), b"root:*").unwrap();
    std::os::unix::fs::symlink("sh", d.join("img/rootfs/bin/ash")).unwrap();
    std::fs::write(
        d.join("img/.meta.jsonl"),
        r#"{"path":"etc/shadow","uid":0,"gid":42,"mode":"0640","applied":{"uid":5,"gid":5}}"#,
    )
    .unwrap();
    let out = d.join("img.tar");
    unified_tarball(&d.join("img/rootfs"), &out).unwrap();
    let mut a = tar::Archive::new(std::fs::File::open(&out).unwrap());
    let mut seen = Vec::new();
    for e in a.entries().unwrap() {
        let mut e = e.unwrap();
        let p = e.path().unwrap().to_string_lossy().into_owned();
        let h = e.header();
        let ids = (h.uid().unwrap(), h.gid().unwrap(), h.mode().unwrap());
        match p.as_str() {
            "rootfs/etc/shadow" => assert_eq!(ids, (0, 42, 0o640), "the intended owner"),
            "rootfs/bin/ash" => assert!(h.entry_type().is_symlink(), "a link stays a link"),
            "metadata.yaml" => {
                let mut t = String::new();
                e.read_to_string(&mut t).unwrap();
                assert!(
                    !t.contains("creation_date: 0\n"),
                    "incus refuses a date of 0: {t}"
                );
            }
            _ => assert_eq!((ids.0, ids.1), (0, 0), "{p}: no record gives the owner 0"),
        }
        seen.push(p);
    }
    for want in [
        "metadata.yaml",
        "rootfs/bin",
        "rootfs/bin/sh",
        "rootfs/bin/ash",
        "rootfs/etc/shadow",
    ] {
        assert!(seen.iter().any(|s| s == want), "{want} in {seen:?}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// One request the stand-in got: the method, the path, and the body.
type Seen = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;

fn read_request(s: &mut std::os::unix::net::UnixStream) -> (String, String, Vec<u8>) {
    let mut raw = Vec::new();
    let mut b = [0u8; 65536];
    let end = loop {
        let n = s.read(&mut b).unwrap();
        raw.extend_from_slice(&b[..n]);
        if let Some(p) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        assert!(n > 0, "the client ended inside the headers");
    };
    let head = String::from_utf8_lossy(&raw[..end]).into_owned();
    let len = head
        .lines()
        .find_map(|l| l.strip_prefix("Content-Length: "))
        .map(|v| v.trim().parse::<usize>().unwrap())
        .unwrap_or(0);
    while raw.len() < end + 4 + len {
        let n = s.read(&mut b).unwrap();
        assert!(n > 0, "the client ended inside the body");
        raw.extend_from_slice(&b[..n]);
    }
    let mut first = head.split(' ');
    let method = first.next().unwrap().to_string();
    let path = first.next().unwrap().to_string();
    (method, path, raw[end + 4..end + 4 + len].to_vec())
}

/// The answer of the stand-in: a sync envelope, an async one, an error,
/// or raw bytes (chunked, as incus sends a log).
fn answer(method: &str, path: &str) -> (u16, Vec<u8>, bool) {
    let sync = |m: Value| {
        (
            200,
            json!({"type": "sync", "metadata": m})
                .to_string()
                .into_bytes(),
            false,
        )
    };
    let op = |id: &str| {
        let v =
            json!({"type": "async", "operation": format!("/1.0/operations/{id}"), "metadata": {}});
        (202, v.to_string().into_bytes(), false)
    };
    let done = |m: Value| sync(json!({"status_code": 200, "metadata": m}));
    match (method, path) {
        ("GET", "/1.0") => sync(json!({"environment": {"server_version": "6.0.0"}})),
        ("GET", p) if p.starts_with("/1.0/images/aliases/") => (
            404,
            br#"{"type":"error","error":"not found","error_code":404}"#.to_vec(),
            false,
        ),
        ("POST", "/1.0/images") => op("import"),
        ("GET", "/1.0/operations/import/wait?timeout=600") => done(json!({"fingerprint": "f00d"})),
        ("POST", "/1.0/instances") => op("create"),
        ("POST", p) if p.ends_with("/exec") => op("exec"),
        ("GET", "/1.0/operations/exec/wait?timeout=600") => done(json!({
            "return": 7,
            "output": {"1": "/1.0/logs/out", "2": "/1.0/logs/err"},
        })),
        ("GET", "/1.0/logs/out") => (200, b"5\r\nhello\r\n0\r\n\r\n".to_vec(), true),
        ("GET", "/1.0/logs/err") => (200, b"0\r\n\r\n".to_vec(), true),
        ("GET", p) if p.starts_with("/1.0/operations/") => done(json!({})),
        ("PUT", _) | ("DELETE", _) => op("state"),
        _ => sync(json!({})),
    }
}

fn stand_in(sock: &std::path::Path) -> Seen {
    let l = UnixListener::bind(sock).unwrap();
    let seen: Seen = Arc::default();
    let log = seen.clone();
    std::thread::spawn(move || {
        for s in l.incoming() {
            let mut s = s.unwrap();
            let (m, p, body) = read_request(&mut s);
            let (status, out, chunked) = answer(&m, &p);
            log.lock().unwrap().push((m, p, body));
            let te = if chunked {
                "Transfer-Encoding: chunked\r\n"
            } else {
                ""
            };
            let head = format!("HTTP/1.1 {status} X\r\n{te}Connection: close\r\n\r\n");
            let _ = s.write_all(head.as_bytes());
            let _ = s.write_all(&out);
        }
    });
    seen
}

#[test]
fn a_run_imports_creates_pushes_starts_execs_and_deletes() {
    let d = scratch("run");
    std::fs::create_dir_all(d.join("root/bin")).unwrap();
    std::fs::write(d.join("root/bin/sh"), b"#!sh").unwrap();
    let sock = d.join("unix.socket");
    let seen = stand_in(&sock);
    // SAFETY: only this test reads PODVM_INCUS_SOCKET.
    unsafe { std::env::set_var("PODVM_INCUS_SOCKET", &sock) };
    let code = super::run::run(&Job {
        verb: "run",
        store_root: &d,
        rootfs: &d.join("root"),
        digest: "sha256:00112233445566778899",
        argv: vec!["echo".into(), "hello".into()],
        env: vec!["PATH=/bin".into()],
        workdir: "/".into(),
        stdin: Some(b"in".to_vec()),
    });
    unsafe { std::env::remove_var("PODVM_INCUS_SOCKET") };
    assert_eq!(code, 7, "the exit code of the command");
    let seen = seen.lock().unwrap().clone();
    let calls: Vec<String> = seen.iter().map(|(m, p, _)| format!("{m} {p}|")).collect();
    let at = |want: &str| {
        calls
            .iter()
            .position(|c| c.starts_with(want))
            .unwrap_or_else(|| panic!("{want} in {calls:#?}"))
    };
    let order = [
        "POST /1.0/images",
        "POST /1.0/images/aliases",
        "POST /1.0/instances|",
        "POST /1.0/instances/podvm-",
        "PUT /1.0/instances/podvm-",
        "DELETE /1.0/instances/podvm-",
    ];
    let pos: Vec<usize> = order.iter().map(|w| at(w)).collect();
    assert!(pos.windows(2).all(|w| w[0] < w[1]), "{calls:#?}");
    let body = |want: &str| -> Value {
        let (_, _, b) = &seen[at(want)];
        serde_json::from_slice(b).unwrap()
    };
    let create = body("POST /1.0/instances|");
    assert_eq!(
        create["config"]["raw.lxc"],
        "lxc.init.cmd = /.podvm/podvm guest-hold"
    );
    assert_eq!(body("POST /1.0/images/aliases")["target"], "f00d");
    let exec = seen
        .iter()
        .find(|(m, p, _)| m == "POST" && p.ends_with("/exec"))
        .map(|(_, _, b)| serde_json::from_slice::<Value>(b).unwrap())
        .unwrap();
    assert_eq!(
        exec["command"],
        json!([
            "/bin/sh",
            "-c",
            "exec \"$@\" </.podvm/stdin",
            "sh",
            "echo",
            "hello"
        ])
    );
    assert_eq!(exec["environment"], json!({"PATH": "/bin"}));
    assert_eq!(exec["record-output"], true);
    for p in ["/.podvm/podvm", "/.podvm/stdin"] {
        assert!(
            calls
                .iter()
                .any(|c| c.ends_with(&format!("files?path={p}|"))),
            "{p}: {calls:#?}"
        );
    }
    let stdin = seen
        .iter()
        .find(|(_, p, _)| p.ends_with("path=/.podvm/stdin"))
        .unwrap();
    assert_eq!(stdin.2, b"in");
    assert!(!d
        .join(format!(
            "incus/{}.tar",
            alias("sha256:00112233445566778899")
        ))
        .exists());
    let _ = std::fs::remove_dir_all(&d);
}
