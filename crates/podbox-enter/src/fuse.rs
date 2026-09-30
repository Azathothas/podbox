//! TODO/packaging.md T-1003: the FUSE rung's staging and read-only server.
//!
//! memfd stages bytes into a fd, tmpfs stages the tree onto a mount, rundir
//! stages it into a directory: the FUSE rung serves the extracted tree over
//! `/dev/fuse` without copying it, and the payload enters the mount. The
//! mount needs what the other rungs do not: the node must open, a `mount(2)`
//! carrying the session fd must attach, and a server must answer the
//! kernel's requests for as long as the payload runs.
//!
//! ⛔ Read-only, by reply rather than by mount flag. Every mutating opcode
//! answers `EROFS` and every unknown one `ENOSYS`: a payload that writes
//! gets the errno, never a silent drop and never a write to the store's
//! own tree.
//!
//! ⛔ The server is a forked child, never a thread: the T-0603 assertion
//! scans this crate, and a thread would make the clone below fork a
//! threaded process. The child mounts, serves, and dies with the run; the
//! parent bounds every wait on it, kills it on every path, and unmounts
//! and removes the mount directory behind it.
//!
//! ⚠ What this module does NOT prove. No reachable machine holds
//! `/dev/fuse` (absent on the lane, uncreatable: `mknod` is `EPERM` even
//! in a user namespace, measured 2026-09-30) or grants a mount
//! (`mount(2)` is `EPERM` even in a user namespace, measured the same
//! day). The live drive therefore proves the refusal arms and the mount
//! failure's cleanup; entry over a real session needs a host with the
//! node and a granted mount, and names it in the refusal below.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use podbox_probe::sys::{self, CBuf};

use crate::stage;

/// The node the rung is named for.
pub const FUSE_NODE: &str = "/dev/fuse";

/// How long the parent waits for the server's mounted word and its serving
/// word before killing it. A bound reached is a refusal, never a longer
/// wait: TODO/RULES.md section 8's own rule.
const READY_TIMEOUT_MS: i64 = 10_000;

/// The request buffer. The kernel hands one request per read, and
/// `FUSE_MIN_READ_BUFFER` (8192, `/usr/include/linux/fuse.h:70`) is its
/// floor; reads are capped at 128 KiB beside the reply cap, so no request
/// sizes this buffer.
const IO_BUFFER: usize = 128 * 1024;

/// The largest single READ this server answers. Reads past it are served
/// whole up to this bound and refused past it: an unbounded reply buffer
/// is the whole-unbounded-response pattern `forbidden-patterns.md`
/// refuses, and no reader the payload uses asks past 128 KiB.
const MAX_READ: u64 = 128 * 1024;

/// Opcodes this server answers, read at `/usr/include/linux/fuse.h:517-566`
/// on the lane rather than remembered.
const OP_LOOKUP: u32 = 1;
const OP_FORGET: u32 = 2;
const OP_GETATTR: u32 = 3;
const OP_READLINK: u32 = 5;
const OP_OPEN: u32 = 14;
const OP_READ: u32 = 15;
const OP_STATFS: u32 = 17;
const OP_RELEASE: u32 = 18;
const OP_INIT: u32 = 26;
const OP_OPENDIR: u32 = 27;
const OP_READDIR: u32 = 28;
const OP_RELEASEDIR: u32 = 29;
const OP_ACCESS: u32 = 34;
const OP_INTERRUPT: u32 = 36;
const OP_DESTROY: u32 = 38;
const OP_BATCH_FORGET: u32 = 42;

/// Mutating opcodes, answered `EROFS` rather than implemented: setattr,
/// symlink, mknod, mkdir, unlink, rmdir, rename, link, write, create and
/// every xattr/flock/fallocate/tmpfile word. Same header, same lines.
const OP_MUTATING: [u32; 24] = [
    4, 6, 8, 9, 10, 11, 12, 13, 16, 20, 21, 22, 23, 24, 25, 31, 32, 33, 35, 37, 43, 45, 46, 47,
];

/// The ABI this server speaks, read at the same header (`:235-238` there
/// the kernel is 7.38). The kernel negotiates down on an older one; the
/// structs below are parsed by the negotiated version, so offering the
/// read one keeps the offer and the bytes consistent.
const ABI_MAJOR: u32 = 7;
const ABI_MINOR: u32 = 38;

/// `dirent.h`'s types for the READDIR reply: unknown, directory, regular,
/// symlink. Stable kernel ABI, like the opcodes above.
const DT_UNKNOWN: u32 = 0;
const DT_DIR: u32 = 4;
const DT_REG: u32 = 8;
const DT_LNK: u32 = 10;

/// `sys/mount.h`'s pair for a userspace mount: neither the set-uid bit
/// nor device nodes pass through it. `MS_PRIVATE` beside them documents
/// its header in `sys.rs:469-475`; these ride the same ABI.
const MS_NOSUID: u64 = 0x2;
const MS_NODEV: u64 = 0x4;

fn errno(n: i32) -> sys::Errno {
    sys::Errno(n)
}

