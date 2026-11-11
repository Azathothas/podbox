//! `podvm run`: milestone M3.
//!
//! [`TODO/milestones.md`](../../../TODO/milestones.md) T-1104,
//! [`TODO/enter.md`](../../../TODO/enter.md) T-0501 to T-0506,
//! [`TODO/cli.md`](../../../TODO/cli.md) T-0802.
//!
//! ⭐ **This verb is the product requirement the whole specification exists
//! for**: an agent that knows `docker` needs zero new knowledge. So its
//! argument surface, its output channels and its exit codes are docker's, and
//! where podvm cannot honour something it says so in one line rather than
//! pretending.

use std::io::Write;

use podvm_enter::{binfmt, Fds, Plan, RootDir};
use podvm_image::clock;
use podvm_image::error::{EXIT_CLI_ERROR, EXIT_FLAG_ERROR};
use podvm_image::platform::Platform;
use podvm_image::transport::Policy;

/// ⭐ The option block, with no first line, because `create` is served by this
/// same parser and its usage must not say `run`. [`usage`] builds that line from
/// the verb the caller actually typed. One block, so a flag documented here
/// cannot be missing from the other verb's help. TODO/cli.md T-0801.
pub const RUN_OPTIONS: &str = "\
  -d, --detach     start the container and print its id, and do not wait
  --name NAME      a name for the container. note: Refused if one already has it.
                   A foreground run leaves a record under it: `ps -a` lists
                   it as Exited and `logs` reads its output
                   (TODO/supervise.md T-1421)
  --rm             remove the extracted rootfs when the payload exits,
                   unless a container record references it: the rootfs is
                   shared, and a failed run must not delete what `keeper`
                   needs (TODO/image.md T-1322)
  -e, --env K=V    set an environment variable. Repeatable; a later one wins
  --env-file F     read KEY=VALUE lines from F into the environment.
                   Repeatable; entries load at the flag's position, so a
                   later -e wins over the file (TODO/cli.md T-0801)
  -w, --workdir D  working directory inside the container
  --device H[:G[:P]]
                   map a host device or file into the container: opened
                   before the chroot and served as a duplicate of the host
                   descriptor where the interposer holds. Repeatable.
                   P takes r, w and m; m parses and grants nothing
                   (TODO/enter.md T-0501)
  -v, --volume H:G[:rw]
                   show the host path H at the guest path G. Repeatable. A
                   path rewrite through PODVM_MAPS on the userland loader
                   family, not a mount: a write lands in H. Refused by name
                   on every other rung, for :ro until TODO/enter.md T-1655,
                   and for a -w or a first program inside G
                   (TODO/enter.md T-1653)
  --unsafe-host-paths
                   podvm's own, userland rung only: opt into the
                   pass-through with no PODVM_MAPS anchoring, so absolute
                   guest paths resolve on the host and a write lands on
                   the host tree. Where chroot is denied, a static payload
                   runs on the memfd family only with this flag, and
                   refuses at 125 without it (TODO/enter.md T-1648). The
                   host-root line prints even under -q, and --strict
                   refuses (TODO/enter.md T-1407)
  -u, --user U:G   run as this identity: numeric uid and gid, or names from
                   the image's own passwd and group files. The requested id
                   is answered through the interposer identity memo for a
                   reachable payload, which the banner names and --strict
                   refuses (TODO/interpose.md T-0711)
  --entrypoint P   replace the image's entrypoint. note: As docker: this also
                   drops the image's Cmd, because those were that
                   entrypoint's default arguments
  --add-host N:IP  add a name to the container's /etc/hosts. Repeatable
  --no-source-fixup
                   leave the image's package sources exactly as extracted.
                   note: podvm rewrites http:// to https:// for a mirror that
                   answers over HTTPS, because tcp/80 HANGS on the runtimes
                   podvm targets. This turns that off, and UNDOES a rewrite
                   an earlier container made in the same shared rootfs
  --no-host-cas    do not append this machine's announced CA bundle
                   ($SSL_CERT_FILE, $CURL_CA_BUNDLE, $REQUESTS_CA_BUNDLE) to
                   the image's own trust store. note: Where this machine
                   intercepts TLS, an https package source then fails to
                   verify inside the container, exactly as it does under
                   docker
  --no-steps       refused: do not run any COMMAND inside the rootfs before the
                   payload. Two fixups cannot be made from outside the
                   chroot -- `pacman-key --init` for an empty keyring, and
                   `openssl rehash` for a hash-indexed CA directory, which is
                   the only trust store libzypp reads -- and podvm names
                   each on the banner before it runs it. This refuses them
                   all, and the fixup log then says what the caller gave up
  --strict         refused: refuse to run at all where anything about this
                   invocation is Degraded or Stub: a flag, the selected rung,
                   or a fixup the completion layer had to make.
                   Kind decides (TODO/cli.md T-1415): safety-relevant
                   differences (no path virtualization, host and libc
                   mixing, escape) still refuse, dev-shim substitutions
                   warn, and --strict=all keeps refusing every one
  -q, --quiet      force the banner off. note: Runs are already quiet by
                   default; this forces silence under --verbose too
                   (TODO/cli.md T-1416)
  --verbose        print the banner. note: -v is NOT this flag: -v is
                   docker's --volume, so the verbose switch is long-only
                   (TODO/cli.md T-1416)
  --platform P     which platform of a multi-platform image to run
  --pull WHEN      never | missing (default) | always
  --insecure-registry HOST, --tls-verify=B
                   as `podvm pull`; used only when something must be fetched
  -t, --tty        refused: REFUSED BY NAME where /dev/ptmx is unusable, rather
                   than silently degraded (TODO/enter.md T-0503)
  --podvm-tier T  podvm's own: machine selects the machine tier, and each
                   other value is a flag error. With no flag the ladder
                   selects, under every name the binary answers to, and
                   `system info` names the entered rung (TODO/machine.md
                   T-1302, TODO/cli.md T-1712)
  --podvm-qemu-arg A
                   podvm's own, machine tier only and refused elsewhere: one
                   token for the emulator per occurrence. Repeatable, and never
                   split on whitespace (TODO/machine.md T-1302)
  --podvm-mem S   podvm's own, machine tier only and refused elsewhere: the
                   guest memory in bytes, with an optional K/M/G/T suffix. A
                   guest over the RLIMIT_FSIZE ceiling is refused before it
                   starts, naming both numbers (TODO/machine.md T-1305)

  refused: podvm run enters a CHROOT, not a container. It shares this machine's
    process table, network, IPC and mount namespaces with the payload. The
    banner on stderr says so on every run and names what the selected rung
    must never claim.

  note: The payload owns stdout. Every word podvm prints goes to stderr, so
    `podvm run <image> cmd | consumer` gives the consumer the payload's
    bytes and nothing else.

  note: The exit code is the payload's own, and a signalled payload is 128+signal.
    125 is podvm failing to run the command, 126 the command found and not
    invocable, 127 not found. Those are docker's.
";

mod parse;
pub use parse::*;

mod foreground;
pub use foreground::*;

mod prepare;
pub(crate) use prepare::*;

mod acquire;
pub(crate) use acquire::*;

#[cfg(test)]
mod tests;
