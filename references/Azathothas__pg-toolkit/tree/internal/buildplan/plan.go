// plan.go - what this tree produces, what produces it, and whether it is stale.
//
// ⭐ **One table, and it is the only place an artefact is named.** Before this
// there were three: the Makefile's targets, its hand-maintained EMBEDDED list,
// and whatever a session typed into a shell for the Rust crates. The crates had
// no install location at all - `tool/pboot` and `tool/pstrip` were built into
// /tmp by whoever needed them - so "what does this repository build" had no
// answer anywhere.
//
// ⛔ **The input set is DERIVED and never typed.** For a Go artefact it comes
// from `go list -deps -json`, which reports the source files AND the
// `go:embed` files of every package in this module that the binary reaches.
// The Makefile listed those embeds by hand, and its own comment records what
// that cost: editing the ELF loader and running make printed "Nothing to be
// done", so the next build compiled the previous loader and an
// eleven-environment run measured a fix that was never in the binary. A list
// nobody maintains cannot drift.
//
// ⚠ **The toolchain is part of the input set.** A different compiler produces
// a different binary, so `go version` and `rustc --version` are hashed with
// the sources. That marks everything stale the day a toolchain moves, which is
// the correct answer rather than a noisy one.
//
// SPDX-License-Identifier: 0BSD
package buildplan

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
)

// Schema is the manifest's version, so a manifest written by an older build
// tool is discarded rather than misread.
const Schema = "pgt-build/1"

// ManifestPath is where the record of the last build lives, repo-relative.
//
// ⚠ Under the output directory on purpose: `--clean` removes that directory,
// which removes the manifest with it, which makes the next run rebuild
// everything. A manifest that outlived the binaries it describes would report
// a tree as current with nothing on disk.
const ManifestPath = "bin/manifest.json"

// OutDir is where an artefact goes when it does not already have a documented
// home. Repo-relative.
const OutDir = "bin"

// Kind is what builds an artefact.
type Kind string

const (
	// Go, for the general tooling: the driver, the checks, the planners.
	Go Kind = "go"
	// Rust, for the binaries that manipulate ELF or run inside an artefact.
	Rust Kind = "rust"
)

// GnuTarget is the target the dogfooded artefacts are built for: the binary
// mode links against glibc, statically, which is the whole point of it.
const GnuTarget = "x86_64-unknown-linux-gnu"

// DogfoodRustflags are the two flags a caller HAS to know, and they are here
// rather than in a page because a caller who has not read the page gets a link
// error whose text names neither of them.
//
// ⛔ `+crt-static` alone makes rustc ask for a static-PIE, and the binary mode's
// wrappers link `-static` with non-PIC startup objects. The two together fail
// inside crtbeginT.o at "relocation R_X86_64_32 cannot be used against local
// symbol", which names glibc's startup code and not the flag that caused it.
const DogfoodRustflags = "-C target-feature=+crt-static -C relocation-model=static"

// DogfoodToolchainDir is the Rust prefix the build environment has to see. The
// chroot has no /opt, so cargo and rustc are bound in from the host.
const DogfoodToolchainDir = "/opt/rust"

// Artefact is one thing this tree produces.
type Artefact struct {
	// Name is what a caller asks for on the command line.
	Name string
	// Kind decides which toolchain builds it.
	Kind Kind
	// Src is the Go import path, or the repo-relative crate directory.
	Src string
	// Out is the repo-relative output path.
	//
	// ⛔ The four Go binaries stay at the repository ROOT and that is frozen
	// output rather than an oversight: every page, script and check in this
	// tree runs `./check`, `./pgb`, `./pg-devenv`. Moving them is a rename
	// with its own entry, not a build script's decision. `bin/` is where the
	// artefacts that had no home at all go.
	Out string
	// Target is the Rust target triple, empty for the host's own.
	Target string
	// Dogfood asks for the BINARY MODE's toolchain rather than the host's
	// cargo.
	//
	// ⛔ TODO/port.md's T-145 is the ruling: anything static this project needs
	// is built by this project. It is not the numbers - plain static glibc is
	// 52 microseconds FASTER on the route than the binary mode's - it is that
	// building this project's own artefacts with this project's own tool finds
	// that tool's edge cases here rather than in a user's hands.
	//
	// ⚠ It costs a bootstrapped build environment and root. A host without one
	// still builds, through the host toolchain, and `./build` SAYS SO for every
	// artefact it falls back on: a fallback nobody is told about is the same
	// answer as a skip.
	Dogfood bool
	// What is one line for `--list`.
	What string
	// Note is why this may not build on some host, empty where none is known.
	Note string
}

