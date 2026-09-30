// Package wrapper builds pgb's C runtime objects, decides the flags a build
// gets, and provides the compiler wrappers themselves.
//
// The wrappers are this same binary invoked under another name. They read one
// manifest written when the wrapper directory was created, then exec the real
// compiler: no shell is started, and nothing is re-parsed per invocation.
//
// SPDX-License-Identifier: 0BSD
package wrapper

import (
	"bytes"
	"fmt"
	"hash/crc32"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	assets "github.com/Azathothas/pg-toolkit"
	"github.com/Azathothas/pg-toolkit/internal/cfg"
	"github.com/Azathothas/pg-toolkit/internal/fail"
	"github.com/Azathothas/pg-toolkit/internal/logx"
	"github.com/Azathothas/pg-toolkit/internal/proc"
)

var log = logx.New("wrapper")

// Builder compiles the runtime objects for one configuration.
type Builder struct {
	C  *cfg.Config
	CC string // the compiler to build the runtime with
}

// NewBuilder resolves the compiler the runtime objects are built by.
func NewBuilder(c *cfg.Config) *Builder {
	cc := firstNonEmpty(os.Getenv("PGT_BINARY_INNER_CC"), os.Getenv("CC"), "cc")
	return &Builder{C: c, CC: cc}
}

// Dir is where the runtime objects for this compiler live.
//
// The key is the compiler's own identity, not the machine: $PGT_STATE is
// bind-mounted into the build environment, so an object compiled by the host's
// gcc would otherwise sit where the pinned environment's gcc looks.
func (b *Builder) Dir() string {
	id := fmt.Sprintf("%s\n%s\n%s",
		captureQuiet(b.CC, "-dumpmachine"),
		firstLine(captureQuiet(b.CC, "--version")),
		assets.Digest())
	return filepath.Join(b.C.State, fmt.Sprintf("runtime-%s-%d",
		unameMachine(), crc32.ChecksumIEEE([]byte(id))))
}

// srcDir materialises the embedded C sources and returns the directory.
func (b *Builder) srcDir() (string, error) {
	dir := b.C.RuntimeSrcDir()
	if err := assets.Materialise(dir); err != nil {
		return "", fail.Cannot("cannot write the carried C runtime to %s: %v", dir, err)
	}
	return dir, nil
}

// Build compiles everything this configuration needs and returns the runtime
// directory. Objects are rebuilt only when their source is newer.
func (b *Builder) Build() (string, error) {
	rd := b.Dir()
	if err := os.MkdirAll(rd, 0o755); err != nil {
		return "", fail.Cannot("cannot create %s: %v", rd, err)
	}
	src, err := b.srcDir()
	if err != nil {
		return "", err
	}

	if err := b.compileIfStale(src, "binary/nssfix.c", filepath.Join(rd, "nssfix.o"), "-O2"); err != nil {
		return "", err
	}
	if b.C.UseIconv {
		arc := filepath.Join(rd, "libpgbruntime.a")
		if stale(filepath.Join(src, "binary/iconv.c"), arc) {
			obj := filepath.Join(rd, "iconv.o")
			if err := b.compile(filepath.Join(src, "binary/iconv.c"), obj, "-O2"); err != nil {
				return "", err
			}
			_ = os.Remove(arc)
			if r, err := proc.Run("ar", "rcs", arc, obj); err != nil || r.Failed() {
				return "", fail.Ran("ar failed for %s", arc)
			}
		}
	}
	if b.C.EmbedLocale {
		if err := b.buildLocaleData(rd, src); err != nil {
			return "", err
		}
	}
	if b.C.EmbedCacert {
		if err := b.buildCacertData(rd, src); err != nil {
			return "", err
		}
	}
	if b.C.EmbedTerminfo {
		if err := b.buildTerminfoData(rd, src); err != nil {
			return "", err
		}
	}
	if b.C.EmbedNetdb {
		if err := b.buildNetdbData(rd, src); err != nil {
			return "", err
		}
	}
	if b.C.EmbedTzdata {
		if err := b.buildTzdataData(rd, src); err != nil {
			return "", err
		}
	}
	if len(b.C.WrapDlopen) > 0 {
		if err := b.buildDlopenTable(rd, src); err != nil {
			return "", err
		}
	}
	if b.C.HostDlopen {
		if err := b.buildProviderTable(rd, src); err != nil {
			return "", err
		}
	}
	return rd, nil
}

