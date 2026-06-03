use std::io::Write;

use anyhow::{Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::Hasher;
use fst::SetBuilder;
use serde::{Deserialize, Serialize};

use crate::{
    format::*,
    rule::{Rule, RuleAction, RuleType},
    ArsError,
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RuleCounts {
    pub block_exact:   usize,
    pub block_suffix:  usize,
    pub block_keyword: usize,
    pub block_regex:   usize,
    pub allow_exact:   usize,
    pub allow_suffix:  usize,
    pub allow_keyword: usize,
    pub allow_regex:   usize,
    pub rewrite:       usize,
}

impl RuleCounts {
    pub fn total(&self) -> usize {
        self.block_exact + self.block_suffix + self.block_keyword + self.block_regex
        + self.allow_exact + self.allow_suffix + self.allow_keyword + self.allow_regex
        + self.rewrite
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArsMetadata {
    pub created_at:   String,
    pub source_files: Vec<String>,
    pub rule_counts:  RuleCounts,
    pub description:  Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewriteEntry {
    pub pattern:   String,
    pub rule_type: RuleType,
    pub target:    String,
}

pub struct ArsBuilder {
    rules:       Vec<Rule>,
    metadata:    Option<ArsMetadata>,
    compression: Compression,
}

impl ArsBuilder {
    pub fn new() -> Self {
        Self { rules: Vec::new(), metadata: None, compression: Compression::Zstd }
    }

    pub fn with_compression(mut self, c: Compression) -> Self {
        self.compression = c; self
    }

    pub fn add_rule(&mut self, rule: Rule)                                { self.rules.push(rule); }
    pub fn add_rules(&mut self, rules: impl IntoIterator<Item = Rule>)   { self.rules.extend(rules); }
    pub fn set_metadata(&mut self, meta: ArsMetadata)                    { self.metadata = Some(meta); }

    fn dedup(&mut self) {
        self.rules.sort_by(|a, b| {
            rule_sort_key(&a.rule_type).cmp(&rule_sort_key(&b.rule_type))
                .then(action_sort_key(&a.action).cmp(&action_sort_key(&b.action)))
                .then(a.pattern.cmp(&b.pattern))
        });
        self.rules.dedup_by(|a, b| {
            a.rule_type == b.rule_type && a.action == b.action && a.pattern == b.pattern
        });
    }

    pub fn build<W: Write>(&mut self, writer: &mut W) -> Result<ArsMetadata> {
        self.dedup();

        let mut block_exact:   Vec<String> = Vec::new();
        let mut block_suffix:  Vec<String> = Vec::new();
        let mut block_keyword: Vec<String> = Vec::new();
        let mut block_regex:   Vec<String> = Vec::new();
        let mut allow_exact:   Vec<String> = Vec::new();
        let mut allow_suffix:  Vec<String> = Vec::new();
        let mut allow_keyword: Vec<String> = Vec::new();
        let mut allow_regex:   Vec<String> = Vec::new();
        let mut rewrites:      Vec<RewriteEntry> = Vec::new();

        for rule in &self.rules {
            match (&rule.action, &rule.rule_type) {
                (RuleAction::Block, RuleType::DomainExact)   => block_exact.push(rule.pattern.clone()),
                (RuleAction::Block, RuleType::DomainSuffix)  => block_suffix.push(rule.pattern.clone()),
                (RuleAction::Block, RuleType::DomainKeyword) => block_keyword.push(rule.pattern.clone()),
                (RuleAction::Block, RuleType::Regex)         => block_regex.push(rule.pattern.clone()),
                (RuleAction::Allow, RuleType::DomainExact)   => allow_exact.push(rule.pattern.clone()),
                (RuleAction::Allow, RuleType::DomainSuffix)  => allow_suffix.push(rule.pattern.clone()),
                (RuleAction::Allow, RuleType::DomainKeyword) => allow_keyword.push(rule.pattern.clone()),
                (RuleAction::Allow, RuleType::Regex)         => allow_regex.push(rule.pattern.clone()),
                (RuleAction::Rewrite { target }, _) => rewrites.push(RewriteEntry {
                    pattern: rule.pattern.clone(),
                    rule_type: rule.rule_type.clone(),
                    target: target.clone(),
                }),
                _ => {}
            }
        }

        let counts = RuleCounts {
            block_exact: block_exact.len(), block_suffix: block_suffix.len(),
            block_keyword: block_keyword.len(), block_regex: block_regex.len(),
            allow_exact: allow_exact.len(), allow_suffix: allow_suffix.len(),
            allow_keyword: allow_keyword.len(), allow_regex: allow_regex.len(),
            rewrite: rewrites.len(),
        };

        let meta = self.metadata.take().unwrap_or_else(|| ArsMetadata {
            created_at:   chrono::Utc::now().to_rfc3339(),
            source_files: vec![],
            rule_counts:  counts.clone(),
            description:  None,
        });

        let mut sections: Vec<(SectionId, Vec<u8>)> = Vec::new();
        sections.push((SectionId::Metadata, serde_json::to_vec(&meta)?));

        if !block_exact.is_empty()   { sections.push((SectionId::BlockExact,   build_fst(&block_exact)?)); }
        if !block_suffix.is_empty()  { sections.push((SectionId::BlockSuffix,  build_fst_reversed(&block_suffix)?)); }
        if !allow_exact.is_empty()   { sections.push((SectionId::AllowExact,   build_fst(&allow_exact)?)); }
        if !allow_suffix.is_empty()  { sections.push((SectionId::AllowSuffix,  build_fst_reversed(&allow_suffix)?)); }
        if !block_keyword.is_empty() { sections.push((SectionId::BlockKeyword, block_keyword.join("\n").into_bytes())); }
        if !allow_keyword.is_empty() { sections.push((SectionId::AllowKeyword, allow_keyword.join("\n").into_bytes())); }
        if !block_regex.is_empty()   { sections.push((SectionId::BlockRegex,   block_regex.join("\n").into_bytes())); }
        if !allow_regex.is_empty()   { sections.push((SectionId::AllowRegex,   allow_regex.join("\n").into_bytes())); }
        if !rewrites.is_empty()      { sections.push((SectionId::Rewrite,       serde_json::to_vec(&rewrites)?)); }

        let mut buf: Vec<u8> = Vec::with_capacity(1 << 20);
        write_header(&mut buf, counts.total() as u32, sections.len() as u32)?;

        for (id, data) in &sections {
            let (compressed, codec) = compress_data(data, self.compression)?;
            write_section_header(&mut buf, *id, codec, compressed.len() as u32, data.len() as u32)?;
            buf.write_all(&compressed)?;
        }

        // Patch header CRC first (bytes 28..32 covers bytes 0..28)
        let mut hh = Hasher::new(); hh.update(&buf[..28]);
        let hcrc = hh.finalize().to_le_bytes();
        buf[28..32].copy_from_slice(&hcrc);

        // File trailer CRC over entire buffer (including patched header CRC)
        let mut h = Hasher::new(); h.update(&buf);
        buf.write_u32::<LittleEndian>(h.finalize())?;

        writer.write_all(&buf)?;
        Ok(meta)
    }
}

impl Default for ArsBuilder { fn default() -> Self { Self::new() } }

// ── Helpers ──────────────────────────────────────────────────────────────────

fn rule_sort_key(rt: &RuleType) -> u8 {
    match rt {
        RuleType::DomainExact   => 0,
        RuleType::DomainSuffix  => 1,
        RuleType::DomainKeyword => 2,
        RuleType::Regex         => 3,
        RuleType::IpCidr4       => 4,
        RuleType::IpCidr6       => 5,
    }
}

fn action_sort_key(a: &RuleAction) -> u8 {
    match a {
        RuleAction::Allow          => 0,
        RuleAction::Block          => 1,
        RuleAction::Rewrite { .. } => 2,
    }
}

fn build_fst(patterns: &[String]) -> Result<Vec<u8>> {
    let mut sorted = patterns.to_vec();
    sorted.sort();
    sorted.dedup();
    let mut b = SetBuilder::memory();
    for s in &sorted {
        b.insert(s).map_err(|e| ArsError::FstError(e.to_string()))?;
    }
    Ok(b.into_inner().map_err(|e| ArsError::FstError(e.to_string()))?)
}

fn build_fst_reversed(patterns: &[String]) -> Result<Vec<u8>> {
    let reversed: Vec<String> = patterns.iter().map(|p| reverse_labels(p)).collect();
    build_fst(&reversed)
}

/// Reverse domain labels: "sub.example.com" → "com.example.sub"
pub fn reverse_labels(domain: &str) -> String {
    domain.split('.').rev().collect::<Vec<_>>().join(".")
}

fn compress_data(data: &[u8], codec: Compression) -> Result<(Vec<u8>, Compression)> {
    match codec {
        Compression::None => Ok((data.to_vec(), Compression::None)),
        Compression::Zstd => {
            let c = zstd::encode_all(data, 9).context("zstd compress")?;
            if c.len() >= data.len() {
                Ok((data.to_vec(), Compression::None))
            } else {
                Ok((c, Compression::Zstd))
            }
        }
    }
}

fn write_header(buf: &mut Vec<u8>, total_rules: u32, section_count: u32) -> Result<()> {
    buf.write_all(MAGIC)?;
    buf.write_u16::<LittleEndian>(FORMAT_VERSION)?;
    buf.write_u8(Compression::Zstd as u8)?;
    buf.write_u8(0)?;
    buf.write_u32::<LittleEndian>(total_rules)?;
    buf.write_u32::<LittleEndian>(section_count)?;
    buf.write_u32::<LittleEndian>(HEADER_SIZE as u32)?;
    buf.write_all(&[0u8; 8])?; // reserved
    buf.write_u32::<LittleEndian>(0)?; // CRC placeholder
    Ok(())
}

fn write_section_header(
    buf: &mut Vec<u8>, id: SectionId, codec: Compression,
    compressed_len: u32, uncompressed_len: u32,
) -> Result<()> {
    buf.write_u8(id as u8)?;
    buf.write_u8(codec as u8)?;
    buf.write_u32::<LittleEndian>(compressed_len)?;
    buf.write_u32::<LittleEndian>(uncompressed_len)?;
    Ok(())
}

extern crate chrono;