// All is the table. ⛔ Order matters exactly once: `pstrip` is built before
// anything is stripped, because it is what does the stripping.
func All() []Artefact {
	return []Artefact{
		{
			Name: "pg-devenv", Kind: Go, Src: "./cmd/pg-devenv", Out: "pg-devenv",
			What: "the ONE entry point for running anything, on any host. docs/devenv.md",
		},
		{
			Name: "check", Kind: Go, Src: "./cmd/check", Out: "check",
			What: "the gate: twenty-one checks over the tracked tree and the history",
		},
		{
			Name: "pg-toolkit", Kind: Go, Src: "./cmd/pg-toolkit", Out: "pg-toolkit",
			What: "THE program: every mode, and the classifier that chooses one. docs/toolkit.md",
			Note: "Linux only: it calls mknod and reads syscall.Stat_t, neither of which exists on Windows",
		},
		{
			// ⛔ The SAME SOURCE, under the name the parked apparatus types.
			// TODO/port.md's T-147 folded the second program into this one,
			// and this row is a compatibility NAME rather than a second
			// program: `main` dispatches on the base name it was invoked as.
			//
			// ⚠ It is a DEBT with an owner. 53 experiment scripts invoke it by
			// path, and `check evidence` binds each of their committed results
			// to the hash of the script that produced it - so renaming them
			// while the measurement work is parked turns 53 measurements into
			// debts nobody may pay. This row goes when they are re-run.
			Name: "pgb", Kind: Go, Src: "./cmd/pg-toolkit", Out: "pgb",
			What: "the compatibility name for `pg-toolkit binary`, for the parked experiments. TODO/port.md T-147",
			Note: "Linux only, and the same program as pg-toolkit built under a second name",
		},
		{
			// ⛔ It had NO target at all and was therefore a DYNAMIC host-glibc
			// binary - measured, not assumed: an INTERP, libc.so.6 and
			// libgcc_s.so.1. It runs on a user's machine as part of a build, so
			// its libc is a portability question rather than a preference.
			Name: "pstrip", Kind: Rust, Src: "tool/pstrip", Out: OutDir + "/pstrip",
			Target: GnuTarget, Dogfood: true,
			What: "removes the section headers and every byte past the last PT_LOAD. docs/appimage/stripper.md",
			Note: "built first, because it is what strips the rest",
		},
		{
			Name: "prun", Kind: Rust, Src: "tool/prun", Out: OutDir + "/prun",
			Target: GnuTarget, Dogfood: true,
			What: "the launcher a bundle carries. docs/appimage/launcher.md",
			Note: "musl is 1.76x slower on the route a bundle takes on every start. TODO/port.md T-145",
		},
		{
			Name: "pboot", Kind: Rust, Src: "tool/pboot", Out: OutDir + "/pboot",
			Target: GnuTarget, Dogfood: true,
			What: "the runtime an artefact starts with: mounts the image and hands off",
		},
	}
}

// OutPath is where this artefact lands, repo-relative.
//
// ⛔ **ONE spelling on every host, and it is the extensionless one.** Every
// page, script, Makefile target and check in this tree runs `./check`,
// `./pgb`, `./pg-devenv`, and a Go binary with no extension runs perfectly
// well on Windows - measured, in this project's own Git Bash, `./check_probe
// docs` at exit 0. Writing `check.exe` there instead produces TWO files for
// one artefact, and the shell resolves the extensionless one, so the stale
// twin silently wins over the build that just ran. That is the exact drift
// this tool exists to remove.
func (a Artefact) OutPath() string { return a.Out }

