//! TODO/machine.md T-1718: the HTTP parse, the image form, the command
//! form, and one `run` against a stand-in incusd on a unix socket.

use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::image::{alias, intended, unified_tarball, IMAGE_FORMAT};
use super::run::{command, env_map, Job};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("podvm-t1718-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_alias_names_the_digest_and_the_format() {
    let me = "fedcba9876543210ffff";
    let a = alias("sha256:0123456789abcdef0123", me);
    assert_eq!(
        a,
        format!("podvm-0123456789abcdef-fedcba9876543210-v{IMAGE_FORMAT}")
    );
    assert_ne!(a, alias("sha256:0123456789abcdee0123", me));
    // TODO/machine.md T-1722: another podvm build is another image.
    assert_ne!(a, alias("sha256:0123456789abcdef0123", "0000000000000000"));
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
    let want: std::collections::BTreeMap<String, String> = [("A", "1"), ("B", "x=y")]
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .into();
    assert_eq!(env, want);
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
    unified_tarball(&d.join("img/rootfs"), &out, b"podvm-bytes").unwrap();
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
            "rootfs/.podvm/podvm" => {
                assert_eq!(ids, (0, 0, 0o755), "podvm is the init (T-1722)");
                let mut b = Vec::new();
                e.read_to_end(&mut b).unwrap();
                assert_eq!(b, b"podvm-bytes");
            }
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
    if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        while !raw[end + 4..].ends_with(b"0\r\n\r\n") {
            let n = s.read(&mut b).unwrap();
            assert!(n > 0, "the client ended inside a chunked body");
            raw.extend_from_slice(&b[..n]);
        }
        let mut body = Vec::new();
        let mut rest = &raw[end + 4..];
        loop {
            let eol = rest.windows(2).position(|w| w == b"\r\n").unwrap();
            let size = usize::from_str_radix(std::str::from_utf8(&rest[..eol]).unwrap().trim(), 16)
                .unwrap();
            rest = &rest[eol + 2..];
            if size == 0 {
                break;
            }
            body.extend_from_slice(&rest[..size]);
            rest = &rest[size + 2..];
        }
        let mut first = head.split(' ');
        let method = first.next().unwrap().to_string();
        let path = first
            .next()
            .unwrap()
            .replace("%2F", "/")
            .replace("%2f", "/");
        return (method, path, body);
    }
    while raw.len() < end + 4 + len {
        let n = s.read(&mut b).unwrap();
        assert!(n > 0, "the client ended inside the body");
        raw.extend_from_slice(&b[..n]);
    }
    let mut first = head.split(' ');
    let method = first.next().unwrap().to_string();
    // A query of the crate escapes the slashes of a path.
    let path = first
        .next()
        .unwrap()
        .replace("%2F", "/")
        .replace("%2f", "/");
    (method, path, raw[end + 4..end + 4 + len].to_vec())
}

