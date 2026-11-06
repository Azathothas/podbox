#!/bin/sh
set -u
grep -rh "define RLIMIT_FSIZE\|define RLIMIT_NOFILE" /usr/include/ 2>/dev/null | head -5
echo "---"
ulimit -Hn; ulimit -Sn
