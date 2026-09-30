// Package rootfs runs a command inside an unpacked root filesystem, in a
// private mount namespace, with nothing of the host's userland visible.
//
// The target distribution's /lib, /usr/lib, /etc and its loader are the only
// ones the process can see. Nothing of the host is mounted in except /proc,
// /sys, /dev and a resolv.conf - each a kernel or network interface rather
// than a userland one. A test that needs an artefact inside passes Copy, so
// the copy is visibly part of the experiment.
//
// This is not a security boundary and not a container: the PID, network, user
// and IPC namespaces are shared with the host unless PrivateNet is set, and
// the kernel is the host kernel.
//
// SPDX-License-Identifier: 0BSD
package rootfs

import (
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"syscall"

	"github.com/Azathothas/pg-toolkit/internal/cfg"
	"github.com/Azathothas/pg-toolkit/internal/fail"
	"github.com/Azathothas/pg-toolkit/internal/logx"
	"github.com/Azathothas/pg-toolkit/internal/proc"
)

var log = logx.New("rootfs")

// Options describes one run inside a root filesystem.
type Options struct {
	Root       string   // the unpacked filesystem
	Copy       []string // SRC[:DEST], copied in before the namespace is entered
	Bind       []string // SRC[:DEST], bind-mounted inside
	Workdir    string   // "" or "/" means the root
	NoNet      bool     // do not replicate resolv.conf or the TLS anchor
	PrivateNet bool     // also unshare the network namespace

	// MaskDev names entries under /dev that must be ABSENT inside.
	//
	// ⛔ Absent, not unopenable. The default bed bind-mounts the host's /dev
	// recursively, so every node the host has is there, and no mount can
	// remove one from a bind: over-mounting a device with another file leaves
	// something that still opens. So a non-empty MaskDev replaces /dev with a
	// fresh tmpfs carrying the documented node set below MINUS these names,
	// which is what a real container's /dev looks like anyway.
	//
	// ⚠ Opt-in, and the default is untouched: every committed measurement was
	// taken against the recursive bind and stays valid.
	MaskDev []string

	// TmpSize is the tmpfs `size=` option for /tmp, e.g. "64m". Empty leaves
	// the kernel default, which is half of RAM.
	TmpSize string
	// TmpNoExec mounts /tmp noexec.
	TmpNoExec bool
	// TmpReadOnly remounts /tmp read-only once it is mounted. ⚠ A tmpfs cannot
	// be mounted read-only in one step and be useful: it is created empty, so
	// anything it must contain is put there first and the remount follows.
	TmpReadOnly bool

	DNS    string // override the nameserver written to /etc/resolv.conf
	Stdin  io.Reader
	Stdout io.Writer
	Stderr io.Writer
	Env    []string // extra NAME=VALUE for the command
}

// innerCommand is the hidden re-entry that does the mounts. pgb re-execs
// itself with CLONE_NEWNS so the mount namespace belongs to the whole child
// process rather than to one thread of this one.
const innerCommand = "__rootfs-inner"

// InnerCommand is the subcommand name the dispatcher must route here.
func InnerCommand() string { return innerCommand }

