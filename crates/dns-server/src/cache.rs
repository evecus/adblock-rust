use dashmap::DashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub name: String,
    pub qtype: u16,
}

struct Entry {
    data: Vec<u8>,
    expires: Instant,
}

pub struct DnsCache {
    inner: DashMap<CacheKey, Entry>,
    max_size: usize,
}

impl DnsCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: DashMap::with_capacity(max_size.min(65536)),
            max_size,
        }
    }

    pub fn get(&self, key: &CacheKey) -> Option<Vec<u8>> {
        let entry = self.inner.get(key)?;
        if entry.expires > Instant::now() {
            Some(entry.data.clone())
        } else {
            drop(entry);
            self.inner.remove(key);
            None
        }
    }

    pub fn insert(&self, key: CacheKey, data: Vec<u8>, ttl_secs: u32) {
        if self.inner.len() >= self.max_size {
            self.evict();
        }
        self.inner.insert(
            key,
            Entry {
                data,
                expires: Instant::now() + Duration::from_secs(ttl_secs as u64),
            },
        );
    }

    fn evict(&self) {
        let now = Instant::now();
        // Remove up to 10 expired entries
        let expired: Vec<_> = self
            .inner
            .iter()
            .filter(|e| e.expires <= now)
            .take(10)
            .map(|e| e.key().clone())
            .collect();
        if !expired.is_empty() {
            for k in expired {
                self.inner.remove(&k);
            }
            return;
        }
        // Fallback: remove arbitrary entry
        if let Some(e) = self.inner.iter().next() {
            let k = e.key().clone();
            drop(e);
            self.inner.remove(&k);
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