// BuildTracer compiles the carried-in ptrace tracer `pg-toolkit binary verify` uses where
// strace cannot follow the subject. It is linked plain `-static`: the
// instrument must not depend on the mechanisms it measures.
func (b *Builder) BuildTracer() (string, error) {
	rd := b.Dir()
	if err := os.MkdirAll(rd, 0o755); err != nil {
		return "", err
	}
	src, err := b.srcDir()
	if err != nil {
		return "", err
	}
	in := filepath.Join(src, "verify/trace.c")
	out := filepath.Join(rd, "pgb-trace")
	if !stale(in, out) {
		return out, nil
	}
	cc := firstNonEmpty(os.Getenv("CC"), "cc")
	r, err := (&proc.Cmd{Argv: []string{cc, "-O2", "-static", "-o", out, in}, Subsys: "wrapper"}).Output()
	if err != nil || r.Failed() {
		return "", fail.Ran("cannot build the carried tracer: %s", strings.TrimSpace(string(r.Stderr)))
	}
	return out, nil
}

func (b *Builder) compileIfStale(srcDir, name, out string, flags ...string) error {
	in := filepath.Join(srcDir, name)
	if !stale(in, out) {
		return nil
	}
	return b.compile(in, out, flags...)
}

func (b *Builder) compile(in, out string, flags ...string) error {
	log.Infof("compiling %s", filepath.Base(out))
	argv := append([]string{b.CC}, flags...)
	argv = append(argv, "-fno-lto", "-c", "-o", out, in)
	r, err := (&proc.Cmd{Argv: argv, Subsys: "wrapper"}).Output()
	if err != nil {
		return fail.Cannot("%s: %v", b.CC, err)
	}
	if r.Failed() {
		return fail.Ran("%s did not compile:\n%s", filepath.Base(in),
			strings.TrimSpace(string(r.Stderr)))
	}
	return nil
}

// stale reports whether out is missing or older than in.
func stale(in, out string) bool {
	oi, err := os.Stat(out)
	if err != nil {
		return true
	}
	ii, err := os.Stat(in)
	if err != nil {
		return false
	}
	return ii.ModTime().After(oi.ModTime())
}

// ---------------------------------------------------------------------------
// Embedded data: terminfo, CA bundle, locale.
// ---------------------------------------------------------------------------

// terminfoRoots are searched in order, and every root is searched for every
// entry: a distribution can ship an empty /usr/share/terminfo and keep the
// real entries in /lib/terminfo.
var terminfoRoots = []string{"/usr/share/terminfo", "/lib/terminfo", "/etc/terminfo", "/usr/lib/terminfo"}

func (b *Builder) buildTerminfoData(rd, src string) error {
	if exists(filepath.Join(rd, "terminfo.o")) && exists(filepath.Join(rd, "terminfo-data.o")) {
		return nil
	}
	var roots []string
	for _, r := range terminfoRoots {
		if isDir(r) {
			roots = append(roots, r)
		}
	}
	if len(roots) == 0 {
		return fail.Cannot("--embed-terminfo needs a terminfo database in the build environment (install ncurses-base)")
	}
	entries := cfg.DefaultTerminfoEntries
	if v := os.Getenv("PGT_BINARY_TERMINFO_ENTRIES"); v != "" {
		entries = strings.Fields(v)
	}

	var buf bytes.Buffer
	fmt.Fprintf(&buf, "/* generated by pgb from %s */\n", strings.Join(roots, " "))
	buf.WriteString("struct pgb_ti_file { const char *name; const unsigned char *data; unsigned len; };\n")
	var found []string
	n := 0
	for _, t := range entries {
		path := findTerminfo(roots, t)
		if path == "" {
			continue
		}
		data, err := os.ReadFile(path)
		if err != nil {
			continue
		}
		fmt.Fprintf(&buf, "static const unsigned char t%d[] = {", n)
		writeBytes(&buf, data)
		buf.WriteString("};\n")
		found = append(found, fmt.Sprintf("%s:%d", t, n))
		n++
	}
	if n == 0 {
		return fail.Cannot("--embed-terminfo found none of %s under %s",
			strings.Join(entries, " "), strings.Join(roots, " "))
	}
	buf.WriteString("const struct pgb_ti_file pgb_ti_files[] = {\n")
	for i, f := range found {
		fmt.Fprintf(&buf, "  { \"%s\", t%d, sizeof t%d },\n", strings.SplitN(f, ":", 2)[0], i, i)
	}
	buf.WriteString("};\n")
	fmt.Fprintf(&buf, "const unsigned pgb_ti_nfiles = %d;\n", n)

	log.Infof("embedding terminfo entries: %s", strings.Join(found, " "))
	gen := filepath.Join(rd, "pgb-terminfo-data.c")
	if err := os.WriteFile(gen, buf.Bytes(), 0o644); err != nil {
		return err
	}
	if err := b.compile(gen, filepath.Join(rd, "terminfo-data.o"), "-O0"); err != nil {
		return err
	}
	return b.compile(filepath.Join(src, "binary/terminfo.c"), filepath.Join(rd, "terminfo.o"), "-O2")
}

