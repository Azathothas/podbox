// dispatch.go - the wrapper itself: pgb invoked as cc, gcc, c++, g++ or cpp.
//
// It reads the manifest written when the wrapper directory was created and
// execs the real compiler. No shell is started and no flags are recomputed, so
// a build system calling it thousands of times pays one file read and one
// exec.
//
// SPDX-License-Identifier: 0BSD
package wrapper

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"syscall"

	"github.com/Azathothas/pg-toolkit/internal/cfg"
	"github.com/Azathothas/pg-toolkit/internal/elfx"
)

// mode is what a command line turns out to be.
type mode int

const (
	modeLink mode = iota
	modeCompile
	modeShared
	modeQuery // -print-*, --version and friends: never decorated
)

// classify decides what an argv means. A query anywhere on the line wins,
// because configure, libtool and CMake parse the output of those and a flag
// pgb added would change what they read.
func classify(args []string) mode {
	m := modeLink
	for _, a := range args {
		switch a {
		case "-c", "-E", "-S", "-M", "-MM":
			if m == modeLink {
				m = modeCompile
			}
			continue
		case "-shared", "-dynamiclib":
			m = modeShared
			continue
		case "-dumpmachine", "-dumpversion", "-dumpspecs", "--version", "-V":
			return modeQuery
		}
		if strings.HasPrefix(a, "-print-") || strings.HasPrefix(a, "--print-") {
			return modeQuery
		}
	}
	return m
}

// rewriteNative replaces the one caller flag pgb overrides. -march=native bakes
// in the build machine's CPU and the binary then dies elsewhere with SIGILL.
// Any other -march the caller passes wins: a project compiling one translation
// unit per ISA level behind a runtime check is doing the right thing.
func rewriteNative(args []string, baseline string) ([]string, bool) {
	changed := false
	out := make([]string, 0, len(args))
	for _, a := range args {
		switch a {
		case "-march=native":
			changed = true
			if baseline != "" {
				out = append(out, "-march="+baseline)
			} else {
				out = append(out, "-mtune=generic")
			}
		case "-mtune=native", "-mcpu=native":
			changed = true
			out = append(out, "-mtune=generic")
		default:
			out = append(out, a)
		}
	}
	return out, changed
}

// callerLibs re-emits the caller's own -l and -L after the plugin objects.
// An object's undefined references can only be resolved by a library that
// comes after it, and the caller cannot fix that because they do not control
// where pgb puts those objects. Repeating a -l is safe: an archive member is
// pulled in once. Nothing is invented, so a plugin needing a library the
// program never named still fails.
func callerLibs(args []string) []string {
	var out []string
	expectValue := ""
	for _, a := range args {
		if expectValue != "" {
			out = append(out, expectValue, a)
			expectValue = ""
			continue
		}
		switch {
		case a == "-l" || a == "-L":
			expectValue = a
		case strings.HasPrefix(a, "-l") || strings.HasPrefix(a, "-L"):
			out = append(out, a)
		}
	}
	return out
}

// ⭐ the c++ runtime a c link turns out to need. TODO T-063.
//
// ⛔ the defect: the driver is chosen by argv[0], so `cc` links get the C link
// line even when an ARCHIVE on that line needs `operator delete` and the
// `__cxxabiv1` vtables. `libicuuc.a` is the case this was found on - a C
// program, a C compiler, and a link that fails on a wall of undefined `_Zd*`
// and `__cxa_*` names naming no file the developer wrote.
//
// ⚠ only the inputs are read, and only on a LINK. A compile never reaches
// here, and a build system that calls the compiler ten thousand times pays
// this on the handful of invocations that actually link. The scan
// short-circuits at the first marker.
//
// ⛔ AND `-nostdlib`/`-nodefaultlibs` suppress it. A caller who has said it is
// supplying its own runtime has said so deliberately, and adding one behind
// its back is the opposite of what this tool does.
//
// ⛔ It also resolves `-lNAME` against `-L`. Skipping every argument that
// begins with `-` sees only archives named as literal paths, which passes a
// synthetic subject and fails a real one. ⚠ Real builds do not name them that
// way. Measured on postgres 18.6, from its own generated
// `src/Makefile.global`:
//
//	ICU_LIBS = -L/.../nix-prefix/lib -licui18n -licuuc -licudata -lpthread -lm
//
// Every one of those starts with `-`, `libicuuc.a` was never opened, and the
// link died on a wall of `undefined reference to 'operator delete(void*,
// unsigned long)'` and `vtable for __cxxabiv1::__si_class_type_info` - the
// exact symbols this function exists to anticipate. ⭐ The selftest passed
// throughout, because its fixture links `cc -o prog main.c libcxxthing.a`,
// a literal path.
//
// ⚠ ONLY `.a` IS resolved from `-l`, and only out of the `-L` directories.
// A shared library carries its own `DT_NEEDED` on libstdc++ and needs nothing
// from us; and not searching the default system directories keeps this off
// `/usr/lib`'s archives, which bounds what a link pays for the scan.
func cxxRuntimeDemand(args []string) (string, string) {
	for _, a := range args {
		if a == "-nostdlib" || a == "-nodefaultlibs" || a == "-nostartfiles" {
			return "", ""
		}
	}
	for _, group := range cxxCandidates(args) {
		for _, p := range group {
			if fi, err := os.Stat(p); err != nil || fi.IsDir() {
				continue
			}
			if need, sym := elfx.NeedsCXXRuntime(p); need {
				return p, sym
			}
			// The first -L directory holding the name is the one ld takes, so
			// a later directory's same-named archive is not this link's.
			break
		}
	}
	return "", ""
}