/// The answer of the stand-in: a sync envelope, an async one, an error,
/// or raw bytes (chunked, as incus sends a log).
fn answer(method: &str, path: &str, inst: &str) -> (u16, Vec<u8>, bool) {
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
    let log = format!("/1.0/instances/{inst}/logs/exec-output/exec_1");
    match (method, path) {
        ("GET", "/1.0") => sync(json!({
            "api_extensions": pod_incus::NEEDED,
            "api_status": "stable",
            "api_version": "1.0",
            "auth": "trusted",
            "public": false,
            "auth_methods": [],
            "config": {},
            "environment": {"server_version": "6.0.0", "driver": "lxc", "server": "incus"},
        })),
        ("GET", p) if p.starts_with("/1.0/images/aliases/") => (
            404,
            br#"{"type":"error","error":"not found","error_code":404}"#.to_vec(),
            false,
        ),
        ("POST", p) if p.starts_with("/1.0/images/aliases") => sync(json!({})),
        ("POST", "/1.0/images") => op("import"),
        ("GET", p) if p.starts_with("/1.0/operations/import/wait") => {
            done(json!({"fingerprint": "f00d", "size": 1}))
        }
        ("POST", "/1.0/instances") => op("create"),
        ("POST", p) if p.ends_with("/exec") => op("exec"),
        ("GET", p) if p.starts_with("/1.0/operations/exec/wait") => done(json!({
            "return": 7,
            "output": {"1": format!("{log}.stdout"), "2": format!("{log}.stderr")},
        })),
        ("GET", p) if p.ends_with(".stdout") => (200, b"5\r\nhello\r\n0\r\n\r\n".to_vec(), true),
        ("GET", p) if p.ends_with(".stderr") => (200, b"0\r\n\r\n".to_vec(), true),
        ("DELETE", p) if p.contains("/logs/") => sync(json!({})),
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
        let mut inst = String::new();
        for s in l.incoming() {
            let mut s = s.unwrap();
            let (m, p, body) = read_request(&mut s);
            if m == "POST" && p.ends_with("/exec") {
                inst = p
                    .trim_start_matches("/1.0/instances/")
                    .trim_end_matches("/exec")
                    .to_string();
            }
            let (status, out, chunked) = answer(&m, &p, &inst);
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
        volumes: vec![
            ("/h/ws".into(), "/workspace".into(), false),
            ("/h/cfg".into(), "/cfg".into(), true),
        ],
        detach: None,
        net_none: false,
        channel: None,
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
    // TODO/machine.md T-1722: each -v is a disk device; :ro is read-only.
    assert_eq!(
        create["devices"],
        json!({
            "podvm-vol0": {"type": "disk", "source": "/h/ws", "path": "/workspace"},
            "podvm-vol1": {"type": "disk", "source": "/h/cfg", "path": "/cfg", "readonly": "true"}
        })
    );
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
    assert!(
        calls
            .iter()
            .any(|c| c.ends_with("files?path=/.podvm/stdin|")),
        "{calls:#?}"
    );
    // TODO/machine.md T-1722: podvm is in the image, not pushed for a run.
    assert!(
        !calls
            .iter()
            .any(|c| c.ends_with("files?path=/.podvm/podvm|")),
        "{calls:#?}"
    );
    let stdin = seen
        .iter()
        .find(|(_, p, _)| p.ends_with("path=/.podvm/stdin"))
        .unwrap();
    assert_eq!(stdin.2, b"in");
    let tars = std::fs::read_dir(d.join("incus"))
        .map(|r| r.count())
        .unwrap_or(0);
    assert_eq!(tars, 0, "the tarball of the import goes");
    let _ = std::fs::remove_dir_all(&d);
}

/// TODO/machine.md T-1722: the flag of the tier on a verb of an instance.
#[test]
fn the_tier_flag_selects_the_incus_verbs_and_leaves_the_rest() {
    use super::verbs::{valid_name, wants};
    let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(wants(&a(&["-a", "--podvm-tier=incus"])), Some(a(&["-a"])));
    assert_eq!(
        wants(&a(&["--podvm-tier", "incus", "web"])),
        Some(a(&["web"]))
    );
    assert_eq!(wants(&a(&["-a"])), None);
    assert_eq!(wants(&a(&["--podvm-tier=machine"])), None);
    assert!(valid_name("web-1"));
    assert!(!valid_name("1web"));
    assert!(!valid_name("web_1"));
    assert!(!valid_name(&"a".repeat(64)));
}

/// The command file of an instance that stays reads back as it was written.
#[test]
fn the_command_file_reads_back() {
    let c = super::Cmd {
        argv: vec!["sh".into(), "-c".into(), "echo 'a b'".into()],
        env: vec!["A=1".into()],
        cwd: "/w".into(),
    };
    assert_eq!(super::Cmd::parse(&c.to_json()).unwrap(), c);
}

/// TODO/cli.md T-1811: another tier than incus on a lifecycle verb is a
/// usage refusal by name, in both spellings.
#[test]
fn eq_form_of_another_tier_on_a_lifecycle_verb_is_refused_by_name() {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        crate::incus::other_tier(&v(&["--podvm-tier=machine", "x"])),
        Some("machine".into())
    );
    assert_eq!(
        crate::incus::other_tier(&v(&["--podvm-tier", "chroot", "x"])),
        Some("chroot".into())
    );
    assert_eq!(
        crate::incus::other_tier(&v(&["--podvm-tier=incus", "x"])),
        None
    );
    assert_eq!(
        crate::incus::intercept(Some("start"), &v(&["--podvm-tier=machine", "nosuch"])),
        Some(podvm_image::error::EXIT_FLAG_ERROR)
    );
    assert_eq!(
        crate::incus::intercept(Some("run"), &v(&["--podvm-tier=machine"])),
        None
    );
}