func findTerminfo(roots []string, term string) string {
	if term == "" {
		return ""
	}
	first := term[:1]
	for _, r := range roots {
		// Two layouts exist: a letter directory, and a hex-of-the-first-byte
		// directory on distributions that use it.
		for _, sub := range []string{first, fmt.Sprintf("%02x", term[0])} {
			p := filepath.Join(r, sub, term)
			if fi, err := os.Stat(p); err == nil && !fi.IsDir() {
				return p
			}
		}
	}
	return ""
}

// caBundleCandidates are the build environment's own trust stores, searched in
// order. The embedded copy is a fallback the runtime uses only where the host
// has none.
var caBundleCandidates = []string{
	"/etc/ssl/certs/ca-certificates.crt", "/etc/pki/tls/certs/ca-bundle.crt",
	"/etc/ssl/ca-bundle.pem", "/etc/ssl/cert.pem",
}

func (b *Builder) buildCacertData(rd, src string) error {
	if exists(filepath.Join(rd, "cacert.o")) && exists(filepath.Join(rd, "cacert-data.o")) {
		return nil
	}
	var chosen string
	for _, c := range caBundleCandidates {
		if fi, err := os.Stat(c); err == nil && fi.Size() > 0 {
			chosen = c
			break
		}
	}
	if chosen == "" {
		return fail.Cannot("--embed-cacert found no CA bundle in the build environment (install ca-certificates)")
	}
	data, err := os.ReadFile(chosen)
	if err != nil {
		return fail.Cannot("cannot read %s: %v", chosen, err)
	}
	log.Infof("embedding CA bundle from %s (%d bytes)", chosen, len(data))
	var buf bytes.Buffer
	fmt.Fprintf(&buf, "/* generated by pgb from %s */\n", chosen)
	buf.WriteString("const unsigned char pgb_cacert_data[] = {")
	writeBytes(&buf, data)
	buf.WriteString("};\n")
	buf.WriteString("const unsigned pgb_cacert_len = sizeof pgb_cacert_data;\n")
	gen := filepath.Join(rd, "pgb-cacert-data.c")
	if err := os.WriteFile(gen, buf.Bytes(), 0o644); err != nil {
		return err
	}
	if err := b.compile(gen, filepath.Join(rd, "cacert-data.o"), "-O0"); err != nil {
		return err
	}
	return b.compile(filepath.Join(src, "binary/cacert.c"), filepath.Join(rd, "cacert.o"), "-O2")
}

