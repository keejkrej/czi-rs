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
- Inspect file header data, subblock directory entries, and attachments.
- Read parsed metadata from the embedded XML document.
- Enumerate frame indices and decode planes into a `Bitmap`.

## Example

```rust,no_run
use czi_rs::{CziFile, Dimension, PlaneIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut czi = CziFile::open("example.czi")?;

    let version = czi.version();
    let sizes = czi.sizes()?;
    let metadata = czi.metadata()?;

    println!("CZI version: {}.{}", version.0, version.1);
    println!("sizes: {sizes:?}");
    println!("channels: {}", metadata.channels.len());

    let plane = PlaneIndex::new()
        .with(Dimension::S, 0)
        .with(Dimension::T, 0)
        .with(Dimension::C, 0)
        .with(Dimension::Z, 0);
    let bitmap = czi.read_plane(&plane)?;

    println!(
        "decoded {:?} plane: {}x{} ({} bytes)",
        bitmap.pixel_type,
        bitmap.width,
        bitmap.height,
        bitmap.as_bytes().len()
    );

    Ok(())
}
```

## Notes

- Pixel data is returned as a raw interleaved byte buffer in `Bitmap::data`.
- Helpers such as `Bitmap::to_u16_vec()` and `Bitmap::to_f32_vec()` are
  available for compatible pixel formats.
- Unsupported compression modes or pixel formats are reported as structured
  `CziError` values.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
