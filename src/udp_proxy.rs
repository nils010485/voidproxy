use crate::buffer_pool::{BufferPool, UdpSessionManager};
use crate::config::Config;
use anyhow::{Context, Result};
use bytes::BytesMut;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
/**
 * UDP proxy implementation for stateless UDP packet forwarding.
 *
 * Handles UDP proxy operations using session management to maintain
 * context for stateless UDP communication with timeout handling.
 */
pub struct UdpProxy {
    config: Arc<Config>,
    session_manager: Arc<UdpSessionManager>,
    instance_id: Uuid,
    instances: crate::instance::InstanceManager,
    buffer_pool: Arc<BufferPool>,
    ip_cache: Arc<crate::ip_cache::IpCache>,
}

impl UdpProxy {
    pub fn new(
        config: Arc<Config>,
        instance_id: Uuid,
        instances: crate::instance::InstanceManager,
    ) -> Self {
        Self {
            config,
            session_manager: Arc::new(UdpSessionManager::new(
                Duration::from_secs(300), // Session timeout
                Duration::from_secs(60),  // Cleanup interval
            )),
            instance_id,
            instances,
            buffer_pool: Arc::new(BufferPool::new(1000, 1000)),
            ip_cache: Arc::new(crate::ip_cache::IpCache::new(
                10_000,
                Duration::from_secs(300),
            )),
        }
    }

    /// Get session metrics for monitoring
    pub async fn get_session_metrics(&self) -> crate::metrics::SessionMetrics {
        crate::metrics::SessionMetrics {
            session_timeout_seconds: self.session_manager.session_timeout().as_secs(),
            cleanup_interval_seconds: self.session_manager.cleanup_interval().as_secs(),
            active_sessions: self.session_manager.active_session_count().await,
        }
    }

    pub async fn run_with_token(&self, cancel_token: Arc<CancellationToken>) -> Result<()> {
        let listen_addr =
            SocketAddr::new(self.config.proxy.listen_ip, self.config.proxy.listen_port);

        let socket = Arc::new(
            UdpSocket::bind(listen_addr)
                .await
                .context("Failed to bind UDP socket")?,
        );

        info!("UDP proxy listening on {}", listen_addr);
        info!(
            "Forwarding to {}:{}",
            self.config.proxy.dst_ip, self.config.proxy.dst_port
        );

        let mut buffer = self.buffer_pool.acquire(65535).await;
        loop {
            tokio::select! {
                // Check for cancellation
                _ = cancel_token.cancelled() => {
                    info!("UDP proxy shutdown signal received for instance {}", self.instance_id);
                    break;
                }
                // Receive UDP packet
                result = socket.recv_from(buffer.as_mut()) => {
                    match result {
                        Ok((len, peer_addr)) => {
                            // Check IP cache first
                            let ip_allowed = self.ip_cache.check_ip(&peer_addr.ip(), |ip| {
                                self.config.is_ip_allowed(ip)
                            }).await;

                            if !ip_allowed {
                                warn!("UDP packet rejected from {}: IP not allowed", peer_addr);
                                continue;
                            }


                            let data = buffer[..len].to_vec();
                            let session_manager = self.session_manager.clone();
                            let config = self.config.clone();
                            let socket_clone = socket.clone();
                            let instance_id = self.instance_id;
                            let instances = self.instances.clone();
                            let peer_addr_for_cleanup = peer_addr;
                            let cancel_token_clone = cancel_token.clone();


                            tokio::spawn(async move {
                                let result = Self::handle_udp_packet_with_token(
                                    data, peer_addr, socket_clone, config, session_manager, instance_id, instances, cancel_token_clone
                                ).await;
                                if let Err(e) = result {
                                    error!("Error handling UDP packet from {}: {}", peer_addr_for_cleanup, e);
                                }
                            });
                        }
                        Err(e) => {
                            if !cancel_token.is_cancelled() {
                                error!("Failed to receive UDP packet: {}", e);
                            }
                        }
                    }
                }
            }
        }

        info!("UDP proxy stopped for instance {}", self.instance_id);
        Ok(())
    }

    async fn handle_udp_packet_with_token(
        data: Vec<u8>,
        peer_addr: SocketAddr,
        socket: Arc<UdpSocket>,
        config: Arc<Config>,
        session_manager: Arc<UdpSessionManager>,
        instance_id: Uuid,
        instances: crate::instance::InstanceManager,
        cancel_token: Arc<CancellationToken>,
    ) -> Result<()> {
        let dst_addr = SocketAddr::new(config.proxy.dst_ip, config.proxy.dst_port);

        debug!(
            "Received {} bytes from UDP client {}",
            data.len(),
            peer_addr
        );

        let _client_socket = match session_manager.get_or_create_session(peer_addr).await {
            Some(session) => {
                let session_manager_clone = session_manager.clone();
                let peer_addr_clone = peer_addr;
                let server_socket = socket.clone();
                let instance_id_clone = instance_id;
                let instances_clone = instances.clone();
                let cancel_token_clone = cancel_token.clone();

                // Spawn response handler for new session
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_udp_responses_with_token(
                        session.client_socket.clone(),
                        peer_addr_clone,
                        server_socket,
                        session_manager_clone,
                        instance_id_clone,
                        instances_clone,
                        cancel_token_clone,
                    )
                    .await
                    {
                        error!("Error handling UDP responses: {}", e);
                    }
                });

                session.local_addr
            }
            None => {
                return Err(anyhow::anyhow!(
                    "Failed to create UDP session for {}",
                    peer_addr
                ));
            }
        };

        socket
            .send_to(&data, dst_addr)
            .await
            .context("Failed to send UDP packet to destination")?;

        debug!(
            "Forwarded {} bytes from {} to {}",
            data.len(),
            peer_addr,
            dst_addr
        );

        // Update traffic statistics using atomic operations
        let bytes_sent = data.len() as u64;
        if bytes_sent > 0 {
            let instances = instances.read().await;
            if let Some(instance) = instances.get(&instance_id) {
                instance.metrics.add_bytes_received(bytes_sent);
            }
        }

        Ok(())
    }

    async fn handle_udp_responses_with_token(
        client_socket: Arc<UdpSocket>,
        peer_addr: SocketAddr,
        server_socket: Arc<UdpSocket>,
        session_manager: Arc<UdpSessionManager>,
        instance_id: Uuid,
        instances: crate::instance::InstanceManager,
        cancel_token: Arc<CancellationToken>,
    ) -> Result<()> {
        let mut buffer = BytesMut::with_capacity(65535);
        loop {
            tokio::select! {
                // Check for cancellation
                _ = cancel_token.cancelled() => {
                    debug!("UDP response handler cancelled for instance {}", instance_id);
                    break;
                }
                // Receive response
                result = client_socket.recv_from(&mut buffer) => {
                    match result {
                        Ok((len, _)) => {
                            let data = &buffer[..len];
                            server_socket.send_to(data, peer_addr).await
                                .context("Failed to send UDP response to client")?;

                            // Reduce logging frequency
                                  debug!("Forwarded {} bytes response to UDP client {}", len, peer_addr);

                            // Update traffic statistics using atomic operations
                            let bytes_received = len as u64;
                            if bytes_received > 0 {
                                let instances = instances.read().await;
                                if let Some(instance) = instances.get(&instance_id) {
                                    instance.metrics.add_bytes_sent(bytes_received);
                                }
                            }
                        }
                        Err(e) => {
                            debug!("UDP connection from {} closed: {}", peer_addr, e);
                            break;
                        }
                    }
                }
            }
        }

        // Clean up session
        session_manager.remove_session(&peer_addr).await;

        Ok(())
    }
}
