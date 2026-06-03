/// Magic bytes for .ars files: "ARS" + version byte
pub const MAGIC: &[u8; 4] = b"ARS\x01";
/// Current format version
pub const FORMAT_VERSION: u16 = 1;
/// Minimum supported version for reading
pub const MIN_VERSION: u16 = 1;

/// Compression codec used for section data
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None = 0,
    Zstd = 1,
}

impl TryFrom<u8> for Compression {
    type Error = crate::ArsError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Compression::None),
            1 => Ok(Compression::Zstd),
            _ => Err(crate::ArsError::InvalidSectionType(v)),
        }
    }
}

/// Section type identifiers in the binary file
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionId {
    /// FST of exact-match block domains
    BlockExact = 0x01,
    /// FST of suffix-match block domains (stored reversed for prefix search)
    BlockSuffix = 0x02,
    /// Aho-Corasick serialized data for keyword block rules
    BlockKeyword = 0x03,
    /// Newline-separated regex patterns for block rules
    BlockRegex = 0x04,
    /// FST of exact-match allow (whitelist) domains
    AllowExact = 0x11,
    /// FST of suffix-match allow domains
    AllowSuffix = 0x12,
    /// Keyword allow rules
    AllowKeyword = 0x13,
    /// Regex allow rules
    AllowRegex = 0x14,
    /// Rewrite rules: JSON array of {pattern, target}
    Rewrite = 0x20,
    /// Bloom filter bytes (optional, for fast pre-screening)
    BloomFilter = 0x30,
    /// Metadata JSON
    Metadata = 0x40,
}

impl TryFrom<u8> for SectionId {
    type Error = crate::ArsError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x01 => Ok(SectionId::BlockExact),
            0x02 => Ok(SectionId::BlockSuffix),
            0x03 => Ok(SectionId::BlockKeyword),
            0x04 => Ok(SectionId::BlockRegex),
            0x11 => Ok(SectionId::AllowExact),
            0x12 => Ok(SectionId::AllowSuffix),
            0x13 => Ok(SectionId::AllowKeyword),
            0x14 => Ok(SectionId::AllowRegex),
            0x20 => Ok(SectionId::Rewrite),
            0x30 => Ok(SectionId::BloomFilter),
            0x40 => Ok(SectionId::Metadata),
            _ => Err(crate::ArsError::InvalidSectionType(v)),
        }
    }
}

/// File header — fixed 32 bytes
/// Layout:
///   [0..4]   magic "ARS\x01"
///   [4..6]   version (LE u16)
///   [6..8]   compression codec (u8) + reserved (u8)
///   [8..12]  total rule count (LE u32)
///   [12..16] section count (LE u32)
///   [16..20] metadata offset from file start (LE u32)
///   [20..28] reserved (zeroed)
///   [28..32] header CRC32 (LE u32, covers bytes 0..28)
pub const HEADER_SIZE: usize = 32;

/// Section header — 10 bytes per section
/// Layout:
///   [0]     section id (SectionId)
///   [1]     compression (Compression)
///   [2..6]  compressed data length (LE u32)
///   [6..10] uncompressed data length (LE u32)
pub const SECTION_HEADER_SIZE: usize = 10;

/// Trailer — 4 bytes at end of file
/// CRC32 over all bytes except the trailer itself
pub const TRAILER_SIZE: usize = 4;
