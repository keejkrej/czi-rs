# czi-rs

Pure Rust reader for Zeiss CZI microscopy files.

`czi-rs` is a small, dependency-light library for reading CZI datasets through
one simple public surface: dataset summary plus frame reads.

## Install

```toml
[dependencies]
czi-rs = "0.2.0"
```

## What it exposes

- Open a `.czi` file once and reuse the handle.
- Read the file format `version()`.
- Build a lightweight dataset summary via `summary()`.
- Enumerate frame indices and decode planes into grayscale `Vec<u16>` buffers.

## Example

```rust,no_run
use czi_rs::CziFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut czi = CziFile::open("example.czi")?;

    let summary = czi.summary()?;
    let version = czi.version();

    println!("CZI version: {}.{}", version.0, version.1);
    println!("logical frames: {}", summary.logical_frame_count);
    println!("sizes: {:?}", summary.sizes);
    println!("channels: {}", summary.channels.len());

    let frame = czi.read_frame_2d(0, 0, 0, 0)?;

    println!("decoded frame pixels: {}", frame.len());

    Ok(())
}
```

## Notes

- `read_frame()` and `read_frame_2d()` return grayscale `Vec<u16>` buffers to
  match the simple image-reader API used by `nd2-rs`.
- Unsupported compression modes or pixel formats are reported as structured
  `CziError` values.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
