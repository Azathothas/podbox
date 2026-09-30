// Package assets carries the files a distributed pgb needs but cannot fetch:
// the C runtime sources it compiles into every build, the pinned target list,
// and the small fixtures its selftests run against.
//
// It lives at the repository root because go:embed can only reach files under
// the embedding package's own directory.
//
// SPDX-License-Identifier: 0BSD
package assets

import (
	"crypto/sha256"
	"embed"
	"encoding/hex"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// The runtime sources are grouped by which tool compiles them, so the pattern
// has to recurse. A `tool/runtime/*.c` glob does not: it silently stops
// matching the moment a source moves into a subdirectory, and because these
// files are compiled at build time rather than by the Go toolchain, the
// failure arrives at run time in a binary that built green.
//
//go:embed all:tool/runtime
var runtimeFS embed.FS

// The launcher crate, carried so a distributed pgb can BUILD the launcher it
// puts in a bundle rather than downloading one. T-100.
//
// ⛔ The patterns are explicit and a recursive `all:` pattern over the crate is
// NOT used, which is the opposite of the runtime's rule above and has its own
// reason: cargo writes its objects into a `target` directory inside the crate,
// that directory is gitignored rather than hidden, and go:embed has no notion
// of gitignore. A recursive pattern here would embed a build tree - hundreds of
// megabytes, and different on every machine - into the binary the moment
// somebody ran cargo in the checkout. The src pattern still recurses, so a new
// source file needs no change here.
//
//go:embed tool/prun/Cargo.toml tool/prun/Cargo.lock tool/prun/src
var launcherFS embed.FS

//go:embed scripts/common/rootfs-images.txt
var dataFS embed.FS

// The narinfo fixtures are the offline oracle for the nix signature path: two
// bodies signed by cache.nixos.org's own key, which this project could not
// have produced. They are embedded for the same reason the target list is.
// Read off the disk beside the binary, they were absent for every pgb not run
// from a checkout, and the selftest reported `could not run` where a real
// cryptographic assertion should have been.
//
//go:embed scripts/common/fixtures/nix
var fixtureFS embed.FS

// RuntimeFS exposes the C runtime sources, rooted so a name is the same
// family-relative path used everywhere else ("binary/nssfix.c").
func RuntimeFS() fs.FS {
	sub, err := fs.Sub(runtimeFS, "tool/runtime")
	if err != nil {
		panic(err) // the embed pattern is a compile-time constant
	}
	return sub
}

// RuntimeFile returns one embedded C source or header by its family-relative
// path, for example "binary/elfload.c".
func RuntimeFile(name string) ([]byte, error) {
	return runtimeFS.ReadFile("tool/runtime/" + name)
}

// RuntimeNames lists the embedded runtime sources as family-relative paths,
// sorted. It walks rather than reading one directory, so a new family
// directory needs no change here.
func RuntimeNames() []string {
	return namesUnder(runtimeFS, "tool/runtime")
}

// namesUnder lists every file under root in an embedded filesystem, as paths
// relative to root, sorted.
func namesUnder(fsys embed.FS, root string) []string {
	var out []string
	err := fs.WalkDir(fsys, root, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			return nil
		}
		out = append(out, strings.TrimPrefix(p, root+"/"))
		return nil
	})
	if err != nil {
		return nil
	}
	sort.Strings(out)
	return out
}

// launcherRoot is where the carried launcher crate sits inside this package,
// written once so a name is the same crate-relative path the tree uses
// ("src/main.rs", "Cargo.toml").
const launcherRoot = "tool/prun"

// LauncherFile returns one carried launcher source by its crate-relative path.
func LauncherFile(name string) ([]byte, error) {
	return launcherFS.ReadFile(launcherRoot + "/" + name)
}

// LauncherNames lists the carried launcher sources as crate-relative paths,
// sorted. Like RuntimeNames it walks, so a source added to the crate needs no
// change here.
func LauncherNames() []string {
	return namesUnder(launcherFS, launcherRoot)
}

// MaterialiseLauncher writes the carried launcher crate into dir, preserving
// the crate layout so cargo is handed the same tree the checkout has.
//
// ⛔ A file already matching is not rewritten, and that is not a saving here
// the way it is for the runtime: cargo decides what to recompile from mtimes,
// so rewriting an unchanged source would rebuild the whole crate on every
// bundle.
func MaterialiseLauncher(dir string) error {
	return materialise(launcherFS, launcherRoot, dir, LauncherNames())
}