// StaleTwin is the other spelling of this artefact's output, which must not be
// left lying beside it, or the empty string where there is none.
//
// ⚠ Windows only, and Go only: `.exe` is the spelling the platform's own
// convention and several of this tree's older builds produced, and one was
// tracked into a commit once. A build removes it so exactly one file answers
// to the name.
func (a Artefact) StaleTwin() string {
	if a.Kind == Go && HostIsWindows() {
		return a.Out + ".exe"
	}
	return ""
}

// HostIsWindows is the one place the host's kind is asked, so a case can drive
// the other branch without a second machine.
var HostIsWindows = func() bool { return goosIsWindows }

// MuslTarget is what the launcher and the runtime were built for until T-145.
//
// ⚠ x86_64 only, and an architecture with no row is refused rather than built
// for the wrong one - the same rule internal/bundle's launcher table has.
// ⛔ Kept as the FALLBACK: a host that cannot reach the binary mode's build
// environment still produces the artefacts, and `./build` says which toolchain
// each one came from rather than leaving a reader to guess.
const MuslTarget = "x86_64-unknown-linux-musl"

// NotBuilt is what this tree carries that is NOT an artefact, with what reads
// it instead.
//
// ⛔ It is printed rather than omitted. `tool/runtime/` is C compiled into a
// USER's binary by `pgb` at build time, so it has no output here at all - and
// a build listing that simply did not mention it is how a session concludes
// the tree does not carry it. AGENTS.md records that exact failure: a moved
// runtime source built green and failed on first use.
func NotBuilt() [][2]string {
	return [][2]string{
		{"tool/runtime/binary", "C compiled into a user's binary by pgb. `./check c-runtime` reads it"},
		{"tool/runtime/appimage", "C loaded into a bundled payload. `./check c-runtime` reads it"},
		{"tool/runtime/verify", "C for the tracer `pg-toolkit binary verify` carries. It never enters a user's binary"},
	}
}

// Find returns the artefact of that name.
func Find(name string) (Artefact, bool) {
	for _, a := range All() {
		if a.Name == name {
			return a, true
		}
	}
	return Artefact{}, false
}

// Record is what the manifest holds for one artefact.
type Record struct {
	// Inputs is the digest of everything that feeds the build.
	Inputs string `json:"inputs"`
	// Made is the digest of the binary this build wrote.
	//
	// ⛔ **Without it the record describes a build rather than a file, and
	// they come apart.** Measured while this tool was being written: a change
	// to the output NAME left the previous binary on disk with a manifest
	// entry whose input digest still matched, and the run reported "up to
	// date" over a binary that build had never written. Anything that
	// replaces the file - another Makefile target, a copy, a partial write -
	// produces the same false green.
	Made string `json:"made"`
	// Output is the repo-relative path written.
	Output string `json:"output"`
	// Bytes is the size after stripping.
	Bytes int64 `json:"bytes"`
	// Stripped says whether pstrip ran on it, so a size can be read.
	Stripped bool `json:"stripped"`
	// Built is when, in UTC, RFC3339.
	Built string `json:"built"`
}

// Manifest is the record of the last build.
type Manifest struct {
	Schema    string            `json:"schema"`
	Artefacts map[string]Record `json:"artefacts"`
}

// LoadManifest reads it, and returns an empty one where there is none or where
// it was written by a different schema.
//
// ⛔ A manifest that cannot be parsed is DISCARDED rather than reported, and
// the consequence is a full rebuild. The alternative - refusing to build
// because a cache file is malformed - makes a corrupt cache a wedged tree.
func LoadManifest(repo string) Manifest {
	empty := Manifest{Schema: Schema, Artefacts: map[string]Record{}}
	b, err := os.ReadFile(filepath.Join(repo, filepath.FromSlash(ManifestPath)))
	if err != nil {
		return empty
	}
	var m Manifest
	if json.Unmarshal(b, &m) != nil || m.Schema != Schema || m.Artefacts == nil {
		return empty
	}
	return m
}

// SaveManifest writes it, creating the output directory.
func SaveManifest(repo string, m Manifest) error {
	m.Schema = Schema
	if err := os.MkdirAll(filepath.Join(repo, filepath.FromSlash(OutDir)), 0o755); err != nil {
		return err
	}
	b, err := json.MarshalIndent(m, "", " ")
	if err != nil {
		return err
	}
	b = append(b, '\n')
	return os.WriteFile(filepath.Join(repo, filepath.FromSlash(ManifestPath)), b, 0o644)
}

