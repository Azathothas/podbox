#!/usr/bin/env python3
"""Apply size-preserving patch to nix-portable v012 x86_64."""
import sys
src, dst = sys.argv[1], sys.argv[2]
data = open(src,'rb').read()
orig = b'  pathsTopLevel="$(find / -mindepth 1 -maxdepth 1 -not -name nix -not -name dev)"\n'
repl = b'  pathsTopLevel="/bin /etc /home /lib /lib64 /opt /proc /sbin /tmp /usr"'
repl = repl + b' ' * (len(orig) - 1 - len(repl)) + b'\n'
assert len(repl) == len(orig) and data.count(orig) == 1
open(dst,'wb').write(data.replace(orig, repl))
