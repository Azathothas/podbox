//! The flags of a language runtime with its own permission model
//! (TODO/cli.md T-1661, issue 176).
//!
//! A profile states each path once, in `fs.read` and `fs.write`. podvm makes
//! the runtime's flags from them, and the command runs under the runtime
//! with these flags: `--profile node-permission -- node a.js` runs `node
//! --permission --allow-fs-read=... a.js`. The runtime is then the boundary
//! inside the payload; podvm's path map stays the boundary of the payload.
//! A runtime check never stands in for the path map.

/// The runtimes that a profile can name.
pub const RUNTIMES: &[&str] = &["node", "deno", "bun", "wasmtime", "workerd"];

/// The command prefix of `runtime` with the flags of `read` and `write`,
/// and the words of a command that the prefix already holds (the program,
/// and the subcommand where the runtime has one).
pub fn prefix(
    runtime: &str,
    read: &[String],
    write: &[String],
) -> Result<(Vec<String>, Vec<String>), String> {
    let p = |s: &str| s.to_string();
    // Each path that the payload writes, it also reads; each path once.
    let mut reads: Vec<&String> = Vec::new();
    for r in read.iter().chain(write.iter()) {
        if !reads.contains(&r) {
            reads.push(r);
        }
    }
    Ok(match runtime {
        // Node's permission model (node 22 and later): with `--permission`,
        // each read and each write outside the lists is ERR_ACCESS_DENIED,
        // and a child process and a worker are refused.
        "node" => {
            let mut a = vec![p("node"), p("--permission")];
            a.extend(reads.iter().map(|r| format!("--allow-fs-read={r}")));
            a.extend(write.iter().map(|w| format!("--allow-fs-write={w}")));
            (a, vec![p("node")])
        }
        // Deno denies each capability that no flag grants: env, net, run
        // and ffi are NotCapable, and `--no-prompt` makes a denial an error.
        "deno" => {
            let mut a = vec![p("deno"), p("run"), p("--no-prompt")];
            if !reads.is_empty() {
                let r: Vec<&str> = reads.iter().map(|s| s.as_str()).collect();
                a.push(format!("--allow-read={}", r.join(",")));
            }
            if !write.is_empty() {
                a.push(format!("--allow-write={}", write.join(",")));
            }
            (a, vec![p("deno"), p("run")])
        }
        // Bun has no permission model: podvm's path map is the only
        // boundary, and the profile says so in `leaves_out`.
        "bun" => (vec![p("bun")], vec![p("bun")]),
        // WASI: the module sees only its preopened directories.
        // The program is podvm's pin in the store (super::tools).
        "wasmtime" => {
            let mut a = vec![super::tools::WASMTIME.guest_program(), p("run")];
            a.extend(reads.iter().map(|d| format!("--dir={d}")));
            (a, vec![p("wasmtime"), p("run")])
        }
        // workerd: `workerd test` of a config that podvm writes, with the
        // worker `/workspace/worker.js` and no outbound service; the
        // binary is podvm's pin in the store (super::workerd).
        "workerd" => (super::workerd::prefix(), vec![p("workerd")]),
        other => {
            return Err(format!(
                "`runtime` takes {}, not {other:?}",
                RUNTIMES.join(", ")
            ))
        }
    })
}

/// The command under `prefix`, without the words of `drops` that the
/// command repeats: `node a.js` and `a.js` give the same argv.
pub fn under(prefix: &[String], drops: &[String], argv: Vec<String>) -> Vec<String> {
    if prefix.is_empty() || argv.is_empty() {
        return argv;
    }
    let mut rest = argv.as_slice();
    for d in drops {
        match rest.first() {
            Some(w) if w == d => rest = &rest[1..],
            _ => break,
        }
    }
    prefix.iter().chain(rest.iter()).cloned().collect()
}

/// The keys of a runtime agree (TODO/cli.md T-1661): `fs.read`, `fs.write`
/// and `fs.exec` need a runtime or the confine layer, and a runtime and
/// `fs.wrap` both prefix the command, so a profile has one of them.
pub(super) fn check(c: &super::model::Checked) -> Result<(), String> {
    match &c.runtime {
        None if !c.confine && !(c.read.is_empty() && c.write.is_empty() && c.exec.is_empty()) => {
            Err(
                "`fs.read` and `fs.write` give the flags of a runtime or the confine layer, \
             and the profile names no `runtime` and no `confine = true` (TODO/cli.md \
             T-1661, TODO/enter.md T-1658)"
                    .to_string(),
            )
        }
        Some(_) if !c.wrap.is_empty() => {
            Err("`runtime` and `fs.wrap` both prefix the command: give one of them".to_string())
        }
        Some(r) => prefix(r, &c.read, &c.write).map(|_| ()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn each_runtime_gets_its_flags_from_the_paths() {
        let (n, d) = prefix("node", &v(&["/workspace"]), &v(&["/workspace/out"])).unwrap();
        assert_eq!(
            n,
            v(&[
                "node",
                "--permission",
                "--allow-fs-read=/workspace",
                "--allow-fs-read=/workspace/out",
                "--allow-fs-write=/workspace/out"
            ])
        );
        assert_eq!(d, v(&["node"]));
        let (n, _) = prefix("deno", &v(&["/workspace"]), &[]).unwrap();
        assert_eq!(
            n,
            v(&["deno", "run", "--no-prompt", "--allow-read=/workspace"])
        );
        let (n, _) = prefix("wasmtime", &v(&["/workspace"]), &v(&["/workspace"])).unwrap();
        assert_eq!(
            n,
            v(&["/opt/podvm-wasmtime/wasmtime", "run", "--dir=/workspace"])
        );
        assert!(prefix("python", &[], &[]).unwrap_err().contains("python"));
    }

    #[test]
    fn a_repeated_program_is_not_given_twice() {
        let (p, d) = prefix("deno", &[], &[]).unwrap();
        let want = v(&["deno", "run", "--no-prompt", "a.ts"]);
        assert_eq!(under(&p, &d, v(&["a.ts"])), want);
        assert_eq!(under(&p, &d, v(&["deno", "a.ts"])), want);
        assert_eq!(under(&p, &d, v(&["deno", "run", "a.ts"])), want);
        assert!(under(&p, &d, Vec::new()).is_empty());
    }
}
