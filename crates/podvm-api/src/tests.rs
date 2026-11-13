//! TODO/cli.md T-1745: the map of a request to argv, the refusals, and the
//! reader of the record.

use super::*;

fn full() -> Run {
    Run {
        image: "lab/one:1".into(),
        command: vec!["/hello".into(), "--list".into()],
        env: vec![("A".into(), "1 2".into())],
        volumes: vec![Volume {
            host: "/srv/data".into(),
            guest: "/data".into(),
            read_only: true,
        }],
        workdir: Some("/data".into()),
        labels: vec![("job".into(), "7".into())],
        ulimits: vec![Ulimit {
            name: "nofile".into(),
            soft: 64,
            hard: Some(128),
        }],
        timeout_s: Some(30),
        tier: Some(Tier::Machine),
        confine: Some("read=/data".into()),
        pull: Pull::Never,
    }
}

#[test]
fn a_request_maps_to_the_argv_of_run() {
    let a = full()
        .argv(Path::new("/p/r.json"), Some(Path::new("/p/env")))
        .unwrap();
    let want: Vec<&str> = vec![
        "run",
        "--rm",
        "--report",
        "/p/r.json",
        "--pull",
        "never",
        "--env-file",
        "/p/env",
        "-v",
        "/srv/data:/data:ro",
        "--workdir",
        "/data",
        "--label",
        "job=7",
        "--ulimit",
        "nofile=64:128",
        "--timeout",
        "30",
        "--podvm-tier",
        "machine",
        "--podvm-confine",
        "read=/data",
        "lab/one:1",
        "/hello",
        "--list",
    ];
    assert_eq!(a, want);
    assert!(
        !a.iter().any(|x| x.contains("1 2")),
        "an env value is never in argv"
    );
}

#[test]
fn a_request_that_a_flag_cannot_carry_is_refused() {
    let mut r = full();
    r.env = vec![("A".into(), "x\ny".into())];
    assert!(matches!(r.check(), Err(Error::Invalid(_))));
    r.env = vec![("A".into(), "s3cr3t ".into())];
    assert!(matches!(r.check(), Err(Error::Invalid(_))));
    let e = r.check().unwrap_err().to_string();
    assert!(
        !e.contains("s3cr3t"),
        "the message names the key, not the value: {e}"
    );
    let mut r = full();
    r.volumes[0].guest = "data".into();
    assert!(r.check().is_err());
    let mut r = full();
    r.image = "-x".into();
    assert!(r.check().is_err());
    let mut r = full();
    r.timeout_s = Some(0);
    assert!(r.check().is_err());
    assert!(
        full().argv(Path::new("/r"), None).is_err(),
        "env needs an env file"
    );
}

#[test]
fn the_job_writes_the_env_file_private_and_removes_its_directory() {
    let job = full().job(Path::new("/nonexistent/podvm")).unwrap();
    let env = job.dir.join("env");
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&job.dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(&env).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(std::fs::read_to_string(&env).unwrap(), "A=1 2\n");
    let dir = job.dir.clone();
    assert!(
        matches!(job.run(), Err(Error::Io(_))),
        "a missing binary does not start"
    );
    assert!(!dir.exists(), "the private directory goes");
}

#[test]
fn the_record_reads_and_another_version_is_refused() {
    let doc = r#"{"schema":1,"podvm":"0.2.0-beta.7","image":"lab/one:1","image_digest":null,
        "rung":"namespace","measured_by":"x","argv":["/hello"],"container":null,"detached":false,
        "exit_code":0,"signal":null,"refused":null,"started":"t","ended":"t","wall_ms":5,
        "user_ms":1,"system_ms":1,"max_rss_kib":900,"timed_out":false,"timeout_s":null,
        "limits":[],"no_new_privs":false,"confine":null,"labels":["job=7"],"degradations":[]}"#;
    let r = Report::parse(doc).unwrap();
    assert!(r.succeeded());
    assert_eq!(r.rung.as_deref(), Some("namespace"));
    assert_eq!(r.labels, ["job=7"]);
    let v2 = doc.replacen("\"schema\":1", "\"schema\":2", 1);
    assert!(Report::parse(&v2)
        .unwrap_err()
        .to_string()
        .contains("version is 2"));
    let wrong = doc.replacen("\"exit_code\":0", "\"exit_code\":\"0\"", 1);
    assert!(Report::parse(&wrong).is_err());
}
