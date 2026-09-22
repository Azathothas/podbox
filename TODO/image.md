# image

`crates/podbox-image`. `TOOL.md` section 6.2, milestone M1.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

The registry plane is ordinary HTTPS and file I/O and works on this runtime.
⛔ **Do not gold-plate it.** Three things about this environment change it, and
one of them cost a prior session real time with an error message that named
neither the cause nor the directory.

---

### T-0201 Registry client, HTTPS only, with no plain-HTTP fallback

Source:      `TOOL.md` section 6.2, section 11.1
Category:    image
Priority:    P0
Effort:      M
Status:      done 2026-09-08

Problem:     tcp/80 egress is broken on the studied runtime. Anything that falls
             back to plain HTTP **hangs** rather than failing, so a fallback
             that exists to improve reliability makes the failure untimed and
             undiagnosable.
Premise:     Read, from a single-session observation of the target. It is
             consistent with every distro fixup in section 6.4 being a protocol fixup
             rather than a mirror fixup.
Approach:    Implement the OCI distribution endpoints actually used: token auth,
             manifest by tag and by digest, manifest lists for
             `linux/amd64`, and blob fetch. HTTPS only. A registry that offers
             only `http://` is a named refusal, not a downgrade.
             Every request carries a timeout and a bounded retry, per
             [RULES.md](RULES.md) section 8.
Decision:    A blocking client. podbox has no reason to be async, and an async
             runtime is a large dependency with nothing to do here. The crate
             sweep for the client and its TLS is T-0905 and T-0906, and neither
             lands without a measured size delta.
Prove:       `podbox pull public.ecr.aws/docker/library/alpine:3.20 && podbox images --format '{{.Digest}}' public.ecr.aws/docker/library/alpine:3.20 | grep -qx "$(docker image inspect public.ecr.aws/docker/library/alpine:3.20 --format '{{index .RepoDigests 0}}' | cut -d@ -f2)"`

**Done 2026-09-08.** `crates/podbox-image/src/registry.rs`, driven by
`experiments/150-image-acquisition.sh`, which exits 0. The `Prove` above is its
clause 1 and the two digests are equal:
`sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b`.

Challenge-driven bearer auth rather than a table of hosts: the 401's
`WWW-Authenticate` names the realm, and the token is cached per
`<endpoint>|<scope>` rather than per host, because a token minted for one
repository does not authorise another and a host-keyed cache would send it
anyway and read the 403 as a permission problem.

⛔ **The refusal is driven, not asserted.** Clause 4 of the script runs
`podbox pull http://registry.invalid/...` under `timeout 30`: exit 124 would be
the hang this refusal exists to prevent, and podbox exits **2** with
`podbox speaks HTTPS only and will not downgrade`. A registry whose token realm
is `http://` is refused separately and is a runtime failure rather than a usage
one, because it is not the caller's input.

⚠ Two things in the entry's `Approach` are narrower in the code than the prose
suggests, and both are deliberate. Retry covers 429 and the five-hundreds only:
a 404 or a 403 retried three times is three wrong answers and a slower message.
And `Retry-After` is honoured in its seconds form; the HTTP-date form is legal
and falls back to the ordinary backoff, because a mis-parsed date is a wait of
unknown length and [RULES.md](RULES.md) section 8 forbids one.

⚠ The manifest read is bounded before it is taken, not after: `take(ceiling + 1)`
ahead of `read_to_end`, so a registry that streams forever cannot exhaust
memory.

⭐ **Three registries, not one.** The challenge-driven auth was driven against
`public.ecr.aws`, `ghcr.io` and `quay.io` as well as Docker Hub, and all four
answer. That is what a table of hosts would not have survived, and it is why
`experiments/140-space-precheck.sh` and `experiments/160-store-gc.sh` could be
moved off the Hub when its anonymous rate limit turned a check about disk space
into a check about somebody else's quota.

---

### T-0202 A content-addressed store, and digest parity with docker

Source:      `TOOL.md` section 5 M1, section 6.2
Category:    image
Priority:    P0
Effort:      M
Status:      done 2026-09-08

Problem:     Without content addressing, `pull` twice costs twice and nothing
             can be verified. Without digest parity with docker, an agent that
             checks a digest against a published one gets a different answer and
             cannot tell why.
Premise:     Read. M1's acceptance is exactly that parity.
Approach:    Store blobs under `sha256:<hex>`, verify every blob against its
             descriptor digest **as it is written** rather than afterwards, and
             key images by manifest digest. Reject a blob whose computed digest
             differs, naming both.
             ⭐ Verify before use, not after download. The reasoning is in the
             corpus at
             `references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:149-157`,
             which checks a payload's content hash before an in-memory exec "so
             an in-memory exec never runs unverified bytes". The same holds for
             a layer about to be extracted as root.
Decision:    One store shared by images and containers, with a lock, rather than
             a store per container. The GC race that decides this is T-0204.
Prove:       `podbox pull public.ecr.aws/docker/library/alpine:3.20 && podbox pull public.ecr.aws/docker/library/alpine:3.20 2>&1 | grep -qi 'already' && sha256sum "$(podbox inspect --format '{{.Store}}' public.ecr.aws/docker/library/alpine:3.20)"/blobs/sha256/* >/dev/null`

**Done 2026-09-08.** `crates/podbox-image/src/store.rs` and
`crates/podbox-image/src/digest.rs`, driven by
`experiments/150-image-acquisition.sh` clauses 2 and 3, which is the `Prove`
above as a script: the second pull prints `55afa1ecc21d: Already exists` and
fetches nothing, and all 4 stored blobs hash to the name they are stored under.

⭐ **The digest recorded is the one computed over the bytes the registry
served for the reference that was asked for.** For a multi-platform tag that is
the **index** digest, not the per-platform manifest digest, and recording the
wrong one is exactly the parity this entry is about: `docker image inspect`
reports the first. The store carries both, as `digest` and `manifest_digest`.
The bytes are written to `blobs/` verbatim; re-serialising a parsed document
would produce a different digest.

⛔ **Verification is on the way in AND on the way out.**
`digest::Verifier` hashes what reached the sink rather than what was offered,
counts what actually arrived rather than trusting the descriptor's `size`, and
the staged file is renamed into `blobs/` only when both matched. `read_blob`
re-hashes on the way out, which fires when something outside podbox has edited
the store, and a test drives that by overwriting a stored blob.

⚠ **`{{.Size}}` is the compressed bytes podbox holds, and the column says
`SIZE (STORED)`.** M1 did not extract, so the uncompressed number had not been
measured here, and printing docker's heading over a different quantity would be
a number that was not measured.

⭐ **CORRECTED 2026-09-08 BY M2, AND THE PREMISE WAS WRONG.** This paragraph
said "docker's `SIZE` is the sum of the uncompressed layers". **Measured on
this host, against the same image by the same digest, it is not.**

| | bytes |
| --- | --- |
| `docker image inspect alpine:latest --format '{{.Size}}'` | 3,857,242 |
| every blob podbox holds for it, summed | **3,857,242** |
| podbox `{{.Size}}` (layers and config, no manifests) | 3,847,002 |
| the layer's real uncompressed length, from gzip `ISIZE` | 8,697,856 |
| `docker history` for the same layer | ~9.07 MB |

docker 29.3.1 here runs the **containerd** image store
(`io.containerd.snapshotter.v1`), and its `.Size` is the size of the content in
the content store, which is compressed. It matches podbox's blob total **to the
byte**; the 10,240 difference against podbox's own `{{.Size}}` is the manifest
and index blobs, which podbox does not count as image payload.

⛔ **So the divergence this entry documented does not exist on this docker, and
the open question that asked what to do about it was asking about a claim
rather than a reading.** ⚠ It is one host and one docker: the classic (non
containerd) image store reports the uncompressed total, and this is recorded as
a reading from this machine rather than as docker's definition.

