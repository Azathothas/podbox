// proc_selftest.go - the exit-code contract and the environment overlay,
// asserted in the binary rather than by running a build.
//
// ⛔ why this package needed one. Every child process pgb starts goes through
// here, and two of its functions decide things the rest of the tree reads as
// fact:
//
//   - `classify` turns a *exec.Cmd's outcome into the 0 / exit N / 128+signal
//     contract `docs/AGENTS.md` section 0b states and every POC decodes  -
//     `poc/common.sh` reads `SIG$((st-128))` straight out of it. A child killed
//     by a signal that came back as a plain non-zero exit would turn every
//     SIGSEGV in the matrix into an ordinary failure, and the tables that
//     distinguish `SIG11` from `exit1` are this project's main instrument.
//   - `mergeEnv` is the engine boundary. `pg-toolkit binary build` re-enters itself inside the
//     environment and the options travel through the environment, so an overlay
//     that appended a duplicate instead of replacing, or dropped an entry,
//     would silently build with the wrong options. ⭐ That defect class has
//     been realised here once: two builds shared a wrapper directory, the flag
//     sets overwrote each other, and neither build reported anything.
//
// ⚠ what IS not covered, said plainly rather than implied. `Run`, `Output` and
// `runStamped` start real children; the cases below use `/bin/sh` for the ones
// whose whole point is a real process outcome, and nothing here exercises
// `Exec` (it replaces this process) or the stamped path's goroutines.
//
// SPDX-License-Identifier: 0BSD
package proc

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/Azathothas/pg-toolkit/internal/selftest"
)

