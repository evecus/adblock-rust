use ars_format::builder::reverse_labels;
use ars_format::reader::ArsReader;

/// Result of a domain query against the ruleset
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchResult {
    /// Domain is allowed to pass through to upstream
    Allow,
    /// Domain should be blocked (NXDOMAIN or 0.0.0.0)
    Block,
    /// Domain should be rewritten to the given target (IP or CNAME)
    Rewrite(String),
    /// No rule matched — default policy applies
    NoMatch,
}

/// Query a loaded ArsReader for a FQDN.
///
/// Priority:
///   1. Allow exact  (whitelist wins fastest)
///   2. Allow suffix
///   3. Allow keyword
///   4. Allow regex
///   5. Rewrite exact/suffix
///   6. Block exact
///   7. Block suffix
///   8. Block keyword
///   9. Block regex
pub fn match_domain(reader: &ArsReader, fqdn: &str) -> MatchResult {
    // Normalize: lowercase, strip trailing dot
    let domain = fqdn.trim_end_matches('.').to_lowercase();

    // ── 1-4: Allow checks (whitelist) ────────────────────────────────────────
    if check_allow(reader, &domain) {
        return MatchResult::Allow;
    }

    // ── 5: Rewrite ────────────────────────────────────────────────────────────
    if let Some(target) = check_rewrite(reader, &domain) {
        return MatchResult::Rewrite(target);
    }

    // ── 6-9: Block checks ─────────────────────────────────────────────────────
    if check_block(reader, &domain) {
        return MatchResult::Block;
    }

    MatchResult::NoMatch
}

fn check_allow(reader: &ArsReader, domain: &str) -> bool {
    // Exact allow
    if let Some(set) = &reader.allow_exact {
        if set.contains(domain) {
            return true;
        }
    }

    // Suffix allow
    if let Some(set) = &reader.allow_suffix {
        if suffix_match(set, domain) {
            return true;
        }
    }

    // Keyword allow
    if let Some(ac) = &reader.allow_keyword {
        if ac.is_match(domain) {
            return true;
        }
    }

    // Regex allow
    if let Some(rs) = &reader.allow_regex {
        if rs.is_match(domain) {
            return true;
        }
    }

    false
}

fn check_block(reader: &ArsReader, domain: &str) -> bool {
    // Exact block
    if let Some(set) = &reader.block_exact {
        if set.contains(domain) {
            return true;
        }
    }

    // Suffix block
    if let Some(set) = &reader.block_suffix {
        if suffix_match(set, domain) {
            return true;
        }
    }

    // Keyword block
    if let Some(ac) = &reader.block_keyword {
        if ac.is_match(domain) {
            return true;
        }
    }

    // Regex block
    if let Some(rs) = &reader.block_regex {
        if rs.is_match(domain) {
            return true;
        }
    }

    false
}

fn check_rewrite(reader: &ArsReader, domain: &str) -> Option<String> {
    for entry in &reader.rewrites {
        let matches = match entry.rule_type {
            ars_format::RuleType::DomainExact => domain == entry.pattern,
            ars_format::RuleType::DomainSuffix => {
                domain == entry.pattern || domain.ends_with(&format!(".{}", entry.pattern))
            }
            ars_format::RuleType::Regex => {
                if let Ok(re) = regex::Regex::new(&entry.pattern) {
                    re.is_match(domain)
                } else {
                    false
                }
            }
            _ => false,
        };
        if matches {
            return Some(entry.target.clone());
        }
    }
    None
}

/// Suffix match using the reversed-label FST.
/// For domain "sub.example.com":
///   - tries "com.example.sub" (exact in FST)
///   - tries "com.example" (suffix rule for *.example.com)
///   - tries "com" (suffix rule for *.com)
fn suffix_match(set: &fst::Set<Vec<u8>>, domain: &str) -> bool {
    let reversed = reverse_labels(domain);

    // Check each suffix level: "com.example.sub", "com.example", "com"
    // We check the full reversed domain and all its prefixes up to each dot
    let bytes = reversed.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'.' || i == bytes.len() - 1 {
            let end = if b == b'.' { i } else { i + 1 };
            let candidate = &reversed[..end];
            if set.contains(candidate) {
                return true;
            }
        }
    }
    // Check the full string too
    set.contains(&reversed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ars_format::format::Compression;
    use ars_format::{ArsBuilder, Rule, RuleAction, RuleType};

    fn engine_from_rules(rules: Vec<Rule>) -> ArsReader {
        let mut b = ArsBuilder::new().with_compression(Compression::None);
        b.add_rules(rules);
        let mut buf = Vec::new();
        b.build(&mut buf).unwrap();
        ArsReader::from_bytes(&buf).unwrap()
    }

    #[test]
    fn test_exact_block() {
        let r = engine_from_rules(vec![Rule::block_exact("ads.example.com")]);
        assert_eq!(match_domain(&r, "ads.example.com"), MatchResult::Block);
        assert_eq!(match_domain(&r, "example.com"), MatchResult::NoMatch);
    }

    #[test]
    fn test_suffix_block_matches_subdomain() {
        let r = engine_from_rules(vec![Rule::block_suffix("example.com")]);
        assert_eq!(match_domain(&r, "sub.example.com"), MatchResult::Block);
        assert_eq!(match_domain(&r, "deep.sub.example.com"), MatchResult::Block);
        assert_eq!(match_domain(&r, "example.com"), MatchResult::Block);
        assert_eq!(match_domain(&r, "notexample.com"), MatchResult::NoMatch);
    }

    #[test]
    fn test_whitelist_overrides_block() {
        let r = engine_from_rules(vec![
            Rule::block_suffix("example.com"),
            Rule::allow_exact("safe.example.com"),
        ]);
        assert_eq!(match_domain(&r, "safe.example.com"), MatchResult::Allow);
        assert_eq!(match_domain(&r, "ads.example.com"), MatchResult::Block);
    }

    #[test]
    fn test_keyword_block() {
        let r = engine_from_rules(vec![Rule {
            action: RuleAction::Block,
            rule_type: RuleType::DomainKeyword,
            pattern: "tracker".into(),
            source: None,
        }]);
        assert_eq!(match_domain(&r, "tracker.example.com"), MatchResult::Block);
        assert_eq!(match_domain(&r, "my-tracker-ads.io"), MatchResult::Block);
        assert_eq!(match_domain(&r, "example.com"), MatchResult::NoMatch);
    }

    #[test]
    fn test_case_insensitive_fqdn() {
        let r = engine_from_rules(vec![Rule::block_exact("ads.example.com")]);
        assert_eq!(match_domain(&r, "ADS.EXAMPLE.COM"), MatchResult::Block);
        assert_eq!(match_domain(&r, "Ads.Example.Com"), MatchResult::Block);
    }

    #[test]
    fn test_trailing_dot_stripped() {
        let r = engine_from_rules(vec![Rule::block_exact("ads.example.com")]);
        assert_eq!(match_domain(&r, "ads.example.com."), MatchResult::Block);
    }
}
