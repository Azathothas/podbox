# incus without root

This page answers the study of T-1717 ([the machine tasks](../TODO/machine.md)):
how podvm can use incus without root and without a large loss of speed, and
the form of the incus back end (T-1718). The source of each measured row is
[experiment 433](../experiments/433-incus-rootless.sh) and
[its run](../experiments/results/incus-rootless.txt) on the cloud host of
2026-11-11, or errand-box at `1ad2fc1` (reference issue 251), as each row
says.

"Without root" has two readings (`errand-box:docs/research/rootless.md`):

* **A, no root at run time.** Root prepares the host one time. After that
  no root daemon runs, and no setuid helper runs other than `newuidmap` and
  `newgidmap`.
* **B, no root ever.** A user with no root installs and runs everything.

## The routes

| Route | Preconditions | Cost | Source |
| --- | --- | --- | --- |
| A: incusd as root of the user's own user namespace | root, one time: the package `incus` (6.0.0 here) and `uidmap`; one subordinate id range of the user that is large enough (100000000 ids here); cgroup subtrees that the user owns (on cgroup v1 with no init, again at each boot); a state directory that the user owns. No `/dev/kvm` for system containers. | busybox work 0.6 to 0.9 times podvm's userland rung; a new instance 0.36 to 0.49 s; `exec` 45 ms; start of a stopped instance 0.1 s | experiment 433 |
| A, the same route on errand-box's host | the same, with btrfs | 0.6 to 1.1 times runc on spawn, files, git, cpu and compile; a new instance 0.9 to 3.5 s; no disk quota | `errand-box:docs/research/rootless.md` |
| A, incus virtual machines | `/dev/kvm` for the user | not measured: the cloud host has no `/dev/kvm`, and incusd says "KVM support is missing" | experiment 433, clause 1 log |
| B: podvm's own microVM (`run --podvm-tier=machine`) | QEMU on `PATH`; `/dev/kvm` makes it fast | about 0.6 s for each run under KVM, about 2.9 s under TCG | T-1656, experiment 412 |
| B: UML or QEMU TCG as a full system | none | 18 to 53 times runc on compile work | `errand-box:docs/research/rootless.md` |

Route A gives podvm incus with no root at run time and at the speed of a
shared kernel. Route B is what a host with no root at all gives; podvm has
it as its own machine tier, and incus does not add to it.

## What experiment 433 found

* incusd 6.0.0 runs as root of the user's namespace: uid 0 of the namespace
  is the user (uid 30033), and the ids 1 to 100000000 are the user's range
  from 2000000000. The `lxc` driver is up. The `qemu` driver is not
  (no `/dev/kvm`).
* An alpine 3.21 system container runs `/sbin/init` as PID 1, as an id of
  the user's range on the host (2000999999), never as host root.
* incusd sits in an empty network namespace: it cannot fetch an image, and
  an instance has no network. The client uploads images over the socket.
  errand-box relays the network through a unix socket (its E-23).
* An OCI image that podvm pulled becomes an incus image: a unified tarball
  of podvm's extracted rootfs with the owner 0 and a `metadata.yaml`. It
  imports and launches in 0.4 to 0.6 s. `raw.lxc` `lxc.init.cmd` makes the
  image's command PID 1, as docker does. `lxc.init.cmd` splits its value on
  spaces and takes no shell quoting.
* Three traps, as errand-box recorded them: `--map-auto` takes the first
  id range of the user, so `useradd`'s own range of 65536 ids must not come
  first; incus's own `/etc/subuid` inside the namespace must lie inside the
  namespace's map; and the guests outlive incusd, so a stop kills the whole
  id range.

Median milliseconds of 5 runs of the busybox workloads
(`experiments/lib/wl-busybox.sh`), in the same alpine 3.21:

| Load | rootless incus | podvm, userland rung |
| --- | --- | --- |
| spawn (300 exec) | 100 | 150 |
| files (3000 files) | 450 | 730 |
| cpu (awk loop) | 1310 | 1510 |

podvm ran as the same user and entered its userland rung, where the
interposer is the cost.

## The form of the back end (T-1718)

* **The daemon.** podvm drives an incusd that the host serves, or starts
  the host's packaged `incusd` in the user's namespace, as experiment 433
  does. Both need the root preparation of route A. A static build of
  `incusd` with liblxc (Go with cgo, built with zig) is not tried: it is a
  packaging task of its own, and podvm stays one static binary without it.
* **The verbs.** `run` is `POST /1.0/instances` with the image and
  `raw.lxc` `lxc.init.cmd`, then a start; `exec` is
  `POST /1.0/instances/NAME/exec`; `ps` is `GET /1.0/instances`; `stop`,
  `start` and `rm` are `PUT .../state` and `DELETE`; `logs` is the console
  log; `cp` is the files API; `-v` is a `disk` device (errand-box's x92
  adds one; not measured here).
* **The images.** incus 6.0 pulls no OCI image. podvm pulls it, extracts
  it, and imports a unified tarball (`POST /1.0/images`), as clause 5 does.
  The tarball should carry the intended owners of podvm's `.meta.jsonl`,
  not the owner 0 for each file.
* **The command.** `lxc.init.cmd` takes no quoting: podvm writes the argv
  into a small script in the image and names that script.
* **The network.** No network in the user's namespace without a relay
  (errand-box E-23). The first back end says so and refuses `-p`.
* **The crate.** The client of the incus REST API over a unix socket is
  generic: it goes to podcrates as a crate, by a COORD issue.

## What stays open

* incus virtual machines and their speed: they need `/dev/kvm`; the runner
  of `kvm.yml` (T-1682) has it.
* The network of an instance under route A.
* A static `incusd`.
