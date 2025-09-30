use crate::buffer_pool::BufferPool;
use crate::config::Config;
use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
#[derive(Clone)]
/**
 * TCP proxy implementation for forwarding TCP connections.
 *
 * Handles TCP proxy operations including connection forwarding,
 * traffic monitoring, and resource management.
 */
pub struct TcpProxy {
    config: Arc<Config>,
    instance_id: Uuid,
    instances: crate::instance::InstanceManager,
    buffer_pool: Arc<BufferPool>,
    ip_cache: Arc<crate::ip_cache::IpCache>,
}
impl TcpProxy {
    pub fn new(
        config: Arc<Config>,
        instance_id: Uuid,
        instances: crate::instance::InstanceManager,
    ) -> Self {
        let ip_cache_ttl = config.proxy.idle_timeout_secs;
        Self {
            config,
            instance_id,
            instances,
            buffer_pool: Arc::new(BufferPool::new(1000, 1000)),
            ip_cache: Arc::new(crate::ip_cache::IpCache::new(
                10_000,
                Duration::from_secs(ip_cache_ttl),
            )),
        }
    }
    pub async fn run_with_token(&self, cancel_token: Arc<CancellationToken>) -> Result<()> {
        let listen_addr =
            SocketAddr::new(self.config.proxy.listen_ip, self.config.proxy.listen_port);
        let listener = TcpListener::bind(listen_addr)
            .await
            .context("Failed to bind TCP listener")?;
        info!("TCP proxy listening on {}", listen_addr);
        info!(
            "Forwarding to {}:{}",
            self.config.proxy.dst_ip, self.config.proxy.dst_port
        );
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("TCP proxy shutdown signal received for instance {}", self.instance_id);
                    break;
                }
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, peer_addr)) => {
                            if cancel_token.is_cancelled() {
                                break;
                            }
                            let ip_allowed = self.ip_cache.check_ip(&peer_addr.ip(), |ip| {
                                self.config.is_ip_allowed(ip)
                            }).await;
                            if !ip_allowed {
                                warn!("Connection rejected from {}: IP not allowed", peer_addr);
                                continue;
                            }
                            let config = self.config.clone();
                            let instance_id = self.instance_id;
                            let instances = self.instances.clone();
                            let buffer_pool = self.buffer_pool.clone();
                            let peer_addr_for_release = peer_addr;
                            let cancel_token_clone = cancel_token.clone();
                            tokio::spawn(async move {
                                let result = Self::handle_connection_with_token(
                                    stream, peer_addr, config, instance_id, instances, buffer_pool, cancel_token_clone
                                ).await;
                                if let Err(e) = result {
                                    error!("Error handling connection from {}: {}", peer_addr_for_release, e);
                                }
                            });
                        }
                        Err(e) => {
                            if !cancel_token.is_cancelled() {
                                error!("Failed to accept TCP connection: {}", e);
                            }
                        }
                    }
                }
            }
        }
        info!("TCP proxy stopped for instance {}", self.instance_id);
        Ok(())
    }
    async fn handle_connection_with_token(
        client_stream: TcpStream,
        peer_addr: SocketAddr,
        config: Arc<Config>,
        instance_id: Uuid,
        instances: crate::instance::InstanceManager,
        buffer_pool: Arc<BufferPool>,
        cancel_token: Arc<CancellationToken>,
    ) -> Result<()> {
        let dst_addr = SocketAddr::new(config.proxy.dst_ip, config.proxy.dst_port);
        let connect_timeout = Duration::from_secs(config.proxy.connect_timeout_secs);
        debug!("New TCP connection from {} to {}", peer_addr, dst_addr);
        let server_stream = match timeout(connect_timeout, TcpStream::connect(dst_addr)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                warn!(
                    "Failed to connect to destination server {} for client {}: {}",
                    dst_addr, peer_addr, e
                );
                let instances = instances.read().await;
                if let Some(instance) = instances.get(&instance_id) {
                    instance.metrics.errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                return Ok(());
            }
            Err(_) => {
                warn!(
                    "Connection timeout to destination server {} for client {} after {}s",
                    dst_addr, peer_addr, config.proxy.connect_timeout_secs
                );
                let instances = instances.read().await;
                if let Some(instance) = instances.get(&instance_id) {
                    instance.metrics.errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                return Ok(());
            }
        };
        let (client_reader, client_writer) = client_stream.into_split();
        let (server_reader, server_writer) = server_stream.into_split();
        let idle_timeout_duration = Duration::from_secs(config.proxy.idle_timeout_secs);
        let idle_timeout_secs = config.proxy.idle_timeout_secs;
        let client_to_server = {
                        let buffer_pool = buffer_pool.clone();
            let instances_for_client = instances.clone();
            let cancel_token_clone = cancel_token.clone();
            let idle_timeout = idle_timeout_duration;
            tokio::spawn(async move {
                let mut buffer = buffer_pool.acquire(8192).await;
                let mut reader = client_reader;
                let mut writer = server_writer;
                let mut total_bytes = 0u64;
                let mut packets_processed = 0u32;
                loop {
                    tokio::select! {
                        _ = cancel_token_clone.cancelled() => {
                            debug!("Client to server task cancelled for instance {}", instance_id);
                            break;
                        }
                        read_result = timeout(idle_timeout, reader.read_buf(buffer.as_mut())) => {
                            match read_result {
                                Ok(Ok(0)) => break,
                                Ok(Ok(n)) => {
                                    if packets_processed % 100 == 0 {
                                        debug!("Read {} bytes from client", n);
                                    }
                                    total_bytes += n as u64;
                                    packets_processed += 1;
                                    if let Err(e) = writer.write_all(&buffer[..n]).await {
                                        error!("Failed to write to server: {}", e);
                                        break;
                                    }
                                    buffer.clear();
                                }
                                Ok(Err(e)) => {
                                    error!("Failed to read from client: {}", e);
                                    break;
                                }
                                Err(_) => {
                                    debug!("Client to server connection idle timeout after {}s", idle_timeout_secs);
                                    break;
                                }
                            }
                        }
                    }
                }
                if total_bytes > 0 {
                    let instances = instances_for_client.read().await;
                    if let Some(instance) = instances.get(&instance_id) {
                        instance.metrics.add_bytes_received(total_bytes);
                    }
                }
            })
        };
        let server_to_client = {
                        let buffer_pool = buffer_pool.clone();
            let instances_for_server = instances.clone();
            let cancel_token_clone = cancel_token.clone();
            let idle_timeout = idle_timeout_duration;
            tokio::spawn(async move {
                let mut buffer = buffer_pool.acquire(8192).await;
                let mut reader = server_reader;
                let mut writer = client_writer;
                let mut total_bytes = 0u64;
                let mut packets_processed = 0u32;
                loop {
                    tokio::select! {
                        _ = cancel_token_clone.cancelled() => {
                            debug!("Server to client task cancelled for instance {}", instance_id);
                            break;
                        }
                        read_result = timeout(idle_timeout, reader.read_buf(buffer.as_mut())) => {
                            match read_result {
                                Ok(Ok(0)) => break,
                                Ok(Ok(n)) => {
                                    if packets_processed % 100 == 0 {
                                        debug!("Read {} bytes from server", n);
                                    }
                                    total_bytes += n as u64;
                                    packets_processed += 1;
                                    if let Err(e) = writer.write_all(&buffer[..n]).await {
                                        error!("Failed to write to client: {}", e);
                                        break;
                                    }
                                    buffer.clear();
                                }
                                Ok(Err(e)) => {
                                    error!("Failed to read from server: {}", e);
                                    break;
                                }
                                Err(_) => {
                                    debug!("Server to client connection idle timeout after {}s", config.proxy.idle_timeout_secs);
                                    break;
                                }
                            }
                        }
                    }
                }
                if total_bytes > 0 {
                    let instances = instances_for_server.read().await;
                    if let Some(instance) = instances.get(&instance_id) {
                        instance.metrics.add_bytes_sent(total_bytes);
                    }
                }
            })
        };
        tokio::select! {
            _ = cancel_token.cancelled() => {
                debug!("Connection handler cancelled for instance {}", instance_id);
            }
            result = client_to_server => {
                if let Err(e) = result {
                    error!("Client to server task failed: {}", e);
                }
            }
            result = server_to_client => {
                if let Err(e) = result {
                    error!("Server to client task failed: {}", e);
                }
            }
        }
        debug!("TCP connection from {} closed", peer_addr);
        Ok(())
    }
}
