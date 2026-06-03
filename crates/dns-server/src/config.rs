use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub dns: DnsConfig,
    #[serde(default)]
    pub upstream: UpstreamConfig,
    #[serde(default)]
    pub rulesets: Vec<RulesetConfig>,
    #[serde(default)]
    pub web: WebConfig,
    #[serde(default)]
    pub log: LogConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    #[serde(default = "def_dns_bind")]   pub bind: String,
    #[serde(default = "def_block_mode")] pub block_mode: BlockMode,
    #[serde(default = "def_log_size")]   pub query_log_size: usize,
    #[serde(default = "def_block_ttl")]  pub block_ttl: u32,
    #[serde(default = "def_cache_size")] pub cache_size: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockMode { NxDomain, ZeroIp, Refused }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    #[serde(default = "def_upstreams")]        pub servers: Vec<String>,
    #[serde(default = "def_upstream_timeout")] pub timeout_ms: u64,
    #[serde(default = "def_true")]             pub failover: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesetConfig {
    pub name: String,
    pub path: PathBuf,
    #[serde(default = "def_true")] pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    #[serde(default = "def_web_bind")] pub bind: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    #[serde(default = "def_log_level")] pub level: String,
}

impl Config {
    /// Load from a JSON file
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let s = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&s)?)
    }
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let s = serde_json::to_string_pretty(self)?;
        std::fs::write(path, s)?;
        Ok(())
    }
    pub fn example() -> &'static str { include_str!("../config.example.json") }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dns: DnsConfig {
                bind: def_dns_bind(), block_mode: def_block_mode(),
                query_log_size: def_log_size(), block_ttl: def_block_ttl(),
                cache_size: def_cache_size(),
            },
            upstream: UpstreamConfig {
                servers: def_upstreams(), timeout_ms: def_upstream_timeout(), failover: true,
            },
            rulesets: vec![],
            web: WebConfig { bind: def_web_bind(), password: None },
            log: LogConfig { level: def_log_level() },
        }
    }
}

impl Default for DnsConfig {
    fn default() -> Self { Config::default().dns }
}
impl Default for UpstreamConfig {
    fn default() -> Self { Config::default().upstream }
}
impl Default for WebConfig {
    fn default() -> Self { Config::default().web }
}
impl Default for LogConfig {
    fn default() -> Self { Config::default().log }
}

fn def_dns_bind()          -> String { "0.0.0.0:53".into() }
fn def_web_bind()          -> String { "0.0.0.0:3000".into() }
fn def_block_mode()        -> BlockMode { BlockMode::NxDomain }
fn def_log_size()          -> usize { 10_000 }
fn def_block_ttl()         -> u32 { 3600 }
fn def_cache_size()        -> usize { 50_000 }
fn def_upstream_timeout()  -> u64 { 3000 }
fn def_log_level()         -> String { "info".into() }
fn def_true()              -> bool { true }
fn def_upstreams()         -> Vec<String> {
    vec!["8.8.8.8:53".into(), "8.8.4.4:53".into()]
}
