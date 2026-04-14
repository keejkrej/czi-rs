use std::env;
use std::path::PathBuf;

use czi_rs::CziFile;

const CZI_TEST_FILE: &str = "CZI_TEST_FILE";

fn demo_fixture() -> Option<PathBuf> {
    let path = env::var_os(CZI_TEST_FILE).map(PathBuf::from)?;
    path.exists().then_some(path)
}

#[test]
fn opens_demo_fixture_and_reads_metadata() {
    let Some(path) = demo_fixture() else {
        eprintln!("skipping: fixture not found in {CZI_TEST_FILE}");
        return;
    };

    let mut czi = CziFile::open(path).expect("open fixture");
    let version = czi.version();
    assert!(version.0 >= 1, "version should be positive");

    let summary = czi.summary().expect("summary");
    assert!(summary.sizes["X"] > 0, "fixture width should be positive");
    assert!(summary.sizes["Y"] > 0, "fixture height should be positive");
}

#[test]
fn reads_first_demo_plane() {
    let Some(path) = demo_fixture() else {
        eprintln!("skipping: fixture not found in {CZI_TEST_FILE}");
        return;
    };

    let mut czi = CziFile::open(path).expect("open fixture");
    let summary = czi.summary().expect("summary");
    let plane = czi.read_frame(0).expect("read first plane");

    assert_eq!(plane.len(), summary.sizes["X"] * summary.sizes["Y"]);
}

#[test]
fn builds_demo_summary() {
    let Some(path) = demo_fixture() else {
        eprintln!("skipping: fixture not found in {CZI_TEST_FILE}");
        return;
    };

    let mut czi = CziFile::open(path).expect("open fixture");
    let summary = czi.summary().expect("summary");

    assert!(summary.version_major >= 1, "summary should expose version");
    assert!(summary.sizes["X"] > 0, "summary should expose width");
    assert!(summary.sizes["Y"] > 0, "summary should expose height");
    assert!(
        summary.logical_frame_count > 0,
        "summary should expose frames"
    );
    assert_eq!(
        summary.channels.len(),
        *summary.sizes.get("C").unwrap_or(&1),
        "summary channels should match channel dimension"
    );
}
