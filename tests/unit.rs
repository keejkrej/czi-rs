//! Unit tests for czi that do not require an external CZI fixture file.

use std::io::{Cursor, Write};
use czi::{CziError, CziFile};

#[test]
fn test_open_nonexistent_fails() {
    let res = CziFile::open("nonexistent_file_xyz.czi");
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.is_file(), "expected file error for missing path");
}

#[test]
fn test_open_invalid_file_fails() {
    let tmp = std::env::temp_dir().join("czi_rs_test_garbage.czi");
    let mut f = std::fs::File::create(&tmp).unwrap();
    f.write_all(&[0u8; 200]).unwrap();
    drop(f);

    let err = match CziFile::open(&tmp) {
        Ok(_) => unreachable!("garbage file should not open successfully"),
        Err(err) => err,
    };
    assert!(
        err.is_file(),
        "invalid file header should be classified as file error"
    );
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_open_reader_invalid_stream() {
    let cursor = Cursor::new(vec![0u8; 100]);
    let err = match CziFile::open_reader(cursor) {
        Ok(_) => unreachable!("empty cursor should fail to parse"),
        Err(err) => err,
    };
    assert!(err.is_file());
    assert_eq!(err.source(), czi::ErrorSource::File);
}

#[test]
fn test_error_classification_helpers() {
    let err = CziError::file_invalid_format("test format");
    assert!(err.is_file());
    assert!(!err.is_input());
    assert!(!err.is_internal());
    assert!(!err.is_unsupported());

    let err = CziError::input_out_of_range("channel", 5, 2);
    assert!(!err.is_file());
    assert!(err.is_input());

    let err = CziError::internal_overflow("dimension multiply");
    assert!(err.is_internal());

    let err = CziError::unsupported_compression("UnknownCodec");
    assert!(err.is_unsupported());
}
