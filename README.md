# czi-rs

Pure Rust reader for Zeiss CZI microscopy files.

`czi-rs` is a small, dependency-light library for inspecting CZI container
structure, reading parsed metadata, and decoding layer-0 image planes into raw
pixel buffers.

## Install

```toml
[dependencies]
czi-rs = "0.1.0"
```

## What it exposes

- Open a `.czi` file once and reuse the handle.
- Build a lightweight dataset summary via `summary()`.
- Inspect file header data, subblock directory entries, and attachments.
- Read parsed metadata from the embedded XML document.
- Enumerate frame indices and decode planes into grayscale `Vec<u16>` buffers.
- Drop to bitmap-level access explicitly when you need raw CZI pixel layouts.

## Example

```rust,no_run
use czi_rs::{CziFile, Dimension, PlaneIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut czi = CziFile::open("example.czi")?;

    let summary = czi.summary()?;
    let version = czi.version();
    let sizes = czi.sizes()?;
    let metadata = czi.metadata()?;

    println!("CZI version: {}.{}", version.0, version.1);
    println!("logical frames: {}", summary.logical_frame_count);
    println!("sizes: {sizes:?}");
    println!("channels: {}", metadata.channels.len());

    let plane = PlaneIndex::new()
        .with(Dimension::S, 0)
        .with(Dimension::T, 0)
        .with(Dimension::C, 0)
        .with(Dimension::Z, 0);
    let frame = czi.read_frame_2d(0, 0, 0, 0)?;
    let bitmap = czi.read_frame_2d_bitmap(0, 0, 0, 0)?;

    println!(
        "decoded frame: {} pixels, bitmap {:?} {}x{} ({} bytes)",
        frame.len(),
        bitmap.pixel_type,
        bitmap.width,
        bitmap.height,
        bitmap.as_bytes().len()
    );

    Ok(())
}
```

## Notes

- `read_frame()` and `read_frame_2d()` return grayscale `Vec<u16>` buffers to
  match the simple image-reader API used by `nd2-rs`.
- `read_frame_bitmap()`, `read_frame_2d_bitmap()`, and `read_plane()` expose
  raw/interleaved bitmap access when you need format-specific details.
- Raw pixel data is stored as an interleaved byte buffer in `Bitmap::data`.
- Helpers such as `Bitmap::to_u16_vec()`, `Bitmap::into_gray_u16()`, and
  `Bitmap::to_f32_vec()` are available for compatible pixel formats.
- Unsupported compression modes or pixel formats are reported as structured
  `CziError` values.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
