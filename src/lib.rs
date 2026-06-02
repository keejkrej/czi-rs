#![doc = include_str!("../README.md")]

mod error;
mod io;
mod metadata;
mod parse;
mod reader;
mod types;

pub use error::{CziError, Result};
pub use io::ReadSeek;
pub use reader::CziFile;
pub use types::{DatasetSummary, SummaryChannel, SummaryScaling};