// Selftest asserts the exit-code contract and the environment overlay.
func Selftest() *selftest.Report {
	r := selftest.New("proc")

	// ---- mergeEnv: the engine boundary -----------------------------------
	join := func(in []string) string { return strings.Join(in, " ") }

	r.Check("an added variable is appended",
		join(mergeEnv([]string{"A=1"}, []string{"B=2"})), "A=1 B=2")

	// ⛔ replaced in place, not appended. A duplicate NAME= in an environment is
	// resolved by the last one on Linux, so an append would happen to work  -
	// until something reads the list rather than the process's environment,
	// which is exactly what ContainerEnvArgs and the chroot bind list do.
	r.Check("an existing variable is REPLACED, keeping its position",
		join(mergeEnv([]string{"A=1", "B=2", "C=3"}, []string{"B=9"})), "A=1 B=9 C=3")

	r.Check("two adds, one replacing and one new",
		join(mergeEnv([]string{"A=1", "B=2"}, []string{"A=9", "C=3"})), "A=9 B=2 C=3")

	// ⚠ An entry with no `=` is not a variable. Passing it through would put a
	// bare word in the child's environment array.
	r.Check("an add with no '=' is dropped",
		join(mergeEnv([]string{"A=1"}, []string{"NOTAVAR", "B=2"})), "A=1 B=2")

	// ⭐ An empty value is not the same as absent, and confusing the two has
	// cost this tree a check that could not fail: `getenv` returns a non-NULL empty
	// string, a caller takes the branch, and the selftest that was supposed to
	// catch it read the value instead of the presence.
	r.Check("a variable can be overlaid with an EMPTY value",
		join(mergeEnv([]string{"A=1"}, []string{"A="})), "A=")

	// The base is not modified: callers pass os.Environ() and reuse it.
	base := []string{"A=1", "B=2"}
	_ = mergeEnv(base, []string{"A=9"})
	r.Check("mergeEnv does not modify its base", join(base), "A=1 B=2")

	// ---- the exit-code contract, on real children ------------------------
	if !Look("/bin/sh") && !Look("sh") {
		r.Skip("proc: no /bin/sh, so the exit-code contract was not measured")
	} else {
		res, err := (&Cmd{Argv: []string{"/bin/sh", "-c", "exit 0"}}).Output()
		if err != nil {
			r.Fail("a child that exits 0", err.Error(), "code 0")
		} else {
			r.CheckInt("a child that exits 0 reports code 0", res.Code, 0)
			r.CheckInt("...and signal 0", res.Signal, 0)
			r.CheckBool("...and Failed() is false", res.Failed(), false)
		}

		res, err = (&Cmd{Argv: []string{"/bin/sh", "-c", "exit 7"}}).Output()
		if err != nil {
			r.Fail("a child that exits 7", err.Error(), "code 7")
		} else {
			r.CheckInt("a child that exits 7 reports code 7", res.Code, 7)
			r.CheckInt("...and signal 0, because it was not signalled", res.Signal, 0)
			r.CheckBool("...and Failed() is true", res.Failed(), true)
		}

		// ⛔ the row every matrix table in this tree depends on. A child killed
		// by SIGKILL must come back as 128+9, with Signal set - not as a plain
		// non-zero exit. `kill -9 $$` from inside the shell kills the shell
		// itself, so the outcome is the shell's own death rather than a child's.
		res, err = (&Cmd{Argv: []string{"/bin/sh", "-c", "kill -9 $$"}}).Output()
		if err != nil {
			r.Fail("a child killed by SIGKILL", err.Error(), "code 137")
		} else {
			r.CheckInt("a child killed by SIGKILL reports 128+9", res.Code, 137)
			r.CheckInt("...and names the signal", res.Signal, 9)
		}

		// Capture trims trailing newlines and nothing else.
		out, err := Capture("/bin/sh", "-c", "printf 'a b\\n\\n'")
		if err != nil {
			r.Fail("Capture on a successful child", err.Error(), "a b")
		} else {
			r.Check("Capture strips trailing newlines only", out, "a b")
		}

		// ⚠ A caller asking for a VALUE has no use for a partial one, so a
		// non-zero exit is an error here even though Run reports it in Result.
		if _, err := Capture("/bin/sh", "-c", "echo out; exit 3"); err == nil {
			r.Fail("Capture on a failing child", "no error", "an error")
		} else {
			r.CheckBool("Capture turns a non-zero exit into an error", true, true)
			r.CheckBool("...and the message names the status",
				strings.Contains(err.Error(), "exit 3"), true)
		}

		// ...and CaptureAllowFail does not, because there the failure is the
		// information the caller wanted.
		out, code := CaptureAllowFail("/bin/sh", "-c", "echo probed; exit 4")
		r.Check("CaptureAllowFail returns the output anyway", out, "probed")
		r.CheckInt("...and the status beside it", code, 4)

		// ⛔ EnvOnly starts from an empty environment. Without it a child would
		// inherit the caller's, and the engine boundary's whole point is that
		// what crosses it is explicit.
		out, err = Capture2(&Cmd{
			Argv:    []string{"/bin/sh", "-c", "echo \"[${PGT_SELFTEST_MARK-unset}]\""},
			Env:     []string{"PATH=/usr/bin:/bin"},
			EnvOnly: true,
		})
		_ = os.Setenv("PGT_SELFTEST_MARK", "leaked")
		defer os.Unsetenv("PGT_SELFTEST_MARK")
		out2, err2 := Capture2(&Cmd{
			Argv:    []string{"/bin/sh", "-c", "echo \"[${PGT_SELFTEST_MARK-unset}]\""},
			Env:     []string{"PATH=/usr/bin:/bin"},
			EnvOnly: true,
		})
		if err != nil || err2 != nil {
			r.Skip("proc: EnvOnly probe could not run")
		} else {
			r.Check("EnvOnly hides the parent's environment", out, "[unset]")
			r.Check("...even when the variable IS set in the parent", out2, "[unset]")
		}
		out3, err3 := Capture2(&Cmd{
			Argv: []string{"/bin/sh", "-c", "echo \"[${PGT_SELFTEST_MARK-unset}]\""},
		})
		if err3 == nil {
			r.Check("...and without EnvOnly the parent's environment IS inherited",
				out3, "[leaked]")
		}
	}

	// ---- the guards ------------------------------------------------------
	// ⛔ An empty argv must be an error and not a panic: c.build() indexes
	// Argv[0] with no check of its own, so this guard is the only thing between
	// a caller's mistake and a crash inside the tool.
	if _, err := (&Cmd{}).Run(); err == nil {
		r.Fail("an empty argv", "no error", "an error")
	} else {
		r.Check("an empty argv is an error, not a panic", err.Error(), "proc: empty argv")
	}

	// Shell composes /bin/sh -c SCRIPT, with extra arguments becoming $0, $1...
	sc := Shell("echo \"$0\"", "zero")
	r.Check("Shell builds /bin/sh -c SCRIPT ARGS",
		strings.Join(sc.Argv, "|"), "/bin/sh|-c|echo \"$0\"|zero")
	r.Check("...and names itself so the log line says which subsystem ran it",
		sc.Subsys, "sh")

	r.CheckBool("Look finds a program that exists", Look("/bin/sh"), true)
	r.CheckBool("Look does not find one that does not",
		Look("pgb-no-such-program-anywhere"), false)

	// ---- ⛔ the program IS resolved against the child's PATH ---------------
	//
	// ⛔ the defect this section exists for. `exec.Command` runs LookPath
	// against the CALLER's environment; the Env set on the Cmd is applied
	// afterwards. `buildx.Inner` puts the wrapper directory on the child's PATH
	// - that is how the entire toolchain is injected - so
	// `pg-toolkit binary build -- cc foo.c` resolved `cc` WITHOUT it and ran the real
	// compiler with none of pgb's flags.
	//
	// ⚠ It never bit the POCs: `poc_in_env` runs `build -- /bin/sh -c "..."`, an
	// absolute path that needs no lookup, and the shell then resolves `cc` out
	// of the environment it was handed. It bit exactly when the command IS a
	// wrapped tool, which is a documented use of `pg-toolkit binary build [--] CMD...`.
	//
	// ⭐ The end-to-end case below is the defect itself: two programs of the
	// same name, the parent's PATH naming one and the child's Env naming the
	// other, and the assertion is on which one ran.
	td, tderr := os.MkdirTemp("", "pgb-proc-path-")
	if tderr != nil {
		r.Skip("proc: no tempdir, so the PATH-resolution cases did not run")
		return r
	}
	defer os.RemoveAll(td)
	mk := func(sub, body string) string {
		d := filepath.Join(td, sub)
		if os.MkdirAll(d, 0o755) != nil {
			return ""
		}
		p := filepath.Join(d, "pgb-selftest-tool")
		if os.WriteFile(p, []byte("#!/bin/sh\necho "+body+"\n"), 0o755) != nil {
			return ""
		}
		return d
	}
	wrapDir, realDir := mk("wrap", "WRAPPER"), mk("real", "REAL")
	if wrapDir == "" || realDir == "" || (!Look("/bin/sh") && !Look("sh")) {
		r.Skip("proc: could not build the two-tool fixture")
		return r
	}

	// lookPathIn on its own, first.
	got, ok := lookPathIn("pgb-selftest-tool", []string{"PATH=" + wrapDir + ":" + realDir})
	r.CheckBool("lookPathIn finds the name on the given PATH", ok, true)
	r.Check("...and takes the FIRST directory that has it",
		got, filepath.Join(wrapDir, "pgb-selftest-tool"))
	got, ok = lookPathIn("pgb-selftest-tool", []string{"PATH=" + realDir + ":" + wrapDir})
	r.Check("...order decides, not the name", got, filepath.Join(realDir, "pgb-selftest-tool"))
	_, ok = lookPathIn("pgb-selftest-tool", []string{"HOME=/root"})
	r.CheckBool("no PATH in the environment resolves nothing", ok, false)
	_, ok = lookPathIn("/bin/sh", []string{"PATH=" + wrapDir})
	r.CheckBool("a name with a separator is not looked up at all", ok, false)
	// ⚠ A readable but non-executable file is not a program, and skipping it
	// is what `ld`-style "found it, cannot run it" confusion comes from.
	_ = os.MkdirAll(filepath.Join(td, "noexec"), 0o755)
	_ = os.WriteFile(filepath.Join(td, "noexec", "pgb-selftest-tool"), []byte("x"), 0o644)
	got, ok = lookPathIn("pgb-selftest-tool",
		[]string{"PATH=" + filepath.Join(td, "noexec") + ":" + realDir})
	r.CheckBool("a non-executable file is skipped", ok, true)
	r.Check("...in favour of the next directory",
		got, filepath.Join(realDir, "pgb-selftest-tool"))

	// ⭐ end to end, and this is the case that was failing. The parent's PATH
	// names `real`; the Env handed to the child names `wrap` first.
	oldPath, hadPath := os.LookupEnv("PATH")
	_ = os.Setenv("PATH", realDir)
	out, err := Capture2(&Cmd{
		Argv: []string{"pgb-selftest-tool"},
		Env:  []string{"PATH=" + wrapDir + ":" + realDir},
	})
	if hadPath {
		_ = os.Setenv("PATH", oldPath)
	} else {
		_ = os.Unsetenv("PATH")
	}
	if err != nil {
		r.Fail("a Cmd runs the program its OWN Env names", err.Error(), "WRAPPER")
	} else {
		// ⛔ Before the fix this printed REAL: the parent's PATH won.
		r.Check("⭐ a Cmd runs the program its own Env's PATH names", out, "WRAPPER")
	}

	return r
}

// Capture2 is Capture over a prepared Cmd, so a case can set Env and EnvOnly.
// It exists for the selftest and is deliberately not part of the package's
// public shape.
func Capture2(c *Cmd) (string, error) {
	res, err := c.Output()
	if err != nil {
		return "", err
	}
	return strings.TrimRight(string(res.Stdout), "\n"), nil
}