fn put_u32(out: &mut [u8], at: usize, v: u32) {
    out[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn put_u64(out: &mut [u8], at: usize, v: u64) {
    out[at..at + 8].copy_from_slice(&v.to_le_bytes());
}

fn get_u32(req: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([req[at], req[at + 1], req[at + 2], req[at + 3]])
}

fn get_u64(req: &[u8], at: usize) -> u64 {
    u64::from_le_bytes([
        req[at],
        req[at + 1],
        req[at + 2],
        req[at + 3],
        req[at + 4],
        req[at + 5],
        req[at + 6],
        req[at + 7],
    ])
}

/// Whether the device node opens: the rung's admission gate.
///
/// `path` is the parameter so tests drive both arms without the node: an
/// absent path refuses naming it, a regular file opens. [`check_node`]
/// pins the real node.
pub fn check_node_at(path: &str) -> Result<(), stage::Refusal> {
    let node = CBuf::new(path).ok_or_else(|| stage::Refusal {
        path: PathBuf::from(path),
        why: "the FUSE node path contains a NUL byte".to_string(),
    })?;
    match sys::open(&node, sys::O_RDWR | sys::O_CLOEXEC, 0) {
        Ok(fd) => {
            let _ = sys::close(fd);
            Ok(())
        }
        Err(e) => Err(stage::Refusal {
            path: PathBuf::from(path),
            why: format!(
                "/dev/fuse did not open ({} ({})): the FUSE rung needs the \
                 node this machine does not hold",
                e.name(),
                e.0
            ),
        }),
    }
}

/// [`check_node_at`] over the real node.
pub fn check_node() -> Result<(), stage::Refusal> {
    check_node_at(FUSE_NODE)
}

/// The mount directory: a unique directory under the store's fuse/ base.
///
/// The server mounts over it and the payload enters it; [`release_mount`]
/// unmounts and removes it on every path. Private: tests reach it through
/// [`serve_tree`]'s refusal arms and the in-module tests below.
fn stage_mount_dir(store_root: &Path) -> Result<PathBuf, stage::Refusal> {
    let base = store_root.join("fuse");
    std::fs::create_dir_all(&base).map_err(|e| stage::Refusal {
        path: base.clone(),
        why: format!("the fuse base could not be prepared: {e}"),
    })?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = base.join(format!("mnt-{}-{nanos}", std::process::id()));
    if dir.exists() {
        return Err(stage::Refusal {
            path: dir,
            why: "the mount directory already exists: refusing rather than mounting over a stale tree"
                .to_string(),
        });
    }
    std::fs::create_dir_all(&dir).map_err(|e| stage::Refusal {
        path: dir.clone(),
        why: format!("the mount directory could not be created: {e}"),
    })?;
    Ok(dir)
}

/// The serving mount: the server's pid and the directory it serves over.
///
/// Created by [`serve_tree`], ended by [`release_mount`] on every path,
/// on success and on failure alike.
#[derive(Debug)]
pub struct ActiveMount {
    /// The forked server's pid, killed and reaped by [`release_mount`].
    pub pid: i64,
    /// The mount target: the payload's root while active.
    pub dir: PathBuf,
}

/// Serve the extracted tree at `backing` over a FUSE mount.
///
/// The node must open, the mount must attach, and the server must answer
/// INIT, each bounded: any failure removes the mount directory and
/// refuses naming the step, so a half-staged FUSE mount never survives
/// to meet the next run. The tree is served, never copied: reads go to
/// the store's own extraction, writes are refused with `EROFS`.
pub fn serve_tree(store_root: &Path, backing: &Path) -> Result<ActiveMount, stage::Refusal> {
    if !backing.is_dir() {
        return Err(stage::Refusal {
            path: backing.to_path_buf(),
            why: "not a directory podbox can serve".to_string(),
        });
    }
    check_node()?;
    let dir = stage_mount_dir(store_root)?;
    let node = CBuf::new(FUSE_NODE).expect("a literal carries no NUL");
    let fd = sys::open(&node, sys::O_RDWR | sys::O_CLOEXEC, 0).map_err(|e| {
        let _ = std::fs::remove_dir_all(&dir);
        stage::Refusal {
            path: PathBuf::from(FUSE_NODE),
            why: format!(
                "/dev/fuse did not open ({} ({})): the FUSE rung needs the \
                 node this machine does not hold",
                e.name(),
                e.0
            ),
        }
    })?;
    let mut pipe = [0i32; 2];
    if sys::pipe2(&mut pipe, sys::O_CLOEXEC).is_err() {
        let _ = sys::close(fd);
        let _ = std::fs::remove_dir_all(&dir);
        return Err(stage::Refusal {
            path: dir,
            why: "the readiness pipe for the FUSE server could not be made".to_string(),
        });
    }
    let (read_end, write_end) = (pipe[0] as i64, pipe[1] as i64);
    let backing_owned = backing.to_path_buf();
    let dir_owned = dir.clone();
    let pid = match unsafe { sys::clone_fork(sys::SIGCHLD) } {
        Ok(0) => child_main(fd, read_end, write_end, &backing_owned, &dir_owned),
        Ok(pid) => pid,
        Err(e) => {
            let _ = sys::close(write_end);
            let _ = sys::close(read_end);
            let _ = sys::close(fd);
            let _ = std::fs::remove_dir_all(&dir);
            return Err(stage::Refusal {
                path: dir,
                why: format!(
                    "clone(2) for the FUSE server failed: {} ({})",
                    e.name(),
                    e.0
                ),
            });
        }
    };
    let _ = sys::close(write_end);
    // ⛔ The parent's copy of the session fd closes once the server is
    // serving: the session lives on the server's copy, and a copy held
    // here would keep the session open past the server's death.
    let outcome = wait_for_server(pid, read_end, &dir, fd);
    let _ = sys::close(read_end);
    outcome
}

/// The parent half of [`serve_tree`]: two bounded words, then the rung.
///
/// The server writes a 4-byte errno after the mount (0 where it
/// attached) and a second one after answering INIT. Between them the
/// parent stats the mount to trigger the INIT. Any other outcome kills
/// and reaps the server, unmounts where mounted, removes the directory,
/// and refuses naming the step.
fn wait_for_server(
    pid: i64,
    read_end: i64,
    dir: &Path,
    fd: i64,
) -> Result<ActiveMount, stage::Refusal> {
    let kill_and_clean = |dir: &Path| {
        let _ = sys::kill(pid, 9);
        let _ = crate::Child { pid }.wait_bounded(5_000);
        if let Some(target) = CBuf::new(&dir.to_string_lossy()) {
            let _ = sys::umount(&target);
        }
        let _ = std::fs::remove_dir_all(dir);
    };
    let first = read_word(read_end, READY_TIMEOUT_MS);
    match first {
        // The mount attached. Stat the target to trigger INIT, then wait
        // for the serving word.
        Some(0) => {
            let _ = std::fs::metadata(dir);
            match read_word(read_end, READY_TIMEOUT_MS) {
                Some(0) => {
                    let _ = sys::close(fd);
                    Ok(ActiveMount {
                        pid,
                        dir: dir.to_path_buf(),
                    })
                }
                _ => {
                    kill_and_clean(dir);
                    Err(stage::Refusal {
                        path: dir.to_path_buf(),
                        why: "the FUSE server mounted but never answered INIT within 10 s: \
                              bound reached, server killed, mount released"
                            .to_string(),
                    })
                }
            }
        }
        // The mount call refused: the child's errno travels in the word.
        Some(n) => {
            let _ = crate::Child { pid }.wait_bounded(5_000);
            if let Some(target) = CBuf::new(&dir.to_string_lossy()) {
                let _ = sys::umount(&target);
            }
            let _ = std::fs::remove_dir_all(dir);
            let e = errno(n as i32);
            Err(stage::Refusal {
                path: dir.to_path_buf(),
                why: format!(
                    "the FUSE mount was refused ({} ({})): the FUSE rung needs a \
                     mount this runtime does not grant",
                    e.name(),
                    e.0
                ),
            })
        }
        // EOF or the bound: the server is gone or wedged.
        None => {
            kill_and_clean(dir);
            Err(stage::Refusal {
                path: dir.to_path_buf(),
                why: "the FUSE server did not mount within 10 s: bound reached, \
                      server killed, mount released"
                    .to_string(),
            })
        }
    }
}

/// One 4-byte little-endian word off the readiness pipe, or `None` on EOF
/// or the bound. A 4-byte write is atomic, so a ready pipe carries the
/// whole word: no framing, no second read.
fn read_word(fd: i64, timeout_ms: i64) -> Option<u32> {
    let mut fds = [sys::PollFd {
        fd: fd as i32,
        events: sys::POLLIN,
        revents: 0,
    }];
    match sys::ppoll(&mut fds, timeout_ms) {
        Ok(n) if n > 0 => {}
        _ => return None,
    }
    let mut buf = [0u8; 4];
    let mut got = 0usize;
    while got < 4 {
        match sys::read(fd, &mut buf[got..]) {
            Ok(0) => return None,
            Ok(n) => got += n as usize,
            Err(e) if e == sys::EINTR => continue,
            Err(_) => return None,
        }
    }
    Some(u32::from_le_bytes(buf))
}

/// End the serving mount: kill and reap the server, unmount, remove.
///
/// Best-effort every half, like [`stage::release_tmpfs`]: the kill is
/// what stops the serving, the unmount is what frees the target, and the
/// removal is what keeps the next run from meeting this one. A directory
/// that was never mounted removes like any other.
pub fn release_mount(mounted: &ActiveMount) {
    // ⚠ SIGKILL, not SIGTERM: the server masks nothing and traps
    // nothing, and a server past its payload must die rather than flush.
    let _ = sys::kill(mounted.pid, 9);
    let _ = crate::Child { pid: mounted.pid }.wait_bounded(5_000);
    if let Some(target) = CBuf::new(&mounted.dir.to_string_lossy()) {
        let _ = sys::umount(&target);
    }
    let _ = std::fs::remove_dir_all(&mounted.dir);
}

/// The server child: mount, report, serve, exit. Never returns.
fn child_main(fd: i64, read_end: i64, write_end: i64, backing: &Path, dir: &Path) -> ! {
    let _ = sys::close(read_end);
    let report = |n: u32| {
        let _ = write_all(write_end, &n.to_le_bytes());
    };
    match mount_fuse(fd, dir) {
        Ok(()) => report(0),
        Err(e) => {
            report(e.0 as u32);
            let _ = sys::close(write_end);
            let _ = sys::close(fd);
            sys::exit_group(1);
        }
    }
    let code = serve_until_destroy(fd, backing, write_end);
    let _ = sys::close(write_end);
    let _ = sys::close(fd);
    sys::exit_group(code);
}

/// `mount(2)` the session fd over `dir`.
///
/// `sys::mount` carries no data argument (the tmpfs rung needs none), so
/// this issues the raw number beside it, the way `memfd.rs` issues its
/// own: source and fstype `fuse`, flags `MS_NOSUID|MS_NODEV`, data
/// `fd=N,rootmode=OOO,user_id=U,group_id=G`. The option shape is the
/// kernel's, read in the corpus at
/// `references/apptainer__apptainer/tree/internal/pkg/runtime/engine/apptainer/container_linux.go:563`.
fn mount_fuse(fd: i64, dir: &Path) -> Result<(), sys::Errno> {
    let mode = std::fs::metadata(dir).map_or(0o755, |m| {
        use std::os::unix::fs::PermissionsExt;
        m.permissions().mode() & 0o7777
    }) | 0o040000;
    let data = format!(
        "fd={fd},rootmode={mode:o},user_id={},group_id={}",
        euid(),
        egid()
    );
    let (Some(source), Some(target), Some(fstype), Some(data)) = (
        CBuf::new("fuse"),
        CBuf::new(&dir.to_string_lossy()),
        CBuf::new("fuse"),
        CBuf::new(&data),
    ) else {
        return Err(errno(22));
    };
    // ⛔ The CBufs outlive the call: the kernel reads every pointer
    // before returning, so nothing here may drop first.
    match unsafe {
        sys::sys(
            sys::SYS_MOUNT,
            [
                source.ptr(),
                target.ptr(),
                fstype.ptr(),
                MS_NOSUID | MS_NODEV,
                data.ptr(),
                0,
            ],
        )
    } {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

/// The mounting ids. No wrapper names them in `sys.rs`, so these issue
/// the raw numbers beside `memfd.rs`'s own raw calls. Neither call can
/// fail; the fallback is root's pair, which the mount then judges.
fn euid() -> u32 {
    unsafe { sys::sys(sys::SYS_GETUID, [0, 0, 0, 0, 0, 0]).map(|n| n as u32) }.unwrap_or(0)
}

/// [`euid`] for the group.
fn egid() -> u32 {
    unsafe { sys::sys(sys::SYS_GETGID, [0, 0, 0, 0, 0, 0]).map(|n| n as u32) }.unwrap_or(0)
}

fn write_all(fd: i64, mut bytes: &[u8]) -> Result<(), sys::Errno> {
    while !bytes.is_empty() {
        match sys::write(fd, bytes) {
            Ok(0) => return Err(errno(5)),
            Ok(n) => bytes = &bytes[n as usize..],
            Err(e) if e == sys::EINTR => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// A looked-up node: the backing path and its parent's id.
///
/// The kernel resolves `.` and `..` itself and never asks for `/`, so
/// every node arrives through [`Fs::lookup`] below, which is the one
/// place paths are built: containment holds by construction, and `..`
/// answers the parent without touching the filesystem.
struct Node {
    path: PathBuf,
    parent: u64,
}

/// The served tree: the live nodes and the counters.
///
/// Node 1 is the root, pinned: FORGET never drops it. Every other id
/// comes from `next`, and a path looked up twice keeps its id while its
/// lookup count stands, which is the kernel's contract.
struct Fs {
    nodes: HashMap<u64, Node>,
    by_path: HashMap<PathBuf, u64>,
    lookups: HashMap<u64, u64>,
    next: u64,
    next_fh: u64,
}

impl Fs {
    fn new(root: &Path) -> Fs {
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            Node {
                path: root.to_path_buf(),
                parent: 1,
            },
        );
        Fs {
            nodes,
            by_path: HashMap::new(),
            lookups: HashMap::new(),
            next: 2,
            next_fh: 1,
        }
    }

    /// Look a name up under a node: the entry bytes, or an errno.
    ///
    /// `..` answers the parent's entry without touching the filesystem,
    /// so no lookup can climb past the backing root. A name carrying `/`
    /// or NUL is `ENOENT`: the kernel never sends one, and anything else
    /// did not come from the kernel.
    fn lookup(&mut self, parent: u64, name: &[u8]) -> Result<Vec<u8>, sys::Errno> {
        if name.is_empty() || name.contains(&0) || name.contains(&b'/') {
            return Err(errno(2));
        }
        let Some(node) = self.nodes.get(&parent) else {
            return Err(errno(2));
        };
        if name == b".." {
            let grand = node.parent;
            return self.entry_bytes(grand).ok_or(errno(2));
        }
        let text = std::str::from_utf8(name).map_err(|_| errno(2))?;
        let path = node.path.join(text);
        let meta = std::fs::symlink_metadata(&path).map_err(map_io)?;
        let id = match self.by_path.get(&path) {
            Some(id) => *id,
            None => {
                let id = self.next;
                self.next += 1;
                self.nodes.insert(
                    id,
                    Node {
                        path: path.clone(),
                        parent,
                    },
                );
                self.by_path.insert(path, id);
                id
            }
        };
        *self.lookups.entry(id).or_insert(0) += 1;
        entry_out(id, &meta)
    }

    /// The entry bytes for a live node, or `None` where it is forgotten.
    fn entry_bytes(&self, id: u64) -> Option<Vec<u8>> {
        let node = self.nodes.get(&id)?;
        let meta = std::fs::symlink_metadata(&node.path).ok()?;
        entry_out(id, &meta).ok()
    }

    /// Drop lookups: `FORGET` carries one id and a count, `BATCH_FORGET`
    /// carries pairs. Neither answers: the kernel does not wait.
    fn forget(&mut self, id: u64, n: u64) {
        if id == 1 {
            return;
        }
        let left = self
            .lookups
            .get(&id)
            .copied()
            .unwrap_or(0)
            .saturating_sub(n);
        if left == 0 {
            self.lookups.remove(&id);
            if let Some(node) = self.nodes.remove(&id) {
                self.by_path.remove(&node.path);
            }
        } else {
            self.lookups.insert(id, left);
        }
    }

    fn getattr(&self, id: u64) -> Result<Vec<u8>, sys::Errno> {
        let node = self.nodes.get(&id).ok_or(errno(2))?;
        let meta = std::fs::symlink_metadata(&node.path).map_err(map_io)?;
        attr_out(&meta)
    }

    /// Read a link's target: the bytes the kernel resolves itself.
    fn readlink(&self, id: u64) -> Result<Vec<u8>, sys::Errno> {
        let node = self.nodes.get(&id).ok_or(errno(2))?;
        let meta = std::fs::symlink_metadata(&node.path).map_err(map_io)?;
        if !meta.file_type().is_symlink() {
            return Err(errno(22));
        }
        let target = std::fs::read_link(&node.path).map_err(map_io)?;
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            Ok(target.as_os_str().as_bytes().to_vec())
        }
        #[cfg(not(unix))]
        {
            target
                .into_os_string()
                .into_string()
                .map_err(|_| errno(22))
                .map(|s| s.into_bytes())
        }
    }

    /// Open a file or a directory for reading: the handle, or an errno.
    ///
    /// Only regular files and directories open. Anything else (a link,
    /// which the kernel resolves itself; a fifo, which would block) is
    /// `EACCES`: present in the listing, never opened.
    fn open(&mut self, id: u64, dir: bool) -> Result<Vec<u8>, sys::Errno> {
        let node = self.nodes.get(&id).ok_or(errno(2))?;
        let meta = std::fs::symlink_metadata(&node.path).map_err(map_io)?;
        let ok = if dir {
            meta.file_type().is_dir()
        } else {
            meta.file_type().is_file()
        };
        if !ok {
            return Err(errno(13));
        }
        let fh = self.next_fh;
        self.next_fh += 1;
        let mut out = vec![0u8; 16];
        put_u64(&mut out, 0, fh);
        put_u32(&mut out, 8, 0);
        put_u32(&mut out, 12, 0);
        Ok(out)
    }

    /// Read file bytes: exactly the asked span, or an errno.
    ///
    /// ⛔ Exactly the span, never a capped one: a short reply reads as
    /// EOF to the kernel, so capping here would truncate the payload.
    /// The [`MAX_READ`] bound above refuses the oversized ask instead.
    fn read(&self, id: u64, offset: u64, size: u64) -> Result<Vec<u8>, sys::Errno> {
        if size > MAX_READ {
            return Err(errno(22));
        }
        let node = self.nodes.get(&id).ok_or(errno(2))?;
        let meta = std::fs::symlink_metadata(&node.path).map_err(map_io)?;
        if meta.file_type().is_dir() {
            return Err(errno(21));
        }
        if !meta.file_type().is_file() {
            return Err(errno(13));
        }
        let mut f = std::fs::File::open(&node.path).map_err(map_io)?;
        use std::io::{Read, Seek, SeekFrom};
        f.seek(SeekFrom::Start(offset)).map_err(map_io)?;
        let mut buf = vec![0u8; size as usize];
        let mut got = 0usize;
        while got < buf.len() {
            match f.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(map_io(e)),
            }
        }
        buf.truncate(got);
        Ok(buf)
    }

    /// List a directory: `.`, `..`, then the entries, each with a
    /// 1-based cookie. Resuming past `offset` skips that many entries,
    /// and a reply never splits an entry: what does not fit waits for
    /// the next ask with a later cookie.
    fn readdir(&self, id: u64, offset: u64, size: u64) -> Result<Vec<u8>, sys::Errno> {
        use std::os::unix::fs::MetadataExt as _;
        let node = self.nodes.get(&id).ok_or(errno(2))?;
        let meta = std::fs::symlink_metadata(&node.path).map_err(map_io)?;
        if !meta.file_type().is_dir() {
            return Err(errno(20));
        }
        let self_ino = meta.ino();
        let parent_ino = self
            .nodes
            .get(&node.parent)
            .and_then(|p| std::fs::symlink_metadata(&p.path).ok())
            .map_or(1, |m| {
                use std::os::unix::fs::MetadataExt;
                m.ino()
            });
        let mut entries: Vec<(u64, u32, Vec<u8>)> = vec![
            (self_ino, DT_DIR, b".".to_vec()),
            (parent_ino, DT_DIR, b"..".to_vec()),
        ];
        let mut children: Vec<_> = std::fs::read_dir(&node.path)
            .map_err(map_io)?
            .filter_map(|e| e.ok())
            .collect();
        // ⚠ Sorted by name: `read_dir` promises no order, and the resume
        // cookies below are positional, so an unstable order would skip
        // or repeat entries across asks.
        children.sort_by_key(|e| e.file_name());
        for child in children {
            #[cfg(unix)]
            let ino = {
                use std::os::unix::fs::DirEntryExt;
                child.ino()
            };
            #[cfg(not(unix))]
            let ino = 0;
            let dtype = child
                .file_type()
                .map(|t| {
                    if t.is_dir() {
                        DT_DIR
                    } else if t.is_file() {
                        DT_REG
                    } else if t.is_symlink() {
                        DT_LNK
                    } else {
                        DT_UNKNOWN
                    }
                })
                .unwrap_or(DT_UNKNOWN);
            #[cfg(unix)]
            let name = {
                use std::os::unix::ffi::OsStrExt;
                child.file_name().as_os_str().as_bytes().to_vec()
            };
            #[cfg(not(unix))]
            let name = child.file_name().to_string_lossy().into_bytes();
            entries.push((ino, dtype, name));
        }
        let mut out = Vec::new();
        for (index, (ino, dtype, name)) in entries.iter().enumerate() {
            let off = (index as u64) + 1;
            if (index as u64) < offset {
                continue;
            }
            let len = align8(24 + name.len());
            if out.len() + len > size as usize {
                break;
            }
            let mut ent = vec![0u8; len];
            put_u64(&mut ent, 0, *ino);
            put_u64(&mut ent, 8, off);
            put_u32(&mut ent, 16, name.len() as u32);
            put_u32(&mut ent, 20, *dtype);
            ent[24..24 + name.len()].copy_from_slice(name);
            out.extend_from_slice(&ent);
        }
        Ok(out)
    }

    /// Answer the access check against the backing file: 0 where the
    /// mode bits allow it, `EACCES` where they do not. The server runs
    /// as the mounter, so the bits are the verdict.
    fn access(&self, id: u64, mask: u32) -> Result<(), sys::Errno> {
        let node = self.nodes.get(&id).ok_or(errno(2))?;
        let meta = std::fs::symlink_metadata(&node.path).map_err(map_io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let mode = meta.mode();
            // ⚠ Owner bits: the mounter owns the serving, so its word
            // decides. Execute on a directory is search, on a file is
            // exec: the same bits answer both.
            let bits = if meta.uid() == euid_sync() {
                (mode >> 6) & 0o7
            } else if meta.gid() == egid_sync() {
                (mode >> 3) & 0o7
            } else {
                mode & 0o7
            };
            if (mask & 0o7) & !(bits) == 0 {
                Ok(())
            } else {
                Err(errno(13))
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (meta, mask);
            Ok(())
        }
    }
}

#[cfg(unix)]
fn euid_sync() -> u32 {
    euid()
}

#[cfg(not(unix))]
fn euid_sync() -> u32 {
    0
}

#[cfg(unix)]
fn egid_sync() -> u32 {
    egid()
}

#[cfg(not(unix))]
fn egid_sync() -> u32 {
    0
}

/// An `io::Error` as the errno the reply carries: the name where the
/// kernel names it, `EIO` past that.
fn map_io(e: std::io::Error) -> sys::Errno {
    use std::io::ErrorKind as K;
    match e.kind() {
        K::NotFound => errno(2),
        K::PermissionDenied => errno(13),
        K::AlreadyExists => errno(17),
        K::InvalidInput => errno(22),
        _ => errno(5),
    }
}

fn align8(n: usize) -> usize {
    (n + 7) & !7
}

#[cfg(unix)]
fn attr_bytes(meta: &std::fs::Metadata, out: &mut [u8; 88]) {
    use std::os::unix::fs::MetadataExt;
    put_u64(out, 0, meta.ino());
    put_u64(out, 8, meta.size());
    put_u64(out, 16, meta.blocks());
    put_u64(out, 24, meta.atime() as u64);
    put_u64(out, 32, meta.mtime() as u64);
    put_u64(out, 40, meta.ctime() as u64);
    put_u32(out, 48, meta.atime_nsec() as u32);
    put_u32(out, 52, meta.mtime_nsec() as u32);
    put_u32(out, 56, meta.ctime_nsec() as u32);
    put_u32(out, 60, meta.mode());
    put_u32(out, 64, meta.nlink() as u32);
    put_u32(out, 68, meta.uid());
    put_u32(out, 72, meta.gid());
    put_u32(out, 76, meta.rdev() as u32);
    put_u32(out, 80, meta.blksize() as u32);
    put_u32(out, 84, 0);
}

#[cfg(not(unix))]
fn attr_bytes(meta: &std::fs::Metadata, out: &mut [u8; 88]) {
    put_u64(out, 8, meta.len());
    if meta.is_dir() {
        put_u32(out, 60, 0o040755);
    } else {
        put_u32(out, 60, 0o100644);
    }
    put_u32(out, 64, 1);
}

/// The LOOKUP/regular reply: entry timeouts of zero (no caching: every
/// answer is fresh, which is the coherent half of serving a tree the
/// store never mutates mid-run) and the file's own attributes.
fn entry_out(id: u64, meta: &std::fs::Metadata) -> Result<Vec<u8>, sys::Errno> {
    let mut out = vec![0u8; 128];
    put_u64(&mut out, 0, id);
    put_u64(&mut out, 8, 0);
    put_u64(&mut out, 16, 0);
    put_u64(&mut out, 24, 0);
    put_u32(&mut out, 32, 0);
    put_u32(&mut out, 36, 0);
    let mut attr = [0u8; 88];
    attr_bytes(meta, &mut attr);
    out[40..128].copy_from_slice(&attr);
    Ok(out)
}

/// The GETATTR reply: the same attributes under their own header.
fn attr_out(meta: &std::fs::Metadata) -> Result<Vec<u8>, sys::Errno> {
    let mut out = vec![0u8; 104];
    put_u64(&mut out, 0, 0);
    put_u32(&mut out, 8, 0);
    put_u32(&mut out, 12, 0);
    let mut attr = [0u8; 88];
    attr_bytes(meta, &mut attr);
    out[16..104].copy_from_slice(&attr);
    Ok(out)
}

/// The INIT reply: the version offered, nothing capable, a 4 KiB largest
/// write (writes are refused anyway) and nanosecond timestamps.
fn init_out() -> Vec<u8> {
    let mut out = vec![0u8; 64];
    put_u32(&mut out, 0, ABI_MAJOR);
    put_u32(&mut out, 4, ABI_MINOR);
    put_u32(&mut out, 8, 0);
    put_u32(&mut out, 12, 0);
    put_u32(&mut out, 20, 4096);
    put_u32(&mut out, 24, 1);
    out
}

/// The STATFS reply: zeros with a 4 KiB block and a 255-name limit.
/// A usage figure would be invented (the backing is the store's own
/// filesystem, and its blocks are not this mount's), so this answers
/// the shape and no number.
fn statfs_out() -> Vec<u8> {
    let mut out = vec![0u8; 96];
    put_u32(&mut out, 32, 4096);
    put_u32(&mut out, 36, 255);
    put_u32(&mut out, 40, 4096);
    out
}

/// A reply frame: the length, the negated errno (0 where it worked),
/// the request's unique, and the payload.
fn reply(unique: u64, error: i32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + payload.len());
    out.extend_from_slice(&(16 + payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&error.to_le_bytes());
    out.extend_from_slice(&unique.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

fn ok(unique: u64, payload: &[u8]) -> Vec<u8> {
    reply(unique, 0, payload)
}

fn err(unique: u64, e: sys::Errno) -> Vec<u8> {
    reply(unique, -(e.0), &[])
}

/// Dispatch one request: the reply, or `None` where the kernel waits
/// for none (`FORGET`, `BATCH_FORGET`, `INTERRUPT`). The second half is
/// whether the loop exits after writing (`DESTROY`).
fn dispatch(fs: &mut Fs, req: &[u8]) -> (Option<Vec<u8>>, bool) {
    if req.len() < 40 {
        return (None, false);
    }
    let len = get_u32(req, 0) as usize;
    if len > req.len() || len < 40 {
        return (None, false);
    }
    let opcode = get_u32(req, 4);
    let unique = get_u64(req, 8);
    let nodeid = get_u64(req, 16);
    let body = &req[40..len];
    let answered = |r: Result<Vec<u8>, sys::Errno>| {
        Some(match r {
            Ok(payload) => ok(unique, &payload),
            Err(e) => err(unique, e),
        })
    };
    match opcode {
        OP_INIT => (Some(ok(unique, &init_out())), false),
        OP_LOOKUP => (answered(fs.lookup(nodeid, cstr(body))), false),
        OP_GETATTR => (answered(fs.getattr(nodeid)), false),
        OP_READLINK => (answered(fs.readlink(nodeid)), false),
        OP_OPEN => {
            let dir = false;
            (answered(fs.open(nodeid, dir)), false)
        }
        OP_OPENDIR => (answered(fs.open(nodeid, true)), false),
        OP_READ | OP_READDIR => {
            if body.len() < 40 {
                return (Some(err(unique, errno(22))), false);
            }
            let arg_offset = get_u64(body, 8);
            let arg_size = get_u32(body, 16) as u64;
            if opcode == OP_READ {
                (answered(fs.read(nodeid, arg_offset, arg_size)), false)
            } else {
                (answered(fs.readdir(nodeid, arg_offset, arg_size)), false)
            }
        }
        OP_RELEASE | OP_RELEASEDIR => (Some(ok(unique, &[])), false),
        OP_ACCESS => {
            let mask = body.first().copied().unwrap_or(0) as u32
                | ((body.get(1).copied().unwrap_or(0) as u32) << 8)
                | ((body.get(2).copied().unwrap_or(0) as u32) << 16)
                | ((body.get(3).copied().unwrap_or(0) as u32) << 24);
            match fs.access(nodeid, mask) {
                Ok(()) => (Some(ok(unique, &[])), false),
                Err(e) => (Some(err(unique, e)), false),
            }
        }
        OP_STATFS => (Some(ok(unique, &statfs_out())), false),
        OP_FORGET | OP_BATCH_FORGET => {
            forget_in(fs, opcode, nodeid, body);
            (None, false)
        }
        OP_INTERRUPT => (None, false),
        OP_DESTROY => (Some(ok(unique, &[])), true),
        _ if OP_MUTATING.contains(&opcode) => (Some(err(unique, errno(30))), false),
        _ => (Some(err(unique, errno(38))), false),
    }
}

/// Apply a forget body: `FORGET` names its node in the header and
/// carries the count in the body; `BATCH_FORGET` carries id/count pairs.
fn forget_in(fs: &mut Fs, opcode: u32, nodeid: u64, body: &[u8]) {
    if opcode == OP_FORGET {
        if body.len() >= 8 {
            fs.forget(nodeid, get_u64(body, 0));
        }
        return;
    }
    if body.len() < 8 {
        return;
    }
    let count = get_u32(body, 0) as usize;
    let mut at = 8usize;
    for _ in 0..count {
        if body.len() < at + 16 {
            return;
        }
        fs.forget(get_u64(body, at), get_u64(body, at + 8));
        at += 16;
    }
}

/// The NUL-terminated name a LOOKUP carries, or an empty span where none
/// is there (which the lookup refuses as `ENOENT`).
fn cstr(body: &[u8]) -> &[u8] {
    match body.iter().position(|b| *b == 0) {
        Some(n) => &body[..n],
        None => &[],
    }
}

/// Serve on `fd` until `DESTROY`: the child's whole job after the mount.
///
/// `ready` carries the serving word once INIT is answered: the parent
/// stats the mount to trigger it, so the first INIT arrives while this
/// loop runs. Every reply is written whole; a write failure ends the
/// server with the payload's fate decided by the kernel (transport
/// errors on a dead mount read as `ENOTCONN`, never as payload bytes).
fn serve_until_destroy(fd: i64, backing: &Path, ready: i64) -> i32 {
    let mut fs = Fs::new(backing);
    let mut buf = vec![0u8; IO_BUFFER];
    // ⚠ A stream may coalesce or split frames (a socket does; the node
    // hands one request per read). `pending` holds what arrived past
    // the frame in hand, so no byte is parsed twice or dropped either
    // way. A length past the buffer is garbage, never an allocation.
    let mut pending: Vec<u8> = Vec::new();
    let mut told = false;
    loop {
        while pending.len() < 40 {
            match sys::read(fd, &mut buf) {
                Ok(0) => return 0,
                Ok(n) => pending.extend_from_slice(&buf[..n as usize]),
                Err(e) if e == sys::EINTR => continue,
                Err(_) => return 1,
            }
        }
        let len = get_u32(&pending, 0) as usize;
        if !(40..=IO_BUFFER).contains(&len) {
            return 1;
        }
        while pending.len() < len {
            match sys::read(fd, &mut buf) {
                Ok(0) => return 1,
                Ok(n) => pending.extend_from_slice(&buf[..n as usize]),
                Err(e) if e == sys::EINTR => continue,
                Err(_) => return 1,
            }
        }
        let frame: Vec<u8> = pending[..len].to_vec();
        pending.drain(..len);
        let (answer, done) = dispatch(&mut fs, &frame);
        if !told {
            // The first request the kernel sends on a fresh mount is
            // INIT: its answer is the serving word the parent waits
            // for. Anything else first is still answered; the word
            // follows the first reply regardless.
            let _ = write_all(ready, &0u32.to_le_bytes());
            told = true;
        }
        if let Some(bytes) = answer {
            if write_all(fd, &bytes).is_err() {
                return 1;
            }
        }
        if done {
            return 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("podbox-fuse-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("back/bin")).unwrap();
        d
    }

    /// The gate opens where the node opens: a regular file stands in
    /// for the node, and the probe-gated staging tests below assert the
    /// refusal arm with the absent real one.
    #[test]
    fn the_gate_opens_where_the_node_opens() {
        let d = tree("gate");
        std::fs::write(d.join("node"), b"x").unwrap();
        assert!(check_node_at(d.join("node").to_str().unwrap()).is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// The gate refuses naming the node where nothing opens: the absent
    /// path reads as the absent `/dev/fuse` does.
    #[test]
    fn the_gate_refuses_naming_the_node_where_nothing_opens() {
        let d = tree("gate-missing");
        let missing = d.join("no-such-node");
        let e = check_node_at(missing.to_str().unwrap()).unwrap_err();
        assert!(e.why.contains("/dev/fuse did not open"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A backing that is not a directory refuses before the store is
    /// touched: no rung serves what nothing resolved, and no rung
    /// litters the store refusing it.
    #[test]
    fn a_missing_backing_refuses_before_touching_the_store() {
        let d = tree("serve-missing");
        let store = d.join("store");
        let e = serve_tree(&store, &d.join("nope")).unwrap_err();
        assert!(e.why.contains("not a directory"), "{e}");
        assert!(!store.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Where the node is absent the serve refuses naming it and leaves
    /// no mount directory: this lane's live arm of the same refusal.
    #[test]
    fn a_missing_node_refuses_naming_it_and_leaves_nothing() {
        let d = tree("serve-node");
        std::fs::write(d.join("back/bin/prog"), b"\x7fELF").unwrap();
        if check_node().is_ok() {
            let _ = std::fs::remove_dir_all(&d);
            return;
        }
        let store = d.join("store");
        let e = serve_tree(&store, &d.join("back")).unwrap_err();
        assert!(e.why.contains("/dev/fuse did not open"), "{e}");
        let staged: Vec<_> = std::fs::read_dir(store.join("fuse"))
            .map(|rd| rd.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();
        assert!(staged.is_empty(), "a refused serve left staging behind");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Releasing a directory that was never mounted removes it: the
    /// removal half of the release does not depend on the unmount half.
    #[test]
    fn releasing_an_unmounted_directory_removes_it() {
        let d = tree("release");
        let dir = d.join("mnt");
        std::fs::create_dir_all(&dir).unwrap();
        release_mount(&ActiveMount {
            pid: 1,
            dir: dir.clone(),
        });
        assert!(!dir.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    fn fixture_tagged(tag: &str) -> (PathBuf, Fs) {
        let d = tree(tag);
        std::fs::write(d.join("back/bin/prog"), b"\x7fELF-payload").unwrap();
        std::fs::write(d.join("back/note.txt"), b"hi").unwrap();
        std::os::unix::fs::symlink("prog", d.join("back/bin/sh")).unwrap();
        let fs = Fs::new(&d.join("back"));
        (d, fs)
    }

    /// A lookup finds the file, keeps its id while looked up, and
    /// refuses what is not there, what carries a slash, and what is
    /// empty: the kernel never sends those, and anything else did not
    /// come from the kernel.
    #[test]
    fn a_lookup_finds_and_refuses() {
        let (d, mut fs) = fixture_tagged("a_lookup_finds_and_refuses");
        let bin = fs.lookup(1, b"bin").unwrap();
        let bin_id = get_u64(&bin, 0);
        assert!(bin_id != 1);
        let prog = fs.lookup(bin_id, b"prog").unwrap();
        let prog_id = get_u64(&prog, 0);
        // A second lookup of the same path keeps its id.
        let again = fs.lookup(bin_id, b"prog").unwrap();
        assert_eq!(get_u64(&again, 0), prog_id);
        assert!(fs.lookup(bin_id, b"absent").is_err());
        assert!(fs.lookup(bin_id, b"a/b").is_err());
        assert!(fs.lookup(bin_id, b"").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// `..` answers the parent without touching the filesystem: no
    /// lookup climbs past the backing root.
    #[test]
    fn a_dotdot_answers_the_parent() {
        let (d, mut fs) = fixture_tagged("a_dotdot_answers_the_parent");
        let bin = fs.lookup(1, b"bin").unwrap();
        let bin_id = get_u64(&bin, 0);
        let up = fs.lookup(bin_id, b"..").unwrap();
        assert_eq!(get_u64(&up, 0), 1);
        let top = fs.lookup(1, b"..").unwrap();
        assert_eq!(get_u64(&top, 0), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A forget drops the node once its lookups stand at zero; the root
    /// never drops.
    #[test]
    fn a_forget_drops_at_zero_and_never_the_root() {
        let (d, mut fs) = fixture_tagged("a_forget_drops_at_zero_and_never_the_root");
        let prog = fs.lookup(1, b"note.txt").unwrap();
        let id = get_u64(&prog, 0);
        fs.lookup(1, b"note.txt").unwrap();
        fs.forget(id, 1);
        assert!(fs.entry_bytes(id).is_some());
        fs.forget(id, 1);
        assert!(fs.entry_bytes(id).is_none());
        fs.forget(1, 99);
        assert!(fs.entry_bytes(1).is_some());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Attributes carry the file's own mode word: a directory reads as
    /// one and a file as one.
    #[test]
    fn attributes_carry_the_mode_word() {
        let (d, mut fs) = fixture_tagged("attributes_carry_the_mode_word");
        let bin = fs.lookup(1, b"bin").unwrap();
        let attr = fs.getattr(get_u64(&bin, 0)).unwrap();
        let mode = get_u32(&attr, 16 + 60);
        assert_eq!(mode & 0o170000, 0o040000, "a directory reads as one");
        let prog = fs.lookup(get_u64(&bin, 0), b"prog").unwrap();
        let attr = fs.getattr(get_u64(&prog, 0)).unwrap();
        let mode = get_u32(&attr, 16 + 60);
        assert_eq!(mode & 0o170000, 0o100000, "a file reads as one");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A link reads as its target; reading a file as a link refuses.
    #[test]
    fn a_link_reads_as_its_target() {
        let (d, mut fs) = fixture_tagged("a_link_reads_as_its_target");
        let bin = get_u64(&fs.lookup(1, b"bin").unwrap(), 0);
        let sh = get_u64(&fs.lookup(bin, b"sh").unwrap(), 0);
        assert_eq!(fs.readlink(sh).unwrap(), b"prog");
        let prog = get_u64(&fs.lookup(bin, b"prog").unwrap(), 0);
        assert_eq!(fs.readlink(prog).unwrap_err(), errno(22));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Bytes are the file's, whole and from the offset: the payload
    /// check the drive asserts.
    #[test]
    fn the_bytes_are_the_files_whole_and_from_the_offset() {
        let (d, mut fs) = fixture_tagged("the_bytes_are_the_files_whole_and_from_the_offset");
        let bin = get_u64(&fs.lookup(1, b"bin").unwrap(), 0);
        let prog = get_u64(&fs.lookup(bin, b"prog").unwrap(), 0);
        assert_eq!(fs.read(prog, 0, 64).unwrap(), b"\x7fELF-payload");
        assert_eq!(fs.read(prog, 5, 7).unwrap(), b"payload");
        assert!(fs.read(prog, 99, 64).unwrap().is_empty());
        assert_eq!(fs.read(1, 0, 64).unwrap_err(), errno(21));
        assert_eq!(fs.read(prog, 0, MAX_READ + 1).unwrap_err(), errno(22));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Only files and directories open: a link, which the kernel
    /// resolves itself, never opens here.
    #[test]
    fn only_files_and_directories_open() {
        let (d, mut fs) = fixture_tagged("only_files_and_directories_open");
        let bin = get_u64(&fs.lookup(1, b"bin").unwrap(), 0);
        assert!(fs.open(bin, true).is_ok());
        let prog = get_u64(&fs.lookup(bin, b"prog").unwrap(), 0);
        assert!(fs.open(prog, false).is_ok());
        assert_eq!(fs.open(prog, true).unwrap_err(), errno(13));
        let sh = get_u64(&fs.lookup(bin, b"sh").unwrap(), 0);
        assert_eq!(fs.open(sh, false).unwrap_err(), errno(13));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A listing carries `.`, `..` and the children, resumes past a
    /// cookie, and ends: the traversal the entry walks.
    #[test]
    fn a_listing_carries_dot_dotdot_children_and_an_end() {
        let (d, mut fs) = fixture_tagged("a_listing_carries_dot_dotdot_children_and_an_end");
        let all = fs.readdir(1, 0, 4096).unwrap();
        let names = dirent_names(&all);
        assert!(names.contains(&b".".to_vec()));
        assert!(names.contains(&b"..".to_vec()));
        assert!(names.contains(&b"bin".to_vec()));
        assert!(names.contains(&b"note.txt".to_vec()));
        // Resume past the first two cookies: `.` and `..` are gone.
        let rest = fs.readdir(1, 2, 4096).unwrap();
        let names = dirent_names(&rest);
        assert!(!names.contains(&b".".to_vec()));
        assert!(names.contains(&b"bin".to_vec()));
        // Past the end: empty, which is the kernel's stop.
        assert!(fs.readdir(1, 99, 4096).unwrap().is_empty());
        // A file is not a listing.
        let bin = get_u64(&fs.lookup(1, b"bin").unwrap(), 0);
        let prog = get_u64(&fs.lookup(bin, b"prog").unwrap(), 0);
        assert_eq!(fs.readdir(prog, 0, 4096).unwrap_err(), errno(20));
        let _ = std::fs::remove_dir_all(&d);
    }

    fn dirent_names(buf: &[u8]) -> Vec<Vec<u8>> {
        let mut names = Vec::new();
        let mut at = 0usize;
        while at + 24 <= buf.len() {
            let namelen = get_u32(buf, at + 16) as usize;
            let len = align8(24 + namelen);
            if at + len > buf.len() || namelen > len {
                break;
            }
            names.push(buf[at + 24..at + 24 + namelen].to_vec());
            at += len;
        }
        names
    }

    /// A write is `EROFS` and an unknown word is `ENOSYS`: the
    /// read-only rung answers, never implements.
    #[test]
    fn a_write_is_rofs_and_an_unknown_word_is_nosys() {
        let (d, mut fs) = fixture_tagged("a_write_is_rofs_and_an_unknown_word_is_nosys");
        let prog = get_u64(&fs.lookup(1, b"note.txt").unwrap(), 0);
        let write_req = frame(16, 7, prog, &[]);
        let (answer, done) = dispatch(&mut fs, &write_req);
        assert_eq!(get_u32(&answer.unwrap(), 4) as i32, -30);
        assert!(!done);
        let bogus = frame(200, 8, prog, &[]);
        let (answer, _) = dispatch(&mut fs, &bogus);
        assert_eq!(get_u32(&answer.unwrap(), 4) as i32, -38);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A FORGET names its node in the header and carries the count in
    /// the body: one forget of two lookups keeps the node, the second
    /// drops it. The header-id half is what the first cut got wrong
    /// (it read both out of the body).
    #[test]
    fn a_forget_names_its_node_in_the_header() {
        let (d, mut fs) = fixture_tagged("a_forget_names_its_node_in_the_header");
        let prog = get_u64(&fs.lookup(1, b"note.txt").unwrap(), 0);
        fs.lookup(1, b"note.txt").unwrap();
        let mut body = vec![0u8; 8];
        put_u64(&mut body, 0, 1);
        let forget = frame(2, 11, prog, &body);
        let (answer, done) = dispatch(&mut fs, &forget);
        assert!(answer.is_none());
        assert!(!done);
        assert!(fs.entry_bytes(prog).is_some(), "one of two lookups stands");
        let (answer, _) = dispatch(&mut fs, &forget);
        assert!(answer.is_none());
        assert!(fs.entry_bytes(prog).is_none(), "the second drops it");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Forgets and interrupts answer nothing: the kernel waits for
    /// none of them.
    #[test]
    fn forgets_and_interrupts_answer_nothing() {
        let (d, mut fs) = fixture_tagged("forgets_and_interrupts_answer_nothing");
        let prog = get_u64(&fs.lookup(1, b"note.txt").unwrap(), 0);
        let mut body = vec![0u8; 8];
        put_u64(&mut body, 0, 1);
        let forget = frame(2, 7, prog, &body);
        assert!(dispatch(&mut fs, &forget).0.is_none());
        let interrupt = frame(36, 9, 0, &[]);
        assert!(dispatch(&mut fs, &interrupt).0.is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// INIT answers the offered version; DESTROY answers and exits.
    #[test]
    fn init_answers_the_version_and_destroy_exits() {
        let (d, mut fs) = fixture_tagged("init_answers_the_version_and_destroy_exits");
        let mut body = vec![0u8; 64];
        put_u32(&mut body, 0, 7);
        put_u32(&mut body, 4, 38);
        let init = frame(26, 3, 0, &body);
        let (answer, done) = dispatch(&mut fs, &init);
        let answer = answer.unwrap();
        assert!(!done);
        assert_eq!(get_u32(&answer, 16), 7);
        let destroy = frame(38, 5, 0, &[]);
        let (answer, done) = dispatch(&mut fs, &destroy);
        assert!(answer.is_some());
        assert!(done);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A reply is a 16-byte header ahead of the payload: the length
    /// carries both, the error is zero, and the unique is echoed. The
    /// socket drive proved this end to end; this pins it at dispatch.
    #[test]
    fn a_reply_is_a_header_ahead_of_the_payload() {
        let (d, mut fs) = fixture_tagged("a_reply_is_a_header_ahead_of_the_payload");
        let mut name = b"note.txt".to_vec();
        name.push(0);
        let (answer, done) = dispatch(&mut fs, &frame(1, 41, 1, &name));
        let answer = answer.unwrap();
        assert!(!done);
        assert_eq!(answer.len(), 16 + 128, "header plus entry");
        assert_eq!(get_u32(&answer, 0) as usize, answer.len());
        assert_eq!(get_u32(&answer, 4), 0);
        assert_eq!(get_u64(&answer, 8), 41);
        assert_eq!(get_u64(&answer, 16), 2, "the payload follows the header");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Garbage answers nothing and exits nothing: a short frame is
    /// dropped rather than parsed.
    #[test]
    fn garbage_answers_nothing() {
        let (d, mut fs) = fixture_tagged("garbage_answers_nothing");
        assert_eq!(dispatch(&mut fs, &[1, 2, 3]).0, None);
        let _ = std::fs::remove_dir_all(&d);
    }

    fn frame(opcode: u32, unique: u64, nodeid: u64, body: &[u8]) -> Vec<u8> {
        let mut req = vec![0u8; 40 + body.len()];
        put_u32(&mut req, 0, 40 + body.len() as u32);
        put_u32(&mut req, 4, opcode);
        put_u64(&mut req, 8, unique);
        put_u64(&mut req, 16, nodeid);
        req[40..].copy_from_slice(body);
        req
    }

    /// The loop serves framed requests off a socket pair: INIT, then a
    /// LOOKUP, a GETATTR and a READ whose bytes are the file's, then
    /// DESTROY. The socket stands in for the node; the framing is what
    /// is proved. Single-threaded: every frame is written up front (the
    /// socket buffers hold them all), then the loop runs inline to
    /// DESTROY, then the replies are read back. No thread: the T-0603
    /// rule holds in tests too.
    ///
    /// ⚠ The LOOKUP id is written as 2, the first allocation after the
    /// root: this test performs no earlier lookup, so the counter
    /// stands there. The reply asserts it back.
    #[test]
    fn the_loop_serves_off_a_socket_pair() {
        use std::io::Write;
        use std::os::unix::net::UnixStream;
        let (d, _) = fixture_tagged("the_loop_serves_off_a_socket_pair");
        let backing = d.join("back");
        let (mut driver, node) = UnixStream::pair().unwrap();
        let raw = {
            use std::os::unix::io::IntoRawFd;
            node.into_raw_fd() as i64
        };
        let mut body = vec![0u8; 64];
        put_u32(&mut body, 0, 7);
        put_u32(&mut body, 4, 38);
        driver.write_all(&frame(26, 1, 0, &body)).unwrap();
        let mut name = b"note.txt".to_vec();
        name.push(0);
        driver.write_all(&frame(1, 2, 1, &name)).unwrap();
        driver.write_all(&frame(3, 3, 2, &[])).unwrap();
        let mut read_body = vec![0u8; 40];
        put_u64(&mut read_body, 0, 77);
        put_u64(&mut read_body, 8, 0);
        put_u32(&mut read_body, 16, 16);
        driver.write_all(&frame(15, 4, 2, &read_body)).unwrap();
        driver.write_all(&frame(38, 5, 0, &[])).unwrap();
        assert_eq!(serve_until_destroy(raw, &backing, -1), 0);
        let init = read_frame(&mut driver, 1);
        assert_eq!(get_u32(&init, 16), 7, "INIT answers the version");
        let found = read_frame(&mut driver, 2);
        assert_eq!(get_u64(&found, 16), 2, "LOOKUP answers id 2");
        let attr = read_frame(&mut driver, 3);
        // 16 header, 16 attr-out head, 60 into the attributes.
        assert_eq!(get_u32(&attr, 16 + 16 + 60) & 0o170000, 0o100000);
        let data = read_frame(&mut driver, 4);
        assert_eq!(&data[16..], b"hi");
        let _ = read_frame(&mut driver, 5);
        let _ = std::fs::remove_dir_all(&d);
    }

    fn read_frame(driver: &mut std::os::unix::net::UnixStream, n: u32) -> Vec<u8> {
        use std::io::Read;
        let mut head = [0u8; 16];
        let mut got = 0usize;
        while got < 16 {
            let n = driver.read(&mut head[got..]).unwrap();
            assert!(n > 0, "the server closed mid-frame");
            got += n;
        }
        let len = u32::from_le_bytes([head[0], head[1], head[2], head[3]]) as usize;
        assert!(
            (16..=1 << 20).contains(&len),
            "frame {n}: a sane frame length, got {len} in {head:?}"
        );
        let mut rest = vec![0u8; len - 16];
        let mut got = 0usize;
        while got < rest.len() {
            let n = driver.read(&mut rest[got..]).unwrap();
            assert!(n > 0, "the server closed mid-frame");
            got += n;
        }
        [head.to_vec(), rest].concat()
    }
}
