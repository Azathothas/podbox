// flags_selftest.go - the link line pgb composes, and the one rule about it
// that is not obvious from reading it.
//
// ⛔ The defect this exists to catch, measured on real host objects.
// --host-dlopen generates a provider table that declares every glibc
// symbol as
//
//	extern char iconv_open[] __attribute__((weak));
//
// which is an UNDEFINED reference - so `-Wl,--wrap=iconv_open` rewrites it to
// `__wrap_iconv_open`, and a WEAK undefined reference does not pull a member
// out of an archive. `__wrap_iconv_open` lives in libpgbruntime.a, so it stayed
// unresolved, the table entry held NULL, and every host object importing iconv
// failed to load naming the UNWRAPPED symbol:
//
//	pgb-elfload: libstdc++.so.6: undefined symbol: iconv_open@GLIBC_2.2.5
//
// Of 71 host objects carrying a PT_TLS, 36 loaded before the fix and 50 after  -
// and the fixed build's outcome is IDENTICAL to a --no-iconv control, which is
// what says the wrap no longer decides anything.
//
// ⭐ so the rule IS about where a wrapper lives, not about iconv. A wrapper in
// an object file named on the command line is always linked; a wrapper inside
// an archive is reachable only from a STRONG reference or a -u. The cases below
// assert that for every symbol pgb wraps, in both directions, so a predicate
// stuck at either value fails one of them.
//
// Offline: it composes flags and reads no disk beyond the paths it prints.
//
// SPDX-License-Identifier: 0BSD
package wrapper

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/Azathothas/pg-toolkit/internal/cfg"
	"github.com/Azathothas/pg-toolkit/internal/selftest"
)

// archiveWrapped are the symbols pgb wraps whose wrapper is compiled into
// libpgbruntime.a rather than into an object file on the link line. ⛔ These
// are the ones a weak undefined reference cannot reach.
var archiveWrapped = []string{"iconv_open", "iconv", "iconv_close"}

// objectWrapped are the symbols pgb wraps whose wrapper is in a .o that is
// named on the link line, so a weak reference resolves without help. Kept
// beside the list above because the distinction is the whole point.
var objectWrapped = []string{"dlopen", "dlsym", "dlclose", "dlerror", "setlocale"}