// Run enters the root filesystem and runs argv, returning the command's own
// exit status.
func Run(o Options, argv []string) (int, error) {
	if o.Root == "" {
		return 2, fail.Cannot("rootfs: no root filesystem given")
	}
	if len(argv) == 0 {
		return 2, fail.Cannot("rootfs: no command given")
	}
	root, err := filepath.Abs(o.Root)
	if err != nil {
		return 2, fail.Cannot("rootfs: cannot resolve %s: %v", o.Root, err)
	}
	if fi, err := os.Stat(root); err != nil || !fi.IsDir() {
		return 2, fail.Cannot("rootfs: %s is not a directory", o.Root)
	}
	// ⛔ The route is probed, not read from the uid (Probe). A user namespace
	// is what a non-root caller gets, and it is not a weaker version of the
	// same thing: without CLONE_NEWUSER the kernel refuses CLONE_NEWNS
	// outright, so an unprivileged caller could not enter a root filesystem at
	// all and every experiment needing one was root-only.
	//
	// ⚠ Root keeps the plain mount namespace wherever it may mount there.
	// Every committed measurement was taken as root on that route, and a user
	// namespace remaps ownership inside the bed and changes what the runs see.
	route := Entry()
	if route == RouteNone {
		if os.Geteuid() == 0 {
			return 2, fail.Cannot("rootfs: root, and a mount or a chroot is refused both in a new mount namespace and in a user namespace here (a seccomp profile, or no CAP_SYS_ADMIN and no user namespaces)")
		}
		return 2, fail.Cannot("rootfs: not root, and this kernel refuses an unprivileged user namespace, or a mount or a chroot inside one (Ubuntu 24.04 refuses the mount by default)")
	}
	log.Debugf("entering through a %s", route)

	// Copies happen outside the namespace so they persist and can be inspected
	// after the run: an experiment's inputs should still be on disk afterwards.
	for _, spec := range o.Copy {
		if err := copyInto(root, spec); err != nil {
			return 2, err
		}
	}
	if !o.NoNet {
		if err := replicateNetwork(root, o.DNS); err != nil {
			return 2, err
		}
	}

	self, err := os.Executable()
	if err != nil {
		return 2, fail.Cannot("rootfs: cannot locate the running pgb: %v", err)
	}

	inner := append([]string{self, innerCommand,
		"--root", root,
		"--workdir", defaultWorkdir(o.Workdir),
	}, bindArgs(o.Bind)...)
	for _, d := range o.MaskDev {
		if d != "" {
			inner = append(inner, "--mask-dev", d)
		}
	}
	if o.TmpSize != "" {
		inner = append(inner, "--tmp-size", o.TmpSize)
	}
	if o.TmpNoExec {
		inner = append(inner, "--tmp-noexec")
	}
	if o.TmpReadOnly {
		inner = append(inner, "--tmp-readonly")
	}
	inner = append(inner, "--")
	inner = append(inner, argv...)

	log.Debugf("entering %s: %s", root, logx.QuoteArgs(argv))

	cmd := exec.Command(inner[0], inner[1:]...)
	cmd.Stdin, cmd.Stdout, cmd.Stderr = o.Stdin, o.Stdout, o.Stderr
	if cmd.Stdout == nil {
		cmd.Stdout = os.Stdout
	}
	if cmd.Stderr == nil {
		cmd.Stderr = os.Stderr
	}
	if len(o.Env) > 0 {
		cmd.Env = append(os.Environ(), o.Env...)
	}
	var extra uintptr
	if o.PrivateNet {
		extra = syscall.CLONE_NEWNET
	}
	cmd.SysProcAttr = route.attr(extra)

	if err := cmd.Run(); err != nil {
		if ee, ok := err.(*exec.ExitError); ok {
			if ws, ok := ee.Sys().(syscall.WaitStatus); ok && ws.Signaled() {
				return 128 + int(ws.Signal()), nil
			}
			return ee.ExitCode(), nil
		}
		return 2, fail.Cannot("rootfs: cannot start the runner: %v", err)
	}
	return 0, nil
}

func bindArgs(binds []string) []string {
	var out []string
	for _, b := range binds {
		if b != "" {
			out = append(out, "--bind", b)
		}
	}
	return out
}

func defaultWorkdir(w string) string {
	if w == "" {
		return "/"
	}
	return w
}

