use super::*;

/// Refuse a cluster member no row names, in `admit`'s wording.
///
/// Expansion happens before admission, so the member (not the cluster)
/// is what has no row. The usage line matches `admit`'s: one refusal
/// shape for one surface.
pub fn refuse_member(verb: &str, member: &str, usage: &str) -> i32 {
    eprintln!(
        "podvm {verb}: unknown option {member:?}. It has no row in the parity \
         table; `podvm system info` lists every flag this verb takes"
    );
    eprint!("{usage}");
    podvm_image::error::EXIT_FLAG_ERROR
}

/// One `--filter key=value` predicate, for `ps` and `images`.
///
/// TODO/cli.md T-1331: caller-side, no new query language. `name=`
/// matches a substring; `label=` matches the labels of a container record
/// (T-1695) and nothing on an image, whose record keeps none. Anything else is the caller's mistake and refuses
/// naming the two keys that exist.
pub struct Filter {
    pub key: String,
    pub value: String,
}

/// Split one `--filter` value into its key and value.
pub fn parse_filter(raw: &str) -> Result<Filter, String> {
    match raw.split_once('=') {
        Some((k, v)) if !k.is_empty() => {
            let key = k.trim().to_string();
            if key == "name" || key == "label" {
                Ok(Filter {
                    key,
                    value: v.to_string(),
                })
            } else {
                Err(format!(
                    "--filter takes name= and label=, not {raw:?}: podvm records \
                     carry no other key to filter on"
                ))
            }
        }
        _ => Err(format!(
            "--filter takes key=value, not {raw:?}: `name=<substring>` selects, \
             `label=<key>[=<value>]` matches the labels of a run"
        )),
    }
}

/// `-aq` becomes `-a -q`; a member that takes a value consumes the rest
/// (`-t5` becomes `-t 5`, `-eFOO=bar` becomes `-e FOO=bar`,
/// `-f{{.Id}}` becomes `-f {{.Id}}`); an unknown member refuses naming
/// the member. Only the first member decides the shape: past a
/// value-taking member the rest is opaque, past a value-less one every
/// member is another flag. Verbs that stop at the image (`run`, `exec`)
/// expand only before it: `run IMG -la` passes `-la` to the payload,
/// exactly as their parsers do past the image name. Every other verb
/// reads its whole argv, and clusters expand anywhere in it. Long flags,
/// lone shorts, a lone `-`, and anything not opening with an
/// alphanumeric short pass through for the parser (and `admit`) to judge
/// as before: a value that starts with `-` must still use the `=` form.
pub fn expand(verb: &str, args: &[String]) -> Result<Vec<String>, String> {
    let under = rows_of(verb);
    // A leading positional is the image (or the whole argv is podvm's):
    // a value follows only its flag, so the walk starts past nothing.
    let bounded = CLUSTER_BOUNDARY.contains(&under);
    let mut prev_dash = !bounded;
    let mut out = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        let bytes = a.as_bytes();
        if bytes.len() > 2
            && bytes[0] == b'-'
            && bytes[1] != b'-'
            && bytes[1].is_ascii_alphanumeric()
        {
            let mut members = a[1..].chars();
            while let Some(c) = members.next() {
                let one = format!("-{c}");
                if CLUSTER_VALUES.contains(&(under, c)) {
                    out.push(one);
                    let rest = members.as_str();
                    if !rest.is_empty() {
                        out.push(rest.to_string());
                    }
                    break;
                }
                if !c.is_ascii_alphanumeric() || flag(under, &one).is_none() {
                    return Err(one);
                }
                out.push(one);
            }
            prev_dash = false;
            continue;
        }
        if a.starts_with('-') && a.len() > 1 {
            out.push(a.clone());
            prev_dash = true;
            continue;
        }
        out.push(a.clone());
        if prev_dash {
            prev_dash = false;
        } else if bounded {
            // The image: the payload owns everything from here, clusters
            // included.
            out.extend(args[i..].iter().cloned());
            break;
        }
    }
    Ok(out)
}
