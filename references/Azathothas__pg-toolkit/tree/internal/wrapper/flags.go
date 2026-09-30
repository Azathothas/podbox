// flags.go - exactly what pgb injects into a compile and a link.
//
// Compile flags LEAD the command line, so they are defaults the caller can
// override. Link flags TRAIL it, because link order is meaning: an archive is
// scanned where it appears.
//
// SPDX-License-Identifier: 0BSD
package wrapper

import (
	"os"
	"path/filepath"

	"github.com/Azathothas/pg-toolkit/internal/cfg"
)

// CompileFlags are prepended to a compile.
func CompileFlags(c *cfg.Config) []string {
	var out []string
	if bl := c.Baseline(); bl != "" {
		out = append(out, "-march="+bl)
	}
	return append(out, "-fno-plt")
}

// LinkFlags are appended to an executable link. cxx selects the C++ driver's
// extra symbol forcing.
//
// -Wl,--eh-frame-hdr is on every link because gcc suppresses it for -static,
// leaving the binary with no PT_GNU_EH_FRAME. The GNU unwinder does not need
// it, but one that discovers tables only through the segment finds nothing and
// terminates at the first throw.
//
// -u,pgb_runtime_anchor forces the NSS constructor to stay: a constructor with
// no referenced symbol is dropped from an archive, and an nssfix that silently
// did not link is the failure the mechanism exists to prevent.
func LinkFlags(c *cfg.Config, rd string, cxx bool) []string {
	out := []string{
		"-static",
		"-Wl,--eh-frame-hdr",
		filepath.Join(rd, "nssfix.o"),
		"-Wl,-u,pgb_runtime_anchor",
	}
	if c.UseIconv {
		out = append(out, "-Wl,--wrap=iconv_open,--wrap=iconv,--wrap=iconv_close")
		// libstdc++ and static libraries the caller re-emits are scanned after
		// -lpgbruntime, so their __wrap_iconv* references would arrive with the
		// defining archive already behind the linker. -u pulls the member in at
		// -lpgbruntime instead.
		//
		// ⛔ And --host-dlopen needs it for a different reason, measured on
		// real host objects. The generated provider table
		// declares every glibc symbol as
		//
		//	extern char iconv_open[] __attribute__((weak));
		//
		// which is an UNDEFINED reference, so --wrap rewrites it to
		// __wrap_iconv_open -- and a WEAK undefined reference does not pull a
		// member out of an archive, so it stayed unresolved and the table
		// entry held NULL. Every host object importing iconv then failed to
		// load, naming the UNWRAPPED symbol:
		//
		//	pgb-elfload: libstdc++.so.6: undefined symbol: iconv_open@GLIBC_2.2.5
		//
		// Found by running LibreOffice's libuno_sal.so.3 and libmergedlo.so
		// through the loader; the control is the same probe built --no-iconv,
		// which loads libuno_sal.so.3 and carries libmergedlo.so past iconv to
		// an unrelated limit. ⚠ The other wrapped symbols do NOT have this
		// problem, and the difference is where the wrapper lives: __wrap_dlopen
		// and __wrap_setlocale are in object files that are always linked, and
		// only the iconv trio sits in an archive.
		if cxx || len(c.WrapDlopen) > 0 || c.HostDlopen {
			out = append(out,
				"-Wl,-u,__wrap_iconv_open",
				"-Wl,-u,__wrap_iconv",
				"-Wl,-u,__wrap_iconv_close")
		}
		out = append(out,
			"-L"+rd, "-lpgbruntime",
			"-L"+filepath.Join(c.LibiconvPrefix, "lib"), "-liconv")
	}
	if c.EmbedLocale {
		out = append(out, "-Wl,--wrap=setlocale",
			filepath.Join(rd, "locale.o"), filepath.Join(rd, "locale-data.o"))
	}
	if c.EmbedCacert {
		out = append(out, "-Wl,-u,pgb_cacert_anchor",
			filepath.Join(rd, "cacert.o"), filepath.Join(rd, "cacert-data.o"))
	}
	if c.EmbedTerminfo {
		out = append(out, "-Wl,-u,pgb_terminfo_anchor",
			filepath.Join(rd, "terminfo.o"), filepath.Join(rd, "terminfo-data.o"))
	}
	if c.EmbedTzdata {
		out = append(out, "-Wl,-u,pgb_tzdata_anchor",
			filepath.Join(rd, "tzdata.o"), filepath.Join(rd, "tzdata-data.o"))
	}
	// ⚠ --wrap, not an anchor, and the difference is which question the
	// mechanism answers. tzdata and terminfo POINT a search variable at
	// carried data before main() runs; there is no TZDIR for /etc/services, so
	// this one has to intercept the CALL. Same shape as --embed-locale.
	if c.EmbedNetdb {
		out = append(out, "-Wl,--wrap=getservbyname,--wrap=getservbyport,"+
			"--wrap=getprotobyname,--wrap=getprotobynumber,"+
			"--wrap=getservbyname_r,--wrap=getservbyport_r,"+
			"--wrap=getprotobyname_r,--wrap=getprotobynumber_r",
			filepath.Join(rd, "netdb.o"), filepath.Join(rd, "netdb-data.o"))
	}
	if len(c.WrapDlopen) > 0 {
		if _, err := os.Stat(filepath.Join(rd, "dlopen-table.o")); err == nil {
			out = append(out, "-Wl,--wrap=dlopen,--wrap=dlsym,--wrap=dlclose,--wrap=dlerror",
				filepath.Join(rd, "dlopen.o"), filepath.Join(rd, "dlopen-table.o"))
			out = append(out, dlopenObjects(rd)...)
		}
	}
	// --host-dlopen reuses the same --wrap redirection, so it is added only
	// when --wrap-dlopen did not already ask for it. binary/dlopen.c consults its
	// own compiled-in plugin table first and falls through to the ELF loader,
	// which is the order that keeps a program's own plugins from being mapped
	// when they are already in the link.
	if c.HostDlopen {
		if flags := hostDlopenLinkFlags(rd, c.TLSReserve); flags != nil {
			if len(c.WrapDlopen) == 0 {
				out = append(out, "-Wl,--wrap=dlopen,--wrap=dlsym,--wrap=dlclose,--wrap=dlerror",
					filepath.Join(rd, "dlopen.o"))
			}
			out = append(out, flags...)
		}
	}
	return out
}
