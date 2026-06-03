mod api_impl;
mod cache;
mod config;
mod dns_codec;
mod handler;
mod query_log;
mod upstream;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use parking_lot::RwLock;
use rule_engine::RuleEngine;
use tracing::info;

use crate::{cache::DnsCache, config::Config, query_log::QueryLog, upstream::Upstream};

pub struct AppState {
    pub config: RwLock<Config>,
    pub engine: RuleEngine,
    pub cache: DnsCache,
    pub log: QueryLog,
    pub upstream: Upstream,
}

#[derive(Parser)]
#[clap(name = "dns-filter", about = "Fast DNS filter daemon", version)]
struct Cli {
    /// Path to config file (JSON)
    #[clap(short, long, default_value = "config.json")]
    config: PathBuf,
    /// Print example config and exit
    #[clap(long)]
    example_config: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.example_config {
        print!("{}", Config::example());
        return Ok(());
    }

    let config = if cli.config.exists() {
        Config::from_file(&cli.config).context("Failed to load config")?
    } else {
        eprintln!(
            "Config '{}' not found, using defaults",
            cli.config.display()
        );
        Config::default()
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(config.log.level.parse()?),
        )
        .init();

    info!("dns-filter starting");

    let upstream = Upstream::new(
        config.upstream.servers.clone(),
        config.upstream.timeout_ms,
        config.upstream.failover,
    )?;

    let state = Arc::new(AppState {
        cache: DnsCache::new(config.dns.cache_size),
        log: QueryLog::new(config.dns.query_log_size),
        engine: RuleEngine::new(),
        upstream,
        config: RwLock::new(config.clone()),
    });

    // Load rulesets
    let paths: Vec<PathBuf> = config
        .rulesets
        .iter()
        .filter(|r| r.enabled)
        .map(|r| r.path.clone())
        .collect();
    if paths.is_empty() {
        tracing::warn!("No rulesets configured — all queries will be forwarded");
    } else {
        state.engine.load_files(&paths)?;
    }

    // Web API in dedicated thread (tiny_http is sync)
    let web_state = state.clone();
    let web_bind = config.web.bind.clone();
    std::thread::spawn(move || {
        if let Err(e) = web_api::run_web_server(web_state, &web_bind) {
            tracing::error!("Web server error: {}", e);
        }
    });

    handler::run_dns_server(state).await?;
    Ok(())
}
