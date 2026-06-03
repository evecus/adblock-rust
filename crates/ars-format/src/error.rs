use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArsError {
    #[error("Invalid magic bytes: expected ARS\\x01")]
    InvalidMagic,
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u16),
    #[error("Checksum mismatch: expected {expected:#010x}, got {got:#010x}")]
    ChecksumMismatch { expected: u32, got: u32 },
    #[error("Invalid section type: {0}")]
    InvalidSectionType(u8),
    #[error("Decompression failed: {0}")]
    DecompressError(String),
    #[error("FST error: {0}")]
    FstError(String),
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Truncated data at section {section}")]
    TruncatedData { section: String },
}
