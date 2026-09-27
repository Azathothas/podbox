# podssh

`crates/podbox-ssh`. Not from `TOOL.md`: the two trees this derives from are
named below and neither is in the corpus.

`podssh` is ssh between two machines that can each reach a rendezvous, where
the agent opens no listening socket and the operator's client is a real `ssh`.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is
the work order. [RULES.md](RULES.md) is how an entry closes.

---

### T-1401 `podssh`: ssh over a rendezvous, with no listening socket on the agent

Source:      `https://github.com/talaria0101/sandssh` and
             `https://github.com/Azathothas/podbox`; licence determinations are
             in [reference-map.md](reference-map.md)
Category:    podssh
Priority:    P1
Effort:      L
Status:      done

Problem:     Two shapes reach a machine that cannot be dialled, and each is
             missing something the other has. `sandssh` gets the transport
             right -- a relay it dials out to, and an ssh server under it -- but
             its client is its own and the experience is its own. `podbox`
             gets "it is just a program" right -- no daemon, no image, a native
             binary -- but has no way to reach a machine that cannot listen.
             An agent on a restricted host needs both: the far side opens
             nothing, and the operator types `ssh`.
Premise:     ⭐ **The transport is a byte pipe and nothing in it is ssh.** A
             `ProxyCommand` is ssh's own mechanism for splicing one in, so a
             correct pipe plus a correct ssh server is a first-class ssh
             session and not an imitation of one. `sandssh`'s reading is that
             the relay protocol is three lines -- a role, a name, a key -- and
             everything above it is opaque. Both were studied rather than
             assumed: `references/` holds neither tree, and
             `crates/podbox-ssh/README.md` records what was taken from each.
