use std::io::Read;

use aho_corasick::AhoCorasick;
use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use crc32fast::Hasher;
use fst::Set;
use regex::RegexSet;

use crate::{
    builder::{ArsMetadata, RewriteEntry},
    format::*,
    ArsError,
};

/// Loaded and ready-to-query ruleset parsed from an .ars file.
pub struct ArsReader {
    pub metadata: ArsMetadata,

    // Block sets
    pub block_exact: Option<Set<Vec<u8>>>,
    pub block_suffix: Option<Set<Vec<u8>>>,
    pub block_keyword: Option<AhoCorasick>,
    pub block_regex: Option<RegexSet>,

    // Allow sets (whitelist)
    pub allow_exact: Option<Set<Vec<u8>>>,
    pub allow_suffix: Option<Set<Vec<u8>>>,
    pub allow_keyword: Option<AhoCorasick>,
    pub allow_regex: Option<RegexSet>,

    // Rewrite rules
    pub rewrites: Vec<RewriteEntry>,
}

impl ArsReader {
    /// Load an .ars file from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, ArsError> {
        if data.len() < HEADER_SIZE + TRAILER_SIZE {
            return Err(ArsError::TruncatedData {
                section: "header".into(),
            });
        }

        // Verify file-level CRC (last 4 bytes)
        let payload = &data[..data.len() - TRAILER_SIZE];
        let stored_crc = u32::from_le_bytes(data[data.len() - 4..].try_into().unwrap());
        let mut hasher = Hasher::new();
        hasher.update(payload);
        let actual_crc = hasher.finalize();
        if actual_crc != stored_crc {
            return Err(ArsError::ChecksumMismatch {
                expected: stored_crc,
                got: actual_crc,
            });
        }

        // Parse header
        let mut cur = std::io::Cursor::new(data);
        let mut magic = [0u8; 4];
        cur.read_exact(&mut magic).map_err(ArsError::Io)?;
        if &magic != MAGIC {
            return Err(ArsError::InvalidMagic);
        }

        let version = cur.read_u16::<LittleEndian>().map_err(ArsError::Io)?;
        if version < MIN_VERSION || version > FORMAT_VERSION {
            return Err(ArsError::UnsupportedVersion(version));
        }

        let _default_compression = cur.read_u8().map_err(ArsError::Io)?;
        let _reserved = cur.read_u8().map_err(ArsError::Io)?;
        let _total_rules = cur.read_u32::<LittleEndian>().map_err(ArsError::Io)?;
        let section_count = cur.read_u32::<LittleEndian>().map_err(ArsError::Io)?;
        let _meta_offset = cur.read_u32::<LittleEndian>().map_err(ArsError::Io)?;
        cur.set_position(HEADER_SIZE as u64); // skip reserved + header CRC

        // Parse sections
        let mut metadata: Option<ArsMetadata> = None;
        let mut block_exact_bytes: Option<Vec<u8>> = None;
        let mut block_suffix_bytes: Option<Vec<u8>> = None;
        let mut block_keyword_patterns: Vec<String> = Vec::new();
        let mut block_regex_patterns: Vec<String> = Vec::new();
        let mut allow_exact_bytes: Option<Vec<u8>> = None;
        let mut allow_suffix_bytes: Option<Vec<u8>> = None;
        let mut allow_keyword_patterns: Vec<String> = Vec::new();
        let mut allow_regex_patterns: Vec<String> = Vec::new();
        let mut rewrites: Vec<RewriteEntry> = Vec::new();

