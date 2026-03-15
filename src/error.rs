use thiserror::Error;

#[derive(Error, Debug)]
pub enum CziError {
    #[error("file error: {source}")]
    File { source: FileError },

    #[error("input error: {source}")]
    Input { source: InputError },

    #[error("internal error: {source}")]
    Internal { source: InternalError },

    #[error("unsupported: {source}")]
    Unsupported { source: UnsupportedError },
}

#[derive(Error, Debug)]
pub enum FileError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid CZI file: {context}")]
    InvalidFormat { context: String },

    #[error("invalid segment magic at offset {offset}: expected '{expected}', got '{actual}'")]
    InvalidMagic {
        offset: u64,
        expected: String,
        actual: String,
    },

    #[error("metadata XML is invalid UTF-8: {context}")]
    InvalidUtf8 { context: String },

    #[error("metadata parse error: {context}")]
    MetadataParse { context: String },

    #[error("decompression error: {context}")]
    Decompression { context: String },
}

#[derive(Error, Debug)]
pub enum InputError {
    #[error("missing required dimension '{dimension}'")]
    MissingDimension { dimension: String },

    #[error("{field} index out of range: got {index}, max {max}")]
    OutOfRange {
        field: String,
        index: usize,
        max: usize,
    },

    #[error("invalid input for {field}: {detail}")]
    InvalidArgument { field: String, detail: String },
}

#[derive(Error, Debug)]
pub enum InternalError {
    #[error("arithmetic overflow during {operation}")]
    Overflow { operation: String },
}

#[derive(Error, Debug)]
pub enum UnsupportedError {
    #[error("unsupported subblock directory schema '{schema}'")]
    DirectorySchema { schema: String },

    #[error("unsupported subblock header schema '{schema}'")]
    SubBlockSchema { schema: String },

    #[error("unsupported compression mode '{mode}'")]
    Compression { mode: String },

    #[error("unsupported pixel type '{pixel_type}'")]
    PixelType { pixel_type: String },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ErrorSource {
    File,
    Input,
    Internal,
    Unsupported,
}

impl CziError {
    pub fn source(&self) -> ErrorSource {
        match self {
            Self::File { .. } => ErrorSource::File,
            Self::Input { .. } => ErrorSource::Input,
            Self::Internal { .. } => ErrorSource::Internal,
            Self::Unsupported { .. } => ErrorSource::Unsupported,
        }
    }

    pub fn file_invalid_format(context: impl Into<String>) -> Self {
        Self::File {
            source: FileError::InvalidFormat {
                context: context.into(),
            },
        }
    }

    pub fn file_invalid_magic(
        offset: u64,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::File {
            source: FileError::InvalidMagic {
                offset,
                expected: expected.into(),
                actual: actual.into(),
            },
        }
    }

    pub fn file_metadata(context: impl Into<String>) -> Self {
        Self::File {
            source: FileError::MetadataParse {
                context: context.into(),
            },
        }
    }

    pub fn file_invalid_utf8(context: impl Into<String>) -> Self {
        Self::File {
            source: FileError::InvalidUtf8 {
                context: context.into(),
            },
        }
    }

    pub fn file_decompression(context: impl Into<String>) -> Self {
        Self::File {
            source: FileError::Decompression {
                context: context.into(),
            },
        }
    }

    pub fn input_out_of_range(field: impl Into<String>, index: usize, max: usize) -> Self {
        Self::Input {
            source: InputError::OutOfRange {
                field: field.into(),
                index,
                max,
            },
        }
    }

    pub fn input_missing_dim(dimension: impl Into<String>) -> Self {
        Self::Input {
            source: InputError::MissingDimension {
                dimension: dimension.into(),
            },
        }
    }

    pub fn input_argument(field: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Input {
            source: InputError::InvalidArgument {
                field: field.into(),
                detail: detail.into(),
            },
        }
    }

    pub fn internal_overflow(operation: impl Into<String>) -> Self {
        Self::Internal {
            source: InternalError::Overflow {
                operation: operation.into(),
            },
        }
    }

    pub fn unsupported_directory_schema(schema: impl Into<String>) -> Self {
        Self::Unsupported {
            source: UnsupportedError::DirectorySchema {
                schema: schema.into(),
            },
        }
    }

    pub fn unsupported_subblock_schema(schema: impl Into<String>) -> Self {
        Self::Unsupported {
            source: UnsupportedError::SubBlockSchema {
                schema: schema.into(),
            },
        }
    }

    pub fn unsupported_compression(mode: impl Into<String>) -> Self {
        Self::Unsupported {
            source: UnsupportedError::Compression { mode: mode.into() },
        }
    }

    pub fn unsupported_pixel_type(pixel_type: impl Into<String>) -> Self {
        Self::Unsupported {
            source: UnsupportedError::PixelType {
                pixel_type: pixel_type.into(),
            },
        }
    }
}

impl From<std::io::Error> for CziError {
    fn from(value: std::io::Error) -> Self {
        Self::File {
            source: FileError::Io(value),
        }
    }
}

pub type Result<T> = std::result::Result<T, CziError>;
