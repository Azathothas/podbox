#!/usr/bin/env python3
"""mkrootfs.py — assemble a bootable initramfs without host mknod.

This runtime class denies mknod, so device nodes cannot be created on
the host — yet a guest initramfs needs /dev/console or PID 1 is silent
(no console to write to, no shell to reach). The node is written
BYTE-WISE into the newc cpio archive: the kernel's own initramfs
unpacker creates it guest-side, beyond the reach of this tenant's
seccomp filter.

newc header: 6-byte magic + 13 fields x 8 hex digits = 110 bytes, then
the NUL-terminated name, header+name padded to 4, data padded to 4.
Fields: ino, mode, uid, gid, nlink, mtime, filesize, devmajor,
devminor, RDEVMAJOR, RDEVMINOR, namesize, check. The device numbers
live in the rdev fields — putting them in devmajor/devminor is the
silent way to ship a character device that is not one.

Usage: mkrootfs.py OUT.cpio [SPEC]...
  SPEC := name,type[,data]
    type: dir|file|char|symlink  (file: DATA is a host path)
Examples:
  mkrootfs.py root.cpio --dir dev --char dev/console:5:1 \
      --file /path/init:init:755
"""
import sys


def hdr(ino, mode, fsize, devmaj, devmin, rdevmaj, rdevmin, name):
    nb = name.encode()
    h = "070701" + "".join(
        f"{v:08X}"
        for v in (ino, mode, 0, 0, 1, 0, fsize, devmaj, devmin,
                  rdevmaj, rdevmin, len(nb) + 1, 0)
    )
    assert len(h) == 110, f"newc header must be 110 bytes, got {len(h)}"
    b = h.encode() + nb + b"\0"
    return b + b"\0" * ((4 - len(b) % 4) % 4)


def pad4(f, n):
    if n % 4:
        f.write(b"\0" * (4 - n % 4))


class Cpio:
    def __init__(self, path):
        self.f = open(path, "wb")
        self.ino = 300000

    def add(self, name, mode, data=b"", rdev=(0, 0)):
        self.f.write(hdr(self.ino, mode, len(data), 0, 0, rdev[0], rdev[1], name))
        if data:
            self.f.write(data)
            pad4(self.f, len(data))
        self.ino += 1

    def trailer(self):
        self.f.write(hdr(0, 0, 0, 0, 0, 0, 0, "TRAILER!!!"))
        pad4(self.f, 0)
        self.f.close()


def main(argv):
    out = argv[1]
    c = Cpio(out)
    args = argv[2:]
    i = 0
    while i < len(args):
        spec = args[i]
        if spec == "--dir":
            c.add(args[i + 1], 0o040755)
            i += 2
        elif spec == "--char":
            # name:maj:min
            name, maj, mnr = args[i + 1].split(":")
            c.add(name, 0o020600, rdev=(int(maj), int(mnr)))
            i += 2
        elif spec == "--file":
            # hostpath:name[:mode]
            parts = args[i + 1].split(":")
            hostpath, name = parts[0], parts[1]
            mode = int(parts[2], 8) if len(parts) > 2 else 0o100755
            c.add(name, mode, open(hostpath, "rb").read())
            i += 2
        elif spec == "--data":
            # literal:name (mode 755 file from stdin text)
            literal, name = args[i + 1].split(":", 1)
            c.add(name, 0o100755, literal.encode())
            i += 2
        elif spec == "--symlink":
            # target:name
            target, name = args[i + 1].split(":", 1)
            c.add(name, 0o120777, target.encode())
            i += 2
        else:
            raise SystemExit(f"unknown spec {spec!r}")
    c.trailer()
    print(out)


if __name__ == "__main__":
    main(sys.argv)
