#!/usr/bin/env sh
# `cc` with one shared object first on every link line, for the glibc
# interposer.
#
# ⛔ WHY PREPEND RATHER THAN `-l`. The linker's `--as-needed` drops a library
# no preceding object has referenced yet, and rustc's own link arguments
# travel after the objects, so a `-ldl` (or a stub path) passed as `-C
# link-arg` arrives too late twice over: dropped outright, or kept but bound
# after `libc.so.6`, which then wins the `dlsym` definition. Both were
# measured green-by-exit in TODO/interpose.md T-1312. First on the line the
# stub is both kept and searched first, so the versioned reference binds the
# stub's `libdl.so.2` definition.
#
# Usage (from `scripts/build-interpose.sh`, which builds the stub first):
#   RUSTFLAGS="-C linker=$ROOT/scripts/gnu-link-stub.sh"
#   PODBOX_DL_STUB=$CRATE/target/dl-stub/libdl-stub.so
#
# Exit: whatever `cc` exits, or 2 when the stub is not named or not a file.
set -u

[ -n "${PODBOX_DL_STUB:-}" ] || {
	echo "gnu-link-stub.sh: PODBOX_DL_STUB is not set" >&2
	exit 2
}
[ -f "$PODBOX_DL_STUB" ] || {
	echo "gnu-link-stub.sh: no stub at $PODBOX_DL_STUB" >&2
	exit 2
}
exec "${CC:-cc}" "$PODBOX_DL_STUB" "$@"