func (b *Builder) buildLocaleData(rd, src string) error {
	// ⛔ the flag IS part of the cache key, and leaving it out would be
	// `download`'s defect in another place: an object built once without
	// --utf8-default would be reused by a build that asked for it, silently.
	stamp := filepath.Join(rd, "pgb-locale-data.flags")
	want := fmt.Sprintf("utf8_default=%d\n", b2i(b.C.UTF8Default))
	if exists(filepath.Join(rd, "locale.o")) && exists(filepath.Join(rd, "locale-data.o")) {
		if got, err := os.ReadFile(stamp); err == nil && string(got) == want {
			return nil
		}
	}
	var dir string
	for _, c := range []string{"/usr/lib/locale/C.utf8", "/usr/lib/locale/C.UTF-8"} {
		if isDir(c) {
			dir = c
			break
		}
	}
	if dir == "" {
		return fail.Cannot("--embed-locale needs a compiled C.UTF-8 in the build environment (locale-gen C.UTF-8)")
	}
	// A glibc locale is a tree: LC_MESSAGES is a directory holding
	// SYS_LC_MESSAGES, and a locale missing one category fails the whole
	// LC_ALL composite at run time with no diagnostic.
	var files []string
	err := filepath.Walk(dir, func(p string, fi os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if fi.Mode().IsRegular() {
			rel, err := filepath.Rel(dir, p)
			if err != nil {
				return err
			}
			files = append(files, rel)
		}
		return nil
	})
	if err != nil {
		return fail.Cannot("cannot read %s: %v", dir, err)
	}
	sort.Strings(files)
	if len(files) == 0 {
		return fail.Cannot("--embed-locale found no files under %s", dir)
	}
	log.Infof("embedding locale from %s (%d files)", dir, len(files))

	var buf bytes.Buffer
	fmt.Fprintf(&buf, "/* generated by pgb from %s */\n", dir)
	buf.WriteString("struct pgb_locale_file { const char *name; const unsigned char *data; unsigned len; };\n")
	for i, f := range files {
		data, err := os.ReadFile(filepath.Join(dir, f))
		if err != nil {
			return err
		}
		fmt.Fprintf(&buf, "static const unsigned char d%d[] = {", i)
		writeBytes(&buf, data)
		buf.WriteString("};\n")
	}
	buf.WriteString("const struct pgb_locale_file pgb_locale_files[] = {\n")
	for i, f := range files {
		fmt.Fprintf(&buf, "  { \"%s\", d%d, sizeof d%d },\n", f, i, i)
	}
	buf.WriteString("};\n")
	fmt.Fprintf(&buf, "const unsigned pgb_locale_nfiles = %d;\n", len(files))
	fmt.Fprintf(&buf, "const char pgb_locale_name[] = \"%s\";\n", filepath.Base(dir))
	// ⭐ --utf8-default: what an unset lang means. experiments/63- measured
	// this as the one axis where native musl beats both glibc columns 11-0,
	// and binary/locale.c says why --embed-locale alone cannot move it.
	fmt.Fprintf(&buf, "const int pgb_utf8_default = %d;\n", b2i(b.C.UTF8Default))

	gen := filepath.Join(rd, "pgb-locale-data.c")
	if err := os.WriteFile(gen, buf.Bytes(), 0o644); err != nil {
		return err
	}
	if err := b.compile(gen, filepath.Join(rd, "locale-data.o"), "-O0"); err != nil {
		return err
	}
	if err := b.compile(filepath.Join(src, "binary/locale.c"), filepath.Join(rd, "locale.o"), "-O2"); err != nil {
		return err
	}
	return os.WriteFile(stamp, []byte(want), 0o644)
}

func b2i(v bool) int {
	if v {
		return 1
	}
	return 0
}

// writeBytes emits a comma-separated decimal byte list.
func writeBytes(buf *bytes.Buffer, data []byte) {
	for _, c := range data {
		fmt.Fprintf(buf, "%d,", c)
	}
}

func exists(p string) bool { _, err := os.Stat(p); return err == nil }

func isDir(p string) bool { fi, err := os.Stat(p); return err == nil && fi.IsDir() }

func firstNonEmpty(v ...string) string {
	for _, s := range v {
		if s != "" {
			return s
		}
	}
	return ""
}

func firstLine(s string) string {
	if before, _, ok := strings.Cut(s, "\n"); ok {
		return before
	}
	return s
}

func captureQuiet(argv ...string) string {
	out, _ := proc.CaptureAllowFail(argv...)
	return out
}

func unameMachine() string {
	if out, err := exec.Command("uname", "-m").Output(); err == nil {
		return strings.TrimSpace(string(out))
	}
	return "unknown"
}

// zoneinfoRoots are searched in order. ⚠ the runtime half of this list IS in
// tool/runtime/binary/tzdata.c and the two must agree: this one decides what gets
// carried, that one decides whether the host already has a database and the
// carried copy should stand aside.
var zoneinfoRoots = []string{"/usr/share/zoneinfo", "/usr/lib/zoneinfo", "/etc/zoneinfo"}

