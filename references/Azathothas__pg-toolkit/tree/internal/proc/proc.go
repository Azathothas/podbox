// Package proc runs child processes.
//
// Every command pgb starts goes through here, which gives one place that logs
// the exact argv before it runs, shell-quoted so a reader can paste it back.
// Commands are argv arrays: nothing is composed into a string and re-split, so
// an argument containing spaces stays one argument. A caller that genuinely
// needs shell semantics passes a script to Shell, which logs the script too.
//
// SPDX-License-Identifier: 0BSD
package proc

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"syscall"

	"github.com/Azathothas/pg-toolkit/internal/logx"
)

var log = logx.New("exec")

// Cmd describes one child process.
type Cmd struct {
	Argv []string
	Dir  string
	// Env entries in NAME=VALUE form, added to (or replacing) the parent's.
	// Use EnvOnly to start from an empty environment instead.
	Env     []string
	EnvOnly bool

	Stdin  io.Reader
	Stdout io.Writer
	Stderr io.Writer

	// Subsys names the caller for the log line; empty means "exec".
	Subsys string

	// Stream marks a command whose output the caller is watching rather than
	// capturing, so it is routed through the timestamper when one is
	// configured. Stdout and Stderr are ignored when it is set.
	Stream bool
}

// Result carries the outcome of a run.
type Result struct {
	Code   int    // exit status; 128+N if the child died on signal N
	Signal int    // 0 when the child exited normally
	Stdout []byte // captured only by Output/Capture
	Stderr []byte
}

// Failed reports a non-zero status.
func (r Result) Failed() bool { return r.Code != 0 }

func (c *Cmd) logger() *logx.Logger {
	if c.Subsys == "" {
		return log
	}
	return logx.New(c.Subsys)
}

func (c *Cmd) build() *exec.Cmd {
	cmd := exec.Command(c.Argv[0], c.Argv[1:]...)
	cmd.Dir = c.Dir
	if c.EnvOnly {
		cmd.Env = c.Env
	} else if len(c.Env) > 0 {
		cmd.Env = mergeEnv(os.Environ(), c.Env)
	}
	// ⛔ The program is resolved against the PATH the CHILD will see, never
	// this process's.
	//
	// `exec.Command` runs LookPath against the CALLER's environment, and the
	// Env set on the Cmd is applied afterwards - so a caller that hands a child
	// a different PATH gets the program its OWN PATH names. ⚠ Measured with a
	// probe rather than reasoned:
	//
	//	parent PATH = .../real       child Env PATH = .../wrap:.../real
	//	exec.Command("mytool")  ->  Path=".../real/mytool"  "I am the real one"
	//
	// ⛔ what that cost. `buildx.Inner` puts the wrapper directory on the
	// child's PATH - that is how the whole toolchain is injected - so
	// `pg-toolkit binary build -- cc foo.c` looked `cc` up without it and ran the real
	// compiler, with none of pgb's flags. ⚠ It never bit the POCs, because
	// `poc_in_env` runs `build -- /bin/sh -c "..."`: an ABSOLUTE path needs no
	// lookup, and the shell then resolves `cc` out of the environment it was
	// handed. It bit exactly when the command IS a wrapped tool, which is a
	// documented use of `pg-toolkit binary build [--] CMD...`.
	//
	// ⚠ AND THE LOUD FAILURE WAS LUCK. What the bypass produced here was
	// `cc: fatal error: cannot execute 'cc1'`, from a malformed
	// `-iprefix /../lib/gcc/...` - a build that stops. On a machine whose real
	// compiler tolerates that prefix the same bypass produces a binary with
	// none of pgb's flags in it and exits 0, which is the same defect with no
	// symptom. ⛔ Do not read "it failed noisily" as a property of the defect.
	//
	// ⚠ only when the caller specified an environment. `exec.Cmd.Env == nil`
	// means "inherit", and then this process's PATH IS the child's, so there is
	// nothing to differ about. ⭐ And a name with a separator is not a lookup at
	// all - `./configure`, which `envx` passes, must stay exactly that.
	// A name this PATH cannot resolve is left as exec resolved it, so the
	// failure is the one it would have reported.
	if cmd.Env != nil {
		if p, ok := lookPathIn(c.Argv[0], cmd.Env); ok {
			cmd.Path = p
		}
	}
	cmd.Stdin = c.Stdin
	cmd.Stdout = c.Stdout
	cmd.Stderr = c.Stderr
	return cmd
}

