#!/bin/sh
# A minimal ELF that classifies as a Go payload, emitted as bytes on stdout.
#
# It is a fixture, not a program: nothing here executes. It exists because no
# row of DISTRO_ROWS_M5 is a Go image and inventing a registry reference is
# refused, so the classifier's Go arm would otherwise have no end-to-end
# driver. What it proves is that the arm fires through `podbox run`: a
# `PT_INTERP` (so it is not declined as static) and a `.note.go.buildid`
# section (so it is declined as Go).
#
# Layout, all little-endian (384 bytes):
#   0-63    ELF header: ET_REL, x86-64, one program header at 64,
#           three section headers at 120, string table index 2.
#   64-119  PT_INTERP pointing at the interpreter string at 340.
#   120-183 section 0 (null), 184-247 `.note.go.buildid` at content 368,
#           248-311 `.shstrtab` at content 312.
#   312-339 section names (28 bytes). 340-367 the interpreter string
#           (27 characters with NUL). 368-383 sixteen content bytes for the
#           note (zeros: the classifier reads the section NAME, never the
#           content).
#
# ⭐ ET_REL on purpose: `execve` of it answers ENOEXEC deterministically, so
# the run that carries it exits 126 (the shell's and docker's "found and not
# invocable") after printing the decline, whatever kernel runs it. A type the
# kernel would execute would need real machine code, and real machine code in
# a fixture is a payload wearing a fixture's clothes.
#
# Usage: sh govictim.sh > go-victim; chmod +x go-victim
# Verify: `file go-victim` reads "ELF 64-bit LSB relocatable, x86-64".
set -u
# ELF header: magic, 64-bit, LE, version, ABI, ET_REL, x86-64, version,
# entry 0, phoff 64, shoff 120, flags 0, ehsize 64, phentsize 56, phnum 1,
# shentsize 64, shnum 3, shstrndx 2.
printf '\177\105\114\106\002\001\001\000\000\000\000\000\000\000\000\000'
printf '\001\000\076\000\001\000\000\000'
printf '\000\000\000\000\000\000\000\000'
printf '\100\000\000\000\000\000\000\000'
printf '\170\000\000\000\000\000\000\000'
printf '\000\000\000\000\100\000\070\000\001\000\100\000\003\000\002\000'
# Program header: PT_INTERP, flags R, offset 340, vaddr 0, paddr 0,
# filesz 27, memsz 27, align 1.
printf '\003\000\000\000\004\000\000\000'
printf '\124\001\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000'
printf '\033\000\000\000\000\000\000\000'
printf '\033\000\000\000\000\000\000\000'
printf '\001\000\000\000\000\000\000\000'
# Section 0: 64 zeros.
printf '\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000'
# Section 1 (.note.go.buildid): name 1, type NOTE, no flags, no addr,
# offset 368, size 16, align 1.
printf '\001\000\000\000\007\000\000\000'
printf '\000\000\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000'
printf '\160\001\000\000\000\000\000\000'
printf '\020\000\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000'
printf '\001\000\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000'
# Section 2 (.shstrtab): name 18, type STRTAB, offset 312, size 28, align 1.
printf '\022\000\000\000\003\000\000\000'
printf '\000\000\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000'
printf '\070\001\000\000\000\000\000\000'
printf '\034\000\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000'
printf '\001\000\000\000\000\000\000\000'
printf '\000\000\000\000\000\000\000\000'
# shstrtab content: NUL .note.go.buildid NUL .shstrtab NUL (29 bytes).
printf '\000\056\156\157\164\145\056\147\157\056\142\165\151\154\144\151\144\000'
printf '\056\163\150\163\164\162\164\141\142\000'
# The interpreter string (27 bytes with NUL).
printf '\057\154\151\142\066\064\057\154\144\055\154\151\156\165\170\055\170\070\066\055\066\064\056\163\157\056\062\000'
# Sixteen note content bytes (zeros; never read).
printf '\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000'
