# Changelog
 
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.0.0] - 2026-10-10

### Changed
- Published to crates.io as `czi` (dropping the legacy `-rs` name).
- Dual-licensed under MIT OR Apache-2.0.
- Full API alignment with companion microscopy crates (e.g. `nd2`).
- Derived `Serialize` and `Deserialize` on `DatasetSummary`, `SummaryChannel`, and `SummaryScaling`.
- Changed `version(&self)` return type to `(u32, u32)` to match format versions and `DatasetSummary`.
- Changed `path(&self)` return type to `Option<&Path>` (`None` for in-memory stream readers).
- Added public `sizes(&self) -> Result<BTreeMap<String, usize>>` method to `CziFile`.
- Re-exported full error taxonomy (`ErrorSource`, `FileError`, `InputError`, `InternalError`, `UnsupportedError`).
- Implemented `Display` and `Hash` for `ErrorSource`.
- Added predicate helpers to `CziError`: `is_file()`, `is_input()`, `is_internal()`, `is_unsupported()`.
- Implemented `std::fmt::Debug` for `CziFile`.
- Added `Reader` type alias for `CziFile`.
- Added comprehensive unit tests and `read_summary` example.
- Standardized README and documentation.

## [0.2.0] - 2026-03-09

### Added
- Initial CZI reader implementation.
- Subblock directory decoding, metadata XML parsing, and ZSTD decompression.
