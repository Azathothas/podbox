// explain.go - the anti-black-box command: every flag pgb injects, and why.
//
// SPDX-License-Identifier: 0BSD
package wrapper

import (
	"fmt"
	"os"
	"strings"

	"github.com/Azathothas/pg-toolkit/internal/cfg"
)

// Explain writes the full account of what a build gets.
func Explain(c *cfg.Config, w *strings.Builder) {
	bl := c.Baseline()
	fmt.Fprintf(w, `pgb %s injects exactly this, and nothing else.

WHERE IT IS INJECTED
  Not into your source and not into your build files. pgb puts a directory of
  compiler wrappers first on PATH and sets CC/CXX to them, so autotools,
  CMake, meson and plain make all pick it up without knowing pgb exists.
  `+"`pg-toolkit binary cc-dir`"+` prints that directory. Each wrapper is pgb itself under
  another name; the flags it injects are in %s beside it, in plain JSON.

  Each wrapper looks at its own argv and decides:
    -c / -E / -S / -M      a compile.  compile flags only
    -shared                a shared library.  PASSED THROUGH UNCHANGED
    -print-* / --version   a query.  PASSED THROUGH UNCHANGED
    anything else          an executable link.  link flags added
  Passing -shared through is what lets a normal ./configure run: its shared
  library probes must keep working or the build fails before it starts.

COMPILE FLAGS
  These LEAD the command line, so they are DEFAULTS and your own flags
  override them. Link flags below still trail, because link order is meaning.

  -fno-plt                       no procedure linkage table indirection
  -march=%-22s CPU baseline. NOT -march=native. A binary built
                                 with -march=native runs on the machine that
                                 built it and crashes elsewhere with SIGILL,
                                 which looks exactly like a portability bug in
                                 this tool and is not one.

  AND THE ONE FLAG OF YOURS pgb REWRITES: -march=native becomes -march=%s,
  -mtune=native and -mcpu=native become -mtune=generic. Any OTHER -march you
  pass wins, because a project compiling one file per ISA level behind a
  runtime CPU check is doing the right thing.

LINK FLAGS
  -static                        no interpreter, no DT_NEEDED, nothing to
                                 resolve on the host
  -Wl,--eh-frame-hdr             gcc DROPS this for every -static link, so a
                                 static executable normally has no
                                 PT_GNU_EH_FRAME. GNU libgcc does not need it
                                 -- crtbeginT.o registers the frames instead --
                                 but an unwinder that reads only the segment
                                 finds nothing, and the failure is
                                 std::terminate at the first throw. +16 KiB
  <nssfix.o>                 passed as a plain object, not from an archive
  -Wl,-u,pgb_runtime_anchor      forces it to stay. A constructor with no
                                 referenced symbol is dropped by the linker,
                                 and an nssfix that silently did not link is
                                 the exact failure it exists to prevent
`, cfg.Version, ManifestName, bl, bl)

	if c.UseIconv {
		w.WriteString(`  -Wl,--wrap=iconv_open          redirect the three public iconv entry points
  -Wl,--wrap=iconv               to GNU libiconv, statically linked. Applies
  -Wl,--wrap=iconv_close         to every object in the link, including static
                                 libraries built before pgb existed
  -lpgbruntime -liconv           the shim lives in an archive, so a program
                                 that never calls iconv_open links none of it
                                 and pays none of the ~900 KiB
`)
	}
	if c.EmbedCacert {
		size := "?"
		if fi, err := os.Stat("/etc/ssl/certs/ca-certificates.crt"); err == nil {
			size = fmt.Sprintf("%d", fi.Size())
		}
		fmt.Fprintf(w, `  -Wl,-u,pgb_cacert_anchor       forces the CA-store constructor to stay. It
                                 probes the host's own trust store in nine
                                 known locations and points SSL_CERT_FILE,
                                 CURL_CA_BUNDLE and SSL_CERT_DIR at it.
  <cacert.o>                 it NEVER overrides a variable you set, and it
                                 never disables verification. Where no host
                                 store exists it writes the embedded copy to
                                 $TMPDIR and points at that -- the only
                                 filesystem write, and only there.
  <cacert-data.o>            the pinned environment's ca-certificates
                                 snapshot, %s bytes. IT AGES: roots are revoked
                                 and expire, so the host's store is always
                                 preferred and this is a fallback, never a
                                 pre-emption
`, size)
	}
	if c.EmbedTerminfo {
		w.WriteString(`  -Wl,-u,pgb_terminfo_anchor     a handful of terminal descriptions, used
  <terminfo.o>               only where the host cannot describe $TERM.
  <terminfo-data.o>          ncurses still consults its compiled-in path
                                 first, so a host entry this binary does not
                                 carry is still reachable
`)
	}
	if c.EmbedTzdata {
		w.WriteString(`  -Wl,-u,pgb_tzdata_anchor       a handful of timezone descriptions, used
  <tzdata.o>                 only where the host has no zone database at
  <tzdata-data.o>            all. Four of the eleven target environments
                                 ship none, and without this glibc answers
                                 TZ=Europe/Berlin with "Europe +0000" -- the
                                 name asked for, at a UTC offset.
`)
	}
	if c.EmbedNetdb {
		w.WriteString(`  -Wl,--wrap=getservbyname...    /etc/services and /etc/protocols, carried
  <netdb.o>                  from the build environment and consulted ONLY
  <netdb-data.o>             when the host's own file did not answer.
                                 getservbyname("http","tcp") returns NULL on
                                 3 of the 11 targets -- debian-11, debian-12
                                 and ubuntu-20.04, ALL GLIBC.
`)
	}
	if c.UTF8Default {
		w.WriteString(`  (--utf8-default)               an UNSET LANG means C.UTF-8, not C. ⛔ A
                                 change to a DOCUMENTED DEFAULT, not a repair:
                                 POSIX leaves the choice to the implementation
                                 when the environment is silent, glibc picks
                                 "C" and this picks "C.UTF-8". It is the one
                                 axis where native musl beats both glibc
                                 columns 11-0 (experiments/63-). ⚠ A program
                                 that assumed a single-byte default now sees a
                                 multibyte one
`)
	}
	if c.EmbedLocale {
		w.WriteString(`  -Wl,--wrap=setlocale           embedded C.UTF-8, materialised ONLY when the
  <locale-data.o>            host cannot answer a UTF-8 setlocale. A
                                 program that never calls setlocale writes
                                 nothing and touches no directory
`)
	}
	if len(c.WrapDlopen) > 0 {
		fmt.Fprintf(w, `  -Wl,--wrap=dlopen              answer the program's OWN plugin loads from a
  -Wl,--wrap=dlsym               table compiled in from the objects you named,
  -Wl,--wrap=dlclose             instead of asking the host's ld.so. Nothing
  -Wl,--wrap=dlerror             is mapped, and no second libc can enter
  <dlopen-table.o>           GENERATED for this build. Its symbols are the
                                 DEFINED, EXTERNAL ones read out of each object
                                 you named, so a file-local static is not in
                                 the table and will not resolve
  <your plugin objects>          on the link line, because the table takes
                                 their addresses
                                 this does NOT load a HOST plugin and is not
                                 trying to. docs/AGENTS.md item 4
  requested                      %s
`, strings.Join(c.WrapDlopen, " "))
	}

	if c.HostDlopen {
		w.WriteString(`  -Wl,--wrap=dlopen              load a HOST shared object with pgb's OWN ELF
  -Wl,--wrap=dlsym               loader and resolve it against the static glibc
  -Wl,--wrap=dlclose             already in this executable. The host's ld.so
  -Wl,--wrap=dlerror             is never consulted and its libc.so.6 is never
                                 mapped: a DT_NEEDED naming a library this
                                 image already contains is ANSWERED, not opened
  <elfload.o>                the loader itself
  <provider-table.o>         GENERATED for this build: every symbol the
                                 pinned static glibc can define, WEAKLY
                                 referenced so listing a name costs a string
                                 and a pointer and pulls no archive member
  -Wl,@pgb-provider-u.rsp        the dial. -u forces those members into the
                                 link so the table has addresses to hand out.
                                 Measured on the spike: 946,752 bytes and
                                 1,251 live entries without it, 2,621,872 and
                                 4,891 with it
  -Wl,--start-group ...          the -u names span libresolv.a, libanl.a and
                                 the rest; a single-pass link needs the group
`)
		// ⭐ Printed whether or not a reserve was asked for, because zero is
		// the interesting value: it is what makes a large module's refusal
		// "the surplus is a constant" rather than a defect.
		fmt.Fprintf(w, `  -DPGT_BINARY_TLS_RESERVE=%d%s
                                 bytes of THIS binary's own thread-local
                                 storage set aside for initial-exec TLS in
                                 loaded objects. glibc's surplus is a constant
                                 -- measured 3,168 bytes of headroom, and
                                 padding the executable moves size and used
                                 together, so it cannot be enlarged. 2 of 904
                                 host objects want more; one wants 56,248.
                                 ⚠ EVERY thread pays for it: default 0
`, c.TLSReserve, map[bool]string{true: "", false: "  (--tls-reserve)"}[c.TLSReserve > 0])
	}

	w.WriteString(`
WHAT EACH ONE IS FOR, WITH THE MEASUREMENT
  NSS      __nss_configure_lookup() pins every database to services glibc
           2.34+ implements inside libc, so the host's /etc/nsswitch.conf
           names nothing that can be dlopen'd.
           experiments/20-static-glibc-nss-dlopen.sh
  iconv    glibc's gconv modules are dlopen'd and carry DT_NEEDED libc.so.6.
           GNU libiconv carries the same tables as archive code.
           experiments/30-gconv-and-locale.sh
  locale   glibc's C.UTF-8 is files on disk, not code in libc.
           experiments/30-gconv-and-locale.sh

WHAT pgb DOES NOT DO, AND WILL NOT PRETEND TO
  - It does not make dlopen() of host plugins work. A static binary that
    dlopens a host shared object pulls a second libc in, which is the failure
    this tool exists to avoid. A program whose CORE function is loading host
    plugins is outside the class pgb serves. docs/limitations.md is explicit.
  - It does not give you NSS data from LDAP, SSSD, NIS or systemd-resolved.
    Keeping those modules out is the fix, and losing them is its cost.
  - It does not lower the kernel ABI floor. Your build glibc decides that.
`)
}
