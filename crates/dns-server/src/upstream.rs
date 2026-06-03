use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio::net::UdpSocket;
use tokio::time::timeout;

pub struct Upstream {
    servers: Vec<SocketAddr>,
    timeout: Duration,
    failover: bool,
}

impl Upstream {
    pub fn new(servers: Vec<String>, timeout_ms: u64, failover: bool) -> Result<Self> {
        let addrs = servers.iter()
            .map(|s| s.parse::<SocketAddr>()
                .map_err(|e| anyhow!("Bad upstream '{}': {}", s, e)))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { servers: addrs, timeout: Duration::from_millis(timeout_ms), failover })
    }

    pub async fn query(&self, query: &[u8]) -> Result<Vec<u8>> {
        let sock = UdpSocket::bind("0.0.0.0:0").await?;
        let mut last_err = anyhow!("no upstreams");
        for (i, server) in self.servers.iter().enumerate() {
            match self.try_one(&sock, query, *server).await {
                Ok(r) => return Ok(r),
                Err(e) => {
                    last_err = e;
                    if !self.failover || i == self.servers.len() - 1 { break; }
                    tracing::warn!(server=%server, "upstream failed, trying next");
                }
            }
        }
        Err(last_err)
    }

    async fn try_one(&self, sock: &UdpSocket, query: &[u8], server: SocketAddr) -> Result<Vec<u8>> {
        sock.send_to(query, server).await?;
        let mut buf = vec![0u8; 4096];
        let (n, _) = timeout(self.timeout, sock.recv_from(&mut buf))
            .await
            .map_err(|_| anyhow!("upstream timeout"))??;
        buf.truncate(n);
        Ok(buf)
    }

    pub fn server_list(&self) -> Vec<String> {
        self.servers.iter().map(|s| s.to_string()).collect()
    }
}
