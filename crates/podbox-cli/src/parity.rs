//! The verb and flag parity table, as DATA.
//!
//! [`TODO/cli.md`](../../../TODO/cli.md) T-0801, `TOOL.md` section 6.8.
//!
//! ⭐ **A tool that needs its user to learn its differences has not replaced
//! anything.** Thousands of agents reach for `docker` because it is the only
//! container language they know, so the differences have to be enumerable
//! rather than discoverable: `podbox system info --format '{{json .Parity}}'`
//! prints this table, and a caller decides from it before running anything.
//!
//! ⛔ **This is the table, not a description of one.** The parsers ask it
//! whether a flag exists, so a flag with no row here is refused by the parser
//! with a message saying the table has no row for it, rather than quietly
//! working. That is what makes "the flag exists and is unlisted" impossible
//! instead of merely discouraged.
//!
//! ⭐ The posture is `dockless`'s, at
//! `references/ylang-ylang__dockless/tree/dockless/run.py:129-160`: it refuses
//! `run --name` up front because the engine beneath it cannot honour it, rather
//! than accepting the flag and losing it. ⛔ dockless carries no licence
//! statement of any kind ([`TODO/reference-map.md`](../../../TODO/reference-map.md)),
//! so the posture is adopted as a design and no line of it is copied.
//! ⚠ Its other half is worth keeping too: where it cannot determine the target
//! it says the guardrail itself is degraded and continues. A guard that goes
//! quiet is worse than one that says it went quiet.

/// The four statuses `TOOL.md` section 6.8 defines, and podbox has no fifth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Real semantics: what docker does, for the same input.
    Native,
    /// Works, with a documented difference that the banner states every time.
    Degraded,
    /// Accepted and does nothing. Listed here, so a caller can see it is a
    /// no-op instead of concluding it worked.
    Stub,
    /// Fails, with a named reason. ⛔ Never silently ignored.
    None,
}

impl Status {
    pub fn word(self) -> &'static str {
        match self {
            Status::Native => "Native",
            Status::Degraded => "Degraded",
            Status::Stub => "Stub",
            Status::None => "None",
        }
    }
}

/// One row: a verb, or a flag of a verb.
///
/// ⚠ `flag` carries every spelling the parser accepts, comma-separated, because
/// the row is the flag rather than one of its names: a table listing `--env`
/// and a parser accepting `-e` is two answers to one question.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    pub verb: &'static str,
    pub flag: Option<&'static str>,
    pub status: Status,
    pub note: &'static str,
}

impl Row {
    /// True when `name` is one of this row's spellings.
    fn names(&self, name: &str) -> bool {
        match self.flag {
            None => false,
            Some(spellings) => spellings.split(',').any(|s| s.trim() == name),
        }
    }
}

use Status::{Degraded, Native, None as NoneStatus, Stub};

/// Short flags that take a value, per rows-key (`rows_of` output).
///
/// Cluster expansion (TODO/cli.md T-1330) reads this and nothing else: a
/// cluster member that takes a value consumes the rest of the cluster as
/// its value, exactly as docker reads it. A short flag that takes a value
/// and is missing here splits wrong (`stop -t5` would read `-t` plus an
/// unknown `-5`), so adding a value-taking short means adding it here.
/// `cluster_expands_docker_clusters` pins the rule; `check-todo.py` check
/// 26 reads this same list, so the gate expands what the binary expands.
pub const CLUSTER_VALUES: &[(&str, char)] = &[
    ("run", 'e'),
    ("run", 'w'),
    ("run", 'u'),
    ("exec", 'e'),
    ("exec", 'w'),
    ("exec", 'u'),
    ("stop", 't'),
    ("kill", 's'),
    ("inspect", 'f'),
    ("info", 'f'),
    ("login", 'u'),
    ("save", 'o'),
    ("load", 'i'),
    ("ps", 'f'),
    ("images", 'f'),
];

/// Rows-keys whose parsers stop at the image: everything past it is the
/// payload's, dashes and all, so clusters stop there too. Every other
/// verb reads its whole argv itself, and clusters expand anywhere in it.
const CLUSTER_BOUNDARY: &[&str] = &["run", "exec"];

/// Refuse a cluster member no row names, in `admit`'s wording.
///
/// Expansion happens before admission, so the member (not the cluster)
/// is what has no row. The usage line matches `admit`'s: one refusal
/// shape for one surface.
pub fn refuse_member(verb: &str, member: &str, usage: &str) -> i32 {
    eprintln!(
        "podbox {verb}: unknown option {member:?}. It has no row in the parity \
         table; `podbox system info` lists every flag this verb takes"
    );
    eprint!("{usage}");
    podbox_image::error::EXIT_FLAG_ERROR
}

