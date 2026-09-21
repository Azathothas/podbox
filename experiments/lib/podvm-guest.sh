#!/bin/sh
# podvm-guest.sh - one way to wrap a rootfs directory as a bootable initramfs.
#
# Sourced, never executed: `. "$HERE/lib/podvm-guest.sh"`. POSIX sh only, so
# both bash scripts and dash jobs can use it.
#
# The assembly is base-plus-extras: the rootfs as newc cpio, then an appended
# raw archive carrying dev/, dev/console 5:1 and the guest's /init. Archives
# concatenate, and the kernel's unpacker reads past the base archive's
# TRAILER where `cpio -t` stops, which is why the node is asserted in the
# appended archive and the boot proves the full assembly (TODO/podvm.md
# T-1303). The /init content is the caller's: a throwaway prints a marker
# and powers off, a long-running one execs a shell (T-1304).
#
# ⛔ No host mknod and no privilege anywhere here: the console node is bytes
# written by the newc writer below, not a device the host creates.

# podvm_base ROOTFS OUT: the rootfs as an uncompressed newc archive.
podvm_base() {
	(cd "$1" && find . -print0 | cpio --quiet -o -H newc --null >"$2") || return 1
}

# podvm_extras INIT_FILE OUT: the appended archive for one /init override.
podvm_extras() {
	python3 - "$1" "$2" <<'PYEOF' || return 1
import sys
init = open(sys.argv[1], "rb").read()
out = sys.argv[2]
def newc(name, mode, filesize=0, rdevmaj=0, rdevmin=0, data=b""):
    name_b = name.encode() + b"\0"
    head = ["070701", "00000000", format(mode, "08x"), "00000000",
            "00000000", "00000001", "00000000", format(filesize, "08x"),
            "00000000", "00000000", format(rdevmaj, "08x"),
            format(rdevmin, "08x"), format(len(name_b), "08x"), "00000000"]
    rec = "".join(head).encode() + name_b
    rec += b"\0" * ((4 - len(rec) % 4) % 4)
    rec += data + b"\0" * ((4 - len(data) % 4) % 4)
    return rec
rec = newc("dev", 0o040755)
rec += newc("dev/console", 0o020600, rdevmaj=5, rdevmin=1)
rec += newc("init", 0o100755, filesize=len(init), data=init)
rec += newc("TRAILER!!!", 0)
open(out, "wb").write(rec)
PYEOF
	[ -s "$2" ] || return 1
}

# podvm_concat BASE EXTRAS OUT: the bootable assembly.
podvm_concat() {
	cat "$1" "$2" >"$3" || return 1
}
