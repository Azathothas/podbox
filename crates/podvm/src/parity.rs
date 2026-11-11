//! The verb and flag parity table, as DATA.
//!
//! [`TODO/cli.md`](../../../TODO/cli.md) T-0801, `TOOL.md` section 6.8.
//!
//! ⭐ **A tool that needs its user to learn its differences has not replaced
//! anything.** Thousands of agents reach for `docker` because it is the only
//! container language they know, so the differences have to be enumerable
//! rather than discoverable: `podvm system info --format '{{json .Parity}}'`
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

/// The four statuses `TOOL.md` section 6.8 defines, and podvm has no fifth.
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
/// `cluster_expands_docker_clusters` pins the rule; the `podvm-gate` check
/// 26 reads this same list, so the gate expands what the binary expands.
pub const CLUSTER_VALUES: &[(&str, char)] = &[
    ("run", 'e'),
    ("run", 'w'),
    ("run", 'u'),
    ("run", 'v'),
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

mod filter;
pub use filter::*;

/// ⛔ THE TABLE. Every verb podvm answers to and every flag it accepts, plus
/// the docker verbs and flags it does not, each with the reason it does not.
///
/// ⚠ A `None` row is not a placeholder for future work: it is the sentence a
/// caller gets instead of a flag being ignored. Removing one makes podvm
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
    Row { verb: "info", flag: Option::None, status: Degraded, note: "podvm has no daemon, so the server half of docker's output is the rung this machine permits instead" },
    Row { verb: "version", flag: Option::None, status: Native, note: "one artefact, so there is one version and no client/server split" },
    Row { verb: "version", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "version", flag: Some("--verbose"), status: Native, note: "reports the build's recorded inputs: commit, toolchain, target, interposer digests, static mode (T-1004)" },
    Row { verb: "man", flag: Option::None, status: Native, note: "renders this manual from the binary's own usage strings and the parity table, paging through $PAGER unless --no-pager or a non-terminal stdout (TODO/cli.md T-1332)" },
    Row { verb: "man", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "man", flag: Some("--no-pager"), status: Native, note: "print to stdout even on a terminal; without it a terminal pages through $PAGER" },
    Row { verb: "probe", flag: Option::None, status: Native, note: "podvm's own verb, with no docker equivalent: what this machine permits, and the rung podvm selects" },
    Row { verb: "doctor", flag: Option::None, status: Native, note: "podvm's own verb, with no docker equivalent: the setup half beside probe, with one fix line per missing piece (T-1337)" },
    Row { verb: "doctor", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "extract", flag: Option::None, status: Native, note: "podvm's own verb, with no docker equivalent: unpack the layers and write the ownership sidecar" },
    // ⭐ M4, and each says the difference from docker's rather than implying
    // there is none. TODO/supervise.md T-0601 to T-0607.
    Row { verb: "create", flag: Option::None, status: Native, note: "writes a created record and starts nothing, as docker's does. note: It is served by run's PARSER, so it takes run's flag set: those rows are listed once, under `run`, rather than copied here where the two could diverge (T-0801)" },
    Row { verb: "start", flag: Option::None, status: Native, note: "returns when the payload has reached its execve, established by a pipe rather than by a sleep (T-0602). A SIGINT or SIGTERM to the launcher is forwarded to the payload; the launcher holds no terminal to name it on, so the payload's signaled exit in the container record is the whole account (TODO/supervise.md T-1335)" },
    Row { verb: "stop", flag: Option::None, status: Degraded, note: "SIGTERM then SIGKILL to the PAYLOAD. podvm has no PID namespace, so a grandchild that reparented is outside its reach and is not signalled" },
    Row { verb: "restart", flag: Option::None, status: Native, note: "stop then start in one verb, naming which half failed (TODO/cli.md T-1331)" },
    Row { verb: "restart", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "kill", flag: Option::None, status: Degraded, note: "signals the payload. A pidfd addresses one process; it does not reach descendants that reparent" },
    Row { verb: "rm", flag: Option::None, status: Native, note: "removes the record and the container's own directory; -f kills a running one first" },
    Row { verb: "ps", flag: Option::None, status: Degraded, note: "reads the container table, never /proc. A container whose launcher was killed reads `dead` with the time it was noticed, and no exit code (T-0604)" },
    Row { verb: "logs", flag: Option::None, status: Degraded, note: "the payload's stdout and stderr, interleaved into one file opened before the chroot, which `-f` follows with a bounded poll (TODO/supervise.md T-1318)" },
    Row { verb: "wait", flag: Option::None, status: Degraded, note: "blocks on the launcher, bounded. refused: A container podvm did not see end has NO exit code and `wait` refuses rather than printing one" },
    Row { verb: "cp", flag: Option::None, status: Degraded, note: "one file at a time either way, or a directory tree with -r; a container or an image on either side, gated through the containment check (TODO/cli.md T-1323)" },
    Row { verb: "cp", flag: Some("-r, --recursive"), status: Native, note: "copy a directory tree with the same checks; symlinks replicate as symlinks, special files are refused (TODO/cli.md T-1323)" },
    Row { verb: "top", flag: Option::None, status: NoneStatus, note: "a chroot shares this machine's process table, so `top` would list the host's processes and call them the container's" },
    Row { verb: "attach", flag: Option::None, status: NoneStatus, note: "not implemented: `logs` is the same bytes and it does not need a signal proxy" },
    Row { verb: "pause", flag: Option::None, status: NoneStatus, note: "freezing a process group needs a cgroup this runtime does not grant" },
    Row { verb: "unpause", flag: Option::None, status: NoneStatus, note: "the counterpart of a verb podvm does not have" },
    Row { verb: "stats", flag: Option::None, status: NoneStatus, note: "resource accounting needs a cgroup this runtime does not grant" },
    Row { verb: "diff", flag: Option::None, status: NoneStatus, note: "podvm extracts into one directory and keeps no upper layer to difference against" },
    Row { verb: "port", flag: Option::None, status: NoneStatus, note: "podvm publishes no ports: the payload shares this machine's network namespace" },
    Row { verb: "rename", flag: Option::None, status: NoneStatus, note: "not implemented: `rm` and a fresh `create` under the other name is what podvm offers" },
    Row { verb: "update", flag: Option::None, status: NoneStatus, note: "there are no resource limits to update" },
    // Building and moving images.
    Row { verb: "build", flag: Option::None, status: NoneStatus, note: "building runs a payload per layer and commits the result; podvm can run one but cannot commit one" },
    Row { verb: "commit", flag: Option::None, status: NoneStatus, note: "podvm cannot restore ownership, so a committed layer would misrepresent what it holds (TODO/extract.md T-0302)" },
    Row { verb: "push", flag: Option::None, status: NoneStatus, note: "podvm reads registries and does not write to them" },
    Row { verb: "login", flag: Option::None, status: Native, note: "writes ~/.docker/config.json, or the configured credential helper where one is named; the password arrives on stdin, never as an argument (TODO/image.md T-0209)" },
    Row { verb: "login", flag: Some("-u, --username"), status: Native, note: "the registry user; the password arrives on stdin, never as an argument (TODO/image.md T-0209)" },
    Row { verb: "login", flag: Some("--password-stdin"), status: Native, note: "read the password from stdin; required, because podvm never prompts (TODO/image.md T-0209)" },
    Row { verb: "login", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "logout", flag: Option::None, status: Native, note: "removes the login `podvm login` stored, from the file holding it or through the configured credential helper; nothing stored reads as not logged in (TODO/image.md T-0209)" },
    Row { verb: "logout", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "save", flag: Option::None, status: Native, note: "writes one image as an OCI-layout tarball, to stdout or -o (TODO/image.md T-1320)" },
    Row { verb: "save", flag: Some("-o, --output"), status: Native, note: "write the tarball to a file instead of stdout (TODO/image.md T-1320)" },
    Row { verb: "save", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "load", flag: Option::None, status: Native, note: "reads an OCI-layout tarball into the store with every blob verified on the way in (TODO/image.md T-1320)" },
    Row { verb: "load", flag: Some("-i, --input"), status: Native, note: "read the tarball from a file instead of stdin (TODO/image.md T-1320)" },
    Row { verb: "load", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "export", flag: Option::None, status: NoneStatus, note: "an exported rootfs would carry the ownership podvm could not apply, and would misrepresent what it holds" },
    Row { verb: "import", flag: Option::None, status: Native, note: "builds a runnable record from a plain rootfs tar; a compressed file is refused by name (TODO/image.md T-1320)" },
    Row { verb: "import", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "history", flag: Option::None, status: NoneStatus, note: "not implemented; `inspect` prints the record podvm holds" },
    Row { verb: "events", flag: Option::None, status: NoneStatus, note: "there is no daemon to emit events" },
    Row { verb: "search", flag: Option::None, status: NoneStatus, note: "not implemented; podvm resolves a reference and does not browse a registry" },
    Row { verb: "network", flag: Option::None, status: NoneStatus, note: "the payload shares this machine's network namespace, so there is nothing to create or attach" },
    Row { verb: "volume", flag: Option::None, status: NoneStatus, note: "podvm cannot mount, so a volume would be a copy pretending to be a mount" },
    Row { verb: "compose", flag: Option::None, status: NoneStatus, note: "not implemented; it needs the lifecycle and a network" },
    Row { verb: "swarm", flag: Option::None, status: NoneStatus, note: "not implemented, and out of the shape TOOL.md section 2.0 describes" },
    Row { verb: "builder", flag: Option::None, status: NoneStatus, note: "the counterpart of a verb podvm does not have" },
    Row { verb: "context", flag: Option::None, status: NoneStatus, note: "there is no daemon to point a context at" },
    Row { verb: "container", flag: Option::None, status: NoneStatus, note: "the sub-command group is not implemented; every verb it holds is a top-level verb here and is listed above" },
    // ------------------------------------------------- issue 60: ten docker
    // verbs with no row anywhere. Each takes an existing reason: no daemon,
    // or out of the shape TOOL.md section 2.0 describes. TODO/cli.md T-0801.
    Row { verb: "manifest", flag: Option::None, status: NoneStatus, note: "podvm resolves a reference and does not browse a registry; `inspect` prints the record it holds" },
    Row { verb: "service", flag: Option::None, status: NoneStatus, note: "services need a swarm scheduler and a daemon, and podvm has neither: out of the shape TOOL.md section 2.0 describes" },
    Row { verb: "stack", flag: Option::None, status: NoneStatus, note: "stacks deploy to a swarm scheduler, and podvm has neither scheduler nor daemon: out of the shape TOOL.md section 2.0 describes" },
    Row { verb: "node", flag: Option::None, status: NoneStatus, note: "nodes are swarm members, and podvm has no swarm: out of the shape TOOL.md section 2.0 describes" },
    Row { verb: "secret", flag: Option::None, status: NoneStatus, note: "there is no daemon to hold secrets; the store holds images, and a credential never enters this tree (TODO/image.md T-0209)" },
    Row { verb: "config", flag: Option::None, status: NoneStatus, note: "there is no daemon to hold configs; the store holds images" },
    Row { verb: "trust", flag: Option::None, status: NoneStatus, note: "podvm always verifies digests (TODO/extract.md T-1315); there is no trust metadata to manage" },
    Row { verb: "plugin", flag: Option::None, status: NoneStatus, note: "there is no daemon to load plugins into" },
    Row { verb: "scan", flag: Option::None, status: NoneStatus, note: "podvm ships no image scanner" },
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
    Row { verb: "run", flag: Some("-i, --interactive"), status: Stub, note: "accepted and a no-op: podvm never detaches stdin, so it is already interactive when the caller's is" },
    Row { verb: "run", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "run", flag: Some("-d, --detach"), status: Native, note: "starts a detached launcher and prints the container id, returning when the payload is running" },
    Row { verb: "run", flag: Some("--name"), status: Native, note: "names the container. A name already in use is refused rather than making the second one unreachable. A foreground run leaves a record under it: `ps -a` lists it as Exited with `logs` reading its output (TODO/supervise.md T-1421)" },
    Row { verb: "run", flag: Some("-v, --volume"), status: Degraded, note: "HOST:GUEST[:rw], repeatable: a path rewrite through PODVM_MAPS on the userland rung's loader family, never a mount or a copy. A write lands in HOST, as with a bind mount; a static program or a raw syscall instruction sees the image's path; /proc/mounts does not list it. Refused by name: on the chroot rung, the memfd family, a forced PODVM_MODE and the machine tier; :ro, until the read-only layer of T-1655; a named or anonymous volume. A working directory or a first program in a volume resolves through the same table. The banner names a noexec source, and --strict refuses (TODO/enter.md T-1653, T-1655)" },
    Row { verb: "run", flag: Some("-p, --publish"), status: NoneStatus, note: "the payload shares this machine's network namespace, so a published port is already this machine's port" },
    Row { verb: "run", flag: Some("--network"), status: NoneStatus, note: "there is no network namespace to select" },
    Row { verb: "run", flag: Some("-u, --user"), status: Degraded, note: "uid[:gid] or a name from the image's own passwd, answered through the interposer identity memo for a reachable payload, which the banner names and --strict refuses (T-0711)" },
    Row { verb: "run", flag: Some("--privileged"), status: NoneStatus, note: "podvm holds every capability bit already and can still do none of what they name" },
    Row { verb: "run", flag: Some("--cap-add"), status: NoneStatus, note: "capabilities are not what is denied here; the filter is" },
    Row { verb: "run", flag: Some("--cap-drop"), status: Degraded, note: "ALL only: PR_SET_NO_NEW_PRIVS in the child, as --security-opt no-new-privileges; podvm does not change the capability sets. Each named capability is refused (T-1655)" },
    Row { verb: "run", flag: Some("-m, --memory"), status: NoneStatus, note: "resource limits need a cgroup this runtime does not grant" },
    Row { verb: "run", flag: Some("--cpus"), status: NoneStatus, note: "resource limits need a cgroup this runtime does not grant" },
    Row { verb: "run", flag: Some("--restart"), status: NoneStatus, note: "not implemented: `stop` then `start` is the same thing and says which half failed (TODO/cli.md T-1331)" },
    Row { verb: "run", flag: Some("--hostname"), status: NoneStatus, note: "sethostname needs a UTS namespace this runtime does not grant" },
    // ⭐ M5 and TODO/cli.md T-0804. Four flags docker does not have, and each
    // is here for the same reason every other row is: a surface with no row is
    // a surface nobody documented.
    Row { verb: "run", flag: Some("--add-host"), status: Native, note: "name:ip, appended to the /etc/hosts the completion layer writes. Repeatable (T-0403)" },
    Row { verb: "run", flag: Some("--no-source-fixup"), status: Native, note: "podvm's own: leave the image's package sources exactly as extracted, http:// and all, and UNDO any rewrite an earlier run made (T-0411)" },
    Row { verb: "run", flag: Some("--no-host-cas"), status: Native, note: "podvm's own: do not append this machine's announced CA bundle ($SSL_CERT_FILE and friends) to the image's own trust store, even where the machine intercepts TLS (T-0407)" },
    Row { verb: "run", flag: Some("--no-steps"), status: Native, note: "podvm's own: run no command inside the rootfs before the payload. Two fixups cannot be made from outside the chroot -- pacman-key for an empty keyring and openssl rehash for a hash-indexed CA directory -- and this refuses both (T-0412)" },
    Row { verb: "run", flag: Some("--strict"), status: Native, note: "podvm's own: refuse rather than run where anything safety-relevant about this invocation is Degraded or Stub: a flag, the selected rung, a failed fixup, a step, or the userland opt-outs. Dev-shim substitutions warn; --strict=all refuses every difference (T-0804, T-1415)" },
    Row { verb: "run", flag: Some("-q, --quiet"), status: Native, note: "podvm's own: force the banner off. Runs are already quiet by default; this forces silence under --verbose too (TODO/cli.md T-1416)" },
    Row { verb: "run", flag: Some("--verbose"), status: Native, note: "podvm's own: print the banner. Long-only: -v is docker's --volume (TODO/enter.md T-1653), so the banner switch cannot take it (TODO/cli.md T-1416)" },
    // ⭐ TODO/machine.md T-1302. Two flags podvm's own tools do not have, and
    // the prefix is the rule: neither docker nor podman uses `--podvm-`, so
    // no flag here can mean one thing here and another there.
    Row { verb: "run", flag: Some("--podvm-tier"), status: Native, note: "podvm's own: machine selects the machine tier, and each other value is a flag error. With no flag the ladder selects, under every name the binary answers to, and `system info` names the entered rung (T-1302, T-1712)" },
    Row { verb: "run", flag: Some("--timeout"), status: Native, note: "podman's: whole seconds over 0. At the bound the launcher of a detached run, or podvm for a foreground run, kills the payload and its tree with SIGKILL, and .End.Reason reads wall (T-1655)" },
    Row { verb: "run", flag: Some("--podvm-qemu-arg"), status: Native, note: "podvm's own, machine tier only and refused elsewhere: one token for the emulator per occurrence, repeatable, never split on whitespace (T-1302)" },
    Row { verb: "run", flag: Some("--podvm-mem"), status: Native, note: "podvm's own, machine tier only and refused elsewhere: the guest memory in bytes with an optional K/M/G/T suffix, judged against the RLIMIT_FSIZE ceiling before anything starts (T-1305)" },
    // ------------------------------------------------ issue 60: the rest of
    // docker's `run` surface. TODO/cli.md T-0801: a flag with no row is a
    // bug in the table, so every docker spelling lands here, honored or
    // refused with its reason. `--env-file` is real behavior; `--label`,
    // `--attach` and `--expose` are Stub; the rest are None.
    Row { verb: "run", flag: Some("--env-file"), status: Native, note: "repeatable; entries load at the flag's position, so a later -e wins over the file and a later file over an earlier flag. Lines are KEY=VALUE, `#` comments and blank lines ignored, one matching quote pair stripped, a line without `=` refused naming its number" },
    Row { verb: "run", flag: Some("--label"), status: Stub, note: "accepted and does nothing: podvm records carry no labels, so `ps --filter label=` matches nothing, and --strict refuses" },
    Row { verb: "run", flag: Some("--attach"), status: Stub, note: "accepted and does nothing: podvm always captures the payload's stdout and stderr together (TODO/supervise.md T-0605), and --strict refuses" },
    Row { verb: "run", flag: Some("--expose"), status: Stub, note: "accepted and does nothing: docker's --expose only documents ports and podvm publishes none, so the run is unchanged, and --strict refuses" },
    Row { verb: "run", flag: Some("--read-only"), status: NoneStatus, note: "the rootfs cannot be remounted read-only without mount(2), which this runtime denies; the payload runs with the store's writability" },
    Row { verb: "run", flag: Some("--mount"), status: NoneStatus, note: "podvm cannot mount(2) on this runtime, so a mount would be a copy pretending to be a mount" },
    Row { verb: "run", flag: Some("--tmpfs"), status: NoneStatus, note: "podvm cannot mount(2) on this runtime, so a tmpfs would be a directory pretending to be one" },
    Row { verb: "run", flag: Some("--volume-driver"), status: NoneStatus, note: "podvm cannot mount(2) on this runtime, so a named volume would be a copy pretending to be a mount" },
    Row { verb: "run", flag: Some("--volumes-from"), status: NoneStatus, note: "podvm cannot mount(2) on this runtime, so another container's volumes cannot be attached" },
    Row { verb: "run", flag: Some("--device"), status: Native, note: "repeatable HOST[:GUEST[:PERMS]], both absolute; opened before the chroot and served as duplicates of host descriptors where the interposer holds, named on the banner; the serve answers failures, so a creating open the image satisfies lands in the image rather than the device; `m` parses and grants nothing; refused on the machine tier (TODO/enter.md T-0501)" },
    Row { verb: "run", flag: Some("--unsafe-host-paths"), status: Native, note: "podvm's own, userland rung only: opt into the pass-through with no PODVM_MAPS anchoring, so absolute guest paths resolve on the host. It also admits a static payload to the memfd family where chroot is denied; without it that payload refuses at 125 (TODO/enter.md T-1648). Its host-root line prints even under -q, and --strict refuses (TODO/enter.md T-1407)" },
    Row { verb: "run", flag: Some("--device-cgroup-rule"), status: NoneStatus, note: "a device rule needs a cgroup this runtime does not grant" },
    Row { verb: "run", flag: Some("--gpus"), status: NoneStatus, note: "no device passthrough: device nodes are shims, not devices (TODO/complete.md T-0401)" },
    Row { verb: "run", flag: Some("--shm-size"), status: NoneStatus, note: "sizing /dev/shm needs a tmpfs mount, and podvm cannot mount(2) on this runtime" },
    Row { verb: "run", flag: Some("--dns, --dns-option, --dns-search"), status: NoneStatus, note: "podvm always installs the host's own resolv.conf (TODO/complete.md T-0402); there is no per-container resolver to configure" },
    Row { verb: "run", flag: Some("--domainname"), status: NoneStatus, note: "the payload shares this machine's network namespace and hostname; sethostname needs a UTS namespace this runtime does not grant" },
    Row { verb: "run", flag: Some("--mac-address"), status: NoneStatus, note: "the payload shares this machine's network namespace and interface; there is no address to assign" },
    Row { verb: "run", flag: Some("--link"), status: NoneStatus, note: "the payload shares this machine's network namespace, so there is nothing to link to" },
    Row { verb: "run", flag: Some("--blkio-weight, --cgroup-parent, --cpu-period, --cpu-quota, --cpu-shares, --cpuset-cpus, --oom-kill-disable, --pids-limit"), status: NoneStatus, note: "resource limits need a cgroup this runtime does not grant" },
    Row { verb: "run", flag: Some("--ulimit"), status: Native, note: "NAME=SOFT[:HARD], -1 for no limit, repeatable: set in the child before the exec on every rung and kept in the record for a detached run. A hard value over podvm's own is refused by name. fsize below 2 TiB ends a .NET payload with SIGXFSZ (T-1655)" },
    Row { verb: "run", flag: Some("--pid, --ipc, --uts, --userns"), status: NoneStatus, note: "the payload shares this machine's namespaces; there is no PID, IPC, UTS or user namespace to select" },
    Row { verb: "run", flag: Some("--isolation"), status: NoneStatus, note: "there is no isolation backend to select: one chroot rung, chosen by the probe" },
    Row { verb: "run", flag: Some("--runtime"), status: NoneStatus, note: "one runtime, not a set: podvm enters a chroot and selects no OCI runtime" },
    Row { verb: "run", flag: Some("--security-opt"), status: Degraded, note: "no-new-privileges only: PR_SET_NO_NEW_PRIVS in the child before the exec (T-1655). Each other option (seccomp, apparmor, label) is refused by name: no security backend takes it" },
    Row { verb: "run", flag: Some("--group-add"), status: NoneStatus, note: "the runtime grants no supplementary groups; --user names the identity instead (TODO/interpose.md T-0711)" },
    Row { verb: "run", flag: Some("--health-cmd"), status: NoneStatus, note: "there is no daemon to run health checks; `ps` reads the container table" },
    Row { verb: "run", flag: Some("--cidfile"), status: NoneStatus, note: "a detached run prints its id on stdout and the record keeps it; no cidfile is written" },
    Row { verb: "run", flag: Some("--detach-keys"), status: NoneStatus, note: "there is no attach session to detach keys from" },
    Row { verb: "run", flag: Some("--log-driver"), status: Stub, note: "json-file accepted and changes nothing: the payload's output is captured into one interleaved file per container either way, raw bytes rather than JSON (TODO/supervise.md T-0605). Any other driver is refused naming it, and --strict refuses" },
    Row { verb: "run", flag: Some("--log-opt"), status: NoneStatus, note: "no log driver takes options here: one file per container (TODO/supervise.md T-0605)" },
    Row { verb: "run", flag: Some("--stop-signal"), status: NoneStatus, note: "stop always sends SIGTERM then SIGKILL after the bounded grace; no per-container stop signal is stored" },
    Row { verb: "run", flag: Some("--stop-timeout"), status: NoneStatus, note: "the stop grace is `stop -t` (default 10); `run` carries no per-container timeout" },
    Row { verb: "run", flag: Some("--sysctl"), status: NoneStatus, note: "the payload shares the host's namespaces, so a per-container sysctl would be a host sysctl; none is applied" },
    Row { verb: "run", flag: Some("--disable-content-trust"), status: NoneStatus, note: "podvm always verifies digests (TODO/extract.md T-1315); disabling trust is refused rather than honored" },
    Row { verb: "run", flag: Some("--init"), status: NoneStatus, note: "no PID namespace and no init to run. The launcher (or podvm, for a foreground run) is the child subreaper of the payload's tree and kills each descendant at the end (T-1708), so no flag is needed" },
    // ------------------------------------------------------ exec's own flags
    Row { verb: "exec", flag: Some("-e, --env"), status: Native, note: "repeatable; a later one wins" },
    Row { verb: "exec", flag: Some("-w, --workdir"), status: Native, note: "chdir inside the new root, after the chroot" },
    Row { verb: "exec", flag: Some("--platform"), status: Native, note: "which platform of a multi-platform image to enter" },
    Row { verb: "exec", flag: Some("-t, --tty"), status: Degraded, note: "REFUSED BY NAME where /dev/ptmx is unusable (T-0503)" },
    Row { verb: "exec", flag: Some("-i, --interactive"), status: Stub, note: "accepted and a no-op, as in run" },
    Row { verb: "exec", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "exec", flag: Some("-d, --detach"), status: NoneStatus, note: "not implemented: an exec podvm does not watch has no exit code to report, which is the state T-0604 exists to avoid" },
    Row { verb: "exec", flag: Some("-u, --user"), status: Degraded, note: "as in run: faked through the interposer identity memo for a reachable payload (T-0711)" },
    Row { verb: "exec", flag: Some("--add-host"), status: Native, note: "name:ip, appended to the /etc/hosts the completion layer writes (T-0403)" },
    Row { verb: "exec", flag: Some("--no-source-fixup"), status: Native, note: "as in run: leave the image's package sources as extracted, and undo an earlier rewrite (T-0411)" },
    Row { verb: "exec", flag: Some("--no-host-cas"), status: Native, note: "as in run: leave the image's own trust store alone (T-0407)" },
    Row { verb: "exec", flag: Some("--no-steps"), status: Native, note: "as in run: run no command inside the rootfs before the command asked for (T-0412)" },
    Row { verb: "exec", flag: Some("--strict"), status: Native, note: "as in run: refuse rather than re-enter where anything safety-relevant is Degraded or Stub; substitutions warn and --strict=all refuses every difference (T-0804, T-1415)" },
    Row { verb: "exec", flag: Some("-q, --quiet"), status: Native, note: "as in run: force the banner off (TODO/cli.md T-1416)" },
    Row { verb: "exec", flag: Some("--verbose"), status: Native, note: "as in run: print the banner, long-only (TODO/cli.md T-1416)" },
    Row { verb: "exec", flag: Some("--unsafe-host-paths"), status: Native, note: "as in run: userland rung only, opt into the pass-through with no PODVM_MAPS anchoring, and admit a static payload to the memfd family where chroot is denied (TODO/enter.md T-1407, T-1648)" },
    Row { verb: "exec", flag: Some("--podvm-tier"), status: Native, note: "as in run: machine selects the machine tier, and each other value is a flag error; with no flag the ladder selects (T-1302)" },
    Row { verb: "exec", flag: Some("--podvm-qemu-arg"), status: Native, note: "as in run: machine tier only and refused elsewhere, one emulator token per occurrence, repeatable, never split (T-1302)" },
    Row { verb: "exec", flag: Some("--podvm-mem"), status: Native, note: "as in run: machine tier only and refused elsewhere, the guest memory in bytes with an optional K/M/G/T suffix, judged against the RLIMIT_FSIZE ceiling before anything starts (T-1305)" },
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
    Row { verb: "images", flag: Some("-a, --all"), status: Stub, note: "accepted for parity: podvm stores no intermediate images, so every image is already listed" },
    Row { verb: "images", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "images", flag: Some("-f, --filter"), status: Native, note: "caller-side predicate: name=<substring> over repository and tag, label= matches nothing (records carry no labels) (TODO/cli.md T-1331)" },
    // ------------------------------------------------------- the rest's flags
    Row { verb: "rmi", flag: Some("-f, --force"), status: Stub, note: "accepted for parity. podvm never prompts, so there is no confirmation to suppress, and it refuses a held image whether or not this is given" },
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
    // ⭐ TODO/milestones.md T-1112. The one verb with no docker equivalent
    // at all: a disposable Windows guest, which is a disk image rather than
    // an OCI rootfs, so it is stated separately instead of folded into `run`.
    Row { verb: "windows", flag: Option::None, status: Native, note: "podvm's own verb, with no docker equivalent: doctor, fetch, setup and run for a disposable Windows guest booted under the machine tier's own profile. The guest is a disk image and the command is one cmd.exe line" },
    Row { verb: "windows", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "windows", flag: Some("--image"), status: Native, note: "the Windows disk image to boot, or for fetch the path to write the download to; a path, not a registry reference" },
    Row { verb: "windows", flag: Some("--url"), status: Native, note: "fetch: the base image to download; nothing is fetched unless this names an origin and no Windows image is ever redistributed" },
    Row { verb: "windows", flag: Some("--sha256"), status: Native, note: "fetch: the pinned digest; without it the download is bounded but unverified, and a mismatch removes the file rather than keeping it" },
    Row { verb: "windows", flag: Some("--max-bytes"), status: Native, note: "fetch: the download ceiling, never above RLIMIT_FSIZE; judged against the declared length before the transfer and against the arriving bytes during it" },
    Row { verb: "windows", flag: Some("--podvm-mem"), status: Native, note: "guest memory in bytes. RLIMIT_FSIZE does not gate it: QEMU's guest memory here is anonymous and the overlay is sparse, so no memory backing file is written; the ceiling binds the download (--max-bytes) (T-1692)" },
    Row { verb: "windows", flag: Some("--podvm-cpus"), status: Native, note: "guest cpus" },
    Row { verb: "windows", flag: Some("--podvm-timeout"), status: Native, note: "how long a run may take before the guest is stopped; a timeout is a refusal, never an empty success" },
    Row { verb: "windows", flag: Some("--podvm-qemu-arg"), status: Native, note: "one extra emulator argument, appended last so a caller can override a default the driver set" },
    Row { verb: "windows", flag: Some("--guest"), status: Native, note: "dos or windows: FreeDOS from its base cache with one typed line, or the disk image with the autostarted agent. Default windows" },
    // ⭐ TODO/podssh.md T-1404. The machine group: podman parity for a
    // guest podvm itself runs. `ssh` is the only member: it boots a
    // disposable Linux guest from the named kernel and initramfs and runs
    // one SSH command in it, reporting the guest command's exit code.
    Row { verb: "machine", flag: Option::None, status: Native, note: "the machine group: ssh into a guest podvm runs (podman parity). ssh boots a Linux guest with an SSH server on its serial line and reports the guest command's exit code (TODO/podssh.md T-1404)" },
    Row { verb: "machine", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "machine ssh", flag: Option::None, status: Native, note: "podvm's own verb, with no docker equivalent: one SSH command in a Linux guest podvm boots, over the guest's serial line with a real ssh client; the exit code is the guest command's (TODO/podssh.md T-1404)" },
    Row { verb: "machine ssh", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "machine ssh", flag: Some("--kernel"), status: Native, note: "the guest kernel; a path that is not a file is refused, never replaced by another guest" },
    Row { verb: "machine ssh", flag: Some("--initramfs"), status: Native, note: "the guest initramfs, carrying the SSH server on its serial console and the account to log in as" },
    Row { verb: "machine ssh", flag: Some("--user-key"), status: Native, note: "the private key the guest authorises for --user" },
    Row { verb: "machine ssh", flag: Some("--user"), status: Native, note: "the guest account (default root)" },
    Row { verb: "machine ssh", flag: Some("--podvm-mem"), status: Native, note: "guest memory, judged against RLIMIT_FSIZE as the machine tier judges its own (T-1305)" },
    Row { verb: "machine ssh", flag: Some("--podvm-timeout"), status: Native, note: "how long the run may take before the guest is stopped; a timeout is a refusal, never an empty success" },
    Row { verb: "machine ssh", flag: Some("--podvm-qemu-arg"), status: Native, note: "one extra emulator argument, appended last so a caller can override a default the driver set" },
    // ⭐ TODO/podssh.md T-1404. The remote group: the verbs that act across
    // a machine boundary. `ssh` is the only member, brought from podvm
    // pull request 67 onto the transport main proves: serve registers with
    // the relay, connect is the operator's ProxyCommand, forward reaches
    // one fixed target with no relay.
    Row { verb: "remote", flag: Option::None, status: Native, note: "the remote group: ssh into a machine somebody else started, across a machine boundary. `ssh` is the only member (TODO/podssh.md T-1404)" },
    Row { verb: "remote", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "remote ssh", flag: Option::None, status: Native, note: "podvm's own verb, with no docker equivalent: ssh across a machine boundary, with a real ssh client on the operator side. The agent registers with `podvm remote ssh serve` and opens no listening socket; the operator reaches it with `podvm remote ssh connect` as an ssh ProxyCommand, or reaches one fixed target with `forward`. `remote` is a namespace, and `podvm ssh` is refused with this row in the message rather than as an unknown command (docs/decisions/remote-verb.md)" },
    Row { verb: "remote ssh", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "remote ssh", flag: Some("--relay"), status: Native, note: "serve and connect: the relay origin both sides dial out to; nothing here listens" },
    Row { verb: "remote ssh", flag: Some("--name"), status: Native, note: "serve and connect: the pair name registered on the relay" },
    Row { verb: "remote ssh", flag: Some("--node-token"), status: Native, note: "serve: the token that registers the pair name" },
    Row { verb: "remote ssh", flag: Some("--connect-token"), status: Native, note: "connect: the token that opens a session on the pair name" },
    Row { verb: "remote ssh", flag: Some("--server"), status: Native, note: "serve: the SSH server command per session; the default is probed before it serves" },
    Row { verb: "remote ssh", flag: Some("--once"), status: Native, note: "serve: exit after the first relay socket ends" },
    Row { verb: "remote ssh", flag: Some("--ready-wait"), status: Native, note: "connect: how long to wait for the far end's ready before failing loud" },
    // ------------------------------------------------ the lifecycle's flags
    Row { verb: "ps", flag: Some("-a, --all"), status: Native, note: "list containers that are not running too" },
    Row { verb: "ps", flag: Some("-q, --quiet"), status: Native, note: "ids only" },
    Row { verb: "ps", flag: Some("--no-trunc"), status: Native, note: "print full container ids" },
    Row { verb: "ps", flag: Some("--format"), status: Native, note: "the same template shape as the other verbs" },
    Row { verb: "ps", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "ps", flag: Some("-f, --filter"), status: Native, note: "caller-side predicate: name=<substring> over name and id prefix, label= matches nothing (records carry no labels) (TODO/cli.md T-1331)" },
    Row { verb: "stop", flag: Some("-t, --time, --timeout"), status: Native, note: "seconds between SIGTERM and SIGKILL. Default 10; 0 sends SIGKILL at once, as docker does (T-1691); a kill is reported on stderr" },
    Row { verb: "stop", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "kill", flag: Some("-s, --signal"), status: Native, note: "by name or number. refused: An unknown one is refused rather than defaulted: sending the wrong signal is not something a caller can notice" },
    Row { verb: "kill", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "rm", flag: Some("-f, --force"), status: Native, note: "kill a running container before removing it" },
    Row { verb: "rm", flag: Some("-v, --volumes"), status: Stub, note: "accepted for parity: podvm has no named or anonymous volumes, so there are none to remove; a -v HOST:GUEST source is the host's own directory, which docker's rm -v keeps too (TODO/enter.md T-1653)" },
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
    Row { verb: "prune", flag: Some("-f, --force"), status: Stub, note: "accepted for parity: podvm never prompts, so there is nothing to suppress" },
    Row { verb: "prune", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "system", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "info", flag: Some("-f, --format"), status: Native, note: "the same template shape as the other verbs, plus `json .Field` for a field that is a document" },
    Row { verb: "info", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "install-names", flag: Option::None, status: Native, note: "installs the docker and podman names as symlinks to this binary, refusing docker where a daemon answers unless --force (T-0803). Invoke it as `system install-names`" },
    Row { verb: "install-names", flag: Some("--dir"), status: Native, note: "install-names: where to put the symlinks. Default: the directory this binary is in" },
    Row { verb: "install-names", flag: Some("--force"), status: Native, note: "install-names: take the `docker` name even where a docker daemon answers, and replace a file that is not already a link to this binary" },
    Row { verb: "install-names", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "abi", flag: Option::None, status: Native, note: "answers whether an object may be preloaded into a libc payload (T-0709). Invoke it as `system abi`" },
    Row { verb: "abi", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    Row { verb: "df", flag: Option::None, status: Native, note: "disk usage: one row per image with stored and extracted bytes, container counts, and what `image prune` would reclaim; accounting never fails the verb (T-1337). Invoke it as `system df`" },
    Row { verb: "df", flag: Some("-h, --help"), status: Native, note: "prints this verb's usage and exits 0" },
    // ⭐ TODO/cli.md T-0803. The names podvm answers to are rows here for the
    // same reason every flag is: a surface with no row is a surface nobody
    // documented, and `names.rs` asserts these exist.
    Row { verb: "docker", flag: Option::None, status: Degraded, note: "podvm answers to the `docker` name on PATH through argv[0], and says so in the banner. It REFUSES to install that name where a working docker daemon is reachable, unless --force (T-0803)" },
    Row { verb: "podman", flag: Option::None, status: Degraded, note: "podvm answers to the `podman` name on PATH through argv[0], and says so in the banner (T-0803)" },
];

mod lookup;
pub use lookup::*;

#[cfg(test)]
mod tests;
