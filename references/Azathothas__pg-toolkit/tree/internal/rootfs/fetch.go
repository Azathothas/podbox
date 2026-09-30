// fetch.go - materialise the test bed named in rootfs-images.txt.
//
// Every environment an experiment runs against comes from here at the pinned
// digest, so a result taken today and one taken next month describe the same
// filesystem.
//
// SPDX-License-Identifier: 0BSD
package rootfs

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/Azathothas/pg-toolkit/internal/cfg"
	"github.com/Azathothas/pg-toolkit/internal/fail"
	"github.com/Azathothas/pg-toolkit/internal/logx"
	"github.com/Azathothas/pg-toolkit/internal/ociimg"
)

// FetchOptions controls a bed materialisation.
type FetchOptions struct {
	Dest      string   // where the filesystems land
	Arch      string   // non-empty re-resolves by tag and drops the pin
	Only      []string // local names to fetch; empty means all
	List      bool     // report what is pinned and what is on disk
	IfMissing bool     // skip a row already on disk at the pinned digest
}

// Fetch pulls the requested rows.
func Fetch(images []cfg.ImageRow, o FetchOptions) error {
	if o.List {
		return list(images, o.Dest)
	}
	want := map[string]bool{}
	for _, w := range o.Only {
		want[w] = true
	}
	matched, failed := 0, 0
	for _, row := range images {
		if len(want) > 0 && !want[row.Name] {
			continue
		}
		matched++
		out := filepath.Join(o.Dest, row.Name)
		if o.IfMissing && onDiskDigest(out) == row.Digest {
			logx.Say("== %s (%s) %s: already at the pinned digest", row.Name, row.Ref, row.Libc)
			continue
		}
		opts := ociimg.Options{Ref: row.Ref, Out: out, Digest: row.Digest}
		if o.Arch != "" {
			// A non-native architecture trades the pin away: the digests are
			// amd64 manifests, so another architecture must go back through the
			// tag, which is not the same input.
			logx.Say("== %s (%s) arch=%s: resolving by TAG, the pinned digest does not apply",
				row.Name, row.Ref, o.Arch)
			opts.Digest = ""
			opts.Arch = o.Arch
		} else {
			logx.Say("== %s (%s) %s", row.Name, row.Ref, row.Libc)
		}
		if _, err := ociimg.Pull(opts); err != nil {
			logx.Warnf("%s: %v", row.Name, err)
			failed++
		}
	}
	if matched == 0 {
		return fail.Cannot("nothing in the image list matched: %s", strings.Join(o.Only, " "))
	}
	if failed > 0 {
		return fail.Ran("%d of %d environments could not be fetched", failed, matched)
	}
	return nil
}

func list(images []cfg.ImageRow, dest string) error {
	logx.Say("%-34s %-20s %-6s %s", "REFERENCE", "NAME", "LIBC", "ON DISK")
	for _, row := range images {
		state := "no"
		dir := filepath.Join(dest, row.Name)
		if fi, err := os.Stat(dir); err == nil && fi.IsDir() {
			have := onDiskDigest(dir)
			switch {
			case have == row.Digest:
				state = "yes (pinned digest)"
			case have != "":
				state = "yes (DIGEST DIFFERS: " + have + ")"
			default:
				state = "yes (no provenance)"
			}
		}
		logx.Say("%-34s %-20s %-6s %s", row.Ref, row.Name, row.Libc, state)
	}
	return nil
}

// onDiskDigest reads the manifest digest a previous pull recorded.
func onDiskDigest(dir string) string {
	b, err := os.ReadFile(filepath.Join(dir, ".oci-provenance"))
	if err != nil {
		return ""
	}
	for line := range strings.SplitSeq(string(b), "\n") {
		if v, ok := strings.CutPrefix(line, "manifest digest:"); ok {
			return strings.TrimSpace(v)
		}
	}
	return ""
}

// Present counts how many of the pinned environments are on disk.
func Present(images []cfg.ImageRow, dest string) int {
	n := 0
	for _, row := range images {
		if fi, err := os.Stat(filepath.Join(dest, row.Name)); err == nil && fi.IsDir() {
			n++
		}
	}
	return n
}

// ParseFetchArgs reads the flags `pg-toolkit binary rootfs fetch` accepts.
func ParseFetchArgs(args []string, def string) (FetchOptions, error) {
	o := FetchOptions{Dest: def}
	for i := 0; i < len(args); i++ {
		a := args[i]
		switch a {
		case "--list":
			o.List = true
		case "--if-missing":
			o.IfMissing = true
		case "--arch", "--dest":
			if i+1 >= len(args) {
				return o, fail.Cannot("rootfs fetch: %s needs a value", a)
			}
			i++
			if a == "--arch" {
				o.Arch = args[i]
			} else {
				o.Dest = args[i]
			}
		default:
			if strings.HasPrefix(a, "-") {
				return o, fail.Cannot("rootfs fetch: unknown argument: %s", a)
			}
			o.Only = append(o.Only, a)
		}
	}
	return o, nil
}

// FormatBed renders which environments are present, for `pg-toolkit binary doctor`.
func FormatBed(images []cfg.ImageRow, dest string) string {
	return fmt.Sprintf("%d present under %s", Present(images, dest), dest)
}