// FlagsSelftest asserts the composition of the injected link line.
func FlagsSelftest() *selftest.Report {
	r := selftest.New("wrapper-flags")

	// ⛔ the runtime directory has to look built, or the --host-dlopen branch
	// is skipped entirely and every case about it passes for the wrong
	// reason. hostDlopenLinkFlags() stats provider-table.o and returns nil
	// without it, so a case asserting dlopen.o is on the line gets nothing and
	// reads as a failure of the flag rather than of the fixture. Nothing is
	// compiled here; the file is
	// empty and only its existence is read.
	rd, err := os.MkdirTemp("", "pgb-flags-selftest")
	if err != nil {
		r.Skip("cannot create a temporary runtime directory: " + err.Error())
		return r
	}
	defer os.RemoveAll(rd)
	// ⚠ BOTH generated tables, not one. --wrap-dlopen stats dlopen-table.o
	// and --host-dlopen stats provider-table.o, and each contributes
	// nothing without its own. Writing only the second made the --wrap-dlopen
	// axis below report "no" for an option that works.
	for _, f := range []string{"provider-table.o", "dlopen-table.o"} {
		if err := os.WriteFile(filepath.Join(rd, f), nil, 0o644); err != nil {
			r.Skip("cannot write into the temporary runtime directory: " + err.Error())
			return r
		}
	}

	joined := func(c *cfg.Config, cxx bool) string {
		return strings.Join(LinkFlags(c, rd, cxx), " ")
	}
	base := func() *cfg.Config {
		c := cfg.Load("/nonexistent")
		// ⚠ Load() reads the environment, and a session that exported
		// PGT_BINARY_OPT_* would otherwise make this selftest measure that session.
		c.UseIconv, c.EmbedLocale, c.EmbedCacert, c.EmbedTerminfo = true, false, false, false
		c.EmbedTzdata = false
		c.EmbedNetdb = false
		c.HostDlopen, c.WrapDlopen, c.TLSReserve = false, nil, 0
		return c
	}

	// --- the wrap itself, both directions --------------------------------
	on := joined(base(), false)
	off := func() string { c := base(); c.UseIconv = false; return joined(c, false) }()
	r.CheckBool("--wrap=iconv_open is injected when iconv is on",
		strings.Contains(on, "--wrap=iconv_open"), true)
	r.CheckBool("and NOT when --no-iconv",
		strings.Contains(off, "--wrap=iconv"), false)

	// --- ⛔ the defect: an archive wrapper under --host-dlopen ------------
	hd := func() string { c := base(); c.HostDlopen = true; return joined(c, false) }()
	for _, s := range archiveWrapped {
		r.CheckBool("--host-dlopen forces __wrap_"+s+" out of the archive",
			strings.Contains(hd, "-Wl,-u,__wrap_"+s), true)
	}
	// The two cases that already forced it, kept so a change that narrowed the
	// condition to --host-dlopen alone is caught.
	cxx := joined(base(), true)
	wd := func() string { c := base(); c.WrapDlopen = []string{"p=x.o"}; return joined(c, false) }()
	for _, s := range archiveWrapped {
		r.CheckBool("a C++ link forces __wrap_"+s,
			strings.Contains(cxx, "-Wl,-u,__wrap_"+s), true)
		r.CheckBool("--wrap-dlopen forces __wrap_"+s,
			strings.Contains(wd, "-Wl,-u,__wrap_"+s), true)
	}
	// ⛔ and the negative direction, or every case above would pass against a
	// build that forced them unconditionally and this would assert nothing.
	// A plain C link with no host loader has no weak reference to serve and
	// pays no libiconv for one.
	for _, s := range archiveWrapped {
		r.CheckBool("a plain C link does NOT force __wrap_"+s,
			strings.Contains(on, "-Wl,-u,__wrap_"+s), false)
	}

	// --- the other half of the rule: wrappers that live in a .o ----------
	// ⚠ These need no -u, and asserting that is what keeps the fix from being
	// copy-pasted into a blanket "force every wrapper", which would link
	// mechanisms nobody asked for.
	both := func() string {
		c := base()
		c.HostDlopen, c.EmbedLocale = true, true
		return joined(c, false)
	}()
	for _, s := range objectWrapped {
		r.CheckBool("__wrap_"+s+" is not forced; its object is on the line",
			strings.Contains(both, "-Wl,-u,__wrap_"+s), false)
	}
	r.CheckBool("--embed-locale puts locale.o on the line",
		strings.Contains(both, "locale.o"), true)
	r.CheckBool("--host-dlopen puts dlopen.o on the line",
		strings.Contains(both, "dlopen.o"), true)

	// --- the flags that are not conditional at all -----------------------
	r.CheckBool("every link is -static", strings.Contains(on, "-static"), true)
	r.CheckBool("every link keeps --eh-frame-hdr",
		strings.Contains(on, "--eh-frame-hdr"), true)
	r.CheckBool("every link anchors the NSS constructor",
		strings.Contains(on, "-Wl,-u,pgb_runtime_anchor"), true)

	// --- ⭐ the axis table. These five are the axes along which two builds
	// can differ, and the defect they exist for is exactly an option that stops
	// changing the flags. Every axis is asserted in BOTH directions against a
	// marker only that option produces, so an option wired to nothing fails
	// here rather than three jobs later in a build environment.
	type axis struct {
		name   string
		set    func(*cfg.Config)
		marker string
	}
	for _, a := range []axis{
		{"--embed-terminfo", func(c *cfg.Config) { c.EmbedTerminfo = true }, "terminfo.o"},
		{"--embed-tzdata", func(c *cfg.Config) { c.EmbedTzdata = true }, "tzdata.o"},
		{"--embed-netdb", func(c *cfg.Config) { c.EmbedNetdb = true }, "netdb.o"},
		{"--embed-cacert", func(c *cfg.Config) { c.EmbedCacert = true }, "cacert.o"},
		{"--embed-locale", func(c *cfg.Config) { c.EmbedLocale = true }, "locale.o"},
		{"--wrap-dlopen", func(c *cfg.Config) { c.WrapDlopen = []string{"p=x.o"} }, "--wrap=dlsym"},
		{"--host-dlopen", func(c *cfg.Config) { c.HostDlopen = true }, "provider-table.o"},
	} {
		c := base()
		a.set(c)
		with := joined(c, false)
		r.CheckBool(a.name+" puts "+a.marker+" on the link line",
			strings.Contains(with, a.marker), true)
		r.CheckBool("and without it, "+a.marker+" is absent",
			strings.Contains(on, a.marker), false)
	}
	// ⛔ --wrap-dlopen needs its generated table on disk before it contributes
	// anything, exactly as --host-dlopen does. Asserted so the axis above
	// cannot pass against a build that emitted the flags unconditionally.
	{
		c := base()
		c.WrapDlopen = []string{"p=x.o"}
		bare, err := os.MkdirTemp("", "pgb-flags-bare")
		if err == nil {
			defer os.RemoveAll(bare)
			r.CheckBool("--wrap-dlopen contributes nothing without its table",
				strings.Contains(strings.Join(LinkFlags(c, bare, false), " "), "--wrap=dlsym"), false)
		} else {
			r.Skip("cannot create a second temporary directory: " + err.Error())
		}
	}

	// --- the compile side ------------------------------------------------
	cf := strings.Join(CompileFlags(base()), " ")
	r.CheckBool("every compile is -fno-plt", strings.Contains(cf, "-fno-plt"), true)
	r.CheckBool("the baseline is the architecture default",
		strings.Contains(cf, "-march="+cfg.DefaultBaseline()), true)
	{
		c := base()
		c.ArchBaseline = "x86-64-v3"
		r.CheckBool("--baseline overrides it",
			strings.Contains(strings.Join(CompileFlags(c), " "), "-march=x86-64-v3"), true)
	}

	// --- ⭐ the wrapper directory is keyed on the options, in one assertion:
	// two option sets that produce different flags must not
	// resolve to one directory, and one option set must be stable.
	dirFor := func(c *cfg.Config) string { return Dir(c, BuildManifest(c, rd)) }
	plain, term := base(), base()
	term.EmbedTerminfo = true
	r.CheckBool("two option sets key to different wrapper directories",
		dirFor(plain) != dirFor(term), true)
	r.CheckBool("and one option set is stable across calls",
		dirFor(plain) == dirFor(base()), true)
	// ⚠ The shared directory is `$State/bin`, NOT the same path as either
	// option-keyed one. ⚠ Asserting it equals one of them is asserting the
	// opposite of what the escape hatch does.
	shared := base()
	shared.SharedWrappers = true
	r.Check("PGT_BINARY_T058_SHARED_WRAPPERS forces the old shared directory back",
		filepath.Base(dirFor(shared)), "bin")
	r.CheckBool("and it is NOT one of the option-keyed directories",
		dirFor(shared) != dirFor(plain) && dirFor(shared) != dirFor(term), true)

	// --- ParsePluginSpec, and what it REFUSES -----------------------------
	// ⚠ The refusals are the half worth asserting: a spec that parsed to an
	// empty plugin would generate an empty table and answer dlopen with it.
	if s, err := ParsePluginSpec("mine=a.o,b.o , c.o"); err != nil {
		r.Fail("ParsePluginSpec accepts NAME=OBJ,OBJ", err.Error(), "no error")
	} else {
		r.Check("ParsePluginSpec keeps the name", s.Name, "mine")
		r.Check("ParsePluginSpec splits and trims the objects",
			strings.Join(s.Objects, "|"), "a.o|b.o|c.o")
	}
	for _, bad := range []string{"", "noequals", "=a.o", "mine=", "mine= , ,"} {
		_, err := ParsePluginSpec(bad)
		r.CheckBool("ParsePluginSpec refuses "+quote(bad), err != nil, true)
	}

	// --- IsWrapperName, both directions ----------------------------------
	r.CheckBool("cc is a wrapper name", IsWrapperName("cc"), true)
	r.CheckBool("gcc is a wrapper name", IsWrapperName("gcc"), true)
	r.CheckBool("pgb is NOT a wrapper name", IsWrapperName("pgb"), false)
	r.CheckBool("the empty string is NOT a wrapper name", IsWrapperName(""), false)

	// --- uniqueSorted ----------------------------------------------------
	r.Check("uniqueSorted deduplicates and orders",
		strings.Join(uniqueSorted([]string{"b", "a", "b", "c", "a"}), "|"), "a|b|c")
	r.Check("uniqueSorted on nothing is nothing",
		strings.Join(uniqueSorted(nil), "|"), "")

	// ---- ⭐ T-063: which link-line inputs are even considered -------------
	//
	// ⛔ The symbol-reading half is `cxx-runtime` in the binary mode's own
	// command table, which needs a compiler and says so when it has none. It
	// moved with the fold, and naming the file it used to live in would be a
	// citation nothing resolves. THIS half is pure: given a link
	// line, which arguments does the detector open at all? It is the half most
	// likely to break, because it is argument parsing.
	//
	// ⚠ Every path below is deliberately NON-EXISTENT. NeedsCXXRuntime is a
	// quiet no for a file it cannot read, so a path that is considered and a
	// path that is skipped both answer "no" -- what is asserted here is the
	// SUPPRESSION rule, where the answer differs for a reason the caller
	// stated.
	r.Check("cxx-demand: -nostdlib suppresses the scan entirely",
		firstOf(cxxRuntimeDemand([]string{"-nostdlib", "libicuuc.a"})), "")
	r.Check("cxx-demand: -nodefaultlibs suppresses it too",
		firstOf(cxxRuntimeDemand([]string{"-nodefaultlibs", "libicuuc.a"})), "")
	r.Check("cxx-demand: -nostartfiles suppresses it too",
		firstOf(cxxRuntimeDemand([]string{"-nostartfiles", "libicuuc.a"})), "")
	// ⚠ And with none of those, a missing archive is still a quiet no -- so
	// this case says the scan RAN and found nothing, not that it was skipped.
	r.Check("cxx-demand: an unreadable input is a quiet no",
		firstOf(cxxRuntimeDemand([]string{"/nonexistent/libicuuc.a"})), "")

	// ---- ⛔ which inputs are considered, which the block above cannot say ---
	//
	// ⛔ The defect this section exists for, and its worst form is the defect
	// written down as the intent:
	//
	//     r.Check("cxx-demand: a flag is never opened as an input",
	//         firstOf(cxxRuntimeDemand([]string{"-L/usr/lib", "-lfoo"})), "")
	//
	// It passed - because `/usr/lib/libfoo.a` does not exist, so the answer is
	// "" whether the argument was considered or skipped. ⚠ Every path in the
	// block above is non-existent for exactly that reason, so NOTHING there
	// could distinguish the two, and the rule its name asserts is the wrong
	// one: `-licuuc` IS how a real build names an archive.
	//
	// ⭐ Measured on the real subject: postgres 18.6's own generated
	// `src/Makefile.global`:
	//
	//     ICU_LIBS = -L/.../nix-prefix/lib -licui18n -licuuc -licudata -lpthread -lm
	//
	// Every one starts with `-`, `libicuuc.a` was never opened, and the link
	// died on `undefined reference to 'operator delete(void*, unsigned long)'`
	// and `vtable for __cxxabiv1::__si_class_type_info`. T-063 arm S.
	//
	// `cxxCandidates` returns what would be opened, so these assert the RULE
	// rather than an outcome that is "no" either way.
	cand := func(args ...string) string {
		var out []string
		for _, g := range cxxCandidates(args) {
			out = append(out, strings.Join(g, "|"))
		}
		return strings.Join(out, " ")
	}
	r.Check("cxx-candidates: a literal archive is considered",
		cand("main.o", "libicuuc.a"), "main.o libicuuc.a")
	// ⭐ the case the defect was: postgres's exact shape.
	r.Check("cxx-candidates: -lNAME resolves against -L",
		cand("-L/p/lib", "-licui18n", "-licuuc"),
		"/p/lib/libicui18n.a /p/lib/libicuuc.a")
	r.Check("cxx-candidates: a separated -L dir is used too",
		cand("-L", "/p/lib", "-licuuc"), "/p/lib/libicuuc.a")
	r.Check("cxx-candidates: every -L is a candidate, in order",
		cand("-L/a", "-L/b", "-licuuc"), "/a/libicuuc.a|/b/libicuuc.a")
	r.Check("cxx-candidates: -l:NAME names the file exactly",
		cand("-L/p", "-l:libicuuc.a"), "/p/libicuuc.a")
	// ⛔ The separated form of -l, which a scan of this shape misses.
	// `-L` had a separated case and `-l` did not: it fell into
	// the "this flag's value is not an input" branch, which is true of `-o`
	// and `-L` and FALSE of `-l`, whose value is the library to resolve. GNU
	// ld documents `-l namespec` with a space and gcc passes it through, so a
	// build system that emits it got the pre-R3 behaviour -- the whole
	// argument skipped -- from code written to fix exactly that.
	r.Check("cxx-candidates: a separated -l NAME resolves too",
		cand("-L/p/lib", "-l", "icuuc"), "/p/lib/libicuuc.a")
	r.Check("cxx-candidates: ...and the separated -l: form",
		cand("-L/p", "-l", ":libicuuc.a"), "/p/libicuuc.a")
	r.Check("cxx-candidates: a separated -l with no -L still resolves to nothing",
		cand("-l", "m"), "")
	// ⛔ without a -l there IS nothing to resolve against, and the system
	// directories are deliberately not searched: it would put every link's
	// scan on /usr/lib's archives for no gain, since a C link that needs the
	// system libstdc++ already gets it from the driver.
	r.Check("cxx-candidates: -lNAME with no -L resolves to nothing",
		cand("-lm", "-lpthread"), "")
	// The negatives: a flag that takes a value must not have its value read as
	// an input, and an ordinary flag names nothing.
	r.Check("cxx-candidates: -o's value is not an input",
		cand("-o", "prog.a"), "")
	r.Check("cxx-candidates: -L's own value is not an input",
		cand("-L", "/p/lib"), "")
	r.Check("cxx-candidates: ordinary flags name nothing",
		cand("-O2", "-Wall", "-static", "-DFOO=1"), "")
	r.Check("cxx-candidates: a source file is not an input to this scan",
		cand("main.c", "other.cpp"), "")

	NetdbParseSelftest(r)
	return r
}