/// One `--filter key=value` predicate, for `ps` and `images`.
///
/// TODO/cli.md T-1331: caller-side, no new query language. `name=`
/// matches a substring; `label=` matches nothing, because no record
/// podbox keeps carries labels, and that is said in the usage rather
/// than hidden. Anything else is the caller's mistake and refuses
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
                    "--filter takes name= and label=, not {raw:?}: podbox records \
                     carry no other key to filter on"
                ))
            }
        }
        _ => Err(format!(
            "--filter takes key=value, not {raw:?}: `name=<substring>` selects, \
             `label=<key>[=<value>]` matches nothing because podbox records \
             carry no labels"
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
    // A leading positional is the image (or the whole argv is podbox's):
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

/// ⛔ THE TABLE. Every verb podbox answers to and every flag it accepts, plus
/// the docker verbs and flags it does not, each with the reason it does not.
///
/// ⚠ A `None` row is not a placeholder for future work: it is the sentence a
/// caller gets instead of a flag being ignored. Removing one makes podbox
/// quieter and less honest, not smaller.
pub const TABLE: &[Row] = &[
    // ------------------------------------------------------------- the verbs
    Row { verb: "run", flag: Option::None, status: Degraded, note: "enters a chroot, never a namespace. The banner names what the selected rung does not provide, on every run. A SIGINT or SIGTERM to a foreground run is forwarded to the payload and named on stderr with the payload's exit (TODO/supervise.md T-1335)" },
    Row { verb: "exec", flag: Option::None, status: Degraded, note: "a fresh chroot re-entry sharing only the filesystem, never an entry into a running container's namespaces (T-0505)" },
    Row { verb: "pull", flag: Option::None, status: Native, note: "HTTPS only. A registry offering only http:// is a named refusal, never a downgrade" },
    Row { verb: "images", flag: Option::None, status: Native, note: "one record per platform of a tag" },
    Row { verb: "rmi", flag: Option::None, status: Native, note: "refuses an image any container references or a running container holds, and names it" },
    Row { verb: "tag", flag: Option::None, status: Native, note: "points a second name at one manifest digest; nothing is fetched" },
    Row { verb: "image", flag: Option::None, status: Native, note: "ls, rm, prune, tag, inspect, pull and extract" },
    Row { verb: "image", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "inspect", flag: Option::None, status: Degraded, note: "images by reference and containers by name, one document either way; --format reads the same fields (TODO/cli.md T-1319)" },
    Row { verb: "verify", flag: Option::None, status: Native, note: "hashes indexed blobs against their digests, one line per mismatch plus a summary; prints provenance for one image (TODO/image.md T-1321)" },
    Row { verb: "system", flag: Option::None, status: Degraded, note: "info, install-names, abi and df. `image prune` removes images; events are not implemented" },
    Row { verb: "info", flag: Option::None, status: Degraded, note: "podbox has no daemon, so the server half of docker's output is the rung this machine permits instead" },
    Row { verb: "version", flag: Option::None, status: Native, note: "one artefact, so there is one version and no client/server split" },
    Row { verb: "version", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "version", flag: Some("--verbose"), status: Native, note: "reports the build's recorded inputs: commit, toolchain, target, interposer digests, static mode (T-1004)" },
    Row { verb: "man", flag: Option::None, status: Native, note: "renders this manual from the binary's own usage strings and the parity table, paging through $PAGER unless --no-pager or a non-terminal stdout (TODO/cli.md T-1332)" },
    Row { verb: "man", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "man", flag: Some("--no-pager"), status: Native, note: "print to stdout even on a terminal; without it a terminal pages through $PAGER" },
    Row { verb: "probe", flag: Option::None, status: Native, note: "podbox's own verb, with no docker equivalent: what this machine permits, and the rung podbox selects" },
    Row { verb: "doctor", flag: Option::None, status: Native, note: "podbox's own verb, with no docker equivalent: the setup half beside probe, with one fix line per missing piece (T-1337)" },
    Row { verb: "doctor", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "extract", flag: Option::None, status: Native, note: "podbox's own verb, with no docker equivalent: unpack the layers and write the ownership sidecar" },
    // ⭐ M4, and each says the difference from docker's rather than implying
    // there is none. TODO/supervise.md T-0601 to T-0607.
    Row { verb: "create", flag: Option::None, status: Native, note: "writes a created record and starts nothing, as docker's does. note: It is served by run's PARSER, so it takes run's flag set: those rows are listed once, under `run`, rather than copied here where the two could diverge (T-0801)" },
    Row { verb: "start", flag: Option::None, status: Native, note: "returns when the payload has reached its execve, established by a pipe rather than by a sleep (T-0602). A SIGINT or SIGTERM to the launcher is forwarded to the payload; the launcher holds no terminal to name it on, so the payload's signaled exit in the container record is the whole account (TODO/supervise.md T-1335)" },
    Row { verb: "stop", flag: Option::None, status: Degraded, note: "SIGTERM then SIGKILL to the PAYLOAD. podbox has no PID namespace, so a grandchild that reparented is outside its reach and is not signalled" },
    Row { verb: "restart", flag: Option::None, status: Native, note: "stop then start in one verb, naming which half failed (TODO/cli.md T-1331)" },
    Row { verb: "restart", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "kill", flag: Option::None, status: Degraded, note: "signals the payload. A pidfd addresses one process; it does not reach descendants that reparent" },
    Row { verb: "rm", flag: Option::None, status: Native, note: "removes the record and the container's own directory; -f kills a running one first" },
    Row { verb: "ps", flag: Option::None, status: Degraded, note: "reads the container table, never /proc. A container whose launcher was killed reads `dead` with the time it was noticed, and no exit code (T-0604)" },
    Row { verb: "logs", flag: Option::None, status: Degraded, note: "the payload's stdout and stderr, interleaved into one file opened before the chroot, which `-f` follows with a bounded poll (TODO/supervise.md T-1318)" },
    Row { verb: "wait", flag: Option::None, status: Degraded, note: "blocks on the launcher, bounded. refused: A container podbox did not see end has NO exit code and `wait` refuses rather than printing one" },
    Row { verb: "cp", flag: Option::None, status: Degraded, note: "one file at a time either way, or a directory tree with -r; a container or an image on either side, gated through the containment check (TODO/cli.md T-1323)" },
    Row { verb: "cp", flag: Some("-r, --recursive"), status: Native, note: "copy a directory tree with the same checks; symlinks replicate as symlinks, special files are refused (TODO/cli.md T-1323)" },
    Row { verb: "top", flag: Option::None, status: NoneStatus, note: "a chroot shares this machine's process table, so `top` would list the host's processes and call them the container's" },
    Row { verb: "attach", flag: Option::None, status: NoneStatus, note: "not implemented: `logs` is the same bytes and it does not need a signal proxy" },
    Row { verb: "pause", flag: Option::None, status: NoneStatus, note: "freezing a process group needs a cgroup this runtime does not grant" },
    Row { verb: "unpause", flag: Option::None, status: NoneStatus, note: "the counterpart of a verb podbox does not have" },
    Row { verb: "stats", flag: Option::None, status: NoneStatus, note: "resource accounting needs a cgroup this runtime does not grant" },
    Row { verb: "diff", flag: Option::None, status: NoneStatus, note: "podbox extracts into one directory and keeps no upper layer to difference against" },
    Row { verb: "port", flag: Option::None, status: NoneStatus, note: "podbox publishes no ports: the payload shares this machine's network namespace" },
    Row { verb: "rename", flag: Option::None, status: NoneStatus, note: "not implemented: `rm` and a fresh `create` under the other name is what podbox offers" },
    Row { verb: "update", flag: Option::None, status: NoneStatus, note: "there are no resource limits to update" },
    // Building and moving images.
    Row { verb: "build", flag: Option::None, status: NoneStatus, note: "building runs a payload per layer and commits the result; podbox can run one but cannot commit one" },
    Row { verb: "commit", flag: Option::None, status: NoneStatus, note: "podbox cannot restore ownership, so a committed layer would misrepresent what it holds (TODO/extract.md T-0302)" },
    Row { verb: "push", flag: Option::None, status: NoneStatus, note: "podbox reads registries and does not write to them" },
    Row { verb: "login", flag: Option::None, status: Native, note: "writes ~/.docker/config.json, or the configured credential helper where one is named; the password arrives on stdin, never as an argument (TODO/image.md T-0209)" },
    Row { verb: "login", flag: Some("-u, --username"), status: Native, note: "the registry user; the password arrives on stdin, never as an argument (TODO/image.md T-0209)" },
    Row { verb: "login", flag: Some("--password-stdin"), status: Native, note: "read the password from stdin; required, because podbox never prompts (TODO/image.md T-0209)" },
    Row { verb: "login", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "logout", flag: Option::None, status: Native, note: "removes the login `podbox login` stored, from the file holding it or through the configured credential helper; nothing stored reads as not logged in (TODO/image.md T-0209)" },
    Row { verb: "logout", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "save", flag: Option::None, status: Native, note: "writes one image as an OCI-layout tarball, to stdout or -o (TODO/image.md T-1320)" },
    Row { verb: "save", flag: Some("-o, --output"), status: Native, note: "write the tarball to a file instead of stdout (TODO/image.md T-1320)" },
    Row { verb: "save", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "load", flag: Option::None, status: Native, note: "reads an OCI-layout tarball into the store with every blob verified on the way in (TODO/image.md T-1320)" },
    Row { verb: "load", flag: Some("-i, --input"), status: Native, note: "read the tarball from a file instead of stdin (TODO/image.md T-1320)" },
    Row { verb: "load", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "export", flag: Option::None, status: NoneStatus, note: "an exported rootfs would carry the ownership podbox could not apply, and would misrepresent what it holds" },
    Row { verb: "import", flag: Option::None, status: Native, note: "builds a runnable record from a plain rootfs tar; a compressed file is refused by name (TODO/image.md T-1320)" },
    Row { verb: "import", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "history", flag: Option::None, status: NoneStatus, note: "not implemented; `inspect` prints the record podbox holds" },
    Row { verb: "events", flag: Option::None, status: NoneStatus, note: "there is no daemon to emit events" },
    Row { verb: "search", flag: Option::None, status: NoneStatus, note: "not implemented; podbox resolves a reference and does not browse a registry" },
    Row { verb: "network", flag: Option::None, status: NoneStatus, note: "the payload shares this machine's network namespace, so there is nothing to create or attach" },
    Row { verb: "volume", flag: Option::None, status: NoneStatus, note: "podbox cannot mount, so a volume would be a copy pretending to be a mount" },
    Row { verb: "compose", flag: Option::None, status: NoneStatus, note: "not implemented; it needs the lifecycle and a network" },
    Row { verb: "swarm", flag: Option::None, status: NoneStatus, note: "not implemented, and out of the shape TOOL.md section 2.0 describes" },
    Row { verb: "builder", flag: Option::None, status: NoneStatus, note: "the counterpart of a verb podbox does not have" },
    Row { verb: "context", flag: Option::None, status: NoneStatus, note: "there is no daemon to point a context at" },
    Row { verb: "container", flag: Option::None, status: NoneStatus, note: "the sub-command group is not implemented; every verb it holds is a top-level verb here and is listed above" },
    // ------------------------------------------------- issue 60: ten docker
    // verbs with no row anywhere. Each takes an existing reason: no daemon,
    // or out of the shape TOOL.md section 2.0 describes. TODO/cli.md T-0801.
    Row { verb: "manifest", flag: Option::None, status: NoneStatus, note: "podbox resolves a reference and does not browse a registry; `inspect` prints the record it holds" },
    Row { verb: "service", flag: Option::None, status: NoneStatus, note: "services need a swarm scheduler and a daemon, and podbox has neither: out of the shape TOOL.md section 2.0 describes" },
    Row { verb: "stack", flag: Option::None, status: NoneStatus, note: "stacks deploy to a swarm scheduler, and podbox has neither scheduler nor daemon: out of the shape TOOL.md section 2.0 describes" },
    Row { verb: "node", flag: Option::None, status: NoneStatus, note: "nodes are swarm members, and podbox has no swarm: out of the shape TOOL.md section 2.0 describes" },
    Row { verb: "secret", flag: Option::None, status: NoneStatus, note: "there is no daemon to hold secrets; the store holds images, and a credential never enters this tree (TODO/image.md T-0209)" },
    Row { verb: "config", flag: Option::None, status: NoneStatus, note: "there is no daemon to hold configs; the store holds images" },
    Row { verb: "trust", flag: Option::None, status: NoneStatus, note: "podbox always verifies digests (TODO/extract.md T-1315); there is no trust metadata to manage" },
    Row { verb: "plugin", flag: Option::None, status: NoneStatus, note: "there is no daemon to load plugins into" },
    Row { verb: "scan", flag: Option::None, status: NoneStatus, note: "podbox ships no image scanner" },
    Row { verb: "checkpoint", flag: Option::None, status: NoneStatus, note: "freezing a process group needs a cgroup this runtime does not grant, as `pause` says" },
    // ------------------------------------------------------- run's own flags
    Row { verb: "run", flag: Some("--rm"), status: Native, note: "removes the extracted rootfs when the payload exits, unless a container record references it (TODO/image.md T-1322)" },
    Row { verb: "run", flag: Some("-e, --env"), status: Native, note: "repeatable; a later one wins" },
    Row { verb: "run", flag: Some("-w, --workdir"), status: Native, note: "chdir inside the new root, after the chroot" },
    Row { verb: "run", flag: Some("--entrypoint"), status: Native, note: "as docker: replacing it also drops the image's Cmd" },
    Row { verb: "run", flag: Some("--platform"), status: Native, note: "a bare word is an architecture, as docker reads it" },
    Row { verb: "run", flag: Some("--pull"), status: Native, note: "never, missing (default) or always" },
    Row { verb: "run", flag: Some("--insecure-registry"), status: Native, note: "docker's flag and docker's meaning; every use is disclosed on stderr" },
    Row { verb: "run", flag: Some("--tls-verify"), status: Native, note: "podman's flag and podman's meaning; every use is disclosed on stderr" },
    Row { verb: "run", flag: Some("-t, --tty"), status: Degraded, note: "REFUSED BY NAME where /dev/ptmx is unusable, rather than running without a pty and letting the payload discover it (T-0503)" },
    Row { verb: "run", flag: Some("-i, --interactive"), status: Stub, note: "accepted and a no-op: podbox never detaches stdin, so it is already interactive when the caller's is" },
    Row { verb: "run", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "run", flag: Some("-d, --detach"), status: Native, note: "starts a detached launcher and prints the container id, returning when the payload is running" },
    Row { verb: "run", flag: Some("--name"), status: Native, note: "names the container. A name already in use is refused rather than making the second one unreachable" },
    Row { verb: "run", flag: Some("-v, --volume"), status: NoneStatus, note: "podbox cannot mount(2) on this runtime, so a volume would be a copy pretending to be a mount" },
    Row { verb: "run", flag: Some("-p, --publish"), status: NoneStatus, note: "the payload shares this machine's network namespace, so a published port is already this machine's port" },
    Row { verb: "run", flag: Some("--network"), status: NoneStatus, note: "there is no network namespace to select" },
    Row { verb: "run", flag: Some("-u, --user"), status: Degraded, note: "uid[:gid] or a name from the image's own passwd, answered through the interposer identity memo for a reachable payload, which the banner names and --strict refuses (T-0711)" },
    Row { verb: "run", flag: Some("--privileged"), status: NoneStatus, note: "podbox holds every capability bit already and can still do none of what they name" },
    Row { verb: "run", flag: Some("--cap-add, --cap-drop"), status: NoneStatus, note: "capabilities are not what is denied here; the filter is" },
    Row { verb: "run", flag: Some("-m, --memory"), status: NoneStatus, note: "resource limits need a cgroup this runtime does not grant" },
    Row { verb: "run", flag: Some("--cpus"), status: NoneStatus, note: "resource limits need a cgroup this runtime does not grant" },
    Row { verb: "run", flag: Some("--restart"), status: NoneStatus, note: "not implemented: `stop` then `start` is the same thing and says which half failed (TODO/cli.md T-1331)" },
    Row { verb: "run", flag: Some("--hostname"), status: NoneStatus, note: "sethostname needs a UTS namespace this runtime does not grant" },
    // ⭐ M5 and TODO/cli.md T-0804. Four flags docker does not have, and each
    // is here for the same reason every other row is: a surface with no row is
    // a surface nobody documented.
    Row { verb: "run", flag: Some("--add-host"), status: Native, note: "name:ip, appended to the /etc/hosts the completion layer writes. Repeatable (T-0403)" },
    Row { verb: "run", flag: Some("--no-source-fixup"), status: Native, note: "podbox's own: leave the image's package sources exactly as extracted, http:// and all, and UNDO any rewrite an earlier run made (T-0411)" },
    Row { verb: "run", flag: Some("--no-host-cas"), status: Native, note: "podbox's own: do not append this machine's announced CA bundle ($SSL_CERT_FILE and friends) to the image's own trust store, even where the machine intercepts TLS (T-0407)" },
    Row { verb: "run", flag: Some("--no-steps"), status: Native, note: "podbox's own: run no command inside the rootfs before the payload. Two fixups cannot be made from outside the chroot -- pacman-key for an empty keyring and openssl rehash for a hash-indexed CA directory -- and this refuses both (T-0412)" },
    Row { verb: "run", flag: Some("--strict"), status: Native, note: "podbox's own: refuse rather than run where anything about this invocation is Degraded or Stub -- a flag, the selected rung, a completion fixup, or a step podbox would run inside the image (T-0804)" },
    // ⭐ TODO/podvm.md T-1302. Two flags podbox's own tools do not have, and
    // the prefix is the rule: neither docker nor podman uses `--podbox-`, so
    // no flag here can mean one thing here and another there.
    Row { verb: "run", flag: Some("--podbox-tier"), status: Native, note: "podbox's own: machine selects the machine tier, chroot the chroot tier. `podvm` is this binary under another name and defaults to machine; an explicit flag wins over argv[0], and podbox states the tier where the two disagree (T-1302)" },
    Row { verb: "run", flag: Some("--podbox-qemu-arg"), status: Native, note: "podbox's own, machine tier only and refused elsewhere: one token for the emulator per occurrence, repeatable, never split on whitespace (T-1302)" },
    Row { verb: "run", flag: Some("--podbox-mem"), status: Native, note: "podbox's own, machine tier only and refused elsewhere: the guest memory in bytes with an optional K/M/G/T suffix, judged against the RLIMIT_FSIZE ceiling before anything starts (T-1305)" },
    // ------------------------------------------------ issue 60: the rest of
    // docker's `run` surface. TODO/cli.md T-0801: a flag with no row is a
    // bug in the table, so every docker spelling lands here, honored or
    // refused with its reason. `--env-file` is real behavior; `--label`,
    // `--attach` and `--expose` are Stub; the rest are None.
    Row { verb: "run", flag: Some("--env-file"), status: Native, note: "repeatable; entries load at the flag's position, so a later -e wins over the file and a later file over an earlier flag. Lines are KEY=VALUE, `#` comments and blank lines ignored, one matching quote pair stripped, a line without `=` refused naming its number" },
    Row { verb: "run", flag: Some("--label"), status: Stub, note: "accepted and does nothing: podbox records carry no labels, so `ps --filter label=` matches nothing, and --strict refuses" },
    Row { verb: "run", flag: Some("--attach"), status: Stub, note: "accepted and does nothing: podbox always captures the payload's stdout and stderr together (TODO/supervise.md T-0605), and --strict refuses" },
    Row { verb: "run", flag: Some("--expose"), status: Stub, note: "accepted and does nothing: docker's --expose only documents ports and podbox publishes none, so the run is unchanged, and --strict refuses" },
    Row { verb: "run", flag: Some("--read-only"), status: NoneStatus, note: "the rootfs cannot be remounted read-only without mount(2), which this runtime denies; the payload runs with the store's writability" },
    Row { verb: "run", flag: Some("--mount"), status: NoneStatus, note: "podbox cannot mount(2) on this runtime, so a mount would be a copy pretending to be a mount" },
    Row { verb: "run", flag: Some("--tmpfs"), status: NoneStatus, note: "podbox cannot mount(2) on this runtime, so a tmpfs would be a directory pretending to be one" },
    Row { verb: "run", flag: Some("--volume-driver"), status: NoneStatus, note: "podbox cannot mount(2) on this runtime, so a named volume would be a copy pretending to be a mount" },
    Row { verb: "run", flag: Some("--volumes-from"), status: NoneStatus, note: "podbox cannot mount(2) on this runtime, so another container's volumes cannot be attached" },
    Row { verb: "run", flag: Some("--device"), status: Native, note: "repeatable HOST[:GUEST[:PERMS]], both absolute; opened before the chroot and served as duplicates of host descriptors where the interposer holds, named on the banner; the serve answers failures, so a creating open the image satisfies lands in the image rather than the device; `m` parses and grants nothing; refused on the machine tier (TODO/enter.md T-0501)" },
    Row { verb: "run", flag: Some("--device-cgroup-rule"), status: NoneStatus, note: "a device rule needs a cgroup this runtime does not grant" },
    Row { verb: "run", flag: Some("--gpus"), status: NoneStatus, note: "no device passthrough: device nodes are shims, not devices (TODO/complete.md T-0401)" },
    Row { verb: "run", flag: Some("--shm-size"), status: NoneStatus, note: "sizing /dev/shm needs a tmpfs mount, and podbox cannot mount(2) on this runtime" },
    Row { verb: "run", flag: Some("--dns, --dns-option, --dns-search"), status: NoneStatus, note: "podbox always installs the host's own resolv.conf (TODO/complete.md T-0402); there is no per-container resolver to configure" },
    Row { verb: "run", flag: Some("--domainname"), status: NoneStatus, note: "the payload shares this machine's network namespace and hostname; sethostname needs a UTS namespace this runtime does not grant" },
    Row { verb: "run", flag: Some("--mac-address"), status: NoneStatus, note: "the payload shares this machine's network namespace and interface; there is no address to assign" },
    Row { verb: "run", flag: Some("--link"), status: NoneStatus, note: "the payload shares this machine's network namespace, so there is nothing to link to" },
    Row { verb: "run", flag: Some("--blkio-weight, --cgroup-parent, --cpu-period, --cpu-quota, --cpu-shares, --cpuset-cpus, --oom-kill-disable, --pids-limit"), status: NoneStatus, note: "resource limits need a cgroup this runtime does not grant" },
    Row { verb: "run", flag: Some("--ulimit"), status: NoneStatus, note: "per-container rlimits are not applied; the payload inherits this process's limits" },
    Row { verb: "run", flag: Some("--pid, --ipc, --uts, --userns"), status: NoneStatus, note: "the payload shares this machine's namespaces; there is no PID, IPC, UTS or user namespace to select" },
    Row { verb: "run", flag: Some("--isolation"), status: NoneStatus, note: "there is no isolation backend to select: one chroot rung, chosen by the probe" },
    Row { verb: "run", flag: Some("--runtime"), status: NoneStatus, note: "one runtime, not a set: podbox enters a chroot and selects no OCI runtime" },
    Row { verb: "run", flag: Some("--security-opt"), status: NoneStatus, note: "no security backend takes per-container options; the syscall filter is the runtime's own" },
    Row { verb: "run", flag: Some("--group-add"), status: NoneStatus, note: "the runtime grants no supplementary groups; --user names the identity instead (TODO/interpose.md T-0711)" },
    Row { verb: "run", flag: Some("--health-cmd"), status: NoneStatus, note: "there is no daemon to run health checks; `ps` reads the container table" },
    Row { verb: "run", flag: Some("--cidfile"), status: NoneStatus, note: "a detached run prints its id on stdout and the record keeps it; no cidfile is written" },
    Row { verb: "run", flag: Some("--detach-keys"), status: NoneStatus, note: "there is no attach session to detach keys from" },
    Row { verb: "run", flag: Some("--log-driver"), status: Stub, note: "json-file accepted and changes nothing: the payload's output is captured into one interleaved file per container either way, raw bytes rather than JSON (TODO/supervise.md T-0605). Any other driver is refused naming it, and --strict refuses" },
    Row { verb: "run", flag: Some("--log-opt"), status: NoneStatus, note: "no log driver takes options here: one file per container (TODO/supervise.md T-0605)" },
    Row { verb: "run", flag: Some("--stop-signal"), status: NoneStatus, note: "stop always sends SIGTERM then SIGKILL after the bounded grace; no per-container stop signal is stored" },
    Row { verb: "run", flag: Some("--stop-timeout"), status: NoneStatus, note: "the stop grace is `stop -t` (default 10); `run` carries no per-container timeout" },
    Row { verb: "run", flag: Some("--sysctl"), status: NoneStatus, note: "the payload shares the host's namespaces, so a per-container sysctl would be a host sysctl; none is applied" },
    Row { verb: "run", flag: Some("--disable-content-trust"), status: NoneStatus, note: "podbox always verifies digests (TODO/extract.md T-1315); disabling trust is refused rather than honored" },
    Row { verb: "run", flag: Some("--init"), status: NoneStatus, note: "no PID namespace and no init to run: orphaned grandchildren reparent to the host init, and no flag changes that" },
    // ------------------------------------------------------ exec's own flags
    Row { verb: "exec", flag: Some("-e, --env"), status: Native, note: "repeatable; a later one wins" },
    Row { verb: "exec", flag: Some("-w, --workdir"), status: Native, note: "chdir inside the new root, after the chroot" },
    Row { verb: "exec", flag: Some("--platform"), status: Native, note: "which platform of a multi-platform image to enter" },
    Row { verb: "exec", flag: Some("-t, --tty"), status: Degraded, note: "REFUSED BY NAME where /dev/ptmx is unusable (T-0503)" },
    Row { verb: "exec", flag: Some("-i, --interactive"), status: Stub, note: "accepted and a no-op, as in run" },
    Row { verb: "exec", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "exec", flag: Some("-d, --detach"), status: NoneStatus, note: "not implemented: an exec podbox does not watch has no exit code to report, which is the state T-0604 exists to avoid" },
    Row { verb: "exec", flag: Some("-u, --user"), status: Degraded, note: "as in run: faked through the interposer identity memo for a reachable payload (T-0711)" },
    Row { verb: "exec", flag: Some("--add-host"), status: Native, note: "name:ip, appended to the /etc/hosts the completion layer writes (T-0403)" },
    Row { verb: "exec", flag: Some("--no-source-fixup"), status: Native, note: "as in run: leave the image's package sources as extracted, and undo an earlier rewrite (T-0411)" },
    Row { verb: "exec", flag: Some("--no-host-cas"), status: Native, note: "as in run: leave the image's own trust store alone (T-0407)" },
    Row { verb: "exec", flag: Some("--no-steps"), status: Native, note: "as in run: run no command inside the rootfs before the command asked for (T-0412)" },
    Row { verb: "exec", flag: Some("--strict"), status: Native, note: "as in run: refuse rather than re-enter where anything about this invocation is Degraded or Stub (T-0804)" },
    Row { verb: "exec", flag: Some("--podbox-tier"), status: Native, note: "as in run: machine selects the machine tier, chroot the chroot tier, and an explicit flag wins over the podvm default with the tier stated (T-1302)" },
    Row { verb: "exec", flag: Some("--podbox-qemu-arg"), status: Native, note: "as in run: machine tier only and refused elsewhere, one emulator token per occurrence, repeatable, never split (T-1302)" },
    Row { verb: "exec", flag: Some("--podbox-mem"), status: Native, note: "as in run: machine tier only and refused elsewhere, the guest memory in bytes with an optional K/M/G/T suffix, judged against the RLIMIT_FSIZE ceiling before anything starts (T-1305)" },
    // ------------------------------------------------------ pull's own flags
    Row { verb: "pull", flag: Some("--platform"), status: Native, note: "a bare word is an architecture, as docker reads it" },
    Row { verb: "pull", flag: Some("--insecure-registry"), status: Native, note: "docker's flag and docker's meaning" },
    Row { verb: "pull", flag: Some("--tls-verify"), status: Native, note: "podman's flag and podman's meaning" },
    Row { verb: "pull", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "pull", flag: Some("-a, --all-tags"), status: Native, note: "fetch every offered tag of a bare repository; a tag on it is refused; a tag with no manifest for this platform is named and skipped (TODO/cli.md T-1331)" },
    Row { verb: "pull", flag: Some("-q, --quiet"), status: Native, note: "per-layer transcript goes nowhere; errors still reach stderr (TODO/cli.md T-1331)" },
    // ---------------------------------------------------- images' own flags
    Row { verb: "images", flag: Some("--format"), status: Native, note: "{{.Field}} placeholders and literal text. No pipelines, no functions, and the `table` prefix is refused by name" },
    Row { verb: "images", flag: Some("-q, --quiet"), status: Native, note: "image IDs only, the same as --format '{{.ID}}'" },
    Row { verb: "images", flag: Some("--digests"), status: Native, note: "show the DIGEST column" },
    Row { verb: "images", flag: Some("--no-trunc"), status: Native, note: "print full IDs and digests" },
    Row { verb: "images", flag: Some("-a, --all"), status: Stub, note: "accepted for parity: podbox stores no intermediate images, so every image is already listed" },
    Row { verb: "images", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "images", flag: Some("-f, --filter"), status: Native, note: "caller-side predicate: name=<substring> over repository and tag, label= matches nothing (records carry no labels) (TODO/cli.md T-1331)" },
    // ------------------------------------------------------- the rest's flags
    Row { verb: "rmi", flag: Some("-f, --force"), status: Stub, note: "accepted for parity. podbox never prompts, so there is no confirmation to suppress, and it refuses a held image whether or not this is given" },
    Row { verb: "rmi", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "inspect", flag: Some("-f, --format"), status: Native, note: "the same template shape as `images --format`" },
    Row { verb: "inspect", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "verify", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "extract", flag: Some("--force"), status: Native, note: "extract again over an existing rootfs" },
    Row { verb: "extract", flag: Some("--platform"), status: Native, note: "which platform, where the store holds more than one" },
    Row { verb: "extract", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "probe", flag: Some("--json"), status: Native, note: "the findings as one JSON document on stdout" },
    Row { verb: "probe", flag: Some("--rows"), status: Native, note: "every probe as one row, in the format verification/probe emits" },
    Row { verb: "probe", flag: Some("--strict"), status: Native, note: "exit non-zero below the namespace rung, so a caller gates without parsing" },
    Row { verb: "probe", flag: Some("--cached"), status: Native, note: "serve $store/probe.json where its key still holds" },
    Row { verb: "probe", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    // ------------------------------------------------ the lifecycle's flags
    Row { verb: "ps", flag: Some("-a, --all"), status: Native, note: "list containers that are not running too" },
    Row { verb: "ps", flag: Some("-q, --quiet"), status: Native, note: "ids only" },
    Row { verb: "ps", flag: Some("--no-trunc"), status: Native, note: "print full container ids" },
    Row { verb: "ps", flag: Some("--format"), status: Native, note: "the same template shape as the other verbs" },
    Row { verb: "ps", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "ps", flag: Some("-f, --filter"), status: Native, note: "caller-side predicate: name=<substring> over name and id prefix, label= matches nothing (records carry no labels) (TODO/cli.md T-1331)" },
    Row { verb: "stop", flag: Some("-t, --time, --timeout"), status: Native, note: "seconds between SIGTERM and SIGKILL. Default 10, and a kill is reported on stderr" },
    Row { verb: "stop", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "kill", flag: Some("-s, --signal"), status: Native, note: "by name or number. refused: An unknown one is refused rather than defaulted: sending the wrong signal is not something a caller can notice" },
    Row { verb: "kill", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "rm", flag: Some("-f, --force"), status: Native, note: "kill a running container before removing it" },
    Row { verb: "rm", flag: Some("-v, --volumes"), status: Stub, note: "accepted for parity: podbox has no volumes, so there are none to remove" },
    Row { verb: "rm", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "logs", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "logs", flag: Some("-f, --follow"), status: Native, note: "follows the container's log file with a bounded poll and exits after the container ends" },
    Row { verb: "logs", flag: Some("--tail"), status: Native, note: "prints the last N lines of the captured file (`--tail N` or `--tail=N`, 0 prints nothing), then follows where `-f` is given instead of replaying the whole file (TODO/cli.md T-1337)" },
    Row { verb: "wait", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "start", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "cp", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "tag", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "prune", flag: Option::None, status: Native, note: "removes untagged images, and tagged ones too with --all. Held or referenced ones are named and kept (T-0204, TODO/image.md T-1322). Invoke it as `image prune`" },
    Row { verb: "prune", flag: Some("-a, --all"), status: Native, note: "take tagged images too, where no container references or holds them" },
    Row { verb: "prune", flag: Some("-f, --force"), status: Stub, note: "accepted for parity: podbox never prompts, so there is nothing to suppress" },
    Row { verb: "prune", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "system", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "info", flag: Some("-f, --format"), status: Native, note: "the same template shape as the other verbs, plus `json .Field` for a field that is a document" },
    Row { verb: "info", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "install-names", flag: Option::None, status: Native, note: "installs the docker, podman and podvm names as symlinks to this binary, refusing docker where a daemon answers unless --force (T-0803). Invoke it as `system install-names`" },
    Row { verb: "install-names", flag: Some("--dir"), status: Native, note: "install-names: where to put the symlinks. Default: the directory this binary is in" },
    Row { verb: "install-names", flag: Some("--force"), status: Native, note: "install-names: take the `docker` name even where a docker daemon answers, and replace a file that is not already a link to this binary" },
    Row { verb: "install-names", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "abi", flag: Option::None, status: Native, note: "answers whether an object may be preloaded into a libc payload (T-0709). Invoke it as `system abi`" },
    Row { verb: "abi", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "df", flag: Option::None, status: Native, note: "disk usage: one row per image with stored and extracted bytes, container counts, and what `image prune` would reclaim; accounting never fails the verb (T-1337). Invoke it as `system df`" },
    Row { verb: "df", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    // ⭐ TODO/cli.md T-0803. The names podbox answers to are rows here for the
    // same reason every flag is: a surface with no row is a surface nobody
    // documented, and `names.rs` asserts these exist.
    Row { verb: "docker", flag: Option::None, status: Degraded, note: "podbox answers to the `docker` name on PATH through argv[0], and says so in the banner. It REFUSES to install that name where a working docker daemon is reachable, unless --force (T-0803)" },
    Row { verb: "podman", flag: Option::None, status: Degraded, note: "podbox answers to the `podman` name on PATH through argv[0], and says so in the banner (T-0803)" },
    Row { verb: "podvm", flag: Option::None, status: Degraded, note: "podbox answers to its own `podvm` name on PATH through argv[0], selecting the machine tier unless --podbox-tier says otherwise, and says both in the banner (T-1302)" },
];

/// The verb's own row, if podbox names it at all.
pub fn verb(name: &str) -> Option<&'static Row> {
    TABLE.iter().find(|r| r.verb == name && r.flag.is_none())
}

/// The row for one flag of one verb, under any spelling the parser accepts.
///
/// ⛔ This is what makes the table binding rather than descriptive. A parser
/// asks here first, so a flag with no row cannot be quietly accepted and a row
/// with no parser arm cannot be quietly ignored.
/// Which verb's rows a verb's flags are listed under.
///
/// ⭐ `create` is served by `run`'s PARSER, so it takes `run`'s flag set. The
/// table holds ONE copy of those rows rather than two that can diverge, and this
/// is the single place that says where a verb's rows live. ⛔ Without it,
/// naming the caller's verb in the diagnostics would also have made `flag`
/// refuse every flag `create` accepts, which is the shape of hole
/// `docs/conventions/code.md` calls a second, quieter surface. T-0801.
///
/// The `image` and `system` groups dispatch to one handler per subverb, so
/// each subverb's rows live under its own name and the refusal names the
/// path the caller typed, as `create` does for `run`. T-0808.
pub fn rows_of(verb: &str) -> &str {
    match verb {
        "create" => "run",
        "image ls" | "image list" => "images",
        "image rm" | "image remove" => "rmi",
        "image prune" => "prune",
        "image tag" => "tag",
        "image inspect" => "inspect",
        "image pull" => "pull",
        "image extract" => "extract",
        "system info" => "info",
        "system install-names" => "install-names",
        "system abi" => "abi",
        "system df" => "df",
        v => v,
    }
}

pub fn flag(verb: &str, name: &str) -> Option<&'static Row> {
    // ⚠ `--flag=value` is one argument to the shell and two things here. The
    // row is for the flag, so the value is cut off before the lookup.
    let name = name.split('=').next().unwrap_or(name);
    let rows = rows_of(verb);
    TABLE.iter().find(|r| r.verb == rows && r.names(name))
}

/// Decide whether a verb's parser may go on to handle `arg`, and say why not.
///
/// ⛔ **This is the guard T-0801 is for.** `Ok(())` means the table has a row
/// and the flag is not refused; `Err(code)` means the caller has already been
/// told, in one line, either that podbox has no such flag or that podbox has it
/// and will not honour it. A parser that decided this itself would be a second
/// table, and the one nobody reads is the one that drifts.
///
/// ⚠ A `Stub` row passes: it is accepted and does nothing, which is a
/// difference the table states rather than a refusal.
pub fn admit(verb: &str, arg: &str, usage: &str) -> Result<(), i32> {
    match flag(verb, arg) {
        Option::None => {
            // ⚠ Point at the verb whose ROWS were consulted, which is not always
            // the one the caller typed: `podbox create` is served by `run`'s
            // parser. Saying "this verb" would send a caller to a row set that
            // does not exist.
            let under = rows_of(verb);
            let where_ = if under == verb {
                "lists every flag this verb takes".to_string()
            } else {
                format!("lists every flag it takes under `{under}`, whose parser serves it")
            };
            eprintln!(
                "podbox {verb}: unknown option {arg:?}. It has no row in the parity \
                 table; `podbox system info` {where_}"
            );
            eprint!("{usage}");
            Err(podbox_image::error::EXIT_FLAG_ERROR)
        }
        Some(r) if r.status == Status::None => {
            // ⛔ Refused UP FRONT, with the reason, rather than accepted and
            // silently lost. `dockless`'s posture, and the whole point of the
            // table being consulted rather than described.
            eprintln!(
                "podbox {verb}: {} is in the parity table with status None: {}",
                r.flag.unwrap_or(arg),
                r.note
            );
            Err(podbox_image::error::EXIT_FLAG_ERROR)
        }
        Some(_) => Ok(()),
    }
}

/// Admit-first, one read path (T-0808): refuse every dash-arg the table
/// does not list before any match arm runs, so an arm for a flag with no
/// row is unreachable. A value that starts with `-` must use the `=` form.
pub fn admit_all(verb: &str, args: &[String], usage: &str) -> Option<i32> {
    for a in args {
        if a.starts_with('-') {
            if let Err(c) = admit(verb, a, usage) {
                return Some(c);
            }
        }
    }
    None
}

/// The arm a verb's parser reaches when [`admit`] passed a flag and the parser
/// has no case for it.
///
/// ⛔ **That is a defect in podbox, not a caller mistake**, and it is reported
/// as one. ⚠ It is also the case a test has to be able to see, and since
/// T-0802 it can no longer be seen in the exit code: a flag error and a runtime
/// failure are both docker's 125, so the sentinel the parser tests used to
/// compare against became equal to the legitimate refusals it was distinguishing
/// itself from. Under `cfg(test)` this panics with the flag's own name, which is
/// a stronger assertion than the code comparison it replaces; in a shipped
/// binary it returns 125 and says what happened, because a runtime that panics
/// on its own argument surface is worse than one that refuses.
pub fn no_arm(verb: &str, flag: &str) -> i32 {
    let msg = format!(
        "podbox {verb}: {flag:?} is in the parity table and this parser has no arm \
         for it. That is a bug in podbox (TODO/cli.md T-0801)"
    );
    if cfg!(test) {
        panic!("{msg}");
    }
    eprintln!("{msg}");
    podbox_image::error::EXIT_RUNTIME_ERROR
}

/// The whole table as a JSON array, one object per row.
///
/// ⭐ T-0801's `Prove` reads exactly this: `length >= 60`, and every `status`
/// one of the four words.
pub fn json() -> String {
    let rows: Vec<serde_json::Value> = TABLE
        .iter()
        .map(|r| {
            serde_json::json!({
                "verb": r.verb,
                "flag": r.flag,
                "status": r.status.word(),
                "note": r.note,
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// The same table for a person, one row per line.
pub fn text() -> String {
    let mut out = String::new();
    let mut last = "";
    for r in TABLE {
        if r.verb != last {
            last = r.verb;
        }
        let name = match r.flag {
            Option::None => r.verb.to_string(),
            Some(f) => format!("{} {}", r.verb, f),
        };
        out.push_str(&format!("{:<34} {:<9} {}\n", name, r.status.word(), r.note));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_parity_note_is_plain_ascii() {
        // TODO/cli.md T-1336: the parity table is machine-readable
        // (`system info --format '{{json .Parity}}'`), so a note needing
        // a codepoint above U+007F forces every downstream parser to
        // handle bytes that carry no meaning.
        let bad: Vec<(&str, usize)> = TABLE
            .iter()
            .map(|r| (r.verb, r.note.bytes().filter(|b| *b > 0x7F).count()))
            .filter(|(_, n)| *n > 0)
            .collect();
        assert!(bad.is_empty(), "non-ASCII parity notes: {bad:?}");
    }

    /// ⭐ T-0801's `Prove`, as a unit test as well as a command: the table is
    /// the contract, so its size and its vocabulary are asserted here rather
    /// than only in an experiment somebody has to remember to run.
    #[test]
    fn the_table_is_at_least_sixty_rows_and_uses_only_the_four_statuses() {
        assert!(TABLE.len() >= 60, "the table has only {} rows", TABLE.len());
        for r in TABLE {
            assert!(
                ["Native", "Degraded", "Stub", "None"].contains(&r.status.word()),
                "{} has a fifth status",
                r.verb
            );
            assert!(!r.note.is_empty(), "{} has no note", r.verb);
        }
    }

    /// ⛔ A `None` row exists to be a sentence, so an empty reason makes the row
    /// worse than absent: the caller is told no and not why.
    #[test]
    fn every_unimplemented_row_says_why() {
        for r in TABLE.iter().filter(|r| r.status == Status::None) {
            assert!(
                r.note.len() > 20,
                "{} {:?} refuses without a reason a caller can act on",
                r.verb,
                r.flag
            );
        }
    }

    #[test]
    fn a_flag_is_found_under_every_spelling_the_parser_accepts() {
        assert_eq!(flag("run", "-e").map(|r| r.status), Some(Status::Native));
        assert_eq!(flag("run", "--env").map(|r| r.status), Some(Status::Native));
        // ⚠ `--env=A=1` is one argument and the row is for the flag.
        assert_eq!(
            flag("run", "--env=A=1").map(|r| r.status),
            Some(Status::Native)
        );
        // ⚠ `--name` was a `None` row until M4 and is `Native` now, so the
        // refusal case is taken from a row that is still one rather than from a
        // name that changed meaning under the test.
        assert_eq!(
            flag("run", "--name").map(|r| r.status),
            Some(Status::Native)
        );
        assert_eq!(
            flag("run", "--privileged").map(|r| r.status),
            Some(Status::None)
        );
        assert!(flag("run", "--nope").is_none());
        // ⚠ Per verb, not global: `--entrypoint` is run's and exec has no such
        // flag, so exec must not find run's row.
        assert!(flag("exec", "--entrypoint").is_none());
    }

    #[test]
    fn no_verb_carries_the_same_flag_twice() {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for r in TABLE {
            let Some(spellings) = r.flag else { continue };
            for s in spellings.split(',') {
                let key = (r.verb, s.trim());
                assert!(!seen.contains(&key), "{} carries {} twice", key.0, key.1);
                seen.push(key);
            }
        }
    }

    #[test]
    fn every_flag_row_names_a_verb_the_table_also_has_a_row_for() {
        for r in TABLE.iter().filter(|r| r.flag.is_some()) {
            assert!(
                verb(r.verb).is_some(),
                "{} has flags and no verb row",
                r.verb
            );
        }
    }

    /// TODO/cli.md T-1331. `--filter` is `name=` and `label=` only, and
    /// anything else refuses naming the two keys that exist.
    #[test]
    fn filter_parses_name_and_label_and_nothing_else() {
        let f = parse_filter("name=web").unwrap();
        assert_eq!(f.key, "name");
        assert_eq!(f.value, "web");
        let f = parse_filter("label=key=value").unwrap();
        assert_eq!(f.key, "label");
        assert_eq!(f.value, "key=value");
        assert!(parse_filter("status=running").is_err());
        assert!(parse_filter("name").is_err());
        assert!(parse_filter("=x").is_err());
    }

    /// TODO/cli.md T-1330. Docker's cluster rule exactly: value-less
    /// shorts split, a value-taking member consumes the rest, an unknown
    /// member refuses naming the member. Longs, lone shorts and values
    /// starting with `-` pass through untouched.
    #[test]
    fn cluster_expands_docker_clusters() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(expand("ps", &v(&["-aq"])), Ok(v(&["-a", "-q"])));
        assert_eq!(expand("run", &v(&["-it"])), Ok(v(&["-i", "-t"])));
        // ⚠ Per verb: `-t` takes a value for `stop` and is a boolean for
        // `run`, so the same spelling expands two ways.
        assert_eq!(expand("stop", &v(&["-t5"])), Ok(v(&["-t", "5"])));
        assert_eq!(
            expand("run", &v(&["-eFOO=bar", "img"])),
            Ok(v(&["-e", "FOO=bar", "img"]))
        );
        // ⛔ Unknown member refuses naming the member, not the cluster.
        assert_eq!(expand("ps", &v(&["-aZ"])), Err("-Z".to_string()));
        assert_eq!(expand("ps", &v(&["-Zq"])), Err("-Z".to_string()));
        // Untouched: longs, lone shorts, `-`, values. A digit member is
        // an unknown member, exactly as docker reads it.
        assert_eq!(
            expand("ps", &v(&["--format", "x", "-a", "-", "img"])),
            Ok(v(&["--format", "x", "-a", "-", "img"]))
        );
        assert_eq!(expand("ps", &v(&["-a1"])), Err("-1".to_string()));
        // ⚠ Past a value-taking member the rest is opaque, braces and
        // all: the template never parses as members.
        assert_eq!(
            expand("inspect", &v(&["-f{{.Id}}", "img"])),
            Ok(v(&["-f", "{{.Id}}", "img"]))
        );
        // ⚠ The image boundary: `run` and `exec` stop expanding past
        // it, so payload flags reach the payload. Every other verb
        // reads its whole argv, and clusters expand anywhere in it.
        assert_eq!(
            expand("run", &v(&["-it", "img", "ls", "-la"])),
            Ok(v(&["-i", "-t", "img", "ls", "-la"]))
        );
        assert_eq!(
            expand("stop", &v(&["c1", "-t5"])),
            Ok(v(&["c1", "-t", "5"]))
        );
        // ⚠ `create` is served by `run`'s parser, so it expands by run's rows.
        assert_eq!(expand("create", &v(&["-it"])), Ok(v(&["-i", "-t"])));
    }

    /// ⛔ `CLUSTER_VALUES` is the one declaration of which shorts take
    /// values. A member listed here with no table row would expand into a
    /// flag `admit` then refuses as unknown, so the list is held to the
    /// table it expands against.
    #[test]
    fn every_cluster_value_has_a_table_row() {
        for (under, c) in CLUSTER_VALUES {
            let one = format!("-{c}");
            assert!(
                flag(under, &one).is_some(),
                "{under} takes a value for `{one}` with no table row"
            );
        }
    }

    /// ⛔ `podbox create --no-steps` was ACCEPTED and acted on while
    /// `podbox system info` listed `--no-steps` for `run` and `exec` only, so
    /// the table under-described the binary and the refusal message sent a
    /// caller to a row set that does not exist. Both halves are asserted here:
    /// a verb that borrows another's rows resolves them, and it says whose.
    #[test]
    fn a_verb_served_by_another_parser_resolves_that_verbs_rows_and_says_so() {
        assert_eq!(rows_of("create"), "run");
        assert_eq!(rows_of("run"), "run", "a verb with its own rows keeps them");
        assert_eq!(rows_of("exec"), "exec");

        // The flag `create` was silently taking, now found through the table.
        assert!(
            flag("create", "--no-steps").is_some(),
            "create is served by run's parser, so run's rows must resolve for it"
        );
        assert!(
            flag("create", "--no-such-flag").is_none(),
            "borrowing rows must not admit a flag that has no row anywhere"
        );

        // ⚠ The row a reader is sent to has to name the borrowing, or the
        // caller looks for `create --no-steps` in the table and finds nothing.
        let note = verb("create").expect("create has a verb row").note;
        assert!(
            note.contains("run's flag set"),
            "the create row must say whose rows it takes, got {note:?}"
        );
    }

    #[test]
    fn the_json_carries_every_row_and_the_status_word() {
        let doc: serde_json::Value = serde_json::from_str(&json()).unwrap();
        let arr = doc.as_array().unwrap();
        assert_eq!(arr.len(), TABLE.len());
        assert!(arr.iter().all(|r| r["status"].is_string()));
        assert!(arr
            .iter()
            .any(|r| r["verb"] == "run" && r["flag"].is_null()));
    }

    /// T-0808: the `image` and `system` groups dispatch to one handler per
    /// subverb, so each subverb resolves the rows its flags live under and
    /// the refusal names the path the caller typed.
    #[test]
    fn group_subverbs_resolve_the_rows_their_flags_live_under() {
        for (sub, home) in [
            ("image ls", "images"),
            ("image list", "images"),
            ("image rm", "rmi"),
            ("image remove", "rmi"),
            ("image prune", "prune"),
            ("image tag", "tag"),
            ("image inspect", "inspect"),
            ("image pull", "pull"),
            ("image extract", "extract"),
            ("info", "info"),
            ("system info", "info"),
            ("system install-names", "install-names"),
            ("system abi", "abi"),
            ("system df", "df"),
        ] {
            assert_eq!(rows_of(sub), home, "{sub} resolves under {home}");
        }
        assert!(flag("image prune", "-a").is_some());
        assert!(flag("image prune", "--no-such-flag").is_none());
        assert!(flag("system install-names", "--dir").is_some());
        assert!(flag("info", "--format").is_some());
    }

    /// T-0808: the pre-pass refuses before any arm runs, and passes
    /// listed flags and positionals through.
    #[test]
    fn admit_all_refuses_before_any_arm_runs() {
        let usage = "usage: podbox images";
        assert_eq!(
            admit_all("images", &["--no-such-flag".to_string()], usage),
            Some(podbox_image::error::EXIT_FLAG_ERROR)
        );
        assert_eq!(admit_all("images", &["-a".to_string()], usage), None);
        assert_eq!(
            admit_all("images", &["some-image".to_string()], usage),
            None
        );
        assert_eq!(admit_all("images", &[], usage), None);
    }

    /// ⭐ TODO/cli.md T-0801, issue 60: the curated docker surface stays
    /// covered. Every flag in the issue's list resolves under `run` (any
    /// status: honored or refused with its reason), and every verb in its
    /// list has a verb row. `scripts/check-todo.py` check 27 holds the
    /// same list from outside the binary; this holds it from inside.
    #[test]
    fn issue_60_curated_surface_stays_covered() {
        const FLAGS: &[&str] = &[
            "--attach",
            "--blkio-weight",
            "--cgroup-parent",
            "--cidfile",
            "--cpu-period",
            "--cpu-quota",
            "--cpu-shares",
            "--cpuset-cpus",
            "--detach-keys",
            "--device",
            "--device-cgroup-rule",
            "--disable-content-trust",
            "--dns",
            "--dns-option",
            "--dns-search",
            "--domainname",
            "--env-file",
            "--expose",
            "--gpus",
            "--group-add",
            "--health-cmd",
            "--init",
            "--ipc",
            "--isolation",
            "--label",
            "--link",
            "--log-driver",
            "--log-opt",
            "--mac-address",
            "--mount",
            "--oom-kill-disable",
            "--pid",
            "--pids-limit",
            "--read-only",
            "--runtime",
            "--security-opt",
            "--shm-size",
            "--stop-signal",
            "--stop-timeout",
            "--sysctl",
            "--tmpfs",
            "--ulimit",
            "--userns",
            "--uts",
            "--volume-driver",
            "--volumes-from",
        ];
        const VERBS: &[&str] = &[
            "manifest",
            "node",
            "plugin",
            "scan",
            "secret",
            "service",
            "stack",
            "trust",
            "checkpoint",
            "config",
        ];
        assert_eq!(FLAGS.len(), 46, "the issue lists 46 run flags");
        assert_eq!(VERBS.len(), 10, "the issue lists 10 verbs");
        for f in FLAGS {
            assert!(
                flag("run", f).is_some(),
                "curated flag {f} has no row under `run`"
            );
        }
        for v in VERBS {
            assert!(verb(v).is_some(), "curated verb {v} has no verb row");
        }
    }
}