// Inner performs the mounts and the chroot. It runs in the child process, in
// its own mount namespace, and never returns on success.
func Inner(args []string) error {
	var root, workdir, tmpSize string
	var binds, maskDev []string
	var argv []string
	var tmpNoExec, tmpReadOnly bool
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--root":
			i++
			if i < len(args) {
				root = args[i]
			}
		case "--workdir":
			i++
			if i < len(args) {
				workdir = args[i]
			}
		case "--bind":
			i++
			if i < len(args) {
				binds = append(binds, args[i])
			}
		case "--mask-dev":
			i++
			if i < len(args) {
				maskDev = append(maskDev, args[i])
			}
		case "--tmp-size":
			i++
			if i < len(args) {
				tmpSize = args[i]
			}
		case "--tmp-noexec":
			tmpNoExec = true
		case "--tmp-readonly":
			tmpReadOnly = true
		case "--":
			argv = args[i+1:]
			i = len(args)
		}
	}
	if root == "" || len(argv) == 0 {
		return fail.Cannot("%s: needs --root and a command", innerCommand)
	}

	for _, d := range []string{"proc", "sys", "dev", "tmp"} {
		_ = os.MkdirAll(filepath.Join(root, d), 0o755)
	}
	mountProc(filepath.Join(root, "proc"), root)
	mountQuiet("none", filepath.Join(root, "sys"), "sysfs", 0, "")
	if len(maskDev) > 0 {
		if err := minimalDev(filepath.Join(root, "dev"), maskDev); err != nil {
			return err
		}
	} else {
		mountQuiet("/dev", filepath.Join(root, "dev"), "", syscall.MS_BIND|syscall.MS_REC, "")
		mountQuiet("none", filepath.Join(root, "dev"), "", syscall.MS_SLAVE|syscall.MS_REC, "")
	}
	mountTmp(filepath.Join(root, "tmp"), tmpSize, tmpNoExec)

	for _, spec := range binds {
		src, dst, ok := strings.Cut(spec, ":")
		if !ok {
			dst = src
		}
		target := filepath.Join(root, dst)
		if fi, err := os.Stat(src); err == nil && !fi.IsDir() {
			_ = os.MkdirAll(filepath.Dir(target), 0o755)
			if _, err := os.Stat(target); err != nil {
				_ = os.WriteFile(target, nil, 0o644)
			}
		} else {
			_ = os.MkdirAll(target, 0o755)
		}
		if err := syscall.Mount(src, target, "", syscall.MS_BIND|syscall.MS_REC, ""); err != nil {
			return fail.Cannot("cannot bind %s onto %s: %v", src, target, err)
		}
	}

	// ⚠ After the binds, deliberately. A tmpfs is created empty, so remounting
	// it read-only before a caller's bind lands would give an environment
	// nothing can be put into rather than one nothing can be written to.
	if tmpReadOnly {
		if err := remountReadOnly(filepath.Join(root, "tmp"), tmpNoExec); err != nil {
			return err
		}
	}

	if err := syscall.Chroot(root); err != nil {
		return fail.Cannot("chroot %s: %v", root, err)
	}
	// The working directory must be set after the chroot, and a directory that
	// does not exist inside is the caller's error rather than the runner's.
	if err := os.Chdir(defaultWorkdir(workdir)); err != nil {
		return fail.Cannot("cd %s inside the root filesystem: %v", workdir, err)
	}

	// The command is exec'd directly rather than through the target's /bin/sh,
	// because a root filesystem under test may not have one - a distroless
	// image does not, and neither does a fixture built to hold exactly one
	// binary.
	path := argv[0]
	if !strings.ContainsRune(path, os.PathSeparator) {
		if p, err := exec.LookPath(path); err == nil {
			path = p
		}
	}
	if err := syscall.Exec(path, argv, os.Environ()); err != nil {
		// The status follows the convention `chroot` and `env` use, because a
		// caller reads it: 127 is "not found", which is also what the kernel
		// reports for a binary whose INTERPRETER is missing, and 126 is
		// "found and not executable". Collapsing both into the runner's own
		// "could not run" would hide the difference between a missing loader
		// and a broken runner.
		fmt.Fprintf(os.Stderr, "pgb: cannot run %s inside %s: %v\n", argv[0], root, err)
		if errors.Is(err, syscall.EACCES) || errors.Is(err, syscall.EPERM) ||
			errors.Is(err, syscall.ENOEXEC) {
			os.Exit(126)
		}
		os.Exit(127)
	}
	return nil
}

// mountProc gives the bed a /proc, and says so when it cannot.
//
// ⛔ A fresh `proc` mount needs a PID namespace the caller owns. Under root
// there is one and nothing here changes. In the USER namespace a non-root
// caller gets, the kernel refuses it with EPERM - and `mountQuiet` swallowed
// that, so the bed had no /proc at all and nothing said so.
//
// ⚠ What that costs is not an error either. `readlink /proc/self/exe` comes
// back empty, the command still exits 0, and any subject that asks where it is
// fails on its own terms: an artefact carrying its own runtime reports
// "Failed to get self runtime exe path: no /proc/self/exe available", which
// reads as a broken artefact and is a missing mount. Measured on all eleven
// environments before this.
//
// ⭐ The fallback is a recursive BIND of the host's /proc, which a user
// namespace does permit. It shows the host's processes inside the bed, which
// the fresh mount does not - so the fallback is reported at debug level rather
// than passed off as the same thing, and an experiment that cares reads
// /proc/1/comm.
func mountProc(target, root string) {
	if err := syscall.Mount("none", target, "proc", 0, ""); err == nil {
		return
	}
	err := syscall.Mount("/proc", target, "", syscall.MS_BIND|syscall.MS_REC, "")
	if err == nil {
		log.Debugf("no private proc mount here; bound the host's /proc into %s instead", root)
		return
	}
	// ⛔ Not silent. Everything downstream that reads /proc will misreport.
	fmt.Fprintf(os.Stderr,
		"pgb: no /proc inside %s (%v); anything asking /proc/self/exe will fail there\n", root, err)
}

