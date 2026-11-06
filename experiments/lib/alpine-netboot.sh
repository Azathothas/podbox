# alpine-netboot.sh - fetch a file of the Alpine 3.22.6 netboot set (T-1687).
#
#   . "$HERE/lib/alpine-netboot.sh"
#   alpine_netboot_fetch vmlinuz-virt "$WORK/vmlinuz-virt" "$CURL_TIMEOUT"
#
# `netboot/` of dl-cdn follows each point release, so a pin by SHA-256 of a
# file there breaks at the next release (PodUtils/podcrates#6). The archive
# of 3.22.6 keeps the pinned files. The function downloads it once into
# ALPINE_NETBOOT_CACHE (default: the directory of DEST), checks its SHA-256,
# and copies boot/NAME to DEST. The caller checks the sum of DEST as before.
# Exit status: 0 copied, 1 the download, the sum or the extraction failed.

ALPINE_NETBOOT_URL="https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/alpine-netboot-3.22.6-x86_64.tar.gz"
ALPINE_NETBOOT_SHA256="74ff3dd7ed96186b5fd4d728dd63ce5947578b40cf48ef792b8420e2463dc267"

alpine_netboot_fetch() {
	an_name=$1
	an_dest=$2
	an_timeout=${3:-600}
	an_cache=${ALPINE_NETBOOT_CACHE:-$(dirname "$an_dest")}
	an_archive=$an_cache/alpine-netboot-3.22.6-x86_64.tar.gz
	mkdir -p "$an_cache" || return 1
	if ! printf '%s  %s\n' "$ALPINE_NETBOOT_SHA256" "$an_archive" | sha256sum -c - >/dev/null 2>&1; then
		# The archive is about 300 MB: the timeout is at least ten minutes.
		[ "$an_timeout" -ge 600 ] 2>/dev/null || an_timeout=600
		curl -fsSL --max-time "$an_timeout" -o "$an_archive" "$ALPINE_NETBOOT_URL" || return 1
		printf '%s  %s\n' "$ALPINE_NETBOOT_SHA256" "$an_archive" | sha256sum -c - >/dev/null 2>&1 || return 1
	fi
	tar -xzOf "$an_archive" "boot/$an_name" >"$an_dest" || return 1
}
