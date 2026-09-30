// selftest.go - prove the isolation, offline.
//
// A positive control as well as an absence: the fixture plants a file the host
// does not have at that path and asserts the chroot sees it, then asserts that
// a host path which certainly exists is invisible. One without the other
// proves nothing.
//
// SPDX-License-Identifier: 0BSD
package rootfs

import (
	"os"
	"path/filepath"

	"github.com/Azathothas/pg-toolkit/internal/proc"
	"github.com/Azathothas/pg-toolkit/internal/selftest"
)

const probeSource = `#include <unistd.h>
#include <sys/stat.h>
int main(int argc, char **argv){ struct stat st;
  if (argc<2) return 2;
  return stat(argv[1], &st)==0 ? 0 : 1; }
`

// Selftest exercises the runner against a fixture filesystem.
func Selftest() *selftest.Report {
	r := selftest.New("rootfs-run")
	// ⛔ A non-root caller is not skipped any more. The runner enters a user
	// namespace when it is not root, so the isolation this asserts is exactly
	// the isolation an unprivileged build gets, and a skip here left that
	// completely unasserted on every machine where pgb is not run as root.
	if Entry() == RouteNone {
		r.Skip("the namespaces the runner enters, or a mount inside them, are refused here")
		return r
	}
	if !proc.Look("cc") {
		r.Skip("no C compiler, so the fixture's static probe cannot be built")
		return r
	}

	dir, err := os.MkdirTemp("", "pgb-rootfs-selftest-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "a writable temporary directory")
		return r
	}
	defer os.RemoveAll(dir)

	root := filepath.Join(dir, "rootfs")
	if err := os.MkdirAll(filepath.Join(root, "bin"), 0o755); err != nil {
		r.Fail("mkdir", err.Error(), "created")
		return r
	}
	if err := os.MkdirAll(filepath.Join(root, "marker"), 0o755); err != nil {
		r.Fail("mkdir", err.Error(), "created")
		return r
	}
	if err := os.WriteFile(filepath.Join(root, "marker", "only-inside"), nil, 0o644); err != nil {
		r.Fail("plant marker", err.Error(), "created")
		return r
	}

	// A statically linked probe is needed: the fixture has no loader.
	src := filepath.Join(dir, "probe.c")
	if err := os.WriteFile(src, []byte(probeSource), 0o644); err != nil {
		r.Fail("write probe", err.Error(), "created")
		return r
	}
	probe := filepath.Join(root, "bin", "exists")
	if res, err := proc.Quiet("cc", "-static", "-O0", "-o", probe, src); err != nil || res.Failed() {
		r.Skip("no static cc available, isolation cases not run")
		return r
	}

	null, err := os.OpenFile(os.DevNull, os.O_WRONLY, 0)
	if err != nil {
		r.Fail("open /dev/null", err.Error(), "opened")
		return r
	}
	defer null.Close()

	run := func(o Options, argv ...string) int {
		o.Root = root
		o.Stdout, o.Stderr = null, null
		code, err := Run(o, argv)
		if err != nil {
			return -1
		}
		return code
	}

	r.Check("sees-rootfs-only-path", itoa(run(Options{}, "/bin/exists", "/marker/only-inside")), "0")
	r.Check("host-etc-invisible", itoa(run(Options{}, "/bin/exists", "/etc/hostname")), "1")
	r.Check("copy-lands-inside",
		itoa(run(Options{Copy: []string{probe + ":/copied"}}, "/copied", "/copied")), "0")
	return r
}

func itoa(n int) string {
	if n < 0 {
		return "runner-error"
	}
	if n == 0 {
		return "0"
	}
	var b []byte
	for n > 0 {
		b = append([]byte{byte('0' + n%10)}, b...)
		n /= 10
	}
	return string(b)
}