func mountQuiet(source, target, fstype string, flags uintptr, data string) {
	if err := syscall.Mount(source, target, fstype, flags, data); err != nil {
		log.Tracef("mount %s on %s: %v", source, target, err)
	}
}

// replicateNetwork copies the two things that are network interfaces rather
// than host userland: the resolver configuration, and the TLS trust anchor the
// caller's own environment already names.
func replicateNetwork(root, dns string) error {
	etc := filepath.Join(root, "etc")
	if err := os.MkdirAll(etc, 0o755); err != nil {
		return fail.Cannot("cannot create %s: %v", etc, err)
	}
	resolv := filepath.Join(etc, "resolv.conf")
	switch {
	case dns != "":
		if err := os.WriteFile(resolv, []byte("nameserver "+dns+"\n"), 0o644); err != nil {
			return fail.Cannot("cannot write %s: %v", resolv, err)
		}
	default:
		if fi, err := os.Stat(resolv); err == nil && fi.Size() > 0 {
			break
		}
		if b, err := os.ReadFile("/etc/resolv.conf"); err == nil && len(b) > 0 {
			_ = os.WriteFile(resolv, b, 0o644)
		}
	}

	// Only the file the caller's variables already name is copied, to the same
	// absolute path so the inherited variable keeps resolving. This adds no
	// trust the caller did not already have.
	if anchor := cfg.CAAnchor(); anchor != "" {
		dst := filepath.Join(root, anchor)
		if _, err := os.Stat(dst); err != nil {
			if err := os.MkdirAll(filepath.Dir(dst), 0o755); err == nil {
				if b, err := os.ReadFile(anchor); err == nil {
					_ = os.WriteFile(dst, b, 0o644)
				}
			}
		}
	}
	return nil
}

// copyInto handles a SRC[:DEST] copy specification.
func copyInto(root, spec string) error {
	src, dst, ok := strings.Cut(spec, ":")
	if !ok {
		dst = "/" + filepath.Base(src)
	}
	target := filepath.Join(root, dst)
	if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
		return fail.Cannot("cannot create %s in the root filesystem: %v", filepath.Dir(dst), err)
	}
	fi, err := os.Stat(src)
	if err != nil {
		return fail.Cannot("cannot copy %s: %v", src, err)
	}
	if fi.IsDir() {
		if r, err := proc.Run("cp", "-a", src, target); err != nil || r.Failed() {
			return fail.Cannot("cannot copy %s -> %s", src, dst)
		}
		return nil
	}
	in, err := os.Open(src)
	if err != nil {
		return fail.Cannot("cannot read %s: %v", src, err)
	}
	defer in.Close()
	out, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, fi.Mode().Perm())
	if err != nil {
		return fail.Cannot("cannot write %s: %v", target, err)
	}
	if _, err := io.Copy(out, in); err != nil {
		out.Close()
		return fail.Cannot("cannot copy %s -> %s: %v", src, dst, err)
	}
	return out.Close()
}