// lookPathIn resolves name against the PATH in env, which is the environment
// the child will actually run with.
//
// ⚠ One named divergence from exec.LookPath, recorded rather than fixed.
// `exec.LookPath` asks the KERNEL whether the
// caller may execute the file (`faccessat`/`X_OK`); this asks whether ANY
// execute bit is set. For root - which is every context this runs in today  -
// the two agree, because root may execute anything carrying any x bit. They
// diverge for an unprivileged caller facing, say, a 0700 file owned by
// somebody else: LookPath would skip it and this would select it, and exec
// would then fail EACCES instead of "not found".
//
// ⛔ it IS not fixed because it cannot be shown to fail here. `RULES.md`
// section "an assertion you have not seen fail is not an assertion" cuts both ways:
// a fix whose selftest passes identically before and after is not a fix, it is
// a change. If a non-root path ever reaches this, the case becomes writable
// and the fix is one `syscall.Access(p, unix.X_OK)`.
func lookPathIn(name string, env []string) (string, bool) {
	if name == "" || strings.ContainsRune(name, os.PathSeparator) {
		return "", false // an explicit path is not looked up
	}
	var path string
	found := false
	for _, e := range env {
		if v, ok := strings.CutPrefix(e, "PATH="); ok {
			path, found = v, true
		}
	}
	if !found {
		return "", false
	}
	for _, dir := range filepath.SplitList(path) {
		if dir == "" {
			dir = "."
		}
		p := filepath.Join(dir, name)
		fi, err := os.Stat(p)
		if err != nil || fi.IsDir() || fi.Mode()&0o111 == 0 {
			continue
		}
		return p, true
	}
	return "", false
}

// announce prints the command about to run, with the working directory and any
// environment the caller added.
func (c *Cmd) announce() {
	lg := c.logger()
	if !lg.DebugEnabled() {
		lg.Infof("run: %s", logx.QuoteArgs(c.Argv))
		return
	}
	var b strings.Builder
	b.WriteString("run: ")
	if c.Dir != "" {
		fmt.Fprintf(&b, "(cd %s) ", logx.Quote(c.Dir))
	}
	for _, e := range c.Env {
		k, v, _ := strings.Cut(e, "=")
		fmt.Fprintf(&b, "%s=%s ", k, logx.Quote(v))
	}
	b.WriteString(logx.QuoteArgs(c.Argv))
	lg.Debugf("%s", b.String())
}

// Run starts the command and waits for it. A non-zero exit is reported in the
// Result, not as an error; err is non-nil only when the process could not be
// started or waited for.
func (c *Cmd) Run() (Result, error) {
	if len(c.Argv) == 0 {
		return Result{}, errors.New("proc: empty argv")
	}
	c.announce()
	cmd := c.build()
	if c.Stream {
		if s := logx.StreamStamper(); s != nil {
			return c.runStamped(cmd, s)
		}
		cmd.Stdout, cmd.Stderr = os.Stdout, os.Stderr
	}
	err := cmd.Run()
	return classify(cmd, err)
}

// runStamped puts a timestamp on every line the child writes and a heartbeat
// on its silences. Both streams share one stamper, so the interleaving the
// child produced is the interleaving that is printed.
func (c *Cmd) runStamped(cmd *exec.Cmd, s *logx.Stamper) (Result, error) {
	defer s.Close()
	// The pipes replace whatever the caller set, and exec refuses to make one
	// while a writer is still attached.
	cmd.Stdout, cmd.Stderr = nil, nil
	// This process has taken the stamping job, so a pgb further in must not
	// take it as well: the environment carries PGT_TS and would be obeyed.
	base := cmd.Env
	if base == nil {
		base = os.Environ()
	}
	cmd.Env = mergeEnv(base, []string{"PGT_TS=0"})
	outPipe, err := cmd.StdoutPipe()
	if err != nil {
		return Result{}, err
	}
	errPipe, err := cmd.StderrPipe()
	if err != nil {
		return Result{}, err
	}
	if err := cmd.Start(); err != nil {
		return Result{}, err
	}
	var wg sync.WaitGroup
	wg.Add(2)
	for _, r := range []io.Reader{outPipe, errPipe} {
		go func(r io.Reader) {
			defer wg.Done()
			_ = s.Pipe(r)
		}(r)
	}
	wg.Wait()
	return classify(cmd, cmd.Wait())
}