// cxxCandidates is the files cxxRuntimeDemand would open, in link order, one
// group per argument that names something.
//
// ⛔ it IS split out because the selftest could not see the defect without it.
// `wrapper-flags` is the pure half - no compiler - so every path it names is
// deliberately non-existent, and `NeedsCXXRuntime` answers "no" for a file it
// cannot read. ⚠ **A path that is considered and a path that is skipped
// therefore give the same answer**, which is exactly why the block asserted
// only the suppression rule, and exactly why `-licuuc` being skipped was
// invisible for as long as it was. One case in it even read *"a flag is never
// opened as an input"* - the defect written down as the intent.
//
// ⭐ Returning the candidates makes "considered" observable without a
// filesystem, so the rule can be pinned rather than inferred from an outcome
// that is "no" either way.
func cxxCandidates(args []string) [][]string {
	var libDirs []string
	for i, a := range args {
		switch {
		case a == "-L":
			if i+1 < len(args) {
				libDirs = append(libDirs, args[i+1])
			}
		case strings.HasPrefix(a, "-L"):
			if d := strings.TrimPrefix(a, "-L"); d != "" {
				libDirs = append(libDirs, d)
			}
		}
	}
	inDirs := func(name string) []string {
		var out []string
		for _, d := range libDirs {
			out = append(out, filepath.Join(d, name))
		}
		return out
	}
	// ⛔ one resolver for both spellings of -l, and that IS the point.
	//
	// This scan already had a `-L dir` case beside its `-Ldir` case and did
	// NOT have the same pair for `-l`: the separated form fell into the
	// "this flag's value is not an input" branch, which is true of `-o` and
	// `-L` and false of `-l`, whose value is the library to resolve. GNU ld
	// documents `-l namespec` with a space and gcc passes it through, so a
	// build system emitting it got the whole argument skipped -- the exact
	// behaviour this function was written to fix.
	//
	// Keeping the two spellings on one code path is what stops them drifting
	// apart again; `wrapper-flags` pins both.
	namespec := func(ns string) []string {
		if rest, ok := strings.CutPrefix(ns, ":"); ok {
			// `-l:libfoo.a` names the file exactly.
			return inDirs(rest)
		}
		// ⚠ Only `.a`. A shared library carries its own DT_NEEDED on
		// libstdc++ and needs nothing from us.
		return inDirs("lib" + ns + ".a")
	}
	var groups [][]string
	skipNext := false
	for i, a := range args {
		if skipNext {
			skipNext = false
			continue
		}
		var g []string
		switch {
		case a == "-l":
			// ⭐ The value IS an input, unlike -L's and -o's.
			if i+1 < len(args) {
				g = namespec(args[i+1])
				skipNext = true
			}
		case a == "-L" || a == "-o":
			// The value is the next argument and is not an input.
			skipNext = i+1 < len(args)
		case strings.HasPrefix(a, "-l"):
			g = namespec(strings.TrimPrefix(a, "-l"))
		case strings.HasPrefix(a, "-"):
			// Any other flag names no input.
		case strings.HasSuffix(a, ".a"), strings.HasSuffix(a, ".o"):
			g = []string{a}
		}
		if len(g) > 0 {
			groups = append(groups, g)
		}
	}
	return groups
}