// ParseOptions reads the flags `pg-toolkit binary rootfs run` accepts, returning the options
// and the command after `--`.
func ParseOptions(args []string) (Options, []string, error) {
	var o Options
	var argv []string
	for i := 0; i < len(args); i++ {
		a := args[i]
		val := func() (string, error) {
			if i+1 >= len(args) {
				return "", fail.Cannot("rootfs: %s needs a value", a)
			}
			i++
			return args[i], nil
		}
		var err error
		var v string
		switch a {
		case "--copy":
			if v, err = val(); err == nil {
				o.Copy = append(o.Copy, v)
			}
		case "--bind":
			if v, err = val(); err == nil {
				o.Bind = append(o.Bind, v)
			}
		case "--dns":
			if v, err = val(); err == nil {
				o.DNS = v
			}
		case "--workdir":
			if v, err = val(); err == nil {
				o.Workdir = v
			}
		case "--mask-dev":
			if v, err = val(); err == nil {
				o.MaskDev = append(o.MaskDev, v)
			}
		case "--tmp-size":
			if v, err = val(); err == nil {
				o.TmpSize = v
			}
		case "--tmp-noexec":
			o.TmpNoExec = true
		case "--tmp-readonly":
			o.TmpReadOnly = true
		case "--no-net":
			o.NoNet = true
		case "--private-net":
			o.PrivateNet = true
		case "--":
			argv = args[i+1:]
			i = len(args)
		default:
			if strings.HasPrefix(a, "-") {
				return o, nil, fail.Cannot("rootfs: unknown argument: %s", a)
			}
			o.Root = a
		}
		if err != nil {
			return o, nil, err
		}
	}
	return o, argv, nil
}

// Describe renders the run for a log line.
func Describe(o Options, argv []string) string {
	return fmt.Sprintf("%s binds=%v workdir=%s -- %s",
		o.Root, o.Bind, defaultWorkdir(o.Workdir), logx.QuoteArgs(argv))
}

// cfg is below this package, so the answer is wired downwards at init rather
// than imported upwards, which would be a cycle.
func init() { cfg.ChrootUsable = func() bool { return Entry() != RouteNone } }

// Route is how Run enters a root filesystem on this machine.
type Route int

const (
	// RouteNone: no route may mount here.
	RouteNone Route = iota
	// RouteMountNS: a new mount namespace in the caller's own user namespace.
	// Root with CAP_SYS_ADMIN takes it.
	RouteMountNS
	// RouteUserNS: a new user namespace and a mount namespace in it, with the
	// caller mapped to root inside. A caller that is not root takes it, and so
	// does a root that may not mount in its own namespace: a container with
	// no CAP_SYS_ADMIN, which is the podman and docker default.
	RouteUserNS
)

func (r Route) String() string {
	switch r {
	case RouteMountNS:
		return "mount namespace"
	case RouteUserNS:
		return "user namespace"
	}
	return "none"
}

// attr is the process attributes that create the route's namespaces, with
// extra CLONE_ flags added.
func (r Route) attr(extra uintptr) *syscall.SysProcAttr {
	if r == RouteUserNS {
		// ⚠ The namespaces are created at CLONE rather than by a later
		// unshare, because the id maps have to be written before the child
		// can do anything privileged in them, and Go writes them only for
		// Cloneflags. One id is mapped, not a range: newuidmap is not assumed
		// present, and one is enough to be root inside.
		//
		// ⛔ Everything in the root filesystem owned by anyone else appears as
		// nobody. A tree unpacked by this same user is unaffected, which is
		// the case this exists for.
		//
		// A mount namespace owned by a new user namespace gets the caller's
		// shared mounts as slave mounts, so no mount made inside reaches the
		// caller (mount_namespaces(7)).
		return &syscall.SysProcAttr{
			Cloneflags:                 syscall.CLONE_NEWUSER | syscall.CLONE_NEWNS | extra,
			UidMappings:                []syscall.SysProcIDMap{{ContainerID: 0, HostID: os.Getuid(), Size: 1}},
			GidMappings:                []syscall.SysProcIDMap{{ContainerID: 0, HostID: os.Getgid(), Size: 1}},
			GidMappingsEnableSetgroups: false,
		}
	}
	// ⛔ Unshareflags, not Cloneflags. For CLONE_NEWNS in Unshareflags, the Go
	// child also makes / private (MS_REC|MS_PRIVATE) before it execs. A mount
	// namespace in the caller's own user namespace keeps shared propagation
	// otherwise, and a mount made inside would appear in the caller's too.
	return &syscall.SysProcAttr{Unshareflags: syscall.CLONE_NEWNS | extra}
}

var (
	entryRoute Route
	entryAsked bool
	entryMu    sync.Mutex
)

