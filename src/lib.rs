#![doc = include_str!("../README.md")]

mod error;
mod metadata;
mod parse;
mod reader;
mod types;

pub use error::{CziError, Result};
pub use reader::CziFile;
pub use types::{DatasetSummary, SummaryChannel, SummaryScaling};