Approach:    A crate of its own so no flag has to serve two products, exposed
             as `podbox ssh` so there is still one binary and one parity table
             ([podvm.md](podvm.md)'s rule). Transports are a `Stream` trait
             with `tcp`, `tls`, `ws`, `wss`, `unix` and `exec` behind it, plus
             HTTP `CONNECT` and SOCKS5 dialers, so a relay is a URL and a
             missing one is a command line rather than a patch. The agent runs
             `serve`, which dials out, registers a name and starts the host's
             own ssh server on a socketpair; the operator runs `connect` under
             `ProxyCommand`. The relay protocol is `sandssh`'s `SANDSSH1` and
             podssh's own `PODSSH1`, so the two interoperate.
Decision:    The ssh server is autodetected and never shipped: `sshd -i` by
             default, `--server` for anything else, `--forward host:port` to
             reach one already running. ⛔ No chroot and no container: the
             agent side is a process the user could have started by hand.
             ⛔ No relay is compiled in. `catalog.rs` records the egress
             measurements with their dates and methods, and a URL is the only
             thing that selects one.
Prove:       `experiments/380-podssh-e2e.sh` exits 0 (report in
             `experiments/results/podssh-e2e.txt`, public-relay reading in
             `experiments/results/podssh-egress.txt`), and
             `cargo test -p podbox-ssh` passes 29 unit tests.

**Done 2026-09-27.** `crates/podbox-ssh` carries the crate;
`podbox ssh <command>` is `podssh`'s parser under the podbox verb, with
rows in `crates/podbox-cli/src/parity.rs`, so `podbox system info` lists it.
The Prove exits 0 with **12 of 12 cases green and none skipped**: the four
transports `unix`, `ws+unix`, `tls+unix` and `wss+unix`, each against a byte
pipe **and** against a real `ssh` client driving `podssh connect` as its
`ProxyCommand` into a real `sshd -i`; `podbox ssh version`; and three interop
cases against the sandssh tree unchanged -- its relay under podssh on both
ends, its client against podssh's relay and server, and its server under
podssh's relay and client. The interop cases are the no-vendor-lock-in claim
made into a measurement: a protocol that only talks to itself is a fork of
sandssh and not a replacement for it. The TLS cases
verify against a generated CA and leaf, not `--insecure`. Separately,
`podssh forward --proxy http-connect://<relay> --target railway.new:22
--expect-banner SSH-` reached a real ssh server through all three public :443
relays, each returning the full 684-byte `SSH-2.0` KEXINIT with `mlkem768` and
`ssh-ed25519` named.

⚠ **The defect this entry closes was found by running it and not by reviewing
it.** With `--server cat`, `ssh` sent its 22-byte version string, `podssh
connect` read it from standard input and wrote it to the relay, and the echo
never came back: `ssh` hung until `timeout` killed it. The identical byte-pipe
case passed, because a closed pipe reaches EOF and a `ProxyCommand`'s
socketpair does not, and `SO_RCVTIMEO` does not apply to it either. The
single-threaded pump therefore blocked in `stdio.read` before it ever read the
relay. The fix is that a source which cannot be timed out is never handed to
the pump: `ChannelReader` (`crates/podbox-ssh/src/transport/mod.rs`) drains
such a descriptor on a helper thread and returns `WouldBlock` when nothing has
arrived, and `Stdio` and `Exec` both read through it.

⭐ **The regression test was shown failing against the old behaviour before it
was kept.** `a_source_the_pump_cannot_time_out_returns_would_block`
(`crates/podbox-ssh/src/util.rs`) asserts the property the pump needs. Against
a `ChannelReader` restored to a direct read of the descriptor -- the shape that
shipped -- the test hangs and `timeout` kills it; against the fix it passes in
0.01 s. `pump`'s polling order was already correct and is not what changed.

⚠ **Three review passes found three more things, and each is fixed with a
test or a named skip.** The generated `sshd_config` is a default podssh
chooses, so it no longer says `PermitRootLogin yes`; it says
`prohibit-password`, because a root key login is the operator's decision via
`--server` and not podssh's to make. The runtime directory
(`/tmp/podssh-<uid>`, a predictable path that holds a host key) is now
`0700`, where it was left at the umask. Both are asserted by
`generated_sshd_config_is_owner_only_and_does_not_permit_root_login`, which
skips by name on a uid with no `/etc/passwd` -- `ssh-keygen` cannot run there
at all -- rather than passing vacuously. The third pass found nothing new in
the transports.

⚠ **What this lane could not do.** The e2e harness runs every case over a unix
socket, because the lane forbids binding a TCP port; and `sshd -i` under an
unprivileged uid needs a working `getpwnam`, which a lane with no
`/etc/passwd` supplies through `SANDSSH_PASSWD` and `LD_PRELOAD`. A normal host
sets neither and the same cases run unchanged, but that is a claim about a
host this lane is not, and the harness is where it is checked. The TCP
transport itself is exercised against the public relays above, which is a real
TCP path.

---

### T-1402 The session is a userspace line discipline, because the cage has no pty

Source:      `https://github.com/talaria0101/sandssh`, its `shell/errandsh`;
             `https://github.com/hackerschoice/hackshell`, its `hackshell.sh`
Category:    podssh
Priority:    P1
Effort:      L
Status:      open

Problem:     ⛔ **T-1401 delivers a transport and proves one-shot commands. It
             does not deliver the interactive session the brief asks for, and
             its 12-of-12 e2e would pass with no shell present at all.** A
             sealed cage has no `/dev/ptmx`, no devpts and `mknod` denied, so
             there is no kernel pty: `ssh -t` fails and the session degrades to
             a dumb pipe with no echo, no line editing, no history and no real
             signals. That is the state `errandsh` exists to fix, and it is the
             state podssh currently ships in.
Premise:     ⭐ **Read, not assumed.** `sandssh/shell/errandsh` is 432 lines of
             Python and is a userspace line discipline: it replaces the login
             shell and reimplements echo, prompt, persistent history, full
             line editing, Ctrl-R, bracketed paste and Tab completion, and it
             sends a **real SIGINT** with `os.killpg` to the job's process
             group. `hackerschoice/hackshell` is 2040 lines of bash and is a
             post-login configuration: stealth plus ergonomics, sourced into a
             shell that must already be interactive. ⚠ Neither is what podssh
             should be: errandsh needs the far side to run it and is not ssh,
             and hackshell cannot help where no interactive shell exists.
             ⭐ **The seam already exists.** `crates/podbox-ssh/src/sshserver.rs`
             hands the child one socketpair for stdin and stdout, which is
             exactly where a line discipline belongs -- above the transport and
             below ssh's own protocol, so podssh never has to read ssh.
Approach:    A `ServerSpec::Shell` beside `Auto`, `Command` and `Forward`, in
             Rust, in `podbox-ssh`: echo, redraw, history, Ctrl-R, bracketed
             paste, Tab completion, and a real SIGINT to the job's process
             group. ⛔ No pty, no `/dev/ptmx`, no chroot. It works with a
             vanilla `ssh` on the operator side and a vanilla `sshd` on the
             agent side, which is the brief's "one side may use vanilla ssh"
             requirement.
             ⚠ **Do not implement ssh to get this.** The crypto, auth and
             channel layer stays the host's `sshd`/`dropbear`; the line
             discipline replaces only the session half. Reimplementing both
             ends would also create new lock-in, which is worse than sandssh
             rather than better.
Decision:    The shell is podssh's, and the shell is the session. `env` and cwd
             persist across commands the way a login session does, because the
             discipline owns one long-lived child rather than one per command.
Prove:       `experiments/370-podssh-shell.sh` drives an **interactive** session
             over a pipe with no pty and asserts echo, line editing and a real
             Ctrl-C reaching the job's process group -- not a one-shot command,
             which is the test T-1401 already has and which passes without this
             entry.

### T-1403 podbox shims the ssh server name, so the line discipline needs no configuration

Source:      [podvm.md](podvm.md) T-1302; [cli.md](cli.md) T-0803;
             [packaging.md](packaging.md) T-1001
Category:    podssh
Priority:    P1
Effort:      M
Status:      open

Problem:     T-1402's line discipline is reachable today as
             `podbox ssh serve --shell`, which is one more thing for an
             operator to type. The brief asks for "seamless, not noticeable",
             and a shell that has to be asked for by name is neither.
Premise:     ⭐ **The pattern exists in this tree, is ruled on, and is tested.**
             `crates/podbox-cli/src/names.rs` carries
             `ALIASES = ["docker", "podman", "podvm"]`: multicall on `argv[0]`,
             **symlinks never wrapper scripts**, and a banner naming which name
             was used. T-0803 rules the shape and T-1001 gives the reason -- a
             wrapper script is a second artefact and it breaks the memfd rung.
Approach:    `ssh`, `sshd` and `dropbear` as further aliases, installed by
             `system install-names` as symlinks. The `sshd`/`dropbear` shim
             injects `PermitTTY no` plus a forced command that is T-1402's
             discipline, so the operator configures nothing and still gets the
             interactive session.
Decision:    ⛔ The banner obligation is not optional: `TOOL.md` section 4.1
             says a caller has to be able to tell which tool ran, so a shimmed
             server says so in one line. ⛔ T-0803's operator ruling carries
             over unchanged -- a shim must not overwrite a real client on PATH
             without an explicit flag, because a machine with a working client
             is a machine where podssh is the wrong tool.
Prove:       `experiments/371-podssh-names.sh` asserts the symlinks are
             symlinks, that the banner names the real tool on every shimmed
             name, and that a real `ssh` on PATH is refused without the flag.

### T-1404 The remote and machine verbs, and the tier that decides where the server lives

Source:      [podvm.md](podvm.md) T-1302, T-1304; this file's T-1401
Category:    podssh
Priority:    P2
Effort:      M
Status:      open

Problem:     `podbox ssh` reaches a host that already has a server. The machine
             tier has no server at all: `podbox exec` crosses a serial line
             (T-1304). Two different far ends need one vocabulary, or an agent
             has to learn which verb belongs to which tier.
Premise:     T-1302 settles that a second binary is a second parity table, so
             these are rows in the one table with the tier as a flag rather
             than a separate product.
Approach:    `podbox remote ssh` -- vanilla `ssh` on the far side, podssh's
             transport underneath. `podbox machine ssh` -- the podman-parity
             name, server inside the guest. Both ride one transport; the tier
             decides only where the server lives.
Decision:    ⚠ The static/interpose honesty rule applies to any identity shim
             on this path: `LD_PRELOAD` cannot reach a static binary, so that
             case is named rather than allowed to no-op, which is the shape
             [interpose.md](interpose.md) already uses for a payload with no
             `PT_INTERP`. ⭐ Where the server runs inside the guest the identity
             question largely goes away, because the guest has its own
             `/etc/passwd` in the image.
Prove:       `experiments/372-podssh-verbs.sh` exits 0, and it drives
             `podbox remote ssh --help` and `podbox machine ssh --help` rather
             than reading the table, so a row with no arm fails it.