// findManifest locates the wrapper directory this invocation came from.
func findManifest() (*Manifest, string, error) {
	if strings.ContainsRune(os.Args[0], os.PathSeparator) {
		dir := filepath.Dir(os.Args[0])
		if m, err := LoadManifest(dir); err == nil {
			return m, dir, nil
		}
	}
	if dir := os.Getenv("PGT_BINARY_WRAPPER_DIR"); dir != "" {
		if m, err := LoadManifest(dir); err == nil {
			return m, dir, nil
		}
	}
	for _, dir := range filepath.SplitList(os.Getenv("PATH")) {
		if dir == "" {
			continue
		}
		if m, err := LoadManifest(dir); err == nil {
			return m, dir, nil
		}
	}
	return nil, "", fmt.Errorf("no %s beside this wrapper, in $PGT_BINARY_WRAPPER_DIR, or on $PATH", ManifestName)
}

// Dispatch runs one wrapper invocation. It does not return on success: the
// real compiler replaces this process.
func Dispatch(name string, args []string) int {
	m, dir, err := findManifest()
	if err != nil {
		fmt.Fprintf(os.Stderr, "%s %s wrapper: %v\n", cfg.Prog(), name, err)
		return 2
	}
	real := m.Real[name]
	if real == "" {
		fmt.Fprintf(os.Stderr, "%s %s wrapper: %s/%s names no real compiler for %q\n", cfg.Prog(),
			name, dir, ManifestName, name)
		return 2
	}
	verbose := os.Getenv("PGT_BINARY_VERBOSE") != "" || os.Getenv("PGT_LOG") == "debug" ||
		os.Getenv("PGT_LOG") == "trace"

	// The preprocessor is a query tool: cpp never links, and decorating it
	// makes a configure test about predefined macros answer nonsense. -march
	// changes the predefined macro set, so the compile flags are dropped too.
	if name == "cpp" {
		return execReal(real, args, verbose, "passthrough")
	}

	md := classify(args)
	if md == modeQuery || md == modeShared {
		what := "query"
		if md == modeShared {
			what = "shared,passthrough"
		}
		return execReal(real, args, verbose, what)
	}

	args, rewrote := rewriteNative(args, m.Baseline)
	if rewrote && verbose {
		fmt.Fprintf(os.Stderr, "pgb[rewrote -march=native to %s]\n", m.Baseline)
	}

	var argv []string
	switch md {
	case modeCompile:
		argv = append(argv, m.Compile...)
		argv = append(argv, args...)
		if verbose {
			fmt.Fprintf(os.Stderr, "pgb[compile] %s %s\n", real, strings.Join(argv, " "))
		}
	default:
		link := m.Link
		cxxDriver := name == "c++" || name == "g++"
		if cxxDriver {
			link = m.LinkCXX
		}
		argv = append(argv, m.Compile...)
		argv = append(argv, args...)
		argv = append(argv, link...)
		// ⛔ appended after the link flags, because a single-pass linker
		// resolves an archive where it appears: -lstdc++ ahead of the archive
		// that needs it resolves nothing.
		if !cxxDriver {
			if from, sym := cxxRuntimeDemand(args); from != "" {
				argv = append(argv, "-lstdc++", "-lm")
				if verbose {
					fmt.Fprintf(os.Stderr,
						"pgb[added -lstdc++: %s has an undefined %s and this is a C link]\n",
						from, sym)
				}
			}
		}
		if m.WrapDlopen {
			argv = append(argv, callerLibs(args)...)
		}
		if verbose {
			fmt.Fprintf(os.Stderr, "pgb[link] %s %s\n", real, strings.Join(argv, " "))
		}
	}
	return execArgv(real, argv)
}

func execReal(real string, args []string, verbose bool, what string) int {
	if verbose {
		fmt.Fprintf(os.Stderr, "pgb[%s] %s %s\n", what, real, strings.Join(args, " "))
	}
	return execArgv(real, args)
}

func execArgv(real string, args []string) int {
	argv := append([]string{real}, args...)
	if err := syscall.Exec(real, argv, os.Environ()); err != nil {
		fmt.Fprintf(os.Stderr, "%s wrapper: cannot exec %s: %v\n", cfg.Prog(), real, err)
		return 2
	}
	return 0
}
