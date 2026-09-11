//! Execute ELF image bytes from memory on Linux and FreeBSD.
//!
//! Pass the image in a `&[u8]`. [`MemFdExecutable`] writes it to an anonymous
//! file and executes it.
//!
//! Linux uses `execveat(2)` first. FreeBSD uses `fexecve(2)`. The library can
//! use a temporary executable file if descriptor execution is not available.
//! Prepared images use the configured file seals. Repeated calls can use the
//! same prepared image. Execution failures return `std::io::Error` values.
//! The library does not write diagnostic text to standard error.
//!
//! # Environment
//!
//! Set `NO_MEMFDEXEC=1` to skip memfd execution and use a temporary file.
//!
//! # Example
//!
//! ```no_run
//! use memfd_ng::{MemFdExecutable, Stdio};
//!
//! let code = std::fs::read("/bin/sh").unwrap();
//! let mut sh = MemFdExecutable::new("sh", &code)
//!     .arg("-c")
//!     .arg("echo in-memory; exit 7")
//!     .stdout(Stdio::piped())
//!     .spawn()
//!     .unwrap();
//!
//! let output = sh.wait_with_output().unwrap();
//! assert_eq!(output.stdout, b"in-memory\n");
//! assert_eq!(output.status.code(), Some(7));
//! ```

mod anon_pipe;
mod child;
mod command_env;
mod cvt;
mod executable;
mod file_desc;
mod output;
mod process;
mod stdio;
mod sys;

pub use child::{Child, ChildStderr, ChildStdin, ChildStdout};
pub use executable::{MemFdExecutable, SealFlags};
pub use file_desc::FileDesc;
pub use output::Output;
pub use process::ExitStatus;
pub use stdio::Stdio;

/// Wire access to the CLOEXEC-pipe protocol for the integration fuzzer.
/// Compiled only under the `test-hooks` feature; the shapes are documented
/// on the individual functions in `sys`.
#[cfg(feature = "test-hooks")]
pub mod protocol {
    pub use crate::sys::{pipe_read, pipe_write_errno, pipe_write_named_path, PipeMsg};
}
