use void_proxy::ip_cache::IpCache;
use std::net::IpAddr;
use std::time::Duration;

#[tokio::test]
async fn test_ip_cache_functionality() {
    let cache = IpCache::new(100, Duration::from_secs(300));
    let ip: IpAddr = "127.0.0.1".parse().unwrap();

    // Test IP checking functionality
    let result1 = cache.check_ip(&ip, |_| true).await;
    assert!(result1);

    // Test caching - second call should use cached result
    let result2 = cache.check_ip(&ip, |_| false).await; // Would return false if not cached
    assert!(result2); // Should still return true due to cache
}

#[tokio::test]
async fn test_ip_cache_ttl() {
    let cache = IpCache::new(10, Duration::from_millis(50)); // Short TTL for testing
    let ip: IpAddr = "127.0.0.1".parse().unwrap();

    // Cache the result
    let result1 = cache.check_ip(&ip, |_| true).await;
    assert!(result1);

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Should call checker function again due to TTL expiration
    let result2 = cache.check_ip(&ip, |_| false).await;
    assert!(!result2);
}