⚠ **And "uncompressed size" is three different numbers**, which is why podbox
names which one it prints. The tar STREAM length (8,697,856 here, read exactly
from gzip's trailer) is what `podbox extract` reports and what the space
precheck needs; the APPLIED filesystem size is what `docker history` shows; and
`du` over the extracted tree is a third. Reporting any of them as "the"
uncompressed size without saying which would be the same defect this paragraph
was written to avoid.

⚠ **The store is one shared directory with a version discriminator**,
`store.json`'s `podbox_store: 1`. A store a later podbox wrote is refused by
name rather than parsed as though its fields meant what they mean here, and the
index is written to a staging name inside the store and renamed over the old
one, so a process killed mid-write leaves the previous index rather than a
truncated one.

---

### T-0203 Check `statvfs` for blocks and inodes, and name the destination

Source:      `TOOL.md` section 6.2, section 8; `paper_final.md` section 8.2 and F7
Category:    image
Priority:    P0
Effort:      S
Status:      done 2026-09-08

Problem:     A payload that overshoots the default temporary directory produces
             an error naming neither space nor the directory. On the studied
             runtime `/tmp` is 64 MiB, which almost no image fits in.
Premise:     ⭐ Read at file and line, and the reason the message is useless is
             visible there.
             `references/mhx__dwarfs/tree/src/utility/filesystem_extractor.cpp:544-552`
             takes `rv` from `write_range_data`, which returns a libarchive
             `la_ssize_t`, then formats it into `"short write: {} != {}"` as
             though it were a byte count. `-20` is `ARCHIVE_WARN`, a status
             code; the underlying `archive_errno` is `ENOSPC`. The message
             mentions neither the errno nor the path.
             ⚠ dwarfs is a split licence, MIT for the read path and GPL-3.0 for
             the write path ([reference-map.md](reference-map.md)), and is read
             only here.
Approach:    Before any download and again before any extraction, `statvfs` the
             destination and check **blocks and inodes both**. On failure, name
             the destination, the free amount, the required amount and the unit.
             Choose the destination by free space among the writable paths
             T-0104 probed, rather than taking `TMPDIR`.
             ⚠ The same trap is in the corpus as a shipped default:
             `references/VHSgunzo__memfd-exec/tree/src/executable.rs:580-584`
             falls back to `env::temp_dir()`, then `/dev/shm`, then
             `$HOME/.cache`, with no space check on any of them.
Decision:    Refuse up front rather than streaming until `ENOSPC`. A partial
             extraction has to be cleaned up on a filesystem that is already
             full, which is the state in which cleanup is least likely to work.
Prove:       `./experiments/140-space-precheck.sh` exits 0

**Done, 2026-09-08.** `crates/podbox-image/src/space.rs`, called before any
download from `crates/podbox-image/src/pull.rs` and before any extraction from
`crates/podbox-extract/src/lib.rs`.
`experiments/140-space-precheck.sh` exits 0, on two real tmpfs mounts rather
than fixtures, and the two refusals it drove are:

```
not enough space at <dest> for alpine:latest: 1012.0 KiB free, 19.7 MiB needed
  (3.7 MiB of payload plus 16.0 MiB of headroom)
not enough inodes at <dest> for alpine:latest: 5 free, 9 needed. This filesystem
  has 256.0 MiB of free space and cannot create the files anyway
```

Destination, free amount, required amount and unit, in both. **0 blobs** were
written before either refusal, which is the whole point of a precheck: a
partial store has to be cleaned up on a filesystem that is already full.

⭐ **CLOSED, 2026-09-08, by M2.** The second call site exists:
`crates/podbox-extract/src/lib.rs` calls the same `space::require` before the
first layer is read. That was the only reason this was `partial`.

⛔ **It is before the layer LOOP, not inside it, and the placement is the
point.** An image with no layers, or one whose layers are all already present,
runs that loop zero times, and a guard inside it would never be reached by the
caller who has nothing. The same rule caught a real defect in M1's
`--format` validation, where a template checked inside the loop over records
was never checked at all against an empty store.

⚠ **What the precheck needs is the UNCOMPRESSED size, and the manifest carries
the compressed one.** gzip's trailer carries `ISIZE`, the uncompressed length
modulo 2^32, so a gzip layer's figure is **read rather than guessed**: measured
on 2026-09-08, alpine's single layer is 3,846,391 bytes compressed and
8,697,856 uncompressed. A zstd layer, or a gzip trailer that has plainly
wrapped, falls back to a multiplier, and `Extracted::uncompressed_estimated`
carries which of the two it was so a caller printing the number can label it.
⛔ The inode figure IS an estimate in every case, and says so: the real count
is the number of entries across every layer, which is not known until the
layers have been read, and reading them is what this check exists to happen
before.

⚠ **Two corrections to the entry, both found by writing it.**

1. The `Prove` named `90-space-precheck.sh`, and 90 is
   `experiments/90-nsswitch-contract.sh`. `experiments/README.md` rules that a
   number is never reused, because a citation of it has to keep meaning what it
   meant. The script is `experiments/140-space-precheck.sh` and the `Prove`
   above names it.
2. The `Approach` says `statvfs`. The code calls **`statfs(2)`**, which is the
   syscall behind `statvfs(3)` and is what
   `crates/podbox-probe/src/writable.rs` already calls, so free space has one
   read path in this tree rather than two.

⚠ **`f_files == 0` means inodes are not counted**, not that there are none.
tmpfs allocates them dynamically, and a check reading `f_ffree` as zero there
would refuse every write on a filesystem with room.

⚠ **"Before any download" means before any LAYER.** The manifest is fetched
first, because the manifest is what says how large the layers are, and it is a
few kilobytes against their megabytes. The consequence is visible in the script:
every clause needs the registry to answer once, so a registry that refuses is a
`SKIP` with exit 2 rather than a failed check.

⚠ **The inode clause had to be made deterministic.** Mounting straight into a
12-inode tmpfs may or may not leave room for the store's own five directories,
so the refusal came sometimes from this check and sometimes from `mkdir`. Both
are honest, and the record flipped between two values on re-runs. The store is
now built while inodes are plentiful and the remainder consumed afterwards, and
the free count at the moment of the pull is recorded beside the verdict.

---

### T-0204 `images`, `rmi`, `tag`, and a store GC that cannot delete a running container's rootfs

Source:      `TOOL.md` section 5 M1, section 6.8
Category:    image
Priority:    P1
Effort:      M
Status:      done 2026-09-09

Problem:     A GC that runs while a container is using an extraction deletes the
             tree out from under it, and the payload's failure names a missing
             file rather than a concurrent deletion.
Premise:     Read at file and line. The mechanism that closes it is
             `references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:275-278`:
             the cache lock guard is held **through exec**, with its fd left
             inheritable, "so a concurrent process's GC cannot delete the
             package while it is still in use".
             ⚠ The opposite failure is also in the corpus. `ruri` tracker issue
             #59 records that its `-U` unmount reached **outside** the container
             when the container directory was under a FUSE mount. A cleanup verb
             that resolves outside its own subtree is the same class of defect.
Approach:    Hold a lock fd on each in-use rootfs, inherited across the exec, and
             have `prune` and `rmi` skip anything locked and say which. Refuse
             any `rm` or `prune` whose resolved target is outside the store, and
             resolve it with the same containment check as T-0304.
Decision:    An inheritable lock fd rather than a pid file. A pid file is stale
             the moment a process dies unexpectedly, and the check that clears a
             stale one is the race this is closing.
Prove:       `podbox run -d --name gc-probe public.ecr.aws/docker/library/alpine:3.20 sleep 30 && ! podbox image prune -af 2>&1 | grep -q "$(podbox inspect --format '{{.Image}}' gc-probe)" && podbox rm -f gc-probe`

**Done 2026-09-09, in the two halves below.** `images`, `image ls`, `rmi`, `image rm`, `tag`,
`image prune` and `inspect` are implemented in `crates/podbox-cli/src/images.rs`
over `crates/podbox-image/src/store.rs`, and the lock is
`crates/podbox-probe/src/sys.rs`'s `flock(2)` on an fd opened **without**
`O_CLOEXEC`, which is the inheritance the mechanism depends on.
`experiments/160-store-gc.sh` exits 0 and drives all five clauses.

⛔ **The half that is not done is the `Prove` above**, which holds the lock with
`podbox run -d`. `run` is M3, [milestones.md](milestones.md) T-1104. The holder
in the script is `flock(1)` taking the same advisory lock on the same file, so
the refusal path is driven for real; what is not yet driven is podbox holding it
across its own exec.

What the script established:

| clause | reading |
| --- | --- |
| a shared blob under two tags | survives removing one of them |
| `rmi` under a holder | exit 125, "is in use by a running container" |
| `image prune -af` under a holder | exit 0, `skipped: alpine:latest is in use`, image kept |
| the holder released | `rmi` exits 0 and frees all 4 blobs |
| a blob path resolving outside the store | refused, and the canary survived |

⭐ **Reachability is computed over what SURVIVES, never over what was
deleted.** Computing it the other way deletes a shared layer with the first
image that goes, and clause 1 is that case.

⚠ **A defect in the harness, not in podbox, and it is worth recording because
it is the mechanism working exactly as intended.**
`flock FILE -c 'sleep N'` runs the sleep as a **child**, which inherits the
locked fd, so killing `flock` leaves the sleeper holding the lock. The first run
of clause 4 failed for that reason. The script now `exec`s the sleeper so one
process holds the fd. That inheritance is precisely what this entry buys.

⚠ **`prune -f` is accepted and does nothing**, and `--help` says so rather than
leaving it as a flag no code reads: docker's `-f` suppresses a confirmation
prompt, and podbox never prompts ([cli.md](cli.md) T-0806).

⭐ **The remaining half closed 2026-09-09 by M3.** The lock and its GC refusal
were driven at M1 without the exec they exist for; `podbox run` is that caller.
It takes the lock **before** checking whether the rootfs is extracted, so a
concurrent `rmi` cannot delete the tree between podbox deciding it is there and
entering it, and calls `Lock::hand_to_payload` immediately before the fork that
leads to the payload's `execve`.

⚠ `--rm` drops the lock **before** removing the rootfs. Holding it while
deleting would be podbox refusing its own request, which is the kind of deadlock
that only appears once the caller exists.

⛔ [T-0211](#t-0211-an-image-lock-outlives-its-holder-whenever-anything-forks) is
what this half cost: handing the fd to the payload the obvious way, by leaving
it inheritable from the moment it was opened, leaked it into every unrelated
fork in between.

---

### T-0205 Re-test podman with `vfs` and `ignore_chown_errors` before repeating "no path exists"

Source:      Found while reading `references/containers__storage` against `paper_final.md` section 6
Category:    image
Priority:    P3
Effort:      S
Status:      done 2026-09-22

Problem:     The corpus states that podman's layer application has no path on
             this runtime. The code carries a configuration option that turns
             exactly that failure into a warning, and nobody has run the
             combination.
Premise:     ⭐ **Read at file and line, and it disagrees with the claim.**
             `references/containers__storage/tree/pkg/archive/archive.go:804-808`:

             ```go
             if ignoreChownErrors {
                     fmt.Fprintf(os.Stderr, "Chown error detected. Ignoring due to ignoreChownErrors flag: %v\n", err)
             } else {
                     return err
             }
             ```

             It is reachable by configuration, not only internally:
             `references/containers__storage/tree/types/options.go:473-474`
             builds `<driver>.ignore_chown_errors` from `storage.conf`, and both
             drivers accept it:
             `references/containers__storage/tree/drivers/vfs/driver.go:59-62`.
             Beside that, `paper_final.md` F6 measured that
             `--storage-driver vfs` **initializes** on podman 4.3.1.
             ⚠ Nothing here is a result. The combination has not been run.
Approach:    Run it in the reconstruction and record the outcome either way.
             ⛔ A negative result is a result and gets committed: if it still
             fails, the reason is worth having, because "no path exists" would
             then be a measured claim rather than an inherited one.
Decision:    This changes nothing about podbox's design and it changes a
             comparative claim podbox's own documentation would otherwise
             repeat. That is why it is P3 and not dropped.
Prove:       `./experiments/95-podman-vfs-ignorechown.sh` exits 0 or 1, never 2, and its output is committed to `experiments/results/`

**Done 2026-09-22.** The combination runs and the corpus claim is
answered: vfs with `ignore_chown_errors` opens the path the bare driver
refuses. Driven on host podman (client 6.1.2, machine 5.8.6) by
`experiments/95-podman-vfs-ignorechown.sh`, exit 0, with the run in
`experiments/results/podman-vfs-ignorechown.txt`. The chown wall is
staged rather than assumed: a seccomp filter
(`experiments/lib/chowndeny.py`) denies chown to any but the caller's
own uid for the unpack tree, which is the target runtime's shape
(mapped ids work, unmapped fail). Without the flag the load is refused
(`lchown /data/greeting: operation not permitted`, a fixture file owned
by uid 1234); with the flag the same load exits 0, and the loaded image
runs and prints hello. Two userns stagings were measured and refused
first: rootless maps every uid into the subuid range so nothing fails,
and a hand-mapped namespace breaks podman's own newuidmap call. One
observation is named rather than hidden: this podman prints no
`ignoreChownErrors` warning on stderr where the corpus
(`pkg/archive/archive.go`) prints one; the outcome is the flag's either
way, and a control with the flag explicitly false fails as without it.

---

### T-0206 A registry fixture, so the acceptance stops depending on somebody else's quota

Source:      Found while re-running `experiments/150-image-acquisition.sh` against its own committed reading
Category:    image
Priority:    P3
Effort:      L
Status:      done

Problem:     M1's acceptance pulls from Docker Hub, and Docker Hub answers
             `HTTP 429: TOOMANYREQUESTS: You have reached your unauthenticated
             pull rate limit` after enough anonymous pulls from one address.
             ⛔ A gate that a third party can turn red is not a gate. It is
             worse than a missing one, because it teaches a session to read
             `exit 2` as noise, and `exit 2` is the state this project uses for
             "could not run".
Premise:     ⭐ **Measured on 2026-09-08, in this repository.** The session that
             implemented M1 exhausted the anonymous quota with its own runs.
             `experiments/results/image-acquisition.txt` is a real reading and
             the script that took it now exits 2 rather than 1 when the quota is
             spent, which is correct and is also a gate nobody can run on demand.
             ⚠ Two of the four scripts were moved off the Hub as a stopgap:
             `experiments/140-space-precheck.sh` and
             `experiments/160-store-gc.sh` compare nothing against docker and
             default to `public.ecr.aws`. `experiments/150-image-acquisition.sh`
             cannot follow them, because its whole question is whether podbox's
             digest equals the one `docker image inspect` reports for the same
             tag.
Approach:    Serve the OCI distribution endpoints podbox uses from the store
             podbox already has, on loopback, and point both podbox and docker
             at it. The store is content-addressed and holds the manifest bytes
             verbatim, which is exactly what `GET /v2/<name>/manifests/<ref>`
             has to return, so the fixture is a reader over
             `crates/podbox-image/src/store.rs` rather than a second store.
             ⚠ Four endpoints, no more: `GET /v2/`, manifests by tag and by
             digest, and blobs. No push, no catalog, no pagination.
             ⛔ HTTPS, with a certificate the fixture generates and both clients
             are pointed at. A fixture that speaks plain HTTP would be the one
             thing `TODO/image.md` T-0201 refuses, wired into the acceptance.
Decision:    A fixture in this tree over the `zot` release binary, not over
             `registry:2`. Pulling the registry image to escape the pull limit
             is circular, and `zot` needs no image pull at all: one pinned
             binary plus a config, a storage dir, a generated cert and an
             htpasswd file, all staged host-side. The licence determination
             [reference-map.md](reference-map.md) requires before a new tree
             is used is recorded there (Apache-2.0, read 2026-09-22), and the
             minimal binary carries no extensions past the four endpoints and
             the auth this fixture needs. The htpasswd-required mode it serves
             is the shape [T-0209](#t-0209-registry-authentication-without-a-credential-ever-entering-this-tree)
             later drives with credentials.
             ⚠ The rejected alternative is authenticating to the Hub: it needs a
             credential, and `docs/security/secrets.md` keeps credentials out of
             this tree, so the acceptance would then run only where somebody has
             one.
Prove:       `./experiments/180-registry-fixture.sh` exits 0 with ALL outbound network blocked

**Done 2026-09-22.** `experiments/180-registry-fixture.sh` exits 0 three
consecutive runs on host podman 6.1.2, with
`experiments/results/registry-fixture.txt` committed: 21 clauses green, no
FAIL, the test password absent from the report. Five answers:

1. What it is. One pinned binary (`zot-linux-amd64-minimal` v2.1.21,
   85,459,246 bytes, sha256 verified every run against the release
   checksums line) plus a config, a seeded storage dir, a generated cert
   and an htpasswd file, all staged host-side. The Prove runs inside one
   `--network=none` driver container where zot and a lane-built podbox meet
   on container loopback. Two configs: open (podbox pull by tag and by
   digest, both inspecting to the seeded digest) and htpasswd-required
   (401 anon with a Basic challenge, 200 authed, wrong password 401, the
   shape [T-0209](#t-0209-registry-authentication-without-a-credential-ever-entering-this-tree)
   later drives with credentials; podbox is anonymous-only until then and
   no podbox clause touches the required config).
2. What it costs. The binary is fixture-only: downloaded per run into
   trap-removed scratch, verified, never committed, never shipped. The
   driver image is stock Debian plus curl (`experiments/180-driver.Dockerfile`,
   base pin shared with `experiments/Dockerfile.target`).
3. Does it pull C. Nothing new enters the artefact: zot is a separate
   process, and the change to the tree is the script, the Dockerfile, one
   opt-in `ENG_NETWORK` knob in `experiments/lib/engine.sh` (`none` only,
   unset by default, anything else refused), and this record.
4. Licence. Apache-2.0, determined in [reference-map.md](reference-map.md)
   before use. The binary is executed, never vendored and never embedded.
5. Binary or image. The pinned binary: no image pull at all, which is what
   beats the entry's `registry:2` Decision (amended above in the same
   change).

⚠ Traps measured on the way, each carried in the script header where it
bites: two zot servers never share one storage dir (the second dies on
the first's `cache.db` lock); curl without `-f` exits 0 on an HTTP error
page, so the size gates the fetch loop's break; NTFS carries no exec bit,
so the binary check is `-f` off-lane; native openssl needs Windows
spellings for every file it touches. What the runs did NOT prove: the
Windows port-forward path (TLS through it aborts; the fixture never uses
it, everything meets on container loopback).

⛔ **CORRECTED 2026-09-09 BY MEASUREMENT, AND THE PREMISE ABOVE WAS WRONG.** The
`Premise` says `150-image-acquisition.sh` "cannot follow them, because its whole
question is whether podbox's digest equals the one `docker image inspect`
reports for the same tag". That reads as though the Hub were required. It is
not: **docker pulls from `ghcr.io` perfectly well**, checked on this host on
2026-09-09, and its `RepoDigests[0]` for
`ghcr.io/pkgforge-dev/archlinux:latest` is
`sha256:b2507f1964270cab3cc190aa8df858521f6a7e45e969146a012877891d5dfc9b`,
which is the value podbox records. The question needs a registry **both tools can
reach**, not the Hub.

⭐ **So the stated problem is gone, and it cost one line.** Every script the
acceptance runs now reaches a registry with no anonymous pull quota, or none at
all:

| script | registry |
| --- | --- |
| `110-`, `130-`, `170-`, `220-`, `260-` | none: no network at all |
| `140-`, `160-` | `public.ecr.aws` |
| `150-`, `270-` | `ghcr.io` |

⚠ **And the operator's note that M2 made this worse was inherited from this
entry rather than measured.** It said `extract` needs a pulled image so the
acceptance spends quota on two. `220-extract-path-safety.sh` builds its store
**by hand** from crafted layers and pulls nothing, so extraction never cost a
pull in the acceptance at all.

⚠ **What the fixture would still buy, which is why this stays open at P3 rather
than closing.** ghcr having no quota today is a policy, not a guarantee, and
`public.ecr.aws` is somebody else's too. A fixture on loopback is the only thing
that makes the acceptance runnable with the network **off**, which is a stronger
property than "no quota" and is the one an unattended agent on a broken network
actually needs. ⛔ It is not closed as "no longer needed": the `Problem` above is
mitigated, not answered, and the `Prove` now says all outbound network blocked
rather than one host.

---

### T-0207 Fetch layers with bounded concurrency, and measure what it buys

Source:      `TOOL.md` section 6.2; `docs/conventions/forbidden-patterns.md`, the resources table
Category:    image
Priority:    P2
Effort:      L
Status:      done 2026-09-22

Problem:     `crates/podbox-image/src/pull.rs` fetches layers one after another.
             `docs/conventions/forbidden-patterns.md` names "a sequential
             awaited loop over independent IO" and what it caused: wall-time
             blowups as the data grows. alpine has one layer and hides it; a
             fifteen-layer image on a link with latency does not.
Premise:     ⭐ **Read in this tree, at file and line.**
             `crates/podbox-image/src/pull.rs` loops over
             `manifest.layers.iter().chain(once(&manifest.config))` and calls
             `client.blob` inside it, so every layer waits for its predecessor.
             ⚠ **Not yet measured.** The claim that this costs wall time here is
             a reading of the code, not a number, and taking the number is part
             of this entry rather than a prerequisite for it.
Approach:    A bounded pool of worker threads over the descriptor list, with the
             bound a named constant and not a per-machine guess. ⛔ The bound
             plus transient locks stays under sixteen concurrent `Lock`s in one
             process: every lock registers a fork-shed slot
             (`crates/podbox-probe/src/sys.rs:702`), and
             [T-1310](#t-1310-the-store-suite-exhausts-the-sixteen-fork-shed-slots-and-the-victim-varies)
             measured what exceeding it costs. Each worker
             stages, verifies and commits through the existing store functions,
             so there is one write path and not two.
             ⛔ The transcript stays in manifest order however the fetches
             interleave. A progress display whose order depends on scheduling is
             a display that reports the machine's mood.
             ⛔ A failure in one worker cancels the rest and removes every
             staged file, rather than leaving a partial store behind.
Decision:    Threads over an async runtime. `TODO/deps.md` T-0906 ruled a
             blocking client on a measured size delta, and adding an async
             runtime to parallelise four downloads reopens a decision that was
             closed against a number.
             ⚠ The genuine fork, with a recommendation: whether the bound is
             fixed or scales with the CPU count. Recommend **fixed**, because
             the constraint is the registry's willingness to serve, not this
             machine's cores, and a CPU-scaled bound on a 96-core builder is how
             a client earns a rate limit.
Prove:       `./experiments/190-parallel-layers.sh` exits 0 and records the wall time of both shapes against a multi-layer image

**Done 2026-09-22.** `crates/podbox-image/src/pull.rs` fetches through a
bounded pool of `FETCH_WORKERS = 4` threads over the descriptor list
(layers plus config), `std::thread::scope` only, no new dependency. The
bound is fixed per the Decision fork: the constraint is the registry, not
cores. The arithmetic: 4 workers hold one staging lock each, plus
`FETCH_LOCK_HEADROOM = 3` transients (the index lock at record time, a
sweep on another thread, one spare) is 7 against the sixteen fork-shed
slots (`crates/podbox-probe/src/sys.rs:702`), pinned by a `const` assert
that refuses the build rather than a test. Each worker builds its own
`Client::with_policy(policy.clone())`, because `Client::blob` takes
`&mut self`, and stages, verifies and commits through the same store
functions the loop used: one write path, not two. Each worker buffers its
lines and the transcript prints in manifest order; a failure sets the
cancellation, stops every worker starting another blob, and removes every
staged file. Committed blobs stay: content-addressed and verified,
harmless, reused by the next pull.

`experiments/190-parallel-layers.sh` exits 0, with the run in
`experiments/results/parallel-layers.txt`. The fixture is the T-0206
shape (pinned zot v2.1.21, loopback, all outbound network blocked), the
image is 8 random 6,291,456-byte layers (`multi:layers`, 50,341,652
stored bytes), and the sequential shape is pull.rs at HEAD lane-built
beside the worktree's pool. Cold store each leg, wall time in
nanoseconds:

| shape | run 1 | run 2 |
| --- | --- | --- |
| sequential | 5,539,108,423 | 5,079,339,904 |
| pooled (4 workers) | 2,503,633,646 | 3,474,055,205 |

Means 5.31 s against 2.99 s, a factor of 1.78 on container loopback,
where the link is bandwidth-bound; a latency-bound link is where the
pool buys more, and that number is not taken here. Both shapes exit 0
and inspect to the seeded manifest digest
`sha256:7d70123008878f90c66d7ac292666addacddc122a8c4fe0fb711b4c69c32bf62`.
The failure leg poisons layer index 3: the pull exits 125 naming the
digest mismatch, zero `*.partial` files remain, the lines it did print
are in manifest order, and no record is written. The pool binary is
2,814,904 bytes against 2,786,224 sequential (+28,680, no new crate).

Unit proof: `cargo test -p podbox-image` 117 passed, 0 failed, and
`cargo clippy -p podbox-image --all-targets -- -D warnings` clean. Each
guard was seen red first in the lane: the bound as a build refusal
(`error[E0080]`, 14 workers planted), the order test carrying job 5's
lines at slot 0 on a rotated merge, the cancel test listing the 4
surviving `*.partial` files with the cleanup pointed elsewhere.
Conditions: lane-built binaries in `rust:1.98.1-bookworm` through
host podman 6.1.2, network none, zot as pinned above.

---

### T-0208 `--platform`, and a store that can hold two variants of one tag

Source:      `TOOL.md` section 6.2; `docs/conventions/forbidden-patterns.md`, the correctness table
Category:    image
Priority:    P2
Effort:      L
Status:      done 2026-09-22

Problem:     podbox resolves an index to `linux/amd64` and records the platform
             it stored, but nothing can ask for another one, and the store keys
             an image by repository and tag with no variant in the key. Pulling
             `linux/arm64` would therefore either be refused or would overwrite
             the amd64 record under the same name.
Premise:     ⭐ **Read at file and line, and the corpus names the failure.**
             `crates/podbox-image/src/oci.rs` hard-codes `OS` and `ARCH`, and
             `crates/podbox-image/src/store.rs`'s `put_record` retains on
             `repository` and `tag` alone. `docs/conventions/forbidden-patterns.md`
             carries the incident: `podman run --platform linux/riscv64 alpine`
             retags the shared local `alpine:latest` to the riscv64 image, so
             the next plain `podman run alpine` fails with `Exec format error`
             and reads as an unrelated breakage.
             ⚠ podbox records `platform` on every record today, so the
             information needed to key by it is already stored and unused.
Approach:    `--platform <os>/<arch>[/<variant>]` on `pull`, defaulting to the
             host's. The store key becomes repository, tag **and** platform.
             `podbox images` gains the variant in its table only where more than
             one is held, so the common output does not change.
             ⛔ A reference that resolves to two stored records and no
             `--platform` is a named refusal listing both, never a pick.
Decision:    Key by platform rather than refusing a second variant. The refusal
             is smaller and is the wrong shape: podbox's audience is automated,
             and a multi-architecture builder is exactly the caller that needs
             two variants at once.
             ⚠ Rejected: defaulting to `linux/amd64` on every host. It is what
             the code does now and it is wrong the moment podbox runs on arm64,
             which is most of the machines its audience rents.
Prove:       `podbox pull --platform linux/arm64 public.ecr.aws/docker/library/alpine:3.20 && podbox images --format '{{.Platform}} {{.Digest}}' public.ecr.aws/docker/library/alpine:3.20 | sort | uniq -c | grep -qx ' *1 linux/amd64 .*' `

**Done 2026-09-22.** The work this entry specifies shipped under
[T-0212](#t-0212-the-platform-is-decided-at-run-time-and-the-store-holds-more-than-one)
and this close verifies it rather than re-implementing it. Every
`Premise` claim is stale: `oci.rs` delegates to the `platform` module
instead of hard-coding OS and ARCH, `put_record` retains on repository,
tag **and** platform (`store.rs`), `find_for` resolves the ambiguity by
name and returns the hits unfiltered where no platform matches, and
`pull`, `images` and `extract` all take `--platform` defaulting through
flag, environment, then the build host. Driven green on host podman
6.1.2 with a lane-built musl binary (`podbox 0.1.0`):
`experiments/270-multiarch-image.sh` exits 0 on five clauses: no flag
pulls the build platform, `--platform` takes the arm64 manifest out of
the same index (2 records, 2 image IDs for one tag), the ELF machine
words read `x86-64` and `ARM aarch64` out of the extracted trees, the
unmatched ask names what the index offers (exit 125), and a malformed
platform is a cli error (exit 1), with the run in
`experiments/results/multiarch-image.txt`. One `Approach` sentence
landed differently and is named: the default `images` table carries no
Platform column; the per-variant display is `--format '{{.Platform}}'`
and `inspect`'s `.Platform`/`.Architecture`/`.Os`, while the common
output is unchanged, which was that sentence's own goal.

---

### T-0209 Registry authentication, without a credential ever entering this tree

Source:      `TOOL.md` section 6.2, section 11.1
Category:    image
Priority:    P2
Effort:      L
Status:      done

Problem:     Every request podbox makes is anonymous. A private registry answers
             401 and podbox has nothing to answer with, so the whole class of
             image an agent sandbox actually runs is unreachable.
Premise:     Read. `crates/podbox-image/src/registry.rs` answers a `Bearer`
             challenge with no credential, which is what the anonymous flow
             needs and is all it does. ⚠ The challenge parser, the realm
             handling and the per-scope token cache are already there and are
             the parts this entry does not have to write.
Approach:    Read `~/.docker/config.json` and `$XDG_RUNTIME_DIR/containers/auth.json`,
             both of which the audience's machines already have, and send Basic
             to the token realm named by the challenge. `podbox login` writes
             the same file docker writes.
             ⛔ **No credential is logged, put in an error, or written into a
             result file**, and `registry.rs`'s `redact` already covers URLs.
             `docs/security/secrets.md` binds.
             ⛔ A credential helper (`credsStore`) is executed only when the
             config names one, never guessed, and a helper that fails is a named
             refusal rather than a silent fall back to anonymous. Falling back
             turns a permission problem into a 404 about a repository that
             exists.
Decision:    Read docker's and podman's files rather than inventing a third.
             podbox answers to both names, and a runtime that needs its own
             credential file has not replaced either.
             ⚠ The genuine fork, with a recommendation: whether `podbox login`
             writes a credential at all, given it lands in a file. Recommend
             **yes, and only through the credential helper where one is
             configured**, because refusing to write is not the same as the
             credential not existing: it just moves it to a shell history.
Prove:       `./experiments/200-registry-auth.sh` exits 0 against the fixture of T-0206 with a required credential

**Done 2026-09-22.** `crates/podbox-image/src/credentials.rs` (one read
path, one write path), the Basic answer in `registry.rs`, and `podbox
login` with its three parity rows. `experiments/200-registry-auth.sh`
exits 0 twice in a row on host podman 6.1.2 against the T-0206 fixture
in required-credential mode, all 15 driver clauses green with all
outbound network blocked (`experiments/results/registry-auth.txt`): the loopback
isolation pair, the required config verifying, the anonymous 401 with
its Basic challenge, login storing owner-only with the user reading
back, authed pulls by tag and by digest to the seeded digest, a
schemeful server naming the same host, a wrong password failing naming
401, an anonymous pull failing naming the missing login, a bogus helper
failing naming the helper, and the two usage shapes at 125 and 1. The
test password is absent from the report in both runs.

Unit proof: 17 credentials tests and 10 login parser tests green inside
a full `dev.sh check` at rc=0. The login tests were seen red first: a
plant job inverted the `--password-stdin` requirement and 4 of 10
failed, then green again on the restored tree. `base64` 0.22.1 adds
zero crates: it is already in `Cargo.lock` as `ureq`'s dependency at
exactly this version, and the lockfile is untouched by this change.

Five findings, each with what settles it:

1. The change first shipped with no login flag rows, so `admit_all`
   refused login's own flags before parsing. Three rows now name
   `-u, --username`, `--password-stdin` and `-h, --help`, the parser
   takes the `=` forms `pull` already takes, and an empty username is
   a flag refusal: the read path skips entries with no user, so
   storing one would write a login that never reads back.
2. The Windows-backed `/w` bind reports every mode as 777 whatever
   `chmod` says (measured: `chmod 600` exits 0, `stat` reads 777;
   tmpfs reads 600). The driver's homes live on tmpfs, so the 600
   proof reads a filesystem with a mode bit.
3. `experiments/lib/engine.sh` `_in_roots` matches drive spellings
   case-sensitively: `C:/...` against a `c:/...` root refuses as
   outside. Not fixed here; the engine conversions belong to
   [T-1212](gate.md).
4. `experiments/325-parity-drive.sh` re-driven green at 164 rows, 202
   driven, 0 mismatches, 2 unreachable here. That retires a
   one-row staleness: [T-1004](packaging.md) added `--verbose`
   without re-driving, so the committed 160-row reading was already
   stale. The 2 unreachable are `run -i` and `exec -i`: the banner
   names no `-i` in this container (rc=0, with and without a pty),
   while the committed lane reading says it does. That disagreement
   is filed under [T-0808](cli.md), not fixed here.
5. The `Decision` fork lands as recommended: `login` writes, and
   where the config names a helper for the host it writes and reads
   only through the helper, with nothing stored beside a failed one.

Residual, each with what would reopen it: a TTY stdin blocks in
`read_to_string` (no guard; reopen with an `IsTerminal` refusal and
its test); `image login` refuses where docker has no such path (the
exit code against docker's own is unmeasured); there is no `logout`
verb; `login` performs no network verification, so wrong credentials
surface as a named 401 at pull (decided: an offline machine cannot
verify, and a check that needs the network is a pull).

⚠ Lane substitution, stated once beside every number above. The
`wsl-toolkit-podbox` base is unusable: `getpwnam(root)` and
`getpwnam(toolkit)` both fail, and `base ensure --probe` changed
nothing. Every Linux step above ran instead in
`docker.io/library/rust:1.98.1-bookworm` through host podman 6.1.2,
the same image the lane uses, with the tree copied and modes
restored from the index. What this does not establish: the
project's own lane procedure. A re-drive from a repaired base
would clear it; `base recreate` touches shared infrastructure and
is not taken unasked.

---

### T-0210 The store's concurrency contract, written down and driven

Source:      Found while reviewing M1 adversarially; `docs/conventions/code.md`, "assume the worst case per feature"
Category:    image
Priority:    P1
Effort:      L
Status:      done 2026-09-09

Problem:     The store takes one exclusive lock around each index
             read-modify-write and nothing else. What happens when two podboxes
             pull different images at once, or one prunes while another pulls,
             is not written down anywhere, so every future change to the store
             is a change to an unstated contract.
Premise:     ⭐ **Partly measured, and the measurement is the reason this is P1
             rather than P3.** Two concurrent `podbox pull` runs of the same
             reference into one store were driven on 2026-09-08: both exited 0,
             the index parsed, one image and four blobs resulted, and no staging
             file was left. ⚠ That is one ordering of one case. The cases NOT
             driven are the interesting ones: a `prune` between another
             process's space precheck and its first blob write, and two pulls of
             DIFFERENT images racing on the same index.
             ⚠ A second gap is read rather than measured:
             `crates/podbox-image/src/store.rs` names a staging file after the
             process id, so a process killed with SIGKILL mid-blob leaves a
             `*.partial` file that nothing ever removes.
Approach:    Write the contract into the module header as invariants, then drive
             each one: what a reader may assume while a writer runs, what a
             `prune` may delete while a `pull` is in flight, and what a crashed
             process may leave. Then close the gaps the driving finds. A sweep
             of orphaned staging files on store open is one of them, and it is
             the "sweep that heals drift the happy path let slip" that
             `docs/conventions/code.md` asks for.
             ⛔ The sweep removes only files this store's own staging directory
             holds, resolved through `crates/podbox-image/src/contain.rs`.
Decision:    A stress experiment rather than a unit test. The failure is a race,
             and a race that only a mock can produce is a race the mock's author
             imagined. ⚠ The suite keeps its deterministic tests for the pieces;
             the contract is proved against real concurrent processes.
Prove:       `./experiments/210-store-concurrency.sh` exits 0 with 8 concurrent workers over 3 references, and the store verifies afterwards

**Done, 2026-09-09.** The contract is seven invariants in
`crates/podbox-image/src/store.rs`'s module header, ⛔ **in the code rather than
here**, because a contract nobody reads while changing the store is not one.
`experiments/210-store-concurrency.sh` drives it in four clauses, against real
concurrent processes rather than a mock.

| clause | what it drives | reading |
| --- | --- | --- |
| 1 | I1, I2: eight writers, three DIFFERENT references, one index | 8 exits of 0, 3 distinct records, 12 blobs, 0 bad, 0 missing, 0 partials |
| 2 | I4: `prune -af` against a live `run` | the payload exits 0, prune names what it skipped, nothing it needed was deleted |
| 3 | I5: a `SIGKILL` mid-pull, then any later command | 1 staging file left, 0 after the next command |
| 4 | I5 from the other side: a sweep against a pull in flight | the pull exits 0 and its blobs verify |

⛔ **THE DEFECT THE CONTRACT FOUND, and it was found by writing I4 down rather
than by running anything:** `rmi` and `prune` asked `Store::in_use` **outside**
the index lock and `delete` took the lock afterwards. A `run` taking its
`Store::hold` in between kept its image lock and lost its blobs, and had the
lock FILE unlinked underneath it, so the next holder would have created a fresh
lock that excluded nobody. The check now happens inside `delete`, under the
lock, and `Store::hold` takes the same lock while it acquires the image lock.
`Held::Refuse` and `Held::Skip` are what let one checked place serve `rmi`'s
refusal and `prune`'s named skip.

⭐ **The orphaned staging file is swept, and what decides "orphaned" is a lock
rather than a pid.** A writer holds an exclusive `flock` on its own staging file
for as long as it is writing, so a file `Store::open` can lock is a file nobody
is writing. ⛔ Never a pid: a pid is reused, and the check that clears a stale
pid file is the race this whole mechanism replaces. Clause 4 is the half that
matters, because a sweep that removes what it can SEE breaks every pull in
flight.

⛔ **A staging name is now unique per CALL.** It carried only the pid, which is
[T-0113](probe.md) one crate over: two threads of one process take one name and
overwrite each other. `cargo test` runs tests in threads and
[T-0207](image.md)'s bounded concurrency would make it reachable in production.
The index's own temporary file had the same shape and the same fix.

⚠ **A trap this experiment paid for, recorded in it:** `timeout 900 podbox pull &`
makes `$!` the pid of **`timeout`**, so `kill -9 "$!"` kills the wrapper and
leaves podbox running. Clause 3 then read the sweep correctly refusing to take a
live writer's file as a defect, and reported a FAIL that was the harness's.

---

### T-0211 An image lock outlives its holder whenever anything forks

Source:      Found by `cargo test --workspace` failing intermittently at the close of M2, then reproduced deliberately
Category:    image
Priority:    P1
Effort:      S
Status:      done 2026-09-12

Problem:     `Store::hold` opens the image lock **without** `O_CLOEXEC`, which
             is [T-0204](image.md)'s mechanism and is right: the guard has to
             survive the exec so a concurrent GC cannot delete a rootfs a
             running payload is using. ⛔ The consequence nothing accounts for
             is that the fd is inherited by **every** child forked while the
             lock is held, not only by the payload it was opened for. Any such
             child keeps the `flock` alive for its whole lifetime, so
             `Store::in_use` reports an image as held after its holder has
             released it, and `rmi` and `prune` refuse an image nothing is
             using.
Premise:     ⭐ **Measured on 2026-09-08, twice: once by accident and once on
             purpose.**

             It first appeared as an intermittent failure of
             `store::tests::an_image_a_container_holds_is_refused_by_rmi_and_skipped_by_prune`
             at `crates/podbox-image/src/store.rs:792`, the assertion
             `!s.in_use(&r).unwrap()` immediately after `drop(held)`. It did not
             reproduce in eleven consecutive runs of that test alone or of the
             whole `podbox-image` suite, and it reproduces in the **full
             workspace run at 2 of 6**. That difference is the diagnosis rather
             than noise: running a suite in isolation removes the other test
             threads, and the other test threads are what fork. They are named:
             `probe_cache`'s tests call `resolve`, which calls `measure`, which
             runs the probe, and the probe is one freshly forked child per
             probe.
             ⚠ **M1's own tip, `106aa6d`, measured 8 of 8 green**, in a worktree
             at that commit. Both the defect and the fork that triggers it are
             in M1's code and M2 touched neither, so M2 shifted the timing of a
             race it did not create. ⛔ Why the probability moved is recorded as
             **not diagnosed** rather than guessed at: it is not needed to fix
             the defect, and the fix is not a timing change.

             ⛔ **"Flake" is not a root cause**, so it was reproduced
             deliberately: hold the lock, `clone_fork` a child that sleeps,
             drop the lock, and ask.

             | when | `in_use` |
             | --- | --- |
             | after `drop(held)`, forked child alive | **true** |
             | after that child exits | **false** |

             ⚠ The forking thread in the test binary is the probe: M0's design
             is one freshly forked child per probe, and `cargo test` runs tests
             in parallel threads of one process.
             ⛔ **It is not confined to tests.** `probe_cache::measure` forks
             the same way, so any future call that measures the probe while an
             image lock is held leaks that lock into 50 short-lived children.
             Today's `pull` takes the probe answer before it holds anything,
             which is why this has never been seen outside the suite.
Approach:    ⛔ **CORRECTED ON 2026-09-09 BY BUILDING IT: the approach written
             here first could not have worked, and the test proved it.** It said
             to open the lock `O_CLOEXEC` and clear `FD_CLOEXEC` before the one
             exec that wants it. `O_CLOEXEC` is close-on-**exec**, and there is
             no close-on-**fork**: a `fork` duplicates every descriptor
             unconditionally, and `flock(2)` is held on the open file
             description the duplicates share, so it lives until the last of
             them closes. The child that reproduces this defect, `clone_accepted`
             in `crates/podbox-probe/src/child.rs:119`, one per `clone` probe,
             **never execs at all**, so no `FD_CLOEXEC` setting can reach it.
             The first build of this entry was red with the entry's own fix
             applied.

             ⭐ **Two defences, because they cover two different failures.**
             1. **The fork.** A fixed-size registry of fds that a fork must
                shed, in `crates/podbox-probe/src/sys.rs`, drained inside
                `clone_fork` itself before it returns into the child.
                `Store::hold` registers its fd and `Lock::drop` deregisters it.
                ⚠ Inside `clone_fork` and not in each caller's `Ok(0)` arm, so a
                caller who knows nothing about image locks still passes through
                it. Exact rather than a blanket close of everything above
                stderr: podbox knows which descriptors these are, and a range
                close would also shut fds a caller handed podbox deliberately.
                ⚠ Fixed-size and atomic because the child drains it between
                `clone` and `execve`, where allocation is not permitted; a
                seventeenth lock is refused by name rather than held unshed.
             2. **The exec.** `O_CLOEXEC` on the lock fd. `std::process::Command`
                forks inside libstd and never passes through `clone_fork`, so
                the shed list cannot reach it and the flag is the only thing
                that does.
             `Lock::hand_to_payload` undoes both for the one descriptor
             [enter.md](enter.md)'s M3 exec hands to the container, immediately
             before the fork that leads to that exec.
             ⛔ The test that found this is the plant. Two tests, one per
             defence, and each mutation turns exactly **one** of them red, which
             is how it was confirmed they are independent rather than one
             mechanism written twice.
Decision:    Fix the inheritance, not the test. Marking the test `#[serial]` or
             giving it its own process would hide a defect that is real outside
             the suite; the suite found something and the finding is the point.
             ⚠ `fcntl(2)` was not in `crates/podbox-probe/src/sys.rs` and is two
             lines there.
             ⛔ **Neither test may wait on a duration.** Both were written with a
             sleep first and both were then intermittent for a *third* reason:
             the child sheds after `clone` returns to it, and a parent that
             asserts before the child is scheduled reads the fd as still open.
             That is the same shape of failure this entry exists for, so each
             test waits on a **fact**, a byte through a pipe from the forked
             child, a line of stdout from the spawned one, and not on a delay.
Prove:       `cargo test -p podbox-image a_fork_while_the_lock_is_held_does_not_extend_it` and `cargo test -p podbox-image a_spawned_process_does_not_inherit_the_lock` both pass; the first fails with the `sys::close_in_children` registration removed from `Store::hold` and the second with `O_CLOEXEC` removed from `Lock::open`, and neither mutation fails both

**Done, 2026-09-12.** `./experiments/157-lock-inheritance-prove.sh` with
`PODBOX_PROVE_RUNS=30`, recorded in
`experiments/results/lock-inheritance-prove.txt`. ⛔ **A pass count out of
stated attempts, because one green run is what this entry was reopened for.**

| what was run | result |
| --- | --- |
| `a_fork_while_the_lock_is_held_does_not_extend_it`, alone, 30 attempts | **30 of 30** |
| `a_spawned_process_does_not_inherit_the_lock`, alone, 30 attempts | **30 of 30** |
| the fork defence removed from `Store::hold` | the fork test exits 101, the exec test exits 0 |
| `O_CLOEXEC` removed from `Lock::open` | the exec test exits 101, the fork test exits 0 |

⭐ **Each mutation reddens exactly one test, which is what makes the two
defences independent rather than one mechanism written twice.** The script
asserts the mutation landed before it reads either test, so a pattern that
matched nothing cannot read as a defence that held, and it restores the file
from a copy rather than with `git checkout --`.

⚠ **Each test is run ALONE, in its own process, which is the shape the `Prove`
names.** [T-0215](image.md) measured that several threads in one process are a
necessary condition for the suite's intermittency, so running these two tests
one per process removes that condition by construction. ⛔ **This entry is
therefore closed and the suite is still red.** T-0215 owns that, and a green
`cargo test --workspace` is not evidence either entry rests on.

### T-0212 The platform is decided at run time, and the store holds more than one

Source:      Found by the operator on 2026-09-09, reading `crates/podbox-image/src/oci.rs`
Category:    image
Priority:    P0
Effort:      M
Status:      done 2026-09-09

Problem:     ⛔ **`oci::ARCH` was the constant `"amd64"`.** Every pull asked for
             `linux/amd64` whatever machine podbox was running on, so a podbox
             built for aarch64, once [T-0911](deps.md) made that possible, would
             have fetched an amd64 rootfs and handed the payload an
             `Exec format error` with nothing to say about why.
             ⛔ **And the store's key was `(repository, tag)`**, so a second
             platform of one tag **deleted the first**: the record went, its
             blobs were collected by the next `prune`, and a caller who pulled
             both had one.
Premise:     ⭐ **Measured on 2026-09-09**, `experiments/results/multiarch-image.txt`,
             against `ghcr.io/pkgforge-dev/archlinux:latest`, which publishes one
             tag across eight platforms.

             | | |
             | --- | --- |
             | records for one tag after pulling two platforms, before | 1 |
             | after | **2**, with two distinct image IDs |
             | `usr/bin/bash` in the `linux/amd64` tree | `x86-64` |
             | the same path in the `linux/arm64` tree | `ARM aarch64` |

             ⚠ The **index digest is the same for both** and that is correct:
             it is what the tag resolved to and is the value
             [T-0202](#t-0202-a-content-addressed-store-and-digest-parity-with-docker)
             is accepted on. What differs is the image ID, which is the config
             digest under the platform-specific manifest.
Approach:    `crates/podbox-image/src/platform.rs`, and the platform travels as
             a value from the flag to the record.
             1. **`--platform os/arch[/variant]`**, then
                `$PODBOX_DEFAULT_PLATFORM`, then `$DOCKER_DEFAULT_PLATFORM`,
                then the platform this binary was built for. ⚠ docker's variable
                is honoured deliberately: podbox answers to `docker` on PATH and
                takes the same flags ([T-0803](cli.md)), so a caller who set it
                for their toolchain meant it here.
             2. ⛔ **A bare word is an ARCHITECTURE**, as docker reads it:
                `--platform arm64` is `linux/arm64`. Reading it as an OS asks a
                registry for `arm64/amd64`.
             3. ⛔ **The rust name is never the OCI name.** rust says `x86_64`
                where a registry says `amd64`, `aarch64` where it says `arm64`,
                `x86` where it says `386`. Each is a 404, or worse a wrong
                manifest, if guessed.
             4. ⛔ **`armv6` and `armv7` do not collapse.** Both normalise to
                `arm` and the variant is the whole difference; losing it runs a
                v7 image on a v6 machine, which is an illegal instruction rather
                than a message. `arm64` and `arm64/v8` **are** one platform
                written two ways, so an absent variant on either side matches
                and a present-and-different one does not.
             5. **`select_platform` runs two passes**, exact variant first. An
                exact match must beat a loose one appearing **earlier** in the
                index, which is the case a single pass gets wrong.
             6. **The platform joins the store's key**, so two architectures of
                one tag coexist as podman's do. `Store::find_for` resolves the
                ambiguity by name, preferring the host's; ⚠ where the store
                holds the image for **no** matching platform it returns the hits
                unfiltered, so the caller can say "held, for another platform"
                rather than "no such image".
Decision:    Keep both platforms rather than replacing, which is podman's
             behaviour and containerd-era docker's, not the classic daemon's.
             ⚠ The classic behaviour is defensible and was rejected for one
             reason: replacing is **silent**, and the thing it silently discards
             took a download. A caller who wanted one can `rmi` the other.
             ⚠ **Pulling a platform this machine cannot execute is allowed and
             is not even a warning at `pull`.** It is what a caller building for
             another machine wants. The refusal belongs at `run`, and only where
             nothing can execute it, which is [T-0506](enter.md).
Prove:       `./experiments/270-multiarch-image.sh` exits 0

**Done 2026-09-09.** Five clauses, all green. Clause 3 is the one that makes the
other four worth anything: a record can *say* `linux/arm64` and hold amd64
bytes, so it reads the ELF machine word out of a binary **inside the extracted
tree** rather than trusting the metadata that was just written.

⚠ Clause 4's refusal names what the index does offer, because a bare 404 leaves
a caller unable to tell a typo from an image that was never built for them. It
exits **125**, a runtime failure, while clause 5's malformed `--platform` exits
**2**, invalid input, which is [T-0110](probe.md)'s contract holding across a
new flag.

⛔ **CORRECTED 2026-09-22, AND THE CLAUSE-5 SENTENCE ABOVE WAS WRONG TWICE.**
[T-0802](cli.md) measured docker's discriminator after this entry closed:
125 is what a flag parser refuses, 1 is what the verb refuses afterwards.
podbox's `pull` parser takes any string as `--platform`'s value and the verb
refuses `a/b/c/d`, which is the `images --format '{{.Nope}}'` row, so clause
5 asserts the cli-error code 1. `experiments/270-multiarch-image.sh` carried
the 125 expectation from before that measurement and went red on it from the
2026-09-21 engine conversion; the run in
`experiments/results/multiarch-image.txt` is green again. If a future `330`
run with a daemon measures docker refusing a malformed `--platform` in its
own parser, this reopens as a code change.

⭐ **The registry is `ghcr.io` and that is part of the entry, not an accident.**
[T-0206](#t-0206-a-registry-fixture-so-the-acceptance-stops-depending-on-somebody-elses-quota)
is open because Docker Hub's anonymous quota can turn this project's acceptance
red; ghcr has none, and `ghcr.io/pkgforge-dev/archlinux` carries the eight
platforms this question needs.

---

⛔ **A SECOND DOOR, FOUND BY THE SESSION'S OWN DOOR SWEEP ON 2026-09-09 AND
FIXED.** `Store::find_for` was given the platform and `Store::find_one` was not,
and `extract` and `inspect` reach the store through the second. `find_one` took
`.next()`, which was harmless only while a store could not hold two records for
one tag; **this entry is what made it hold one per platform**, so the same line
then meant `podbox extract alpine` silently unpacked whichever platform was
pulled most recently. Selected **by position**, which
`docs/conventions/code.md` forbids and which `Store::find`'s own comment calls
out three functions above.

⭐ **`find_one_for` prefers the host's platform and refuses what survives that.**
Preferring rather than demanding, because a store holding exactly one foreign
platform and asked for nothing in particular is not ambiguous; ambiguity is two
or more surviving the preference, and then podbox names them and stops.
`extract` grew `--platform`; `inspect` did not, because docker's has none either
and the refusal is now the honest answer there.

| | |
| --- | --- |
| `podbox extract <img>` on a two-platform store | the host's, `linux/amd64` |
| `podbox extract --platform linux/arm64 <img>` | the other one |
| `podbox extract --platform linux/riscv64 <img>` | exit 125, `The store holds it for linux/amd64, linux/arm64` |

⚠ **The door sweep is the only lens that finds this.** Every test of `find_for`
passed throughout, because `find_for` was never the door that was open.


### T-0213 A registry with no certificate, or one nothing trusts, and the refusal kept

Source:      Asked for by the operator on 2026-09-09
Category:    image
Priority:    P0
Effort:      M
Status:      done 2026-09-09

Problem:     ⛔ **podbox could not reach a local registry at all.** A registry on
             loopback speaks plain HTTP, or HTTPS with a self-signed
             certificate, and [T-0201](#t-0201-registry-client-https-only-with-no-plain-http-fallback)
             refused both with no way to say otherwise. That is most of the
             registries a developer actually runs, and podbox answering "HTTPS
             only" to `localhost:5000` is podbox being useless rather than
             careful.
Premise:     ⭐ **T-0201's finding is real and is kept.** It refused a plain-HTTP
             **fallback** because tcp/80 egress is black-holed on the runtimes
             podbox targets, so an automatic downgrade **hangs** rather than
             failing. ⚠ What that never justified is refusing a registry the
             caller has explicitly named. The distinction this entry draws is
             between a **fallback** and an **instruction**.

             ⛔ **And the rule that forbade this did not exist.**
             `crates/podbox-image/src/tls.rs` said "there is no skip
             verification path here... `docs/security/remote-ops.md` and the
             environment note in `AGENTS.md` both say never to disable
             verification". Checked on 2026-09-09: `remote-ops.md` says nothing
             about TLS at all, and `AGENTS.md`'s note is an instruction to
             **the agent** about this development container's intercepting
             proxy, not a rule about what podbox may offer. A rule was cited
             where it was not written.
Approach:    `crates/podbox-image/src/transport.rs`, a policy resolved once and
             then carried by value.
             1. **`--insecure-registry HOST`**, repeatable, docker's flag and
                docker's meaning: for that host, do not verify, and fall back to
                plain HTTP if HTTPS cannot connect.
             2. **`--tls-verify=false`**, podman's flag and podman's meaning:
                per **invocation**, not per host. ⛔ It does **not** permit plain
                HTTP: not verifying a certificate and not having one are
                different asks, and clause 5 asserts they stay different.
             3. `$PODBOX_INSECURE_REGISTRIES`, comma separated, and a config
                file at `$PODBOX_CONFIG` else
                `$XDG_CONFIG_HOME/podbox/registries.conf`, one host per line.
                ⛔ A bad line is refused with its **line number**, never skipped:
                a silently ignored entry is a registry the caller believes is
                permitted and is not.
             4. ⛔ **Every downgrade is announced on stderr**, naming the
                registry, and an ordinary pull says nothing. A disclosure
                printed on every run is a disclosure nobody reads.
             5. The `http://` refusal moves from `Reference::parse` to `pull`,
                because only there is the policy in scope and only there can the
                message **name the flag that would permit it**.
Decision:    An instruction, never a discovery. podbox does not probe a registry
             to learn that it speaks HTTP; it is told, and only then does one
             failed HTTPS connect turn into one HTTP retry.
             ⚠ **Two agents, not one with a switch.** A `ureq::Agent` carries one
             `ClientConfig`, one proxy setting and one connection pool, so
             verifying for one registry and not another through the same pool is
             how a verified connection gets reused for an unverified request.
             They are keyed on `(verify, proxy)` and built lazily.
Prove:       `./experiments/280-insecure-registry.sh` exits 0

**Done 2026-09-09.** Seven clauses, all green, against two real `registry:2`
instances: one plain HTTP, one with a certificate generated by the script.

⭐ **A defect this found that no unit test could, and it is the reason clause 3
leaves the proxy set.** The first working build pulled the loopback registry
**through the environment's intercepting proxy** and came back `HTTP 405`, which
reads as a broken registry and is a proxy refusing a non-CONNECT request. ⛔ A
loopback registry is never reached through a proxy now, whatever the environment
says, because a proxy cannot reach the caller's own loopback; `NO_PROXY` is
honoured on top of that, both spellings, matching a bare host or any parent
domain. ⚠ `ureq` 2's `try_proxy_from_env` does not consult `NO_PROXY` at all, so
this had to be decided in podbox rather than delegated.

⚠ **A bare IPv6 address has colons and no port**, so splitting an endpoint at
its last colon turns `::1` into the host `::`. The first `is_loopback` did
exactly that and its own test caught it.

⚠ **The clause that keeps clause 3 honest is clause 4**: naming one registry
insecure must change nothing for a second one, and it is asserted rather than
assumed.

---

### T-0214 A blob body cut off mid-stream is not retried, and the bounded retry is around the wrong thing

Source:      Measured by `experiments/240-distro-sweep.sh` on 2026-09-09
Category:    image
Priority:    P2
Effort:      S
Status:      done

Problem:     ⛔ **`Registry::get` retries the REQUEST and `Registry::blob`
             streams the BODY, so a transfer that dies after the first byte is a
             failed pull with no second attempt.** The M5 sweep lost a whole row
             to it: `fedora` reported `no-pull`, and the reason was one
             truncated body rather than anything about the image, the registry
             or the row's own subject.
Premise:     ⭐ **Measured, and it reproduces the same way twice: it fails, and
             then the identical command succeeds.** From
             `experiments/results/sweep/fedora.out`, 2026-09-09:

             ```
             no-pull quay.io/fedora/fedora@sha256:e78cd1a6…ee95c
             podbox: GET /v2/fedora/fedora/blobs/sha256:419b79a0…1d21:
               reading the blob body after 2097153 byte(s):
               response body closed before all bytes were read
             ```

             ⚠ 2,097,153 is 2 MiB and one byte, which is the shape of a proxy
             cutting a transfer rather than a registry ending one. The same pull,
             run again by hand with no other change, completed: `419b79a05f4b:
             Pull complete`.
             ⚠ `crates/podbox-image/src/registry.rs:236-250` is the loop that
             gives up; `crates/podbox-image/src/registry.rs:255-268` is the
             bounded retry, and it is one level too high to see this.
Approach:    Put the retry around the whole of one blob rather than around the
             request that starts it.
             1. `blob` takes a sink it can RESTART. The staging file is the
                caller's, so the contract has to say who truncates it: either the
                caller hands a factory, or `blob` is told the staging path and
                owns the file. ⭐ Recommend the second, because the verifier
                already owns the "did every byte arrive" question and a caller
                that resets a sink it did not fill is a second place to get it
                wrong;
             2. the same `ATTEMPTS` and the same `backoff`, from the same
                constants, so a blob is not retried on a schedule of its own;
             3. ⛔ **A partial body is never committed and never counted.** The
                digest is computed over the bytes that arrived, so a restarted
                attempt starts a new verifier;
             4. ⛔ **A Range request is NOT the mechanism.** Resuming at an offset
                means trusting that the prefix already written is the prefix of
                the blob podbox asked for, which is exactly what the digest exists
                to establish and cannot establish until the last byte.
Decision:    Retry the transfer, not the byte range. A truncated body is cheap to
             refetch at these sizes and a resumed one cannot be verified until it
             is whole anyway.
             ⚠ The genuine fork, with a recommendation: whether a retried blob is
             announced in the transcript. Recommend **yes, one line**, naming the
             attempt and the byte count that arrived: podbox's audience is
             automated, and a pull that quietly took three tries is a link
             degrading with nothing to show for it.
Prove:       `./experiments/240-distro-sweep.sh` reports `no_pull 0`, and `a_restarted_staging_file_holds_only_the_second_attempt` asserts the sink half


**Done 2026-09-09.** `crates/podbox-image/src/registry.rs`, and the retry moved
one level down rather than being added: `Client::blob` loops over the whole
transfer, `Client::drain` is one body, and `Registry::get`'s own bounded retry
still covers the request that starts each attempt.

⭐ **The sink is restarted rather than resumed**, through a `Restart` trait
implemented for `StagedFile`. ⛔ `set_len(0)` AND a rewind: truncating alone
leaves the offset where the cut-short attempt left it, so the retry lands two
megabytes in and the file is a hole followed by the second attempt --  which on
a sparse filesystem reads back as NUL bytes and fails the digest a long way from
the cause. `a_restarted_staging_file_holds_only_the_second_attempt` asserts it
by writing something SHORTER the second time, which is the only shape that
catches a missing rewind.

⛔ **Only a transport failure is retried.** A digest or size mismatch is a
registry serving different bytes than its descriptor declares, and asking it
again is the spiral `docs/conventions/forbidden-patterns.md` names.

⚠ **The `BufWriter` came out.** `blob` restarts the sink, and a `BufWriter` has
no way to discard what it is holding. The fetch already writes one 128 KiB chunk
at a time, so it was adding a copy rather than a saving.

⚠ **The retry is announced, one line**, naming the attempt: podbox's audience is
automated, and a pull that quietly took three tries is a link degrading with
nothing to show for it.

⛔ **THE RETRY HAS NOT BEEN OBSERVED FIRING, and that is written down rather
than implied.** What is established is the failure -- twice, at the same byte
count -- and the sink half, by
`a_restarted_staging_file_holds_only_the_second_attempt`. The sweep run after
this landed reported `no_pull 0` and an isolated cold pull of the same blob
completed with no retry line, so the truncation is intermittent and this run
does not say which of the two cleared it. ⚠ `240-distro-sweep.sh` now keeps the
pull transcript of EVERY row rather than only of a row that failed to pull,
because the retry fires inside a pull that then succeeds: a transcript kept only
on failure can never show it.

⛔ **The `Prove` was amended twice** and the second time for that reason. It
first asked for a fault injected into `150-image-acquisition.sh`, which would
assert the loop against a fault this project invented rather than the one it
met; and then for `no_pull 0`, which the sweep can reach without the retry ever
running. What it names now is the pair that is actually asserted: the acceptance
pulling every row, and the unit test for the restart.

---

### T-0215 Four lock tests fail in two runs of five, and the gate has never said so

Source:      `crates/podbox-image/src/store.rs`; `crates/podbox-probe/src/sys.rs`
Category:    image
Priority:    P0
Effort:      M
Status:      done

Problem:     ⛔ **`cargo test --workspace` is not deterministic, and the tests
             that are not deterministic are the lock tests.** Measured on
             2026-09-11 inside `docker.io/library/rust:1.98.1-bookworm`:
             **5 of 12 runs failed**, and four different tests were involved:

             ```text
             run  2  a_fork_while_the_lock_is_held_does_not_extend_it
             run  4  a_fork_while_the_lock_is_held_does_not_extend_it
             run  5  an_image_a_container_holds_is_refused_by_rmi_and_skipped_by_prune
             run  5  two_holders_of_one_image_both_have_to_go_before_it_is_free
             run  7  a_spawned_process_does_not_inherit_the_lock
             run  9  a_spawned_process_does_not_inherit_the_lock
             run  9  two_holders_of_one_image_both_have_to_go_before_it_is_free
             ```

             ⛔ **What it costs if it is a product defect and not a test one**:
             `in_use` is what `prune` asks before it deletes an image's blobs,
             and the launcher's held lock is the only thing that answers. A
             wrong `false` deletes the blobs a running container is about to
             execute out of, which is the defect T-0211 and I4 were paid for.
             ⚠ **The count in the title is two short and the set is six, not
             four.** `experiments/153-store-lock-race.sh` added two on
             2026-09-12, both about the sweep rather than about `in_use`:
             `opening_a_store_sweeps_what_a_killed_process_left` and
             `the_sweep_takes_an_abandoned_partial_and_leaves_a_held_one`. The
             title keeps its wording because it is how this entry is cited.
             ⭐ **AND EVERY FAILURE CAPTURED SO FAR IS IN THE SAFE DIRECTION.**
             Every one of them read a lock as HELD when nothing held it: `rmi`
             refuses an image nothing uses, and a sweep leaves an abandoned file
             behind. ⛔ **A wrong `false` has never been observed**, and the
             assertion that would catch one,
             `two_holders_of_one_image_both_have_to_go_before_it_is_free`'s
             middle line, has not failed once. ⚠ The clause table below carries
             the run counts; the failing runs themselves were not counted across
             all passes, so no total is quoted here. So the cost quoted above is
             still the cost if the direction ever reverses, and it is not what
             has been measured. The entry stays P0 because the gate is red.
Premise:     ⭐ **Two conditions are necessary and both are measured.** Several
             threads in ONE process: `experiments/153-store-lock-race.sh`
             clause 2 reads 0 of 12 three times and 0 of 20 twice. A concurrent
             fork: clause 6 reads 0 of 20 in each of four takings and 0 of 30 in
             each of two more. ⚠ Neither is the cause, and the fork's role is
             not the one it looks like: a fork makes a SECOND REFERENCE to the
             open file description, which is what stops the holder's own `close`
             from completing the release.
             ⛔ **The refusal has no holder at all.** Captured at the
             `EWOULDBLOCK` itself rather than at the assertion: no row in
             `/proc/locks`, no descriptor on the inode in any process on the
             host, and the SAME descriptor succeeding on the next attempt a
             microsecond later. The `Done` record below carries the captures and
             the fix that follows from them.
             ⚠ **The filesystem is not the cause**, which cost a run to find
             out. Clause 7 moved every lock from this container's `overlayfs`
             `/tmp` to a `tmpfs` and changed nothing else: 6 and 10 of 20, then
             9 and 7 of 20.
             ⛔ **FIVE MECHANISMS WERE CLOSED BEFORE THIS ONE AND NOT ONE OF
             THEM MOVED THE RATE.** Every one asked which descriptor a forked
             child holds, and the answer was that no child held anything. Two of
             the five are KEPT in the tree, on [T-0211](image.md)'s invariant
             and NOT on any measurement here: `Lock::try_acquire` registers every
             lock it builds, and `sys::shed_after_fork` drains the shed table in
             a child libstd forked. ⚠ A change kept on an invariant rather than
             on a measurement is recorded as exactly that, so a later reader
             cannot find it in the history and conclude it was the fix.
             ⭐ **The five, the figures they were closed on, the clause series
             behind them and the three wrong readings of the fork control are
             in**
             [`../docs/history/2026-09-12-store-lock-race-dead-ends.md`](../docs/history/2026-09-12-store-lock-race-dead-ends.md),
             moved there in their original words when this entry closed. They
             are a dead end with what it cost, and a reader does not need them
             to use the lock code correctly today.

Approach:    ⛔ **Establish the blast radius before fixing anything.** The first
             question is not how to fix it; it is whether a single-threaded
             podbox process can reach it at all. A race that only a test harness
             with many threads in one process can produce is a test defect and is
             fixed in the test; one that a launcher and a `prune` can produce is
             a P0 in the product, and the two fixes are different.
             1. ✅ **Done on 2026-09-12.** Reproduced under a loop, with the
                failing assertion and the descriptor state captured.
                `experiments/153-store-lock-race.sh` and
                `experiments/results/store-lock-race.txt`.
             2. ✅ **Done. Every candidate about which descriptors a CHILD holds
                was closed and none of them moved the rate.** The `Premise`
                carries the five, each with what it was worth. ⚠ It took FOUR
                readings of clause 6 to get the fork question right, three of
                them with a fork still in the run, and the entry keeps that so
                the next control is derived from the call graph rather than from
                the test names.
             3. ✅ **The gate was the second finding and it is answered.** CI ran
                `cargo test --workspace` and reported green while the suite
                failed two runs in five, because a single run is not evidence
                for a racy suite.
                `releasing_a_lock_frees_it_even_while_a_duplicate_descriptor_lives`
                is deterministic, so one run is now enough.
             4. ✅ **The holder is named, and there was never one.** Threads in
                one process are necessary, a fork is necessary, and the refusal
                itself is spurious. Four measurements were named and all four
                are taken:
                a. ✅ **Clause 7 ruled the filesystem out.** It moved every lock
                   from this container's `overlayfs` `/tmp` to a `tmpfs` through
                   `TMPDIR` and changed nothing else. The failure arrived at
                   **6 and 10 of 20**, and at **9 and 7 of 20** on a second
                   taking. ⭐ A negative result, and it costs the next session
                   the run it would have made;
                b. ✅ **Registering the other eight `Lock` sites changed
                   nothing**, and neither did shedding on libstd's fork, and
                   neither did widening the pre-registration window, and neither
                   did hooking the one spawn that was bare by design;
                c. ✅ **The instrument was moved to the moment of the refusal,
                   and that is what answered it.** Everything before it ran from
                   the assertion, microseconds too late: the shortest refusal it
                   chased had already cleared, and the committed captures show
                   the transient gone on the next attempt 1 to 4 us later;
                d. ✅ **The same descriptor, retried at once, is the reading
                   that settled it.** Nothing else changes: the same fd, the
                   same operation, microseconds later.
Decision:    ⭐ **`Lock::drop` releases the lock EXPLICITLY, with
             `flock(LOCK_UN)`, before it closes the descriptor.**
             ⛔ **Closing is not releasing while anything else holds a reference
             to the same open file description**, and a `fork` makes exactly
             that. `LOCK_UN` removes the record in the releasing thread, whatever
             the reference count is, so the release is finished when the line
             is.
             ⚠ **One lock is exempt and it is the one that is meant to be
             inherited.** [`Lock::hand_to_payload`] sets a flag that turns the
             explicit release off, because the payload holds a duplicate of that
             same description and `LOCK_UN` would take the lock from it too.
             ⛔ **The close stays and is not replaced.** It is what makes the
             lock correct across an unexpected death, where no `Drop` runs.
             ⛔ **No change to the eight registrations or to the shed hook**,
             which are kept on [T-0211](image.md)'s invariant and not on this
             measurement, and the `Premise` says so in those words.
             ⚠ The tempting fix was to serialise the lock tests, which makes the
             suite green and answers nothing. ⛔ It stayed refused, and the
             answer came from the instrument instead.
Prove:       `cargo test -p podbox-image releasing_a_lock_frees_it_even_while_a_duplicate_descriptor_lives` and `cargo test -p podbox-image a_lock_handed_to_the_payload_outlives_this_process_dropping_it` both pass, and `PODBOX_RACE_CLAUSES="0 1 6 12 13 14" ./experiments/153-store-lock-race.sh` reports the subject at 0 of 30 in both passes, clause 12 red on the first guard and on the subject, and clause 14 red on the second, and the script itself exits 0. ⛔ Clauses 12 and 14 DELETE the release and the exemption in turn and assert the red, because a fix nobody has seen fail is worth as little as a check nobody has seen fail


**Done 2026-09-12, and the mechanism is named rather than guessed.**

⛔ **`close(2)` IS NOT A RELEASE WHILE ANYTHING ELSE REFERENCES THE SAME OPEN
FILE DESCRIPTION.** `Lock::drop` released by closing its descriptor. A `fork`
makes a second reference to the description, so after one the holder's own
`close` no longer completes the release: the record is taken away when the LAST
reference goes, and where that is the child, it happens asynchronously with
respect to this process's next `flock`. `in_use` read those windows as an image
still in use after its holder released it, and a sweep left an abandoned file
behind. ⭐ That is why a concurrent fork was a necessary condition all along,
and why nothing about WHICH descriptors a child holds ever moved the rate.

⭐ **The reading that settled it, taken at the `EWOULDBLOCK` itself rather than
at the assertion.** Every instrument before it ran while the assertion message
was being formatted, by which time the refusal was over: the committed captures
show it gone on the next attempt, 1 to 4 us later. ⛔ **Clause 13 takes it**: it
deletes the release, turns the capture on, and runs the subject thirty times.
All thirty fail, and the captures carry **two signatures, which are one
mechanism in two phases**.

⭐ **While the second reference is OPEN, the lock is genuinely held** and the
kernel says so. Thirty captures, one per run:

```text
AT THE REFUSAL, on inode 132694, probe fd 28:
  kernel: 4: FLOCK  ADVISORY  READ 101032 00:29:132694 0 EOF
  the same fd was still refused after 77792 retries and 20000 us, so a holder is real
```

⚠ That is the regression test's duplicate descriptor, holding the description
open across a drop that no longer releases. The retry counts run from 54,536 to
106,929 over the instrument's full 20 ms bound.

⛔ **While it is being TORN DOWN, the lock is refused with nothing holding it
anywhere.** Eleven captures in the same thirty runs:

```text
AT THE REFUSAL, on inode 132740, probe fd 25:
  kernel: NO FLOCK ROW ON THIS INODE, yet the flock was refused
  THE SAME FD SUCCEEDED on retry 1 after 2 us, so there was no holder
  no descriptor on this inode in ANY process, the probe's own apart
```

⚠ Most clear on the first retry, 1 to 3 us later. The slowest ran 344 us and
1372 retries, which is still three orders of magnitude short of a holder.

⭐ **That second signature is the race as it arrived for three sessions**, and
it is why every earlier instrument answered "nobody": there was nobody, by the
time anything looked. The refusal is the tail of a release that the holder's own
`close` did not finish.

⭐ **The fix is one syscall and it is in the releasing thread.**
`Lock::drop` calls `flock(fd, LOCK_UN)` before the close, so the record is
removed here rather than whenever the last reference happens to go.
`Lock::hand_to_payload` sets a flag that turns it off for the one lock that is
meant to be inherited, because the payload holds a duplicate of that same
description.

⭐ **Proved from both sides, and `experiments/results/store-lock-race.txt` is
the run:**

| the reading | before | after |
| --- | --- | --- |
| clause 1, the subject | 5 to 12 of 20 across twenty passes | ⭐ **0 of 30, twice** |
| clause 12, the release deleted, the subject | - | ⛔ **30 of 30** |
| clause 12, the release deleted, the regression test | - | ⛔ **red, exit 101** |
| clause 13, the captures with the release deleted | - | 30 with a real holder, 11 with none at all |
| clause 14, the exemption made unconditional | - | ⛔ **red, the payload guard** |
| clause 6, the fork control | 0, 0 | 0, 0 of 30 |

⭐ **`releasing_a_lock_frees_it_even_while_a_duplicate_descriptor_lives` is the
guard, and it is DETERMINISTIC where the defect was one run in two.**
`F_DUPFD_CLOEXEC` gives a second reference to one open file description, which
is exactly what a child gets, so the test needs no second process, no thread and
no timing. ⛔ Clause 12 deletes the `LOCK_UN` line and asserts the test goes
red, because a fix nobody has seen fail is worth as little as a check nobody has
seen fail.

⛔ **AND THE EXEMPTION HAS ITS OWN GUARD, BECAUSE IT IS THE HALF THAT WOULD
BREAK IN SILENCE.** `a_lock_handed_to_the_payload_outlives_this_process_dropping_it`
hands a lock to a payload, holds the payload's duplicate, drops this process's
`Lock`, and asserts the image still reads as in use. ⚠ Without the exemption a
running container's image would read as free the moment the launcher's `Lock`
went, and a concurrent `prune` could delete the rootfs it is executing out of,
which is what [T-0204](image.md) and invariant I4 were paid for. ⭐ Clause 14
makes the release unconditional and asserts the guard goes red.

⚠ **AND THE EXEMPTION KEEPS THE ASYNCHRONOUS RELEASE, WHICH IS THE ONE PLACE
THAT IS CORRECT.** A handed lock is released by the LAST reference and never by
this thread, because that is what handing it means. So the window this entry is
about survives for that one lock: after a container really ends, its image can
read as in use for a few hundred microseconds. ⛔ **That is the safe direction**,
it self-corrects on the next attempt, and the dangerous direction cannot happen
because the lock is genuinely held while the payload lives.
⭐ **The guard says so rather than asserting immediacy.** A first draft of it
asserted the image was free the instant the payload's descriptor closed, and it
failed **9 and 13 of 30** for exactly the reason this entry names. It waits for
the release to ARRIVE now, which is the property the design provides.

⚠ **What this does NOT claim.** The blast radius is unchanged: the product, as
measured, could not reach this. Several threads in one process are a necessary
condition, clause 2 at 0 of 12 three times and 0 of 20 twice, and the process
that holds an image lock for a container's life is measured single-threaded in
`experiments/results/lifecycle-loop.txt`. ⭐ The defect was real and the fix is
in the product, because a `prune` in a future threaded caller would have hit it
and the failure was only ever in the safe direction by luck.

---

### T-1310 The store suite exhausts the sixteen fork-shed slots, and the victim varies

Source:      `TODO/PROGRESS.md` 2026-09-21 record (inherited figures, re-taken in task 1); `crates/podbox-image/src/store.rs:1003`; `crates/podbox-probe/src/sys.rs:702`
Category:    image
Priority:    P1
Effort:      S
Status:      done

Problem:     Full parallel runs of the `podbox-image` suite intermittently refuse
             with `this process already holds 16 locks, which is every slot
             podbox has ... (T-0211)`. The victim varies with scheduling
             (`two_holders_of_one_image...`, `two_staging_calls...`,
             `two_platforms...`). A serial run passes 97 of 97. The pool is
             process-wide and fixed at `FORK_CLOSE_SLOTS`
             (`crates/podbox-probe/src/sys.rs:702`), and every
             `Lock::try_acquire` takes one slot
             (`crates/podbox-image/src/store.rs:1003-1021`). Libtest runs the
             suite in threads of one process, and each `two_*` test holds two or
             more locks at once, so parallel neighbours exhaust the pool and the
             test that asks last is refused.
Premise:     ⭐ **The figures above are inherited, not measured here.** They come
             from the 2026-09-21 record, written in this same continuous session:
             three reds around one change, all with the 16-slot signature, then
             97 of 97 with `--test-threads=1`, then a fully green fourth gate.
             Task 1 re-takes them in this lane before any code changes, because a
             rate from another lane is a reading from that lane.
             ⭐ **What was checked here, at file and line.**
             `FORK_CLOSE_SLOTS` is 16
             (`crates/podbox-probe/src/sys.rs:702`). Every lock registers
             (`crates/podbox-image/src/store.rs:1003-1021`), including staging
             locks: a `StagedFile` holds one for its life
             (`crates/podbox-image/src/store.rs:867-871`). So do transient sweep
             and `in_use` probes. Every store test reaches `Store::open` through
             `scratch` or directly. A production process holds at most three at
             once by inspection: one image lock plus transient index or staging
             locks. The spawn path is asserted single-threaded
             (`nothing_on_the_spawn_path_can_spawn_a_thread`,
             `crates/podbox-supervise/src/launcher.rs:565`). `pull` resolves the
             probe before it stages anything
             (`crates/podbox-image/src/pull.rs:93`, and [T-0211](image.md)'s
             premise). So no production shape on record reaches sixteen. Only
             the harness does, and its thread count scales with the machine's
             cores.
             ⛔ **Two predecessors refused the test-only fix, and this entry is
             distinguished from both.** [T-0211](image.md) refused `#[serial]`
             because the fork leak was reachable in production. [T-0215](image.md)
             refused serialising the lock tests because the late release was a
             product defect. Here the exhausted pool is unreachable in production
             by the inspection above, so the suite is the only holder that
             exceeds the design, and the fix belongs in the suite. If task 1
             refutes the inspection, the decision flips to candidate 2, and this
             paragraph carries the flip.
Approach:    Three candidates, each with what refutes it. A fourth pass follows
             for what the three miss.
             1. Serialise every store test under one suite mutex (test-only).
                Each test takes it. `FORKING_TESTS` goes away, subsumed: fork
                tests must also exclude store tests, because a forked child
                inherits another thread's lock, which is T-0211's shape. A
                deterministic ceiling test fills all sixteen slots under the
                mutex and asserts the seventeenth is refused by name. That pins
                the ceiling and proves the mutex excludes. Refuted if any
                production path holds sixteen locks. Inspection says none does.
                T-0207's future pool is the risk, and the contract below bounds
                it.
             2. A growable slot table in the product, with no ceiling. The parent
                allocates and never frees. The child only reads atomics and
                closes, so the fork path stays signal-safe. Plausible mechanism,
                **not recommended**. A bug in it leaks locks silently, which is
                the exact class this family fights, and it answers T-0207, a
                consumer that does not exist yet.
             3. A larger fixed number. **Refuted.** The libtest thread count
                scales with machine cores, so demand scales with the machine, and
                any fixed number is the same wall moved. The ceiling row
                (`docs/conventions/forbidden-patterns.md:65`) names it.
             4. Second pass. Transient sweep and `in_use` locks also consume
                slots, so the mutex covers every test that reaches the store, not
                only the holders. The probe crate's test binary owns a separate
                table (one process per test binary), so it stays out. The
                gate-rate question, one run is not evidence for a racy suite,
                stays with [T-1204](gate.md)'s neighbourhood. This entry's Prove
                uses repeats, which is evidence for that question without
                settling it.
Decision:    Implement candidate 1. One `STORE_TESTS` mutex in the `store.rs`
             test module. Every test that reaches the store takes it, with the
             `into_inner` poison precedent `FORKING_TESTS` already sets. The
             ceiling test pins sixteen plus refusal. One contract line goes into
             [T-0207](image.md) in the same implementation change: its pool bound
             plus transient locks stays under sixteen, or that entry grows the
             table with its own measurement. No gate-check change, so no plant is
             owed. Recommend **P1, S**: the product is unaffected, and the work
             is one mutex, one test, one script, and repeated runs.
             In scope: a deterministic parallel suite; the ceiling pinned by
             test; the T-0207 contract line.
             Out of scope: gate rate reporting; any change to the sixteen-slot
             mechanism; T-0207's bound value.
             Tasks: (1) re-measure in this lane: parallel runs to first refusal
             or N, serial control twice, all with conditions; (2) the mutex
             conversion plus the ceiling test; (3)
             `experiments/326-store-contention-prove.sh` (0 ran and matched, 1
             ran and did not, 2 could not run) with committed results; (4) three
             review passes, the gate, and the record.
             Pitfalls: a test that forgets the mutex reintroduces the flake
             silently, so the implementation review greps every test body for the
             acquisition line. One mutex only, never nested, so no deadlock
             shape. Poison uses `into_inner`, never a panic.
             Acceptance gate: `docs/methodology/gate.md`, all three parts. Part
             (a) runs in the Linux lane (`sh scripts/windows/run-in-base.sh`).
             Part (b) is the 326 script green there. Part (c) asks the true,
             consistent, and cold-usable questions over the entry, the test diff,
             and the script.
             No item here needs the operator. No credential, no remote system, no
             ruling: the ruling this family needed arrived with the
             authorisation, and the open gate-rate question is recorded where it
             belongs rather than decided here.
Prove:       `./experiments/326-store-contention-prove.sh` exits 0

**Done 2026-09-21.** One `STORE_TESTS` mutex in the `store.rs` test module,
taken once by every test in it (24 of 24 by audit). `FORKING_TESTS` is gone,
subsumed. The ceiling test fills all sixteen slots under the mutex and asserts
the seventeenth is refused. One contract line into T-0207 bounds its future
pool under sixteen.

| the reading | before | after |
| --- | --- | --- |
| parallel suite, default threads, 10 runs, nproc 20 | 8 refused with the 16-slot signature, victims across eight tests | 10 green, 0 with the signature |
| serial control | 2 green | green |
| ceiling test alone | - (new) | 3 green |
| mutex audit (test functions vs acquisitions) | - (new) | 24 vs 24, equal |
| `dev.sh check` (fmt, clippy, build, workspace tests, gate) | green on main (CI success at `a18cdda`) | green |

Task 1 ran on the unmodified tree at `f9aa0bb`
(`experiments/results/store-contention-prefix.txt`). The after column ran in
the same lane on the working tree of this change
(`experiments/results/store-contention-prove.txt`,
`experiments/326-store-contention-prove.sh`). No flip: task 1 confirmed the
production-hold inspection, so candidate 1 stands as decided.
