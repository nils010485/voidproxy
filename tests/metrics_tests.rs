use void_proxy::metrics::InstanceMetrics;

#[tokio::test]
async fn test_instance_metrics() {
    let metrics = InstanceMetrics::new();

    assert_eq!(metrics.bytes_sent.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(metrics.bytes_received.load(std::sync::atomic::Ordering::Relaxed), 0);

    metrics.add_bytes_sent(1024);
    metrics.add_bytes_received(2048);

    assert_eq!(metrics.bytes_sent.load(std::sync::atomic::Ordering::Relaxed), 1024);
    assert_eq!(metrics.bytes_received.load(std::sync::atomic::Ordering::Relaxed), 2048);
}

#[tokio::test]
async fn test_metrics_overflow_protection() {
    let metrics = InstanceMetrics::new();

    // Test near overflow
    metrics.bytes_sent.store(u64::MAX - 1000, std::sync::atomic::Ordering::Relaxed);
    metrics.add_bytes_sent(500);
    assert_eq!(metrics.bytes_sent.load(std::sync::atomic::Ordering::Relaxed), u64::MAX - 500);

    // Test overflow - should saturate
    metrics.add_bytes_sent(1000);
    assert_eq!(metrics.bytes_sent.load(std::sync::atomic::Ordering::Relaxed), u64::MAX);
}