//! The options of `run` and `create`, as `parse` fills them.

#[derive(Debug)]
pub(crate) struct Opts {
    pub(crate) rm: bool,
    pub(crate) detach: bool,
    pub(crate) name: Option<String>,
    pub(crate) env: Vec<String>,
    pub(crate) workdir: Option<String>,
    pub(crate) entrypoint: Option<String>,
    pub(crate) platform: Option<String>,
    pub(crate) pull: String,
    pub(crate) insecure: Vec<String>,
    pub(crate) tls_verify: Option<bool>,
    pub(crate) tty: bool,
    pub(crate) image: Option<String>,
    pub(crate) command: Vec<String>,
    /// `--user`. Carried raw here and resolved against the image once its
    /// rootfs exists, because a name means the image's own passwd entry.
    pub(crate) user: Option<String>,
    /// `--podvm-tier`. Carried raw here and resolved in `prepare`.
    /// TODO/machine.md T-1302.
    pub(crate) tier: Option<crate::tier::Want>,
    /// `--podvm-qemu-arg`. One emulator token per occurrence, carried for
    /// the machine tier's driver (T-1303); refused on every other tier
    /// rather than silently dropped.
    pub(crate) qemu_args: Vec<String>,
    /// `--podvm-mem`. The validated guest memory in bytes, carried for the
    /// machine tier's ceiling check in `prepare`; refused on every other
    /// tier rather than silently dropped. TODO/machine.md T-1305.
    pub(crate) mem: Option<u64>,
    /// `--podvm-cpus`. The guest's vCPUs, machine tier only and refused on
    /// every other tier. TODO/cli.md T-1654 (`limits.cpus` of a profile).
    pub(crate) cpus: Option<u32>,
    /// `--podvm-net none`: the guest of the machine tier has no NIC
    /// (TODO/machine.md T-1751, PodUtils/podrunner#83).
    pub(crate) net_none: bool,
    /// `-i`: the incus tier pushes the piped stdin of `run -d` (T-1777).
    pub(crate) interactive: bool,
    /// `--podvm-channel HOST_SOCKET:127.0.0.1:PORT`: one channel from a
    /// TCP port of the guest to a Unix socket of the host (T-1751).
    pub(crate) channel: Option<(std::path::PathBuf, u16)>,
    /// `--podvm-sealed`: the run comes as a bundle on stdin and reaches
    /// the guest through the channel only (T-1751, ask V-7 of podswarm).
    pub(crate) sealed: bool,
    /// `--profile` and `-s`: the profile of the run and its overrides.
    /// TODO/cli.md T-1654.
    pub(crate) profile: Option<String>,
    pub(crate) sets: Vec<String>,
    /// `fs.wrap` of the profile: a prefix of the command.
    pub(crate) wrap: Vec<String>,
    /// The words of the command that `wrap` already holds: the runtime of
    /// the profile (TODO/cli.md T-1661).
    pub(crate) drops: Vec<String>,
    /// `needs` of the profile: the programs that the image must have
    /// (TODO/cli.md T-1659).
    pub(crate) needs: Vec<String>,
    /// `--network broker` with `--ports` and `--allow`: the egress broker
    /// of the run (TODO/net.md T-1665).
    pub(crate) broker: bool,
    /// `--podvm-confine=SPEC`: the confine layer (TODO/enter.md T-1658).
    pub(crate) confine: Option<podvm_enter::confine::Ask>,
    pub(crate) ports: Vec<u16>,
    pub(crate) allow: Vec<String>,
    /// `--device`. Carried raw here and parsed now: a malformed shape is a
    /// flag error, before anything is fetched. TODO/enter.md T-0501.
    pub(crate) devices: Vec<String>,
    /// `--unsafe-host-paths`. Opts into the userland pass-through with no
    /// `PODVM_MAPS` anchoring. TODO/enter.md T-1407.
    pub(crate) unsafe_host_paths: bool,
    /// `-v`. Parsed now, so a shape podvm cannot serve is a flag error
    /// before anything is fetched. TODO/enter.md T-1653.
    pub(crate) volumes: Vec<podvm_enter::userland::Volume>,
    /// `--timeout SECONDS` (podman's spelling): the wall bound of the run,
    /// after which the payload's tree is killed. TODO/cli.md T-1655.
    pub(crate) timeout: Option<u64>,
    /// `--read-only` and the guest paths of the `:ro` volumes. TODO/cli.md
    /// T-1655: the interposer's read-only layer serves them with `EROFS`.
    pub(crate) read_only: bool,
    pub(crate) ro_volumes: Vec<String>,
    /// The guest paths of `--tmpfs`, made writable directories of the run.
    pub(crate) tmpfs: Vec<String>,
    /// `size=` of `--tmpfs /tmp`: the cap of `/tmp` on the syscall tier
    /// (TODO/enter.md T-1657); no other tier enforces it.
    pub(crate) tmp_size: Option<u64>,
    /// `--ulimit` and `--security-opt no-new-privileges`. TODO/cli.md T-1655.
    pub(crate) hardening: podvm_supervise::table::Hardening,
    /// M5 and T-0804. ⚠ Carried in one struct so `run`, `create` and the
    /// launcher cannot each grow their own copy of the same three answers.
    pub(crate) ask: crate::complete::Ask,
}

impl Opts {
    /// TODO/cli.md T-1649. The first word of the caller where `argv[0]`
    /// is the Entrypoint of the image and the caller gave a command: the
    /// word that a refusal of that script names after `--entrypoint`.
    /// `None` under `--entrypoint`, with no command, or with no
    /// Entrypoint (the command is then `argv` itself).
    pub(crate) fn retry_word(&self, image_entrypoint: Option<&[String]>) -> Option<&str> {
        if self.entrypoint.is_some() || image_entrypoint.is_none_or(<[String]>::is_empty) {
            return None;
        }
        self.command.first().map(String::as_str)
    }
}

impl Opts {
    /// The options with no flag given.
    pub(crate) fn new() -> Opts {
        Opts {
            rm: false,
            detach: false,
            name: None,
            env: Vec::new(),
            workdir: None,
            entrypoint: None,
            platform: None,
            pull: "missing".into(),
            insecure: Vec::new(),
            tls_verify: None,
            tty: false,
            image: None,
            command: Vec::new(),
            user: None,
            tier: None,
            qemu_args: Vec::new(),
            mem: None,
            cpus: None,
            net_none: false,
            sealed: false,
            interactive: false,
            channel: None,
            profile: None,
            sets: Vec::new(),
            wrap: Vec::new(),
            drops: Vec::new(),
            needs: Vec::new(),
            broker: false,
            confine: None,
            ports: Vec::new(),
            allow: Vec::new(),
            devices: Vec::new(),
            unsafe_host_paths: false,
            volumes: Vec::new(),
            timeout: None,
            read_only: false,
            ro_volumes: Vec::new(),
            tmpfs: Vec::new(),
            tmp_size: None,
            hardening: Default::default(),
            ask: crate::complete::Ask::default(),
        }
    }
}
