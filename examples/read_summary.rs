use std::env;
use std::process;

use czi::{CziFile, Result};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-czi-file>", args[0]);
        process::exit(1);
    }

    let path = &args[1];
    let mut czi = CziFile::open(path)?;

    println!("=== CZI File Information ===");
    println!("Version: {:?}", czi.version());

    let summary = czi.summary()?;
    println!("Logical frames: {}", summary.logical_frame_count);
    println!("Sizes: {:?}", summary.sizes);
    println!("Channels: {:?}", summary.channels);

    if let Some(scaling) = &summary.scaling {
        println!("Scaling: {:?}", scaling);
    }

    let first_plane = czi.read_frame_2d(0, 0, 0, 0)?;
    println!("First plane pixels: {}", first_plane.len());

    Ok(())
}
