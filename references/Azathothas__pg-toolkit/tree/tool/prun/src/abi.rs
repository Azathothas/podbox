//! The note a tool reads to decide which C library a program was built for.
//!
//! ⚠ **This is a claim about the payload, made by the launcher, and it is not
//! true of the launcher itself.** `prun` is a static musl binary; the bundle
//! it starts carries a glibc payload, and a tool that probes `/proc/self/exe`
//! for an ABI tag is asking about the program, not about the thing that
//! started it. Without the note such a tool sees no tag at all and takes the
//! branch it keeps for "unknown", which on more than one of them means
//! refusing to run.
//!
//! ⛔ It is a note section and nothing else: no code, no symbol, no behaviour.
//! Nothing this crate can run observes it, so `scripts/common/prun-build.sh`
//! reads it off the BUILT binary rather than trusting this file, with a control
//! that removes the section and requires the reading to come back empty.
//!
//! ⚠ **On a gnu target this is the SECOND tag in the file.** The startup code
//! writes its own first - measured at `Linux 3.2.0` - so anything reading the
//! first note it finds gets the toolchain's answer and not this one. The
//! shipped target is musl, where this is the only tag.
//!
//! SPDX-License-Identifier: 0BSD

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
std::arch::global_asm!(
    r#"
    .section .note.ABI-tag,"a",%note
    .p2align 2
    .long 4              /* name size: "GNU\0" */
    .long 16             /* descriptor size: four words */
    .long 1              /* NT_GNU_ABI_TAG */
    .asciz "GNU"
    .long 0              /* ELF_NOTE_OS_LINUX */
    .long 2              /* earliest kernel this claims: 2.6.0 */
    .long 6
    .long 0
    "#
);
