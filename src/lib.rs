#![doc = include_str!("../README.md")]

mod error;
mod io;
mod metadata;
mod parse;
mod reader;
mod types;

pub use error::{
    CziError, ErrorSource, FileError, InputError, InternalError, Result, UnsupportedError,
};
pub use io::ReadSeek;
pub use reader::CziFile;
pub use types::{DatasetSummary, SummaryChannel, SummaryScaling};

/// Type alias for [`CziFile`] for ergonomic and generic usage across reader libraries.
pub type Reader = CziFile;
