// plan_selftest.go - the drift arithmetic, driven without a toolchain.
//
// ⛔ **The properties here are the ones a build tool gets wrong silently.** A
// digest that ignores a rename, a classifier that calls an unreadable input
// current, a cleaner that would remove a path outside its own directory: each
// produces a build that looks like it worked.
//
// ⚠ Offline, and it needs neither `go` nor `cargo`: every case builds its own
// files in a temporary directory. The parts that DO shell out - `go list`,
// `cargo` - live in cmd/build, because a selftest that needed a toolchain
// could not run in the place this one has to.
//
// SPDX-License-Identifier: 0BSD
package buildplan

import (
	"errors"
	"os"
	"path/filepath"
	"strings"

	"github.com/Azathothas/pg-toolkit/internal/selftest"
)

// Selftest asserts the table, the digest, the classifier and the cleaner's
// boundary.
func Selftest() *selftest.Report {
	r := selftest.New("buildplan")

	// --- the table -------------------------------------------------------
	//
	// ⛔ Every name unique and every output unique. Two artefacts writing one
	// path is a build where the second silently replaces the first, and the
	// only symptom is the wrong program under a familiar name.
	names := map[string]bool{}
	outs := map[string]bool{}
	dupName, dupOut := "", ""
	for _, a := range All() {
		if names[a.Name] {
			dupName = a.Name
		}
		if outs[a.Out] {
			dupOut = a.Out
		}
		names[a.Name], outs[a.Out] = true, true
	}
	r.Check("no two artefacts share a name", dupName, "")
	r.Check("no two artefacts write one path", dupOut, "")
	r.CheckBool("every artefact has a source and an output", allHaveOutputs(), true)
	r.CheckBool("every artefact says what it is", allHaveWhat(), true)

	// ⭐ pstrip is what strips the rest, so it has to be built before them. A
	// table reordered without noticing would leave every later artefact
	// unstripped, which nothing else here would report.
	r.CheckBool("pstrip comes before the artefacts it strips", pstripIsBeforeRust(), true)

	// ⛔ The four documented root binaries are FROZEN OUTPUT. Every page,
	// script and check in this tree runs ./check, ./pgb, ./pg-devenv, so a
	// table that moved one into bin/ would break all of them at once.
	for _, n := range []string{"pgb", "check", "pg-devenv", "pg-toolkit"} {
		a, ok := Find(n)
		r.CheckBool("the table carries "+n, ok, true)
		if ok {
			r.Check(n+" stays at the repository root", a.Out, n)
		}
	}
	_, ok := Find("no-such-artefact")
	r.CheckBool("a name the table does not carry is not found", ok, false)

	// --- one spelling per artefact ---------------------------------------
	//
	// ⛔ The output path carries no extension on ANY host, and the stale twin
	// is what a build removes. Both branches are driven here rather than on
	// two machines: with two files present the shell resolves the
	// extensionless one, so a `.exe` left beside a fresh build is a stale
	// binary answering to the name.
	realHost := HostIsWindows
	defer func() { HostIsWindows = realHost }()

	HostIsWindows = func() bool { return false }
	gob, _ := Find("check")
	r.Check("on a non-Windows host the output is extensionless", gob.OutPath(), "check")
	r.Check("and there is no twin to remove", gob.StaleTwin(), "")

	HostIsWindows = func() bool { return true }
	r.Check("on Windows the output is STILL extensionless", gob.OutPath(), "check")
	r.Check("and the twin a build removes is the .exe", gob.StaleTwin(), "check.exe")

	rustb, _ := Find("pstrip")
	r.Check("a Rust artefact has no twin even on Windows", rustb.StaleTwin(), "")
	HostIsWindows = realHost

	// ⛔ What is NOT built is listed rather than omitted. A build listing that
	// simply did not mention tool/runtime is how a session concludes the tree
	// does not carry it.
	r.CheckBool("the runtime C is named as not-an-artefact", len(NotBuilt()) > 0, true)

	// --- the digest ------------------------------------------------------
	dir, err := os.MkdirTemp("", "buildplan")
	if err != nil {
		r.Skip("no temporary directory: " + err.Error())
		return r
	}
	defer os.RemoveAll(dir)

	write := func(rel, body string) {
		p := filepath.Join(dir, filepath.FromSlash(rel))
		_ = os.MkdirAll(filepath.Dir(p), 0o755)
		_ = os.WriteFile(p, []byte(body), 0o644)
	}
	write("a/one.go", "package a\n")
	write("a/two.go", "package a // two\n")

	base, err := SetDigest(dir, []string{"a/one.go", "a/two.go"})
	if err != nil {
		r.Skip("the digest could not be taken: " + err.Error())
		return r
	}
	r.CheckInt("a digest is 64 hex characters", len(base), 64)

	// ⚠ ORDER must not matter. `go list` returns packages in dependency
	// order, so a digest that depended on the caller's order would change
	// when an unrelated import was added, and rebuild everything.
	rev, _ := SetDigest(dir, []string{"a/two.go", "a/one.go"})
	r.Check("the order the caller lists files in does not change it", rev, base)

	write("a/two.go", "package a // TWO\n")
	moved, _ := SetDigest(dir, []string{"a/one.go", "a/two.go"})
	r.CheckBool("changing a byte changes it", moved != base, true)

	// ⛔ A RENAME moves it, and this is the case a content-only digest gets
	// wrong. AGENTS.md carries what moving a runtime source cost this tree:
	// the bytes were identical and the build was not.
	write("a/two.go", "package a // two\n")
	write("a/three.go", "package a // two\n")
	renamed, _ := SetDigest(dir, []string{"a/one.go", "a/three.go"})
	r.CheckBool("renaming a file changes it, with the same bytes", renamed != base, true)

	withTC, _ := SetDigest(dir, []string{"a/one.go", "a/two.go"}, "go1.24")
	other, _ := SetDigest(dir, []string{"a/one.go", "a/two.go"}, "go1.25")
	r.CheckBool("the toolchain is part of the input set", withTC != other, true)
	r.CheckBool("and a toolchain string is not the same as none", withTC != base, true)

	_, err = SetDigest(dir, []string{"a/absent.go"})
	r.CheckBool("a missing input refuses rather than hashing nothing", err != nil, true)

	// --- crate files -----------------------------------------------------
	write("crate/Cargo.toml", "[package]\n")
	write("crate/src/main.rs", "fn main() {}\n")
	write("crate/target/release/thing", "an object cargo wrote\n")
	files, err := CrateFiles(dir, "crate")
	if err != nil {
		r.Skip("the crate walk failed: " + err.Error())
		return r
	}
	r.CheckInt("the crate walk finds the sources", len(files), 2)
	// ⛔ target/ excluded. Including it makes every build change the next
	// build's input set, so the tree is never up to date and always rebuilds.
	r.CheckBool("and it does not descend into target/", anyHasSegment(files, "target"), false)

	// --- the classifier --------------------------------------------------
	a := Artefact{Name: "thing", Out: "bin/thing"}
	m := Manifest{Schema: Schema, Artefacts: map[string]Record{}}

	// ⛔ THE ONE THAT MUST NOT READ AS CURRENT. An input digest that could not
	// be taken says so; the safe-looking answer is the dangerous one, and it
	// is the vacuous pass this repository keeps meeting.
	st := Classify(dir, a, "", errors.New("no go on PATH"), m)
	r.Check("an input set that cannot be read is not called up to date", string(st.State), string(Unknown))
	r.CheckBool("and it says why", st.Why != "", true)

	st = Classify(dir, a, "abc", nil, m)
	r.Check("no binary on disk reads as not built", string(st.State), string(Absent))

	write("bin/thing", "a binary\n")
	st = Classify(dir, a, "abc", nil, m)
	r.Check("a binary with no record reads as outdated", string(st.State), string(Stale))

	// ⛔ THE FALSE GREEN THIS TREE ACTUALLY PRODUCED, twice. A record that
	// matches the INPUTS says nothing about the file on disk: while this tool
	// was being written, a change to an output name left an older binary
	// there and the run reported "up to date" over it.
	m.Artefacts["thing"] = Record{Inputs: "abc", Output: "bin/thing"}
	st = Classify(dir, a, "abc", nil, m)
	r.Check("a record that does not name a binary reads as outdated", string(st.State), string(Stale))
	r.CheckBool("and it says the record does not say which binary",
		strings.Contains(st.Why, "does not say which binary"), true)

	made, derr := FileDigest(filepath.Join(dir, "bin", "thing"))
	r.CheckBool("the binary on disk can be digested", derr == nil, true)
	m.Artefacts["thing"] = Record{Inputs: "abc", Made: made, Output: "bin/thing"}
	st = Classify(dir, a, "abc", nil, m)
	r.Check("a record that names this exact binary reads as up to date", string(st.State), string(Current))

	st = Classify(dir, a, "def", nil, m)
	r.Check("a binary whose sources moved reads as outdated", string(st.State), string(Stale))
	r.CheckBool("and it says a source changed", strings.Contains(st.Why, "source changed"), true)

	write("bin/thing", "a DIFFERENT binary\n")
	st = Classify(dir, a, "abc", nil, m)
	r.Check("a binary something else replaced reads as outdated", string(st.State), string(Stale))
	r.CheckBool("and it says the binary is not the recorded one",
		strings.Contains(st.Why, "not the one this record describes"), true)

	// --- the manifest round trip ------------------------------------------
	m2 := Manifest{Schema: Schema, Artefacts: map[string]Record{
		"thing": {Inputs: "abc", Output: "bin/thing", Bytes: 7, Stripped: true, Built: "2026-09-07T00:00:00Z"},
	}}
	if err := SaveManifest(dir, m2); err != nil {
		r.Skip("the manifest could not be written: " + err.Error())
		return r
	}
	back := LoadManifest(dir)
	r.Check("the manifest round trips", back.Artefacts["thing"].Inputs, "abc")
	r.CheckBool("and it carries whether the artefact was stripped", back.Artefacts["thing"].Stripped, true)

	// ⛔ A manifest from another schema is DISCARDED rather than misread, and
	// the consequence is a rebuild - which is the safe direction.
	mp := filepath.Join(dir, filepath.FromSlash(ManifestPath))
	_ = os.WriteFile(mp, []byte(`{"schema":"other/9","artefacts":{"thing":{"inputs":"abc"}}}`), 0o644)
	r.CheckInt("a manifest of another schema is discarded", len(LoadManifest(dir).Artefacts), 0)

	_ = os.WriteFile(mp, []byte("not json at all"), 0o644)
	r.CheckInt("and so is one that does not parse", len(LoadManifest(dir).Artefacts), 0)

	// --- the ELF probe ----------------------------------------------------
	//
	// ⭐ Read from the FILE rather than assumed from GOOS: a cross-compiled
	// Linux binary on a Windows host is an ELF and a host build is not.
	write("elf", "\x7fELF and then some")
	write("pe", "MZ and then some")
	write("tiny", "ab")
	r.CheckBool("an ELF is recognised by its bytes", IsELF(filepath.Join(dir, "elf")), true)
	r.CheckBool("a PE is not", IsELF(filepath.Join(dir, "pe")), false)
	r.CheckBool("a file too short to have magic is not", IsELF(filepath.Join(dir, "tiny")), false)
	r.CheckBool("nor is an absent file", IsELF(filepath.Join(dir, "absent")), false)

	// --- what the cleaner may touch ---------------------------------------
	//
	// ⛔ The cleaner removes only what is under the output directory, and a
	// traversal must not pass. TODO/RULES.md and `./check removals` exist
	// because a script once deleted the checkout.
	r.CheckBool("the output directory is inside itself", UnderOut("bin"), true)
	r.CheckBool("a path under it is inside it", UnderOut("bin/pstrip"), true)
	r.CheckBool("a traversal out of it is not", UnderOut("bin/../scripts"), false)
	r.CheckBool("a sibling that shares its prefix is not", UnderOut("binaries/x"), false)
	r.CheckBool("the repository root is not", UnderOut("."), false)
	r.CheckBool("and neither is an absolute path", UnderOut("/bin"), false)

	// The root binaries are NOT under it, which is what stops a clean from
	// treating them as build scratch.
	for _, n := range []string{"pgb", "check", "pg-devenv", "pg-toolkit"} {
		r.CheckBool(n+" is not inside the output directory", UnderOut(n), false)
	}

	// --- what a build may actually delete ---------------------------------
	//
	// ⛔ **This tree has a DATE for the defect these refuse.** A script ended
	// with a recursive removal of a variable pointing at a drive mount and
	// 29,339 files went in one call. `./check removals` polices tracked SHELL
	// scripts and reads no Go, so these cases are the only thing standing
	// between `--clean` and the checkout.
	safe := func(p string) bool { ok, _ := SafeToRemove(p); return ok }
	why := func(p string) string { _, w := SafeToRemove(p); return w }

	r.CheckBool("the output directory may be removed", safe("bin"), true)
	r.CheckBool("something under it may be removed", safe("bin/pstrip"), true)
	r.CheckBool("an artefact the table names may be removed", safe("pgb"), true)

	// ⛔ THE ONE WITH THE DATE. An empty path joins to the checkout itself,
	// so `RemoveAll(filepath.Join(repo, ""))` is `RemoveAll(repo)`.
	r.CheckBool("an EMPTY path is refused", safe(""), false)
	r.CheckBool("and it says why", strings.Contains(why(""), "checkout itself"), true)
	r.CheckBool("whitespace is not a path either", safe("   "), false)
	r.CheckBool("the checkout itself is refused", safe("."), false)
	r.CheckBool("so is its parent", safe(".."), false)
	r.CheckBool("an absolute path is refused", safe("/usr"), false)
	r.CheckBool("a traversal out of the output directory is refused", safe("bin/../scripts"), false)
	r.CheckBool("a traversal anywhere in the path is refused", safe("bin/../../etc"), false)
	r.CheckBool("a sibling sharing the prefix is refused", safe("binaries"), false)
	r.CheckBool("a source directory is refused", safe("internal"), false)
	r.CheckBool("a tracked file is refused", safe("Makefile"), false)
	r.CheckBool("and the refusal says it is not what a build wrote",
		strings.Contains(why("Makefile"), "this build's table produces"), true)

	// Every output the cleaner would remove is either under the output
	// directory or a root binary this table names. Nothing else.
	stray := ""
	for _, o := range Outputs() {
		if UnderOut(o) {
			continue
		}
		if _, known := Find(strings.TrimPrefix(o, "./")); !known {
			stray = o
		}
	}
	r.Check("the cleaner's list names nothing outside the table", stray, "")

	return r
}

func allHaveOutputs() bool {
	for _, a := range All() {
		if a.Out == "" || a.Src == "" {
			return false
		}
	}
	return true
}

func allHaveWhat() bool {
	for _, a := range All() {
		if a.What == "" {
			return false
		}
	}
	return true
}

// pstripIsBeforeRust says whether pstrip is built before every other Rust
// artefact, which is what lets it strip them in the same run.
func pstripIsBeforeRust() bool {
	seen := false
	for _, a := range All() {
		if a.Name == "pstrip" {
			seen = true
			continue
		}
		if a.Kind == Rust && !seen {
			return false
		}
	}
	return seen
}

func anyHasSegment(list []string, seg string) bool {
	for _, s := range list {
		for _, part := range strings.Split(filepath.ToSlash(s), "/") {
			if part == seg {
				return true
			}
		}
	}
	return false
}