        for _ in 0..section_count {
            let id_byte = cur.read_u8().map_err(ArsError::Io)?;
            let codec_byte = cur.read_u8().map_err(ArsError::Io)?;
            let compressed_len = cur.read_u32::<LittleEndian>().map_err(ArsError::Io)? as usize;
            let _uncompressed_len = cur.read_u32::<LittleEndian>().map_err(ArsError::Io)?;

            let pos = cur.position() as usize;
            if pos + compressed_len > data.len() - TRAILER_SIZE {
                return Err(ArsError::TruncatedData {
                    section: format!("section id={id_byte:#04x}"),
                });
            }
            let raw = &data[pos..pos + compressed_len];
            cur.set_position((pos + compressed_len) as u64);

            let codec = Compression::try_from(codec_byte)?;
            let decompressed =
                decompress(raw, codec).map_err(|e| ArsError::DecompressError(e.to_string()))?;

            let section_id = SectionId::try_from(id_byte)?;
            match section_id {
                SectionId::Metadata => {
                    metadata = Some(serde_json::from_slice(&decompressed).map_err(ArsError::Json)?);
                }
                SectionId::BlockExact => block_exact_bytes = Some(decompressed),
                SectionId::BlockSuffix => block_suffix_bytes = Some(decompressed),
                SectionId::BlockKeyword => {
                    let s = String::from_utf8_lossy(&decompressed);
                    block_keyword_patterns = s.lines().map(|l| l.to_string()).collect();
                }
                SectionId::BlockRegex => {
                    let s = String::from_utf8_lossy(&decompressed);
                    block_regex_patterns = s.lines().map(|l| l.to_string()).collect();
                }
                SectionId::AllowExact => allow_exact_bytes = Some(decompressed),
                SectionId::AllowSuffix => allow_suffix_bytes = Some(decompressed),
                SectionId::AllowKeyword => {
                    let s = String::from_utf8_lossy(&decompressed);
                    allow_keyword_patterns = s.lines().map(|l| l.to_string()).collect();
                }
                SectionId::AllowRegex => {
                    let s = String::from_utf8_lossy(&decompressed);
                    allow_regex_patterns = s.lines().map(|l| l.to_string()).collect();
                }
                SectionId::Rewrite => {
                    rewrites = serde_json::from_slice(&decompressed).map_err(ArsError::Json)?;
                }
                SectionId::BloomFilter => {} // handled by caller if needed
            }
        }

        let metadata = metadata.ok_or_else(|| ArsError::TruncatedData {
            section: "metadata section missing".into(),
        })?;

        // Build runtime structures
        let block_exact = block_exact_bytes
            .map(|b| Set::new(b).map_err(|e| ArsError::FstError(e.to_string())))
            .transpose()?;
        let block_suffix = block_suffix_bytes
            .map(|b| Set::new(b).map_err(|e| ArsError::FstError(e.to_string())))
            .transpose()?;
        let allow_exact = allow_exact_bytes
            .map(|b| Set::new(b).map_err(|e| ArsError::FstError(e.to_string())))
            .transpose()?;
        let allow_suffix = allow_suffix_bytes
            .map(|b| Set::new(b).map_err(|e| ArsError::FstError(e.to_string())))
            .transpose()?;

        let block_keyword = if block_keyword_patterns.is_empty() {
            None
        } else {
            Some(
                AhoCorasick::builder()
                    .ascii_case_insensitive(true)
                    .build(&block_keyword_patterns)
                    .map_err(|e| ArsError::FstError(e.to_string()))?,
            )
        };
        let allow_keyword = if allow_keyword_patterns.is_empty() {
            None
        } else {
            Some(
                AhoCorasick::builder()
                    .ascii_case_insensitive(true)
                    .build(&allow_keyword_patterns)
                    .map_err(|e| ArsError::FstError(e.to_string()))?,
            )
        };

        let block_regex = if block_regex_patterns.is_empty() {
            None
        } else {
            Some(RegexSet::new(&block_regex_patterns).map_err(ArsError::RegexError)?)
        };
        let allow_regex = if allow_regex_patterns.is_empty() {
            None
        } else {
            Some(RegexSet::new(&allow_regex_patterns).map_err(ArsError::RegexError)?)
        };

        Ok(Self {
            metadata,
            block_exact,
            block_suffix,
            block_keyword,
            block_regex,
            allow_exact,
            allow_suffix,
            allow_keyword,
            allow_regex,
            rewrites,
        })
    }

    /// Load from a file path
    pub fn from_file(path: &std::path::Path) -> Result<Self, ArsError> {
        let data = std::fs::read(path).map_err(ArsError::Io)?;
        Self::from_bytes(&data)
    }
}

fn decompress(data: &[u8], codec: Compression) -> Result<Vec<u8>> {
    match codec {
        Compression::None => Ok(data.to_vec()),
        Compression::Zstd => zstd::decode_all(data).context("zstd decompression failed"),
    }
}
