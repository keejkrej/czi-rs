# czi

[![Crates.io](https://img.shields.io/crates/v/czi.svg)](https://crates.io/crates/czi)
[![Documentation](https://docs.rs/czi/badge.svg)](https://docs.rs/czi)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A fast, memory-efficient, pure Rust library for reading Zeiss CZI (`.czi`) microscopy files.

`czi` provides a clean, ergonomic, high-performance API designed for bioimaging pipelines, image analysis, and scientific computing. It handles subblock directories, multi-dimensional coordinate mapping, metadata XML parsing, and decompression without any C/C++ dependencies.

## Features

- **Pure Rust**: 100% safe, portable Rust with zero external C/C++ libraries.
- **Consistent Reader API**: Mirrors the bioimaging API convention established across sister formats (like [`nd2`](https://crates.io/crates/nd2)).
- **Multi-Dimensional Navigation**: Read frames by linear sequence index or by multidimensional coordinates (Scene, Time, Channel, Z-stack).
- **Compression Support**: Handles uncompressed subblocks, ZSTD (`zstd0`), and high/low packed ZSTD (`zstd1`).
- **Comprehensive Metadata**: Extracts image dimensions, channel names/colors/types, physical voxel calibration (X/Y/Z scaling in microns), and document details.
- **Streaming & In-Memory**: Read directly from the filesystem with `CziFile::open` or from any in-memory/stream buffer implementing `Read + Seek` with `CziFile::open_reader`.
- **Categorized Error Handling**: Structured errors classified into `File`, `Input`, `Internal`, and `Unsupported` sources.

## Installation

Add `czi` to your `Cargo.toml`:

```toml
[dependencies]
czi = "1.0.0"
```

## Quick Start

```rust,no_run
use czi::{CziFile, Result};

fn main() -> Result<()> {
    // Open a CZI file from disk
    let mut czi = CziFile::open("sample.czi")?;

    // Format version
    let (major, minor) = czi.version();
    println!("CZI format version: {major}.{minor}");

    // Summary metadata overview
    let summary = czi.summary()?;
    println!("Dimensions: {:?}", summary.sizes);
    println!("Logical frame count: {}", summary.logical_frame_count);
    for channel in &summary.channels {
        println!("Channel #{}: {:?}", channel.index, channel.name);
    }

    if let Some(scaling) = &summary.scaling {
        println!("Voxel size: X={:?}, Y={:?}, Z={:?} {:?}",
            scaling.x, scaling.y, scaling.z, scaling.unit);
    }

    // Read a 2D plane at (Scene=0, Time=0, Channel=0, Z=0)
    let plane_pixels = czi.read_frame_2d(0, 0, 0, 0)?;
    let width = summary.sizes["X"];
    let height = summary.sizes["Y"];
    assert_eq!(plane_pixels.len(), width * height);

    // Or read by flat frame index (0..summary.logical_frame_count)
    let first_frame = czi.read_frame(0)?;
    println!("Read first frame with {} pixels", first_frame.len());

    Ok(())
}
```

## Dimensional Coordinates

Microscopy images commonly record data across multiple experimental dimensions:

| Axis | Meaning |
|:---:|---|
| **X** | Width (columns) |
| **Y** | Height (rows) |
| **C** | Channel (fluorescence wavelength / detector) |
| **Z** | Focal plane (depth) |
| **T** | Time point |
| **S** | Scene / Stage position |

`czi` supports coordinate-based plane extraction via `read_frame_2d(s, t, c, z)`:

```rust,no_run
use czi::CziFile;

# fn run() -> czi::Result<()> {
let mut czi = CziFile::open("timelapse.czi")?;

// Read scene 0, timepoint 3, channel 1, z-slice 5
let plane = czi.read_frame_2d(0, 3, 1, 5)?;
# Ok(())
# }
```

Decoded pixel buffers are returned as `Vec<u16>` in row-major order (`height * width`).

## Reading from Memory / Custom Streams

For network streams, archives, or test fixtures, `open_reader` accepts any source implementing `Read + Seek`:

```rust,no_run
use std::io::Cursor;
use czi::CziFile;

# fn run(data: Vec<u8>) -> czi::Result<()> {
let cursor = Cursor::new(data);
let mut czi = CziFile::open_reader(cursor)?;
let summary = czi.summary()?;
# Ok(())
# }
```

## Error Handling

Errors are returned as [`CziError`](https://docs.rs/czi/latest/czi/enum.CziError.html) with source categories:

```rust,no_run
use czi::{CziError, CziFile, ErrorSource};

let res = CziFile::open("nonexistent.czi");
if let Err(err) = res {
    match err.source() {
        ErrorSource::File => eprintln!("File or corruption error: {err}"),
        ErrorSource::Input => eprintln!("Invalid index or parameter: {err}"),
        ErrorSource::Unsupported => eprintln!("Unsupported feature/codec: {err}"),
        ErrorSource::Internal => eprintln!("Internal overflow: {err}"),
    }

    // Or use the ergonomic predicate methods:
    if err.is_file() {
        // Handle filesystem / format errors
    }
}
```

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