// buildTzdataData compiles a handful of zone files into the binary. It is the
// same shape as buildTerminfoData because glibc honours TZDIR exactly as
// ncurses honours TERMINFO. `experiments/97-` carries the measurement.
func (b *Builder) buildTzdataData(rd, src string) error {
	if exists(filepath.Join(rd, "tzdata.o")) && exists(filepath.Join(rd, "tzdata-data.o")) {
		return nil
	}
	var roots []string
	for _, r := range zoneinfoRoots {
		if isDir(r) {
			roots = append(roots, r)
		}
	}
	if len(roots) == 0 {
		return fail.Cannot("--embed-tzdata needs a zone database in the build environment (install tzdata)")
	}
	zones := cfg.DefaultTzdataZones
	if v := os.Getenv("PGT_BINARY_TZDATA_ZONES"); v != "" {
		zones = strings.Fields(v)
	}

	var buf bytes.Buffer
	fmt.Fprintf(&buf, "/* generated by pgb from %s */\n", strings.Join(roots, " "))
	buf.WriteString("struct pgb_tz_file { const char *name; const unsigned char *data; unsigned len; };\n")
	var found []string
	total := 0
	n := 0
	for _, z := range zones {
		path := findZone(roots, z)
		if path == "" {
			continue
		}
		data, err := os.ReadFile(path)
		if err != nil {
			continue
		}
		fmt.Fprintf(&buf, "static const unsigned char z%d[] = {", n)
		writeBytes(&buf, data)
		buf.WriteString("};\n")
		found = append(found, z)
		total += len(data)
		n++
	}
	if n == 0 {
		return fail.Cannot("--embed-tzdata found none of %s under %s",
			strings.Join(zones, " "), strings.Join(roots, " "))
	}
	buf.WriteString("const struct pgb_tz_file pgb_tz_files[] = {\n")
	for i, z := range found {
		fmt.Fprintf(&buf, "  { \"%s\", z%d, sizeof z%d },\n", z, i, i)
	}
	buf.WriteString("};\n")
	fmt.Fprintf(&buf, "const unsigned pgb_tz_nfiles = %d;\n", n)

	log.Infof("embedding %d timezone(s), %d bytes: %s", n, total, strings.Join(found, " "))
	genPath := filepath.Join(rd, "pgb-tzdata-data.c")
	if err := os.WriteFile(genPath, buf.Bytes(), 0o644); err != nil {
		return err
	}
	if err := b.compile(genPath, filepath.Join(rd, "tzdata-data.o"), "-O0"); err != nil {
		return err
	}
	return b.compile(filepath.Join(src, "binary/tzdata.c"), filepath.Join(rd, "tzdata.o"), "-O2")
}

// findZone resolves a zone name under the roots. ⚠ A zone name CONTAINS a
// directory ("Europe/Berlin"), so unlike a terminfo entry there is no letter
// or hex subdirectory to guess: the name IS the relative path.
func findZone(roots []string, zone string) string {
	if zone == "" || strings.HasPrefix(zone, "/") || strings.Contains(zone, "..") {
		return ""
	}
	for _, r := range roots {
		p := filepath.Join(r, zone)
		if fi, err := os.Stat(p); err == nil && !fi.IsDir() {
			return p
		}
	}
	return ""
}

// netdbFiles are the two files this mechanism carries, in the order the
// runtime half checks them. ⚠ the runtime half IS tool/runtime/binary/netdb.c
// and it does NOT read these paths: it calls glibc, which does. This list is
// only what the BUILD reads to fill the table.
var netdbFiles = struct{ Services, Protocols string }{"/etc/services", "/etc/protocols"}

