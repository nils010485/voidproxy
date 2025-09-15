use void_proxy::buffer_pool::BufferPool;

#[tokio::test]
async fn test_buffer_pool_functionality() {
    let pool = BufferPool::new(10, 5);

    // Test buffer acquisition
    let buffer = pool.acquire(1024).await;
    assert!(buffer.capacity() >= 1024);

    // Test buffer operations
    let mut buffer_mut = buffer;
    buffer_mut.extend_from_slice(b"test data");
    assert_eq!(buffer_mut.len(), 9);

    buffer_mut.clear();
    assert_eq!(buffer_mut.len(), 0);
}

#[tokio::test]
async fn test_buffer_pool_different_sizes() {
    let pool = BufferPool::new(10, 5);

    // Test small buffer
    let small_buffer = pool.acquire(512).await;
    assert!(small_buffer.capacity() >= 512);

    // Test medium buffer
    let medium_buffer = pool.acquire(4096).await;
    assert!(medium_buffer.capacity() >= 4096);

    // Test large buffer
    let large_buffer = pool.acquire(16384).await;
    assert!(large_buffer.capacity() >= 16384);
}