func classify(cmd *exec.Cmd, err error) (Result, error) {
	var r Result
	if err == nil {
		r.Code = cmd.ProcessState.ExitCode()
		return r, nil
	}
	var ee *exec.ExitError
	if errors.As(err, &ee) {
		if ws, ok := ee.Sys().(syscall.WaitStatus); ok && ws.Signaled() {
			r.Signal = int(ws.Signal())
			r.Code = 128 + r.Signal
			return r, nil
		}
		r.Code = ee.ExitCode()
		return r, nil
	}
	return r, err
}

// Output runs the command and captures stdout and stderr.
func (c *Cmd) Output() (Result, error) {
	var so, se bytes.Buffer
	c.Stdout, c.Stderr = &so, &se
	r, err := c.Run()
	r.Stdout, r.Stderr = so.Bytes(), se.Bytes()
	return r, err
}

// Run is the common case: argv, inherited streams.
func Run(argv ...string) (Result, error) {
	return (&Cmd{Argv: argv, Stdout: os.Stdout, Stderr: os.Stderr}).Run()
}

// Quiet runs a command discarding its output, for probes.
func Quiet(argv ...string) (Result, error) {
	return (&Cmd{Argv: argv, Stdout: io.Discard, Stderr: io.Discard}).Run()
}

// Capture runs a command and returns its trimmed stdout. A non-zero exit is an
// error here, because a caller asking for a value has no use for a partial one.
func Capture(argv ...string) (string, error) {
	c := &Cmd{Argv: argv}
	r, err := c.Output()
	if err != nil {
		return "", err
	}
	if r.Failed() {
		return "", fmt.Errorf("%s: exit %d: %s", argv[0], r.Code,
			strings.TrimSpace(string(r.Stderr)))
	}
	return strings.TrimRight(string(r.Stdout), "\n"), nil
}

// CaptureAllowFail returns stdout and the status without treating a non-zero
// exit as an error, for probes whose failure is information.
func CaptureAllowFail(argv ...string) (string, int) {
	c := &Cmd{Argv: argv}
	r, err := c.Output()
	if err != nil {
		return "", -1
	}
	return strings.TrimRight(string(r.Stdout), "\n"), r.Code
}

// Shell runs a script with /bin/sh. The script is logged in full, so a
// construct inside it is visible before it executes rather than after it
// misbehaves. Extra arguments become $0, $1, ... as sh defines them.
func Shell(script string, args ...string) *Cmd {
	argv := append([]string{"/bin/sh", "-c", script}, args...)
	return &Cmd{Argv: argv, Subsys: "sh"}
}

// Look reports whether a program is on PATH.
func Look(name string) bool {
	_, err := exec.LookPath(name)
	return err == nil
}

// LookPath resolves a program on PATH.
func LookPath(name string) (string, error) { return exec.LookPath(name) }

// mergeEnv overlays add onto base, replacing entries with the same name.
func mergeEnv(base, add []string) []string {
	idx := make(map[string]int, len(base))
	out := make([]string, len(base))
	copy(out, base)
	for i, e := range out {
		if k, _, ok := strings.Cut(e, "="); ok {
			idx[k] = i
		}
	}
	for _, e := range add {
		k, _, ok := strings.Cut(e, "=")
		if !ok {
			continue
		}
		if i, seen := idx[k]; seen {
			out[i] = e
			continue
		}
		idx[k] = len(out)
		out = append(out, e)
	}
	return out
}

// Exec replaces this process with another, for the engine boundary where pgb
// has nothing left to do after handing over.
func Exec(argv []string, env []string) error {
	if len(argv) == 0 {
		return errors.New("proc: empty argv")
	}
	path, err := exec.LookPath(argv[0])
	if err != nil {
		return err
	}
	log.Debugf("exec: %s", logx.QuoteArgs(argv))
	if env == nil {
		env = os.Environ()
	}
	return syscall.Exec(path, argv, env)
}