// buildNetdbData compiles the build environment's /etc/services and
// /etc/protocols into the binary.
//
// ⭐ the whole FILE, not a subset, and that is the difference from
// --embed-tzdata: tzdata is ~1,800 files and this is a few kilobytes of text
// that parses down to a couple of thousand short strings. There is no
// judgement call about which services matter, so there is no list to get
// wrong.
//
// ⚠ it IS the build environment's FILE, not the IANA registry, so what a
// binary carries is decided by the pinned image and moves when the pin moves.
// The count is printed, and `pg-toolkit binary explain` repeats it.
func (b *Builder) buildNetdbData(rd, src string) error {
	if exists(filepath.Join(rd, "netdb.o")) && exists(filepath.Join(rd, "netdb-data.o")) {
		return nil
	}
	services, serr := os.ReadFile(netdbFiles.Services)
	protocols, perr := os.ReadFile(netdbFiles.Protocols)
	if serr != nil && perr != nil {
		return fail.Cannot("--embed-netdb needs %s or %s in the build environment (install netbase)",
			netdbFiles.Services, netdbFiles.Protocols)
	}
	svc := parseServices(string(services))
	pro := parseProtocols(string(protocols))
	if len(svc) == 0 && len(pro) == 0 {
		return fail.Cannot("--embed-netdb parsed no entries out of %s or %s",
			netdbFiles.Services, netdbFiles.Protocols)
	}

	var buf bytes.Buffer
	fmt.Fprintf(&buf, "/* generated by pgb from %s and %s */\n",
		netdbFiles.Services, netdbFiles.Protocols)
	buf.WriteString("struct pgb_serv_ent { const char *name; const char *proto; unsigned short port; };\n")
	buf.WriteString("struct pgb_proto_ent { const char *name; int number; };\n")
	buf.WriteString("const struct pgb_serv_ent pgb_serv_table[] = {\n")
	for _, e := range svc {
		fmt.Fprintf(&buf, "  { %q, %q, %d },\n", e.name, e.proto, e.port)
	}
	buf.WriteString("};\n")
	fmt.Fprintf(&buf, "const unsigned pgb_serv_count = %d;\n", len(svc))
	buf.WriteString("const struct pgb_proto_ent pgb_proto_table[] = {\n")
	for _, e := range pro {
		fmt.Fprintf(&buf, "  { %q, %d },\n", e.name, e.number)
	}
	buf.WriteString("};\n")
	fmt.Fprintf(&buf, "const unsigned pgb_proto_count = %d;\n", len(pro))

	log.Infof("embedding %d service(s) and %d protocol(s) from %s and %s",
		len(svc), len(pro), netdbFiles.Services, netdbFiles.Protocols)
	genPath := filepath.Join(rd, "pgb-netdb-data.c")
	if err := os.WriteFile(genPath, buf.Bytes(), 0o644); err != nil {
		return err
	}
	if err := b.compile(genPath, filepath.Join(rd, "netdb-data.o"), "-O0"); err != nil {
		return err
	}
	return b.compile(filepath.Join(src, "binary/netdb.c"), filepath.Join(rd, "netdb.o"), "-O2")
}

type servEntry struct {
	name, proto string
	port        int
}
type protoEntry struct {
	name   string
	number int
}

// ParseServices reads /etc/services' format: `name port/proto [aliases...]`,
// `#` to end of line is a comment.
//
// ⭐ aliases become entries of their own. `www 80/tcp http` means a caller
// asking for either name must get 80, and a table that stored the alias list
// separately would have to be searched twice. Flattening costs a pointer per
// alias and removes a whole code path from the runtime half.
func ParseServices(text string) []servEntry { return parseServices(text) }

func parseServices(text string) []servEntry {
	var out []servEntry
	seen := map[string]bool{}
	for line := range strings.SplitSeq(text, "\n") {
		if i := strings.IndexByte(line, '#'); i >= 0 {
			line = line[:i]
		}
		f := strings.Fields(line)
		if len(f) < 2 {
			continue
		}
		portStr, proto, ok := strings.Cut(f[1], "/")
		if !ok {
			continue
		}
		port, err := strconv.Atoi(portStr)
		if err != nil || port < 0 || port > 65535 {
			continue
		}
		for _, name := range append([]string{f[0]}, f[2:]...) {
			key := name + "/" + proto
			if seen[key] {
				continue
			}
			seen[key] = true
			out = append(out, servEntry{name: name, proto: proto, port: port})
		}
	}
	return out
}

// ParseProtocols reads /etc/protocols: `name number [aliases...]`.
func ParseProtocols(text string) []protoEntry { return parseProtocols(text) }

func parseProtocols(text string) []protoEntry {
	var out []protoEntry
	seen := map[string]bool{}
	for line := range strings.SplitSeq(text, "\n") {
		if i := strings.IndexByte(line, '#'); i >= 0 {
			line = line[:i]
		}
		f := strings.Fields(line)
		if len(f) < 2 {
			continue
		}
		n, err := strconv.Atoi(f[1])
		if err != nil {
			continue
		}
		for _, name := range append([]string{f[0]}, f[2:]...) {
			if seen[name] {
				continue
			}
			seen[name] = true
			out = append(out, protoEntry{name: name, number: n})
		}
	}
	return out
}
