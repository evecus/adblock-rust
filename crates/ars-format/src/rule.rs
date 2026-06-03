use serde::{Deserialize, Serialize};

/// What action to take when a rule matches
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleAction {
    /// Block the domain (respond with NXDOMAIN or 0.0.0.0)
    Block,
    /// Allow the domain (whitelist, overrides block rules)
    Allow,
    /// Rewrite to a specific IP or CNAME
    Rewrite { target: String },
}

/// How the rule pattern matches
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleType {
    /// Exact domain match: example.com
    DomainExact,
    /// Suffix match: matches example.com and *.example.com
    /// Corresponds to +.example.com (mihomo) or ||example.com^ (adguard)
    DomainSuffix,
    /// Keyword match: matches any domain containing the string
    DomainKeyword,
    /// Full regex match on the FQDN
    Regex,
    /// IPv4 CIDR (for blocking by IP after resolution - future)
    IpCidr4,
    /// IPv6 CIDR
    IpCidr6,
}

/// A single parsed rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    pub rule_type: RuleType,
    /// The pattern string (domain, keyword, regex pattern, CIDR string)
    pub pattern: String,
    /// Optional source tag (which ruleset file this came from)
    pub source: Option<String>,
}

impl Rule {
    pub fn block_suffix(domain: &str) -> Self {
        Self {
            action: RuleAction::Block,
            rule_type: RuleType::DomainSuffix,
            pattern: domain.to_lowercase(),
            source: None,
        }
    }

    pub fn block_exact(domain: &str) -> Self {
        Self {
            action: RuleAction::Block,
            rule_type: RuleType::DomainExact,
            pattern: domain.to_lowercase(),
            source: None,
        }
    }

    pub fn allow_suffix(domain: &str) -> Self {
        Self {
            action: RuleAction::Allow,
            rule_type: RuleType::DomainSuffix,
            pattern: domain.to_lowercase(),
            source: None,
        }
    }

    pub fn allow_exact(domain: &str) -> Self {
        Self {
            action: RuleAction::Allow,
            rule_type: RuleType::DomainExact,
            pattern: domain.to_lowercase(),
            source: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_suffix_normalized() {
        let r = Rule::block_suffix("Example.COM");
        assert_eq!(r.pattern, "example.com");
        assert_eq!(r.rule_type, RuleType::DomainSuffix);
        assert_eq!(r.action, RuleAction::Block);
    }

    #[test]
    fn test_allow_exact() {
        let r = Rule::allow_exact("Safe.org");
        assert_eq!(r.pattern, "safe.org");
        assert_eq!(r.action, RuleAction::Allow);
    }
}
