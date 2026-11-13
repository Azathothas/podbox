//! The built-in profiles. TODO/cli.md T-1654, and the provider shapes of
//! T-1659 (issue 172), and the language runtimes of T-1661 (issue 176).
//!
//! Each is TOML, checked by the same code as a user's file. A user file of
//! the same name under the profile directory takes the place of one. Each
//! names what it does not model (`leaves_out`).

/// The names, in the order of `profile list`.
pub const NAMES: &[&str] = &[
    "default",
    "workspace",
    "readonly",
    "machine",
    "incus",
    "lambda-al2023",
    "vercel-sandbox",
    "cloudflare-sandbox",
    "busybox-bare",
    "distroless-static",
    "node-shell",
    "node-permission",
    "deno-permissions",
    "bun-runtime",
    "errand",
    "playwright-chromium",
    "workerd-test",
    "wasmtime-wasi",
];

/// The text of a built-in profile.
pub fn text(name: &str) -> Option<&'static str> {
    Some(match name {
        "default" => {
            r#"description = "the ladder selects the rung; no table changes a flag"
leaves_out = ["each limit that needs a cgroup (memory, cpus, pids) on a rootfs tier"]
[fs]
backend = "auto"
"#
        }
        "workspace" => {
            r#"description = "the current directory at /workspace, the working directory there"
extends = "default"
[fs]
workspace = "."
workdir = "/workspace"
"#
        }
        "readonly" => {
            r#"description = "workspace, with a read-only root and an empty /tmp"
extends = "workspace"
leaves_out = ["the size of /tmp (enforced on the syscall tier only)", "a raw system call past the interposer"]
[fs]
read_only = true
tmpfs = ["/tmp"]
"#
        }
        "machine" => {
            r#"description = "workspace, in a guest of the machine tier with 1 GiB and 2 vCPUs"
extends = "workspace"
leaves_out = ["a write of the guest to /workspace: it is a copy"]
[fs]
backend = "machine"
[limits]
memory = "1g"
cpus = 2
"#
        }
        "incus" => {
            r#"description = "a disposable instance of an incusd (TODO/machine.md T-1718)"
extends = "default"
leaves_out = ["volumes and the network of the instance (T-1718)"]
[fs]
backend = "incus"
"#
        }
        // TODO/cli.md T-1659: the provider shapes of issue 172.
        "lambda-al2023" => {
            r#"description = "the shape of AWS Lambda on provided:al2023: user 993:990, a read-only root, /tmp, the Lambda variables"
extends = "workspace"
leaves_out = [
  "Lambda's invocation protocol (the Runtime Interface Emulator)",
  "its 15-minute limit and its network",
  "memory 128m and pids 1024: no cgroup on a rootfs tier (-s limits.memory=128m with fs.backend = machine)",
  "the size of /tmp (512m; enforced on the syscall tier only)",
]
[fs]
image = "public.ecr.aws/lambda/provided:al2023"
entrypoint = ""
user = "993:990"
read_only = true
tmpfs = ["/tmp:size=512m"]
[env.set]
AWS_LAMBDA_FUNCTION_NAME = "podvm"
AWS_LAMBDA_FUNCTION_MEMORY_SIZE = "128"
AWS_LAMBDA_FUNCTION_VERSION = "$LATEST"
AWS_REGION = "us-east-1"
AWS_EXECUTION_ENV = "AWS_Lambda_provided.al2023"
LAMBDA_TASK_ROOT = "/var/task"
LAMBDA_RUNTIME_DIR = "/var/runtime"
TZ = ":UTC"
PATH = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/opt/bin"
"#
        }
        "vercel-sandbox" => {
            r#"description = "the shape of a Vercel sandbox: amazonlinux:2023 as user 1000:1000"
extends = "workspace"
leaves_out = ["Vercel's own isolation (a microVM), replaced by the host's"]
[fs]
image = "public.ecr.aws/amazonlinux/amazonlinux:2023"
user = "1000:1000"
[env.set]
USER = "vercel-sandbox"
LOGNAME = "vercel-sandbox"
"#
        }
        "cloudflare-sandbox" => {
            r#"description = "the shape of a Cloudflare sandbox: Alpine as root, a read-only root, /workspace"
extends = "workspace"
leaves_out = [
  "the sandbox-shim of cloudflare/sandbox:1.0.3",
  "Cloudflare's own isolation (a container of its own), replaced by the host's",
  "curl: the image has none",
]
[fs]
image = "public.ecr.aws/docker/library/alpine:3.22"
read_only = true
tmpfs = ["/tmp"]
"#
        }
        "busybox-bare" => {
            r#"description = "the barest shell: busybox:musl"
extends = "workspace"
leaves_out = [
  "TLS validation of busybox wget: expired, self-signed and wrong-host certificates exit 0",
  "curl, bash and python3: the image has none",
]
[fs]
image = "public.ecr.aws/docker/library/busybox:musl"
"#
        }
        "distroless-static" => {
            r#"description = "no shell and no coreutils: distroless/static-debian12; run a static program of the project"
extends = "workspace"
leaves_out = ["a shell: sh -c refuses by name; a static program of /workspace runs"]
[fs]
image = "gcr.io/distroless/static-debian12"
"#
        }
        "errand" => {
            r#"description = "the policy of errand's bailey backend (issue 170): the confine layer with bailey's read list and devices, /tmp writable"
extends = "default"
confine = true
leaves_out = [
  "the project at /workspace and the state at /state: -v serves them under the confine layer on the userland rung; on the namespace rung -v waits for TODO/cli.md T-1761",
  "bailey's built environment (only PATH, LANG, LC_ALL, TERM, HOME, XDG_*)",
  "bailey's egress per port: TCP is free unless net.ports names the ports",
  "file_max, tmp_size and shm_size",
]
[fs]
read = ["?/etc/ld.so.cache", "?/etc/ld.so.conf", "?/etc/ld.so.conf.d", "?/etc/ssl", "?/etc/ca-certificates", "?/etc/pki", "?/etc/nsswitch.conf", "?/etc/services", "?/etc/protocols", "?/etc/alternatives", "?/etc/passwd", "?/etc/group"]
write = ["?/tmp", "?/dev/full", "?/dev/tty"]
"#
        }
        "playwright-chromium" => {
            r#"description = "headless Chromium of Playwright (chrome-headless-shell) under the confine layer: --no-sandbox; the arguments after -- go to the browser"
extends = "workspace"
confine = true
leaves_out = [
  "Chromium's own sandbox: it needs user namespaces, so --no-sandbox is set, and the boundary is the confine layer, not the browser; this is no boundary against a hostile page",
  "the namespace rung: it mounts no /proc, and Chromium finds its ICU data through /proc/self/exe; the profile runs on the loader family, where chroot is denied",
  "the network: a page loads from /workspace (file://); TCP is free unless net.ports names the ports",
]
[fs]
image = "mcr.microsoft.com/playwright:v1.65.0-noble@sha256:9beb546099e11f52d62e505537914870e01a67e0e6b00ae41da4775324e73dbe"
entrypoint = ""
wrap = ["/ms-playwright/chromium_headless_shell-1254/chrome-headless-shell-linux64/chrome-headless-shell", "--headless", "--no-sandbox", "--disable-dev-shm-usage", "--disable-gpu"]
read = ["?/etc/fonts", "?/etc/ssl", "?/etc/passwd", "?/etc/group", "?/etc/nsswitch.conf", "?/etc/ld.so.cache", "?/etc/localtime", "?/usr/share/fonts"]
write = ["/workspace", "?/tmp"]
exec = ["/ms-playwright"]
[env.set]
HOME = "/tmp"
"#
        }
        "node-shell" => {
            r#"description = "Node and npm, nothing else: node:lts-slim"
extends = "workspace"
leaves_out = ["curl: the image has none"]
[fs]
image = "public.ecr.aws/docker/library/node:lts-slim"
"#
        }
        // TODO/cli.md T-1661: language runtimes with their own permission
        // model. `fs.read` and `fs.write` give the runtime's flags; the
        // workspace volume and the image give podvm's path map.
        "node-permission" => {
            r#"description = "node --permission: reads and writes in /workspace only, no child process"
extends = "workspace"
runtime = "node"
leaves_out = [
  "the network: Node's permission model has no network rule (net.mode = broker on a tier that holds it)",
  "a native addon: --permission refuses each one",
]
[fs]
image = "public.ecr.aws/docker/library/node:lts-slim"
read = ["/workspace"]
write = ["/workspace"]
"#
        }
        "wasmtime-wasi" => {
            r#"description = "wasmtime run of a WASI module: only /workspace is preopened, no socket"
extends = "workspace"
runtime = "wasmtime"
leaves_out = [
  "a socket: WASI preview 1 gives the module none, and wasmtime grants no network without -S inherit-network",
  "the arm64 host: the pin is the x86_64 binary of the release",
]
[fs]
image = "gcr.io/distroless/cc-debian12@sha256:e5d81ddde149641e2a9ba55be4545bc125c67de07508b03ba4c22e6eb0ded5aa"
read = ["/workspace"]
"#
        }
        "workerd-test" => {
            r#"description = "workerd test of /workspace/worker.js: no outbound service, no TCP connect, under the confine layer"
extends = "workspace"
runtime = "workerd"
confine = true
leaves_out = [
  "a binding of the worker: the config that podvm writes has one worker and no service but `none`",
  "the arm64 host: the pin is the x86_64 binary of the npm package",
]
[fs]
image = "gcr.io/distroless/cc-debian12@sha256:e5d81ddde149641e2a9ba55be4545bc125c67de07508b03ba4c22e6eb0ded5aa"
read = ["/workspace", "/opt/podvm-workerd", "?/etc/ssl", "?/etc/passwd"]
exec = ["/opt/podvm-workerd"]
[net]
ports = []
"#
        }
        "deno-permissions" => {
            r#"description = "deno run --no-prompt: reads and writes in /workspace only; env, net, run and ffi denied"
extends = "workspace"
runtime = "deno"
leaves_out = ["a grant of env, net, run or ffi: give it with fs.wrap and no runtime"]
[fs]
image = "docker.io/denoland/deno:debian"
read = ["/workspace"]
write = ["/workspace"]
"#
        }
        "bun-runtime" => {
            r#"description = "bun in a guest of the machine tier: Bun has no permission model, and the guest is the boundary"
extends = "workspace"
runtime = "bun"
leaves_out = [
  "a permission model: Bun has none, so each path of the guest is open to the payload",
  "a rootfs tier: Bun makes its own system calls, past the interposer, so the path map does not hold it (measured: /workspace/b.js named the host's /workspace)",
  "a write to /workspace: the guest has a copy",
]
[fs]
image = "docker.io/oven/bun:slim"
backend = "machine"
[limits]
memory = "1g"
"#
        }
        _ => return None,
    })
}
