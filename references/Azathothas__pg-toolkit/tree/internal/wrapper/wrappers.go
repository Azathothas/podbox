// wrappers.go - the compiler wrapper directory.
//
// The directory is keyed on the OPTIONS, not on the invocation: $PGT_STATE is
// bind-mounted into the build environment, so one fixed name would be the
// directory every concurrent `pg-toolkit binary build` executes its compiler out of, and the
// flags depend on the options. Content-addressing also means `pg-toolkit binary cc-dir`
// prints a stable name and a killed build leaves a directory the next build
// with the same options reuses.
//
// SPDX-License-Identifier: 0BSD
package wrapper

import (
	"encoding/json"
	"fmt"
	"hash/crc32"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/Azathothas/pg-toolkit/internal/cfg"
	"github.com/Azathothas/pg-toolkit/internal/fail"
)

// wrapperNames maps the name a build system calls to the tool it stands for.
var wrapperNames = []struct{ Name, Real string }{
	{"cc", "cc"}, {"gcc", "gcc"}, {"c++", "c++"}, {"g++", "g++"}, {"cpp", "cpp"},
}

// ManifestName is the file a wrapper reads to learn its flags.
const ManifestName = "pgb-wrappers.json"

// Manifest is what one wrapper directory injects.
type Manifest struct {
	Version    int               `json:"version"`
	Compile    []string          `json:"compile"`
	Link       []string          `json:"link"`
	LinkCXX    []string          `json:"linkCxx"`
	Baseline   string            `json:"baseline"`
	WrapDlopen bool              `json:"wrapDlopen"`
	Real       map[string]string `json:"real"`
	RuntimeDir string            `json:"runtimeDir"`
}

// IsWrapperName reports whether argv[0] names a compiler wrapper.
func IsWrapperName(name string) bool {
	for _, w := range wrapperNames {
		if w.Name == name {
			return true
		}
	}
	return false
}

// RealCompiler resolves a compiler on PATH, skipping pgb's own wrapper
// directories. Once wrappers are option-keyed there can be several of them and
// one is on PATH during a build, so a plain lookup would wrap a wrapper.
func RealCompiler(c *cfg.Config, want string) (string, error) {
	for _, dir := range filepath.SplitList(os.Getenv("PATH")) {
		if dir == "" {
			dir = "."
		}
		if underState(c.State, dir) {
			continue
		}
		p := filepath.Join(dir, want)
		fi, err := os.Stat(p)
		if err != nil || fi.IsDir() || fi.Mode()&0o111 == 0 {
			continue
		}
		return p, nil
	}
	return "", exec.ErrNotFound
}

func underState(state, dir string) bool {
	if state == "" {
		return false
	}
	return dir == state || strings.HasPrefix(dir, state+string(os.PathSeparator))
}

// BuildManifest resolves the compilers and computes the flags for a config.
func BuildManifest(c *cfg.Config, runtimeDir string) *Manifest {
	m := &Manifest{
		Version:    1,
		Compile:    CompileFlags(c),
		Link:       LinkFlags(c, runtimeDir, false),
		LinkCXX:    LinkFlags(c, runtimeDir, true),
		Baseline:   c.Baseline(),
		WrapDlopen: len(c.WrapDlopen) > 0,
		Real:       map[string]string{},
		RuntimeDir: runtimeDir,
	}
	for _, w := range wrapperNames {
		if p, err := RealCompiler(c, w.Real); err == nil {
			m.Real[w.Name] = p
		}
	}
	return m
}

// Dir is the wrapper directory for this configuration.
func Dir(c *cfg.Config, m *Manifest) string {
	if c.SharedWrappers {
		// The escape hatch experiments/87- uses to reproduce the single shared
		// directory its control depends on. Nothing in pgb sets it.
		return filepath.Join(c.State, "bin")
	}
	var b strings.Builder
	b.WriteString(strings.Join(m.Compile, " ") + "\n")
	b.WriteString(strings.Join(m.Link, " ") + "\n")
	b.WriteString(strings.Join(m.LinkCXX, " ") + "\n")
	b.WriteString(m.Baseline + "\n")
	if m.WrapDlopen {
		b.WriteString("dl\n")
	}
	for _, w := range wrapperNames {
		b.WriteString(m.Real[w.Name] + "\n")
	}
	return filepath.Join(c.State, fmt.Sprintf("bin-%d", crc32.ChecksumIEEE([]byte(b.String()))))
}

// Make creates or refreshes the wrapper directory and returns its path.
//
// Nothing is removed: wiping and recreating would take the wrappers away from
// a build between two compiler invocations. Each file is written to a
// temporary name and renamed into place, which is atomic and leaves an
// already-exec'd wrapper untouched.
func Make(c *cfg.Config, m *Manifest) (string, error) {
	wd := Dir(c, m)
	if err := os.MkdirAll(wd, 0o755); err != nil {
		return "", fail.Cannot("cannot create %s: %v", wd, err)
	}

	self, err := os.Executable()
	if err != nil {
		return "", fail.Cannot("cannot locate the running pgb: %v", err)
	}
	if resolved, err := filepath.EvalSymlinks(self); err == nil {
		self = resolved
	}

	data, err := json.MarshalIndent(m, "", "  ")
	if err != nil {
		return "", err
	}
	if err := atomicWrite(filepath.Join(wd, ManifestName), append(data, '\n'), 0o644); err != nil {
		return "", err
	}

	for _, w := range wrapperNames {
		if m.Real[w.Name] == "" {
			continue
		}
		if err := linkOrCopy(self, filepath.Join(wd, w.Name)); err != nil {
			return "", fail.Cannot("cannot install the %s wrapper: %v", w.Name, err)
		}
	}
	return wd, nil
}

// linkOrCopy points dst at the pgb binary. A symlink is preferred so the
// wrappers cost nothing; a copy is the fallback where symlinks are refused.
func linkOrCopy(self, dst string) error {
	if cur, err := os.Readlink(dst); err == nil && cur == self {
		return nil
	}
	tmp := dst + ".tmp"
	_ = os.Remove(tmp)
	if err := os.Symlink(self, tmp); err == nil {
		return os.Rename(tmp, dst)
	}
	if err := copyFile(self, tmp, 0o755); err != nil {
		return err
	}
	return os.Rename(tmp, dst)
}

func copyFile(src, dst string, mode os.FileMode) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()
	out, err := os.OpenFile(dst, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, mode)
	if err != nil {
		return err
	}
	if _, err := io.Copy(out, in); err != nil {
		out.Close()
		return err
	}
	return out.Close()
}

func atomicWrite(path string, data []byte, mode os.FileMode) error {
	tmp := path + ".tmp"
	if err := os.WriteFile(tmp, data, mode); err != nil {
		return err
	}
	return os.Rename(tmp, path)
}

// LoadManifest reads a wrapper directory's manifest.
func LoadManifest(dir string) (*Manifest, error) {
	b, err := os.ReadFile(filepath.Join(dir, ManifestName))
	if err != nil {
		return nil, err
	}
	var m Manifest
	if err := json.Unmarshal(b, &m); err != nil {
		return nil, fmt.Errorf("%s/%s: %w", dir, ManifestName, err)
	}
	return &m, nil
}

func sortStrings(s []string) {
	for i := 1; i < len(s); i++ {
		for j := i; j > 0 && s[j] < s[j-1]; j-- {
			s[j], s[j-1] = s[j-1], s[j]
		}
	}
}
