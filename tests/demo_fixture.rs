use std::path::Path;

use czi_rs::{CziFile, Dimension};

const DEMO_CZI_PATH: &str = r"C:\Users\ctyja\data\czi\Demo LISH 4x8 15pct 647.czi";

fn demo_fixture() -> Option<&'static Path> {
    let path = Path::new(DEMO_CZI_PATH);
    path.exists().then_some(path)
}

#[test]
fn opens_demo_fixture_and_reads_metadata() {
    let Some(path) = demo_fixture() else {
        eprintln!("skipping: fixture not found at {}", DEMO_CZI_PATH);
        return;
    };

    let mut czi = CziFile::open(path).expect("open fixture");
    assert!(
        !czi.subblocks().is_empty(),
        "fixture should expose subblocks"
    );

    let sizes = czi.sizes().expect("sizes");
    assert!(sizes["X"] > 0, "fixture width should be positive");
    assert!(sizes["Y"] > 0, "fixture height should be positive");

    let xml = czi.metadata_xml().expect("metadata xml");
    assert!(!xml.is_empty(), "fixture should contain XML metadata");
    assert!(
        xml.contains("<ImageDocument"),
        "fixture metadata should look like CZI XML"
    );

    let metadata = czi.metadata().expect("parsed metadata");
    assert!(
        metadata
            .image
            .sizes
            .get(&Dimension::X)
            .copied()
            .unwrap_or(0)
            > 0,
        "parsed metadata should expose SizeX"
    );
}

#[test]
fn reads_first_demo_plane() {
    let Some(path) = demo_fixture() else {
        eprintln!("skipping: fixture not found at {}", DEMO_CZI_PATH);
        return;
    };

    let mut czi = CziFile::open(path).expect("open fixture");
    let sizes = czi.sizes().expect("sizes");
    let plane = czi.read_frame(0).expect("read first plane");

    assert_eq!(plane.width as usize, sizes["X"]);
    assert_eq!(plane.height as usize, sizes["Y"]);
    assert_eq!(plane.data.len(), plane.stride * plane.height as usize);
}
