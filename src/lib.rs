#![doc = include_str!("../README.md")]

pub mod error;
mod metadata;
mod parse;
mod reader;
pub mod types;

pub use error::{CziError, Result};
pub use reader::CziFile;
pub use types::*;
