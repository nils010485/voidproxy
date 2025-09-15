use bytes::BytesMut;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
#[derive(Clone)]
/**
 * A thread-safe buffer pool for efficient memory management in proxy operations.
 *
 * This pool manages three tiers of buffers to reduce memory allocation overhead:
 * - Small buffers (≤ 1024 bytes)
 * - Medium buffers (≤ 8192 bytes)
 * - Large buffers (> 8192 bytes, up to 65535 bytes)
 *
 * The pool includes concurrency limiting to prevent excessive memory usage
 * and automatic buffer recycling when buffers are dropped.
 */
pub struct BufferPool {
    small_buffers: Arc<Mutex<VecDeque<BytesMut>>>,
    medium_buffers: Arc<Mutex<VecDeque<BytesMut>>>,
    large_buffers: Arc<Mutex<VecDeque<BytesMut>>>,
    max_pool_size: usize,
    concurrency_limiter: Arc<Semaphore>,
}
impl BufferPool {
    pub fn new(max_pool_size: usize, max_concurrent: usize) -> Self {
        Self {
            small_buffers: Arc::new(Mutex::new(VecDeque::new())),
            medium_buffers: Arc::new(Mutex::new(VecDeque::new())),
            large_buffers: Arc::new(Mutex::new(VecDeque::new())),
            max_pool_size,
            concurrency_limiter: Arc::new(Semaphore::new(max_concurrent)),
        }
    }
    pub async fn acquire(&self, size: usize) -> PooledBuffer {
        let _permit = self.concurrency_limiter.acquire().await.unwrap();
        let buffer = if size <= 1024 {
            self.get_buffer(&self.small_buffers, 1024).await
        } else if size <= 8192 {
            self.get_buffer(&self.medium_buffers, 8192).await
        } else {
            self.get_buffer(&self.large_buffers, 65535).await
        };
        PooledBuffer {
            buffer,
            pool: std::sync::Arc::new(self.clone()),
            size_hint: size,
        }
    }
    async fn get_buffer(
        &self,
        pool: &Arc<Mutex<VecDeque<BytesMut>>>,
        default_size: usize,
    ) -> BytesMut {
        let mut pool_guard = pool.lock().await;
        pool_guard
            .pop_front()
            .unwrap_or_else(|| BytesMut::with_capacity(default_size))
    }
    async fn return_buffer(&self, mut buffer: BytesMut, size_hint: usize) {
        buffer.clear();
        let pool = if size_hint <= 1024 {
            &self.small_buffers
        } else if size_hint <= 8192 {
            &self.medium_buffers
        } else {
            &self.large_buffers
        };
        let mut pool_guard = pool.lock().await;
        if pool_guard.len() < self.max_pool_size {
            pool_guard.push_back(buffer);
        }
      }
}
/**
 * A pooled buffer that automatically returns itself to the buffer pool when dropped.
 *
 * This struct provides a wrapper around a BytesMut buffer that ensures efficient
 * memory management by automatically returning the buffer to the appropriate pool
 * when it's no longer needed. The size_hint helps the pool determine which
 * buffer tier to return the buffer to.
 */
