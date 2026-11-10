//! One error type for `pull` and `extract`.
//!
//! `pod-extract` gives a `pod_core::Failure`: a refused entry names the entry
//! AND the layer digest (`TODO/extract.md` T-0304), and
//! `pod_extract::refused` gives that cause back. The store's own errors stay
//! `podvm_image::Error`.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// A failure of the extraction: a refused entry, a blob whose digest does
    /// not agree, a compression this build cannot read, a refused system call
    /// or the space precheck.
    Extract(pod_core::Failure),
    /// Passed through from `podvm-image`, so a caller has one error type for
    /// `pull` and `extract` rather than two.
    Image(podvm_image::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Extract(e) => write!(f, "{e}"),
            Error::Image(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<pod_core::Failure> for Error {
    fn from(e: pod_core::Failure) -> Error {
        Error::Extract(e)
    }
}

impl From<podvm_image::Error> for Error {
    fn from(e: podvm_image::Error) -> Error {
        Error::Image(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
