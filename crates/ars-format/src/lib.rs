pub mod builder;
pub mod error;
pub mod format;
pub mod reader;
pub mod rule;

pub use builder::{ArsBuilder, ArsMetadata, RuleCounts};
pub use error::ArsError;
pub use reader::ArsReader;
pub use rule::{Rule, RuleAction, RuleType};

/// Integration test: build a small ruleset and query it
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::builder::reverse_labels;

    fn make_ars(rules: Vec<Rule>) -> Vec<u8> {
        let mut builder = ArsBuilder::new()
            .with_compression(format::Compression::None);
        builder.add_rules(rules);
        let mut buf = Vec::new();
        builder.build(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_exact_block_match() {
        let rules = vec![Rule::block_exact("ads.example.com")];
        let data = make_ars(rules);
        let reader = ArsReader::from_bytes(&data).unwrap();
        assert!(reader.block_exact.as_ref().unwrap().contains("ads.example.com"));
        assert!(!reader.block_exact.as_ref().unwrap().contains("example.com"));
    }

    #[test]
    fn test_suffix_fst_reversed_storage() {
        // Suffix rules are stored reversed for FST prefix search
        let rules = vec![Rule::block_suffix("example.com")];
        let data = make_ars(rules);
        let reader = ArsReader::from_bytes(&data).unwrap();
        let fst = reader.block_suffix.as_ref().unwrap();
        // Stored as "com.example" (reversed)
        assert!(fst.contains("com.example"));
        assert!(!fst.contains("example.com"));
    }

    #[test]
    fn test_keyword_loaded() {
        let rules = vec![Rule {
            action: RuleAction::Block,
            rule_type: RuleType::DomainKeyword,
            pattern: "tracker".to_string(),
            source: None,
        }];
        let data = make_ars(rules);
        let reader = ArsReader::from_bytes(&data).unwrap();
        assert!(reader.block_keyword.is_some());
    }

    #[test]
    fn test_allow_rule_loaded() {
        let rules = vec![
            Rule::block_suffix("example.com"),
            Rule::allow_exact("safe.example.com"),
        ];
        let data = make_ars(rules);
        let reader = ArsReader::from_bytes(&data).unwrap();
        assert!(reader.block_suffix.is_some());
        assert!(reader.allow_exact.is_some());
    }

    #[test]
    fn test_dedup() {
        let rules = vec![
            Rule::block_exact("dup.com"),
            Rule::block_exact("dup.com"),
            Rule::block_exact("dup.com"),
        ];
        let data = make_ars(rules);
        let reader = ArsReader::from_bytes(&data).unwrap();
        assert_eq!(reader.metadata.rule_counts.block_exact, 1);
    }

    #[test]
    fn test_checksum_tamper_detected() {
        let rules = vec![Rule::block_exact("test.com")];
        let mut data = make_ars(rules);
        // Tamper with a byte in the middle
        let mid = data.len() / 2;
        data[mid] ^= 0xFF;
        assert!(matches!(ArsReader::from_bytes(&data), Err(ArsError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_reverse_labels() {
        assert_eq!(reverse_labels("sub.example.com"), "com.example.sub");
        assert_eq!(reverse_labels("com"), "com");
        assert_eq!(reverse_labels("a.b.c.d"), "d.c.b.a");
    }

    #[test]
    fn test_metadata_preserved() {
        let rules = vec![Rule::block_exact("test.com")];
        let data = make_ars(rules);
        let reader = ArsReader::from_bytes(&data).unwrap();
        assert_eq!(reader.metadata.rule_counts.block_exact, 1);
        assert!(!reader.metadata.created_at.is_empty());
    }
}
