use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use rule_engine::MatchResult;
use tokio::net::UdpSocket;
use tracing::{debug, warn};

use crate::{
    cache::CacheKey,
    config::BlockMode,
    dns_codec::DnsMessage,
    query_log::{QueryAction, QueryLogEntry},
    AppState,
};

pub async fn run_dns_server(state: Arc<AppState>) -> Result<()> {
    let bind = state.config.read().dns.bind.clone();
    let sock = Arc::new(UdpSocket::bind(&bind).await?);
    tracing::info!(bind = %bind, "DNS server listening");

    let mut buf = vec![0u8; 4096];
    loop {
        let (n, peer) = sock.recv_from(&mut buf).await?;
        let raw = buf[..n].to_vec();
        let sock2 = sock.clone();
        let state2 = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle(state2, sock2, raw, peer).await {
                warn!(peer=%peer, error=%e, "query handler error");
            }
        });
    }
}

async fn handle(
    state: Arc<AppState>,
    sock: Arc<UdpSocket>,
    raw: Vec<u8>,
    peer: SocketAddr,
) -> Result<()> {
    let start = Instant::now();

    let msg = match DnsMessage::parse(&raw) {
        Ok(m) if m.is_query() => m,
        Ok(_) => return Ok(()), // not a query, ignore
        Err(e) => {
            warn!(peer=%peer, "parse error: {}", e);
            return Ok(());
        }
    };

    let q = match msg.questions.first() {
        Some(q) => q.clone(),
        None => return Ok(()),
    };

    let domain = q.name.clone();
    debug!(peer=%peer, domain=%domain, qtype=q.qtype, "query");

    let cfg = state.config.read().clone();
    let cache_key = CacheKey {
        name: domain.clone(),
        qtype: q.qtype,
    };

    // ── Cache hit ─────────────────────────────────────────────────────────────
    if let Some(mut cached) = state.cache.get(&cache_key) {
        DnsMessage::patch_id(&mut cached, msg.id);
        sock.send_to(&cached, peer).await?;
        state.log.push(QueryLogEntry {
            ts: chrono::Utc::now(),
            client: peer.to_string(),
            domain,
            qtype: qtype_str(q.qtype),
            action: QueryAction::Cache,
            upstream: None,
            latency_ms: start.elapsed().as_millis() as u64,
            cached: true,
        });
        return Ok(());
    }

    // ── Rule match ────────────────────────────────────────────────────────────
    let verdict = state.engine.query(&domain);
    let block_ttl = cfg.dns.block_ttl;

    let (response, action, upstream_used) = match &verdict {
        MatchResult::Block => {
            let resp = match cfg.dns.block_mode {
                BlockMode::NxDomain => msg.nxdomain(),
                BlockMode::ZeroIp => msg.zero_ip(block_ttl),
                BlockMode::Refused => msg.refused(),
            };
            (resp, QueryAction::Block, None)
        }
        MatchResult::Rewrite(target) => (msg.rewrite(target, 300), QueryAction::Rewrite, None),
        MatchResult::Allow | MatchResult::NoMatch => match state.upstream.query(&raw).await {
            Ok(mut resp) => {
                let ttl = DnsMessage::min_ttl(&resp).unwrap_or(60).max(5);
                DnsMessage::patch_id(&mut resp, msg.id);
                state.cache.insert(cache_key, resp.clone(), ttl);
                let srv = state.upstream.server_list().into_iter().next();
                (resp, QueryAction::Allow, srv)
            }
            Err(e) => {
                warn!(domain=%domain, "upstream error: {}", e);
                (msg.servfail(), QueryAction::Allow, None)
            }
        },
    };

    sock.send_to(&response, peer).await?;

    state.log.push(QueryLogEntry {
        ts: chrono::Utc::now(),
        client: peer.to_string(),
        domain,
        qtype: qtype_str(q.qtype),
        action,
        upstream: upstream_used,
        latency_ms: start.elapsed().as_millis() as u64,
        cached: false,
    });

    Ok(())
}

fn qtype_str(t: u16) -> String {
    match t {
        1 => "A",
        28 => "AAAA",
        5 => "CNAME",
        15 => "MX",
        16 => "TXT",
        2 => "NS",
        6 => "SOA",
        _ => "OTHER",
    }
    .to_string()
}