// Entry is Probe, asked at most once. The probe forks, and engine detection
// runs several times in one command.
func Entry() Route {
	entryMu.Lock()
	defer entryMu.Unlock()
	if !entryAsked {
		entryRoute, entryAsked = Probe(), true
	}
	return entryRoute
}

// Probe returns the first route on which a child can enter the namespaces, and
// mount and chroot inside them. It tries each route, and reads no sysctl and no
// uid.
//
// ⛔ An absence is not a zero and a sysctl is not the answer: the two knobs
// that gate this differ between distributions, one of them does not exist on
// several, and a container runtime or a seccomp profile can refuse the call
// with every knob saying yes.
//
// ⛔ The mount, not only the namespace. Ubuntu 24.04 lets an unprivileged
// process create a user namespace and, by its default AppArmor policy, refuses
// a mount inside it. A probe of the CLONE alone chose this engine there, and
// the run then failed at its first mount. The rule is the operation the engine
// needs, never a privilege that usually implies it (docs/research/podbox.md).
//
// ⛔ Root is probed too, not assumed. A root process in a container with no
// CAP_SYS_ADMIN may not mount in its own namespace, and a probe that read the
// uid chose this engine there. That root may still create a user namespace and
// mount in it (rootless podman allows it by default), so it takes RouteUserNS.
// scripts/common/userns-fixture.sh measures these cases.
func Probe() Route {
	if os.Geteuid() == 0 && probeRoute(RouteMountNS) {
		return RouteMountNS
	}
	if probeRoute(RouteUserNS) {
		return RouteUserNS
	}
	return RouteNone
}

// probeRoute starts the probe child on route r, with this process's mount
// namespace as its argument. The child exits 0 only when its mount and its
// chroot succeeded (EntryProbe).
func probeRoute(r Route) bool {
	self, err := os.Executable()
	if err != nil {
		return false
	}
	ns, err := os.Readlink("/proc/self/ns/mnt")
	if err != nil {
		return false
	}
	cmd := exec.Command(self, entryProbe, ns)
	cmd.Stdout, cmd.Stderr = io.Discard, io.Discard
	cmd.SysProcAttr = r.attr(0)
	return cmd.Run() == nil
}

// EntryProbe is what the probe child runs, in the namespaces it was started
// in: one tmpfs mount over the temporary directory, then a chroot into it.
// Those are the two operations Inner cannot do without. Nothing outside the
// namespace sees the mount, and the child exits straight after.
//
// ⛔ The chroot too, not only the mount. A seccomp profile can refuse chroot(2)
// and allow mount(2), and the run then fails at its chroot after it has mounted
// everything. podbox measured that host: its probe said yes and every entry
// died at chroot EPERM (docs/research/podbox.md).
//
// ⛔ It refuses to mount in its caller's mount namespace. It is a hidden
// command, and run by hand as root it would put a tmpfs over the temporary
// directory of that namespace. The caller gives its namespace as the one
// argument. The child cannot read it from /proc/PPID/ns/mnt: from inside a new
// user namespace the kernel refuses that read (EACCES).
func EntryProbe(args []string) error {
	own, err := os.Readlink("/proc/self/ns/mnt")
	if err != nil {
		return fail.Cannot("%s: cannot read this process's mount namespace: %v", entryProbe, err)
	}
	if len(args) != 1 || args[0] == own {
		return fail.Cannot("%s: not in a mount namespace other than its caller's. pg-toolkit starts this command; do not run it by hand", entryProbe)
	}
	if err := syscall.Mount("", "/", "", syscall.MS_REC|syscall.MS_PRIVATE, ""); err != nil {
		return err
	}
	dir := os.TempDir()
	if err := syscall.Mount("none", dir, "tmpfs", 0, ""); err != nil {
		return err
	}
	return syscall.Chroot(dir)
}

// entryProbe is the argument the probe child is given. The dispatcher answers
// it with EntryProbe, and the exit status is the whole reading.
const entryProbe = "__entry-probe"

// EntryProbeArg is the argument name, for the dispatcher.
func EntryProbeArg() string { return entryProbe }

// devNodes is the /dev a masked bed gets: the nodes a program can reasonably
// expect and nothing else. ⭐ It is close to what a real container provides,
// which is the environment the masked rows are about in the first place.
//
// ⛔ Every one is BIND-mounted from the host's own /dev rather than created,
// because `mknod` needs a privilege the unprivileged path does not have and
// this has to work the same either way.
var devNodes = []string{
	"null", "zero", "full", "random", "urandom", "tty", "console", "ptmx", "fuse",
}

