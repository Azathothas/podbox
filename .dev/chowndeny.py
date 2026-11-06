#!/usr/bin/env python3
"""Deny chown to any uid but the caller's own, then exec argv.

Used by experiments/95-podman-vfs-ignorechown.sh to stage the exact wall
TODO/image.md T-0205 names: layer application where chown to an unmapped id
fails. The filter is inherited across exec, so
`podman unshare python3 chowndeny.py podman load` unpacks layers with every
chown to another owner answered EPERM while the caller's own files are
unaffected. (The outer `podman unshare` sets the userns up first: a filter
installed earlier would need NO_NEW_PRIVS, which breaks podman's own
newuidmap call.)
"""
import ctypes
import os
import struct
import sys

AUDIT_ARCH_X86_64 = 0xC000003E
# (nr, uid-arg-index): chown, fchown, lchown, fchownat on x86_64.
DENY = ((92, 2), (93, 1), (16, 2), (260, 2))

BPF_LD = 0x00
BPF_W = 0x00
BPF_ABS = 0x20
BPF_JMP = 0x05
BPF_JEQ = 0x10
BPF_K = 0x00
BPF_RET = 0x06
SECCOMP_RET_ALLOW = 0x7FFF0000
SECCOMP_RET_ERRNO = 0x00050000


def _stmt(code, k):
    return struct.pack("HBBI", code, 0, 0, k)


def _jump(code, k, jt, jf):
    return struct.pack("HBBI", code, jt, jf, k)


def install():
    euid = os.geteuid()
    # One tuple per instruction; jumps are patched once every index is
    # known, so no hand arithmetic decides a verdict.
    ins = [("ld", 4), ("jarch",), ("ld", 0)]
    for nr, uid_arg in DENY:
        ins.append(("jnr", nr))
        ins.append(("ld", 16 + 8 * uid_arg))
        ins.append(("juid",))
        ins.append(("errno",))
    ins.append(("allow",))
    allow_idx = len(ins) - 1
    out = []
    for i, op in enumerate(ins):
        if op[0] == "ld":
            out.append(_stmt(BPF_LD + BPF_W + BPF_ABS, op[1]))
        elif op[0] == "jarch":
            out.append(_jump(BPF_JMP + BPF_JEQ + BPF_K, AUDIT_ARCH_X86_64,
                             0, allow_idx - i - 1))
        elif op[0] == "jnr":
            out.append(_jump(BPF_JMP + BPF_JEQ + BPF_K, op[1], 0, 3))
        elif op[0] == "juid":
            out.append(_jump(BPF_JMP + BPF_JEQ + BPF_K, euid,
                             allow_idx - i - 1, 0))
        elif op[0] == "errno":
            out.append(_stmt(BPF_RET + BPF_K, SECCOMP_RET_ERRNO | 1))
        elif op[0] == "allow":
            out.append(_stmt(BPF_RET + BPF_K, SECCOMP_RET_ALLOW))
    blob = b"".join(out)
    buf = (ctypes.c_ubyte * len(blob))(*blob)
    fprog = struct.pack("HHP", len(out), 0, ctypes.addressof(buf))
    libc = ctypes.CDLL("libc.so.6", use_errno=True)
    PR_SET_NO_NEW_PRIVS = 38
    PR_SET_SECCOMP = 22
    SECCOMP_MODE_FILTER = 2
    if libc.prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), "prctl(NO_NEW_PRIVS)")
    if libc.prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, fprog, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), "prctl(SECCOMP)")
    # Keep buf alive across the exec: it is referenced until now, and the
    # kernel copied the program at install time, so exec is safe.
    del buf


def main(argv):
    if len(argv) < 2:
        sys.stderr.write("usage: chowndeny.py CMD [ARGS...]\n")
        return 2
    install()
    os.execvp(argv[1], argv[1:])
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
