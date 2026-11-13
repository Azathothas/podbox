//! `inspect --podvm-tier=incus NAME...`, TODO/machine.md T-1722: the
//! state of an instance of podvm in incus, as JSON or through `--format`,
//! with the fields of [`FIELDS`].

use pod_incus::FileContent;
use podvm_image::error::{EXIT_CLI_ERROR, EXIT_FLAG_ERROR, EXIT_RUNTIME_ERROR};

use super::verbs::{block_on, connect, ours, IMAGE_KEY};

/// The fields of an instance. `EnteredRung` is `incus`; `ExitCode` is the
/// code of the command of a stopped instance, or a dash.
pub const FIELDS: &[&str] = &[
    "Id",
    "Name",
    "Image",
    "State",
    "Status",
    "Running",
    "ExitCode",
    "Created",
    "Cmd",
    "EnteredRung",
];

/// `inspect` of each name; the exit is 1 where a name is not an instance.
pub fn inspect(args: &[String]) -> i32 {
    let mut template: Option<String> = None;
    let mut names: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-f" | "--format" => match it.next() {
                Some(t) => template = Some(t.clone()),
                None => {
                    eprintln!("podvm inspect: --format needs a template");
                    return EXIT_FLAG_ERROR;
                }
            },
            t if t.starts_with("--format=") => template = Some(t["--format=".len()..].to_string()),
            f if f.starts_with('-') => {
                eprintln!(
                    "podvm inspect: {f} is not served with --podvm-tier=incus (TODO/machine.md T-1722)"
                );
                return EXIT_FLAG_ERROR;
            }
            n => names.push(n.to_string()),
        }
    }
    if names.is_empty() {
        eprintln!("podvm inspect: name an instance");
        return EXIT_FLAG_ERROR;
    }
    if let Some(t) = &template
        && let Err(bad) = crate::format::check(t, FIELDS)
    {
        eprintln!("podvm inspect: {bad}");
        return EXIT_CLI_ERROR;
    }
    block_on("inspect", async move {
        let c = match connect().await {
            Ok((c, _)) => c,
            Err(e) => {
                eprintln!("podvm inspect: incus tier: {e}");
                return EXIT_RUNTIME_ERROR;
            }
        };
        let mut code = 0;
        let mut docs = Vec::new();
        for n in &names {
            let i = match c.instance(n).await {
                Ok(i) if ours(&i) => i,
                Ok(_) => {
                    eprintln!("podvm inspect: {n}: not an instance of podvm");
                    code = 1;
                    continue;
                }
                Err(e) => {
                    eprintln!("podvm inspect: {n}: {e}");
                    code = 1;
                    continue;
                }
            };
            let exit = if i.status == "Running" {
                "-".to_string()
            } else {
                match c.get_file(n, super::EXIT).await {
                    Ok(f) => match f.content {
                        FileContent::Bytes(b) => String::from_utf8_lossy(&b).trim().to_string(),
                        _ => "-".to_string(),
                    },
                    Err(_) => "-".to_string(),
                }
            };
            let cmd = match c.get_file(n, super::CMD).await {
                Ok(f) => match f.content {
                    FileContent::Bytes(b) => {
                        super::Cmd::parse(&b).map(|c| c.argv).unwrap_or_default()
                    }
                    _ => Vec::new(),
                },
                Err(_) => Vec::new(),
            };
            let fields = fields(&i, &exit, &cmd);
            match &template {
                Some(t) => match crate::format::render(t, &fields) {
                    Ok(line) => println!("{line}"),
                    Err(bad) => {
                        eprintln!("podvm inspect: {bad}");
                        return EXIT_CLI_ERROR;
                    }
                },
                None => docs.push(json(&i, &exit, &cmd)),
            }
        }
        if template.is_none() && !docs.is_empty() {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::Value::Array(docs)).unwrap_or_default()
            );
        }
        code
    })
}

/// The fields of an instance for `--format`.
pub fn fields(i: &pod_incus::Instance, exit: &str, cmd: &[String]) -> Vec<(&'static str, String)> {
    let image = i
        .config
        .get(IMAGE_KEY)
        .cloned()
        .unwrap_or_else(|| "-".into());
    vec![
        ("Id", i.name.clone()),
        ("Name", i.name.clone()),
        ("Image", image),
        ("State", i.status.to_lowercase()),
        ("Status", i.status.clone()),
        ("Running", (i.status == "Running").to_string()),
        ("ExitCode", exit.to_string()),
        ("Created", i.created_at.clone()),
        ("Cmd", cmd.join(" ")),
        ("EnteredRung", "incus".to_string()),
    ]
}

/// The JSON of an instance, in docker's shape where a field has one.
pub fn json(i: &pod_incus::Instance, exit: &str, cmd: &[String]) -> serde_json::Value {
    serde_json::json!({
        "Id": i.name,
        "Name": i.name,
        "Image": i.config.get(IMAGE_KEY),
        "Created": i.created_at,
        "State": {
            "Status": i.status.to_lowercase(),
            "Running": i.status == "Running",
            "ExitCode": exit.parse::<i64>().ok(),
        },
        "Config": { "Cmd": cmd },
        "EnteredRung": "incus",
    })
}