// devDirs are the directories a masked /dev also carries.
var devDirs = []string{"pts", "shm"}

// devLinks are the symlinks a masked /dev carries, target first.
var devLinks = [][2]string{
	{"/proc/self/fd", "fd"},
	{"/proc/self/fd/0", "stdin"},
	{"/proc/self/fd/1", "stdout"},
	{"/proc/self/fd/2", "stderr"},
}

// minimalDev replaces dst with a tmpfs carrying devNodes minus mask.
//
// ⛔ This is what makes an absence real. A bind-mounted /dev cannot have a node
// removed from it, and over-mounting one leaves a file that still opens, so a
// criterion reading "the artefact must fall back when /dev/fuse is missing"
// would be reading a lie. Here the node is genuinely not there and `open`
// answers ENOENT.
//
// ⚠ A name in mask that is not in devNodes is not an error: the caller may be
// masking something this list never carried, and the result - absent - is the
// same.
func minimalDev(dst string, mask []string) error {
	masked := map[string]bool{}
	for _, m := range mask {
		masked[strings.TrimPrefix(strings.TrimPrefix(m, "/dev/"), "/")] = true
	}
	if err := syscall.Mount("none", dst, "tmpfs", 0, "mode=0755"); err != nil {
		return fail.Cannot("cannot put a tmpfs on %s to mask %v: %v", dst, mask, err)
	}
	for _, n := range devNodes {
		if masked[n] {
			continue
		}
		src := filepath.Join("/dev", n)
		if _, err := os.Stat(src); err != nil {
			// ⚠ Not an error. The host may not have every node, and a bed
			// missing one the host also lacks is the host's shape rather than
			// a defect here.
			continue
		}
		target := filepath.Join(dst, n)
		if err := os.WriteFile(target, nil, 0o666); err != nil {
			return fail.Cannot("cannot make a mount point for /dev/%s: %v", n, err)
		}
		if err := syscall.Mount(src, target, "", syscall.MS_BIND, ""); err != nil {
			return fail.Cannot("cannot bind /dev/%s into the masked /dev: %v", n, err)
		}
	}
	for _, d := range devDirs {
		src := filepath.Join("/dev", d)
		if _, err := os.Stat(src); err != nil {
			continue
		}
		if masked[d] {
			continue
		}
		target := filepath.Join(dst, d)
		if err := os.MkdirAll(target, 0o755); err != nil {
			return fail.Cannot("cannot make %s: %v", target, err)
		}
		if err := syscall.Mount(src, target, "", syscall.MS_BIND|syscall.MS_REC, ""); err != nil {
			return fail.Cannot("cannot bind /dev/%s into the masked /dev: %v", d, err)
		}
	}
	for _, l := range devLinks {
		if masked[l[1]] {
			continue
		}
		_ = os.Symlink(l[0], filepath.Join(dst, l[1]))
	}
	return nil
}

// tmpFlags turns the tmpfs options into a mount flag word.
func tmpFlags(noexec bool) uintptr {
	var f uintptr
	if noexec {
		f |= syscall.MS_NOEXEC
	}
	return f
}

// mountTmp puts the bed's tmpfs on /tmp, with the size and noexec a caller
// asked for. ⚠ Both default to nothing, which is the mount every committed
// measurement was taken against.
func mountTmp(dst, size string, noexec bool) {
	data := ""
	if size != "" {
		data = "size=" + size
	}
	mountQuiet("none", dst, "tmpfs", tmpFlags(noexec), data)
}

// remountReadOnly makes an already-mounted tmpfs read-only.
//
// ⛔ MS_REMOUNT carries the WHOLE flag word rather than adding to it, so a
// noexec asked for at mount time has to be repeated here or the remount
// silently clears it. That is the defect this function exists to not have.
func remountReadOnly(dst string, noexec bool) error {
	flags := syscall.MS_REMOUNT | syscall.MS_RDONLY | tmpFlags(noexec)
	if err := syscall.Mount("none", dst, "tmpfs", flags, ""); err != nil {
		return fail.Cannot("cannot remount %s read-only: %v", dst, err)
	}
	return nil
}