// LauncherDigest is a stable identifier for the carried launcher crate, used
// as part of a cache key so a pgb carrying different sources does not reuse a
// launcher built from the old ones.
//
// ⛔ It is SEPARATE from Digest above rather than folded into it. Digest keys
// the compiled C objects, and folding the crate in would invalidate every one
// of them each time a Rust source moved - a rebuild of work that did not
// change, on a path that already costs minutes.
func LauncherDigest() string {
	h := sha256.New()
	for _, name := range LauncherNames() {
		b, err := LauncherFile(name)
		if err != nil {
			continue
		}
		fmt.Fprintf(h, "%s\n%d\n", name, len(b))
		h.Write(b)
	}
	return hex.EncodeToString(h.Sum(nil))[:16]
}

// RootfsImages is the digest-pinned list of target environments.
func RootfsImages() []byte {
	b, err := dataFS.ReadFile("scripts/common/rootfs-images.txt")
	if err != nil {
		panic(err)
	}
	return b
}

// Materialise writes the runtime sources into dir, preserving the family
// subdirectories so a compile command names the same path the tree does. A
// file whose contents already match is not rewritten, so timestamps stay
// stable and a rebuild is not triggered by the copy itself.
func Materialise(dir string) error {
	return materialise(runtimeFS, "tool/runtime", dir, RuntimeNames())
}

// materialise writes the named files out of an embedded filesystem into dir,
// preserving the subdirectories so a compile command names the same path the
// tree does.
func materialise(fsys embed.FS, root, dir string, names []string) error {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	for _, name := range names {
		want, err := fsys.ReadFile(root + "/" + name)
		if err != nil {
			return err
		}
		dst := filepath.Join(dir, filepath.FromSlash(name))
		if err := os.MkdirAll(filepath.Dir(dst), 0o755); err != nil {
			return err
		}
		if got, err := os.ReadFile(dst); err == nil && sameBytes(got, want) {
			continue
		}
		tmp := dst + ".tmp"
		if err := os.WriteFile(tmp, want, 0o644); err != nil {
			return err
		}
		if err := os.Rename(tmp, dst); err != nil {
			return err
		}
	}
	return nil
}

// MaterialiseNixFixtures writes the carried narinfo fixtures into dir and
// returns how many it wrote. Like Materialise, a file already matching is left
// alone.
func MaterialiseNixFixtures(dir string) (int, error) {
	entries, err := fs.ReadDir(fixtureFS, "scripts/common/fixtures/nix")
	if err != nil {
		return 0, err
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return 0, err
	}
	n := 0
	for _, e := range entries {
		if e.IsDir() {
			continue
		}
		want, err := fixtureFS.ReadFile("scripts/common/fixtures/nix/" + e.Name())
		if err != nil {
			return n, err
		}
		dst := filepath.Join(dir, e.Name())
		n++
		if got, err := os.ReadFile(dst); err == nil && sameBytes(got, want) {
			continue
		}
		tmp := dst + ".tmp"
		if err := os.WriteFile(tmp, want, 0o644); err != nil {
			return n, err
		}
		if err := os.Rename(tmp, dst); err != nil {
			return n, err
		}
	}
	return n, nil
}

// Digest is a stable identifier for the whole embedded runtime, used as part
// of a cache key so a pgb carrying different sources does not reuse objects
// compiled from the old ones.
func Digest() string {
	h := sha256.New()
	for _, name := range RuntimeNames() {
		b, err := RuntimeFile(name)
		if err != nil {
			continue
		}
		fmt.Fprintf(h, "%s\n%d\n", name, len(b))
		h.Write(b)
	}
	return hex.EncodeToString(h.Sum(nil))[:16]
}

// EmbeddedManifest renders name and size for each carried file, for
// `pg-toolkit binary doctor`.
func EmbeddedManifest() string {
	var b strings.Builder
	for _, name := range RuntimeNames() {
		d, _ := RuntimeFile(name)
		fmt.Fprintf(&b, "  %-22s %7d bytes\n", name, len(d))
	}
	for _, name := range LauncherNames() {
		d, _ := LauncherFile(name)
		fmt.Fprintf(&b, "  %-22s %7d bytes\n", launcherRoot+"/"+name, len(d))
	}
	fmt.Fprintf(&b, "  %-22s %7d bytes\n", "rootfs-images.txt", len(RootfsImages()))
	return b.String()
}

func sameBytes(a, b []byte) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}