// quote renders a spec for a case name, so an empty one is visible.
func quote(s string) string {
	if s == "" {
		return "the empty string"
	}
	return "\"" + s + "\""
}

// firstOf turns the detector's (input, symbol) pair into one comparable value.
func firstOf(a, _ string) string { return a }

// NetdbParseSelftest exercises the two file parsers offline.
//
// ⛔ why they are asserted rather than read. `/etc/services` is a text format
// with three shapes that all look alike - a comment on its own line, a comment
// at the end of a real line, and an entry with aliases - and the third is the
// one that decides whether `getservbyname("www","tcp")` answers. A parser that
// dropped aliases would produce a table that is right about `http` and wrong
// about `www`, which no build-time check would notice.
func NetdbParseSelftest(r *selftest.Report) {
	const services = `# a comment line
http		80/tcp		www www-http	# WorldWideWeb HTTP
https		443/tcp
domain		53/udp
notaport	xx/tcp
short
`
	svc := ParseServices(services)
	find := func(name, proto string) int {
		for _, e := range svc {
			if e.name == name && e.proto == proto {
				return e.port
			}
		}
		return -1
	}
	r.CheckInt("a plain entry parses", find("http", "tcp"), 80)
	r.CheckInt("⭐ an ALIAS becomes an entry of its own", find("www", "tcp"), 80)
	r.CheckInt("...and so does the second alias", find("www-http", "tcp"), 80)
	r.CheckInt("the protocol is part of the key", find("domain", "udp"), 53)
	r.CheckInt("...so the same name under another protocol is absent",
		find("domain", "tcp"), -1)
	r.CheckInt("a non-numeric port is skipped, not zero", find("notaport", "tcp"), -1)
	r.CheckInt("a line with one field is skipped", len(ParseServices("short\n")), 0)
	r.CheckInt("a comment-only file yields nothing", len(ParseServices("# x\n#y\n")), 0)

	const protocols = `# comment
ip	0	IP		# internet protocol
tcp	6	TCP
notanumber	zz
`
	pro := ParseProtocols(protocols)
	num := func(name string) int {
		for _, e := range pro {
			if e.name == name {
				return e.number
			}
		}
		return -1
	}
	r.CheckInt("a protocol parses", num("tcp"), 6)
	r.CheckInt("⭐ its ALIAS parses too", num("IP"), 0)
	r.CheckInt("a non-numeric protocol number is skipped", num("notanumber"), -1)
}