pub struct PooledBuffer {
    buffer: BytesMut,
    pool: Arc<BufferPool>,
    size_hint: usize,
}
impl PooledBuffer {
    /**
 * Returns a mutable reference to the underlying BytesMut buffer.
 *
 * This method allows direct access to the buffer contents for reading and writing.
 * The buffer will still be automatically returned to the pool when this PooledBuffer
 * is dropped.
 *
 * Returns:
 *   A mutable reference to the BytesMut buffer.
 */
pub fn as_mut(&mut self) -> &mut BytesMut {
        &mut self.buffer
    }
    /**
 * Clears the buffer contents.
 *
 * This method removes all data from the buffer but preserves the allocated capacity.
 * The buffer will still be returned to the pool when this PooledBuffer is dropped.
 */
pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
impl Drop for PooledBuffer {
    fn drop(&mut self) {
        let pool = self.pool.clone();
        let buffer = std::mem::take(&mut self.buffer);
        let size_hint = self.size_hint;
        tokio::spawn(async move {
            pool.return_buffer(buffer, size_hint).await;
        });
    }
}
impl std::ops::Deref for PooledBuffer {
    type Target = BytesMut;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
impl std::ops::DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
#[derive(Clone)]
/**
 * Represents a UDP session for stateless UDP proxy operations.
 *
 * A UDP session tracks the client socket and local address used for forwarding
 * UDP packets. Since UDP is stateless, sessions are used to maintain context
 * for response packets and manage session lifecycle with timeout handling.
 */
pub struct UdpSession {
    pub client_socket: Arc<tokio::net::UdpSocket>,
    pub local_addr: std::net::SocketAddr,
    pub last_activity: Instant,
}
impl UdpSession {
    /**
     * Creates a new UDP session with the given client socket and local address.
     *
     * Arguments:
     *   client_socket - The UDP socket used for communication with the client
     *   local_addr - The local address bound to this session
     *
     * Returns:
     *   A new UdpSession instance with the current timestamp as last_activity
     */
    pub fn new(
        client_socket: Arc<tokio::net::UdpSocket>,
        local_addr: std::net::SocketAddr,
    ) -> Self {
        Self {
            client_socket,
            local_addr,
            last_activity: Instant::now(),
        }
    }
    /**
     * Updates the last activity timestamp for this session.
     *
     * This method should be called whenever there is new activity on the session
     * to prevent premature timeout and session cleanup.
     */
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }
    /**
     * Checks if this session has expired based on the given timeout duration.
     *
     * Arguments:
     *   timeout - The duration after which a session is considered expired
     *
     * Returns:
     *   true if the session has expired, false otherwise
     */
    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }
}
/**
 * Manages UDP sessions for stateless UDP proxy operations.
 *
 * The UdpSessionManager handles the lifecycle of UDP sessions, including creation,
 * cleanup of expired sessions, and session lookup. It automatically cleans up
 * inactive sessions to prevent resource leaks and manages concurrent access to
 * the session store.
 */
pub struct UdpSessionManager {
    sessions: Arc<tokio::sync::RwLock<std::collections::HashMap<std::net::SocketAddr, UdpSession>>>,
    session_timeout: Duration,
    cleanup_interval: Duration,
}
impl UdpSessionManager {
    /**
 * Creates a new UdpSessionManager with the specified timeout and cleanup interval.
 *
 * Arguments:
 *   session_timeout - The duration after which sessions are considered expired
 *   cleanup_interval - The interval at which expired sessions are cleaned up
 *
 * Returns:
 *   A new UdpSessionManager instance with an automatic cleanup task running
 */
pub fn new(session_timeout: Duration, cleanup_interval: Duration) -> Self {
        let sessions = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
          Self::start_cleanup_task(sessions.clone(), session_timeout, cleanup_interval);
        Self {
            sessions,
            session_timeout,
            cleanup_interval,
        }
    }
    fn start_cleanup_task(
        sessions: Arc<
            tokio::sync::RwLock<std::collections::HashMap<std::net::SocketAddr, UdpSession>>,
        >,
        timeout: Duration,
        cleanup_interval: Duration,
    ) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                let mut sessions_guard = sessions.write().await;
                let initial_count = sessions_guard.len();
                sessions_guard.retain(|_, session| !session.is_expired(timeout));
                let removed = initial_count - sessions_guard.len();
                if removed > 0 {
                    tracing::debug!("Cleaned up {} expired UDP sessions", removed);
                }
            }
        });
    }
    pub async fn get_or_create_session(
        &self,
        peer_addr: std::net::SocketAddr,
    ) -> Option<UdpSession> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&peer_addr) {
            session.update_activity();
            return None; 
        }
        let bind_addr = if peer_addr.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };
        match tokio::net::UdpSocket::bind(bind_addr).await {
            Ok(client_socket) => {
                let local_addr = client_socket.local_addr().unwrap();
                let session = UdpSession::new(Arc::new(client_socket), local_addr);
                sessions.insert(peer_addr, session.clone());
                Some(session)
            }
            Err(e) => {
                tracing::error!("Failed to bind UDP socket for {}: {}", peer_addr, e);
                None
            }
        }
    }
    pub async fn remove_session(&self, peer_addr: &std::net::SocketAddr) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(peer_addr);
    }
    /**
     * Get the current session timeout duration.
     */
    pub fn session_timeout(&self) -> Duration {
        self.session_timeout
    }
    /**
     * Get the cleanup interval duration.
     */
    pub fn cleanup_interval(&self) -> Duration {
        self.cleanup_interval
    }
    /**
     * Get the number of active sessions.
     */
    pub async fn active_session_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}
