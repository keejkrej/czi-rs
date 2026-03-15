//! Pure Rust reader for Zeiss CZI microscopy files.
//!
//! The API is intentionally shaped like `nd2-rs`: open a file once, inspect
//! metadata and dimensions, then read decoded frames or individual subblocks.

pub mod error;
mod metadata;
mod parse;
mod reader;
pub mod types;

pub use error::{CziError, Result};
pub use reader::CziFile;
pub use types::*;
