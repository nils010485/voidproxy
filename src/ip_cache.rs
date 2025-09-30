use lru::LruCache;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
pub struct IpCache {
    cache: Arc<RwLock<LruCache<IpAddr, CacheEntry>>>,
    ttl: Duration,
}
#[derive(Clone)]
struct CacheEntry {
    allowed: bool,
    created_at: Instant,
}
impl IpCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(capacity)
                    .unwrap_or(std::num::NonZeroUsize::new(1).unwrap()),
            ))),
            ttl,
        }
    }
    pub async fn check_ip(&self, ip: &IpAddr, checker: impl Fn(&IpAddr) -> bool) -> bool {
        let mut cache = self.cache.write().await;
        if let Some(entry) = cache.get(ip) {
            if entry.created_at.elapsed() <= self.ttl {
                return entry.allowed;
            }
            cache.pop(ip);
        }
        let allowed = checker(ip);
        cache.put(
            *ip,
            CacheEntry {
                allowed,
                created_at: Instant::now(),
            },
        );
        allowed
    }
}