// State is what a run has to say about one artefact.
type State string

const (
	// Current means the inputs are unchanged and the output is on disk.
	Current State = "up to date"
	// Stale means an input moved since the recorded build.
	Stale State = "outdated"
	// Absent means there is no output on disk at all.
	Absent State = "not built"
	// Unknown means the input digest could not be taken, so nothing is claimed.
	//
	// ⛔ It is NOT "up to date". A toolchain this host does not have makes the
	// inputs unreadable, and reporting that as current would be the vacuous
	// pass this repository keeps meeting.
	Unknown State = "cannot tell"
)

// Status is one artefact's state, with what produced it.
type Status struct {
	Artefact Artefact
	State    State
	// Inputs is the digest taken now, empty when it could not be taken.
	Inputs string
	// Why is the reason for Unknown, or the input that moved for Stale.
	Why string
}

// Classify compares the input digest against the manifest and the disk.
func Classify(repo string, a Artefact, now string, err error, m Manifest) Status {
	s := Status{Artefact: a, Inputs: now}
	if err != nil {
		s.State, s.Why = Unknown, err.Error()
		return s
	}
	out := filepath.Join(repo, filepath.FromSlash(a.OutPath()))
	if _, statErr := os.Stat(out); statErr != nil {
		s.State = Absent
		return s
	}
	rec, ok := m.Artefacts[a.Name]
	if !ok {
		s.State, s.Why = Stale, "no record of how it was built"
		return s
	}
	if rec.Inputs != now {
		s.State, s.Why = Stale, "a source changed since it was built"
		return s
	}
	// ⛔ The record has to describe THIS file, and a record that does not name
	// a binary at all is stale rather than trusted.
	//
	// ⚠ There is no compatibility branch here on purpose. A `Made` of "" could
	// only come from a manifest this schema never wrote, and treating it as
	// "assume current" is precisely the false green the field exists to
	// remove - measured, over a binary an older Makefile target had left.
	if rec.Made == "" {
		s.State, s.Why = Stale, "the record does not say which binary it wrote"
		return s
	}
	got, err := FileDigest(out)
	if err != nil {
		s.State, s.Why = Unknown, "the binary is there and cannot be read: "+err.Error()
		return s
	}
	if got != rec.Made {
		s.State, s.Why = Stale, "the binary on disk is not the one this record describes"
		return s
	}
	s.State = Current
	return s
}

// FileDigest is the sha256 of one file's bytes.
func FileDigest(p string) (string, error) {
	f, err := os.Open(p)
	if err != nil {
		return "", err
	}
	defer f.Close()
	h := sha256.New()
	if _, err := io.Copy(h, f); err != nil {
		return "", err
	}
	return hex.EncodeToString(h.Sum(nil)), nil
}

// SetDigest is the digest of a set of files, keyed by their repo-relative
// paths, plus any extra strings a caller wants inside it.
//
// ⛔ The PATH is hashed beside the content, so renaming a file changes the
// digest even when its bytes do not. Without that, moving a source from one
// package to another is invisible - and AGENTS.md records that this tree has a
// silent failure mode built on exactly that move.
//
// ⚠ The list is sorted here rather than trusted from the caller: `go list`
// returns packages in dependency order, and a digest that depended on that
// order would change when an unrelated import was added.
func SetDigest(repo string, rel []string, extra ...string) (string, error) {
	files := append([]string(nil), rel...)
	sort.Strings(files)
	h := sha256.New()
	for _, e := range extra {
		fmt.Fprintf(h, "extra\x00%s\x00", e)
	}
	for _, r := range files {
		d, err := FileDigest(filepath.Join(repo, filepath.FromSlash(r)))
		if err != nil {
			return "", fmt.Errorf("%s: %w", r, err)
		}
		fmt.Fprintf(h, "file\x00%s\x00%s\x00", r, d)
	}
	return hex.EncodeToString(h.Sum(nil)), nil
}

// CrateFiles is every file that feeds a Rust build, repo-relative and sorted.
//
// ⛔ `target/` is excluded, and it has to be: cargo writes its objects there,
// so a walk that included them would make every build change the input set of
// the next one - a tree that is never up to date and always rebuilds.
func CrateFiles(repo, crate string) ([]string, error) {
	root := filepath.Join(repo, filepath.FromSlash(crate))
	var out []string
	err := filepath.WalkDir(root, func(p string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, rerr := filepath.Rel(repo, p)
		if rerr != nil {
			return rerr
		}
		rel = filepath.ToSlash(rel)
		if d.IsDir() {
			if d.Name() == "target" {
				return filepath.SkipDir
			}
			return nil
		}
		out = append(out, rel)
		return nil
	})
	if err != nil {
		return nil, err
	}
	sort.Strings(out)
	return out, nil
}

// IsELF says whether a file begins with the ELF magic.
//
// ⭐ Read from the FILE rather than assumed from GOOS. A cross-compiled Linux
// binary sitting on a Windows host is an ELF, and a host build on Windows is
// not, so the question is about the bytes rather than about the machine.
func IsELF(p string) bool {
	f, err := os.Open(p)
	if err != nil {
		return false
	}
	defer f.Close()
	var b [4]byte
	if n, _ := io.ReadFull(f, b[:]); n != 4 {
		return false
	}
	return b == [4]byte{0x7f, 'E', 'L', 'F'}
}

// Outputs is every path a build writes, for `--clean`.
//
// ⚠ It returns the MANIFEST too. A clean that left it behind would describe
// binaries that are gone.
func Outputs() []string {
	var out []string
	for _, a := range All() {
		out = append(out, a.OutPath())
	}
	out = append(out, ManifestPath)
	return out
}

// UnderOut says whether a repo-relative path is inside the output directory.
//
// ⛔ Used by the cleaner, which may remove only what a build wrote. It is a
// prefix test on a CLEANED path, so `bin/../scripts` does not pass it.
func UnderOut(rel string) bool {
	c := path.Clean(filepath.ToSlash(rel))
	return c == OutDir || strings.HasPrefix(c, OutDir+"/")
}

// SafeToRemove says whether a repo-relative path is one a build may delete,
// and returns the reason it is not when it is not.
//
// ⛔ **This tree has a DATE for the defect this exists to stop.** A script
// ended with a recursive removal of a variable that pointed at the Windows
// drive mount and 29,339 files went in one call; `rm` there does not use the
// recycle bin. The three layers that answer for it are
// `scripts/common/lib/guard.sh`, `./check removals` and a snapshot taken
// outside the tree before anything runs.
//
// ⚠ **None of the three reaches this code.** `./check removals` reads tracked
// SHELL SCRIPTS, so the one Go program in this tree that deletes things had
// less standing between it and the checkout than the shell does. The rules are
// the guard's own, applied here to a path rather than to a command:
//
//	an EMPTY path joins to the checkout itself
//	an ABSOLUTE path is not this build's to touch
//	a path with a parent segment leaves the directory it claims to be in
//	a path that is not the output directory or under it, or one of the
//	artefacts this table names, is not something a build wrote
func SafeToRemove(rel string) (bool, string) {
	if strings.TrimSpace(rel) == "" {
		return false, "an empty path, which joins to the checkout itself"
	}
	if filepath.IsAbs(rel) || strings.HasPrefix(rel, "/") {
		return false, "an absolute path, which is not this build's to touch"
	}
	c := path.Clean(filepath.ToSlash(rel))
	if c == "." || c == ".." {
		return false, "the checkout itself"
	}
	for _, seg := range strings.Split(c, "/") {
		if seg == ".." {
			return false, "a parent segment, which leaves the directory it claims to be in"
		}
	}
	if UnderOut(c) {
		return true, ""
	}
	for _, a := range All() {
		if c == path.Clean(filepath.ToSlash(a.OutPath())) {
			return true, ""
		}
		if tw := a.StaleTwin(); tw != "" && c == path.Clean(filepath.ToSlash(tw)) {
			return true, ""
		}
	}
	return false, "not the output directory, nor anything this build's table produces"
}
