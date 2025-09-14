use crate::instance::{
    CreateInstanceRequest, InstanceManager, ProxyInstance, UpdateInstanceRequest,
};
use crate::metrics::MetricsManager;
use crate::storage::StorageManager;
use crate::tcp_proxy::TcpProxy;
use crate::udp_proxy::UdpProxy;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

pub struct InstanceService {
    instances: InstanceManager,
    running_instances: Arc<RwLock<HashMap<Uuid, InstanceHandle>>>,
    storage: Arc<StorageManager>,
    metrics_manager: Arc<MetricsManager>,
}

struct InstanceHandle {
    tcp_handle: Option<tokio::task::JoinHandle<()>>,
    udp_handle: Option<tokio::task::JoinHandle<()>>,
    tcp_proxy: Option<std::sync::Arc<crate::tcp_proxy::TcpProxy>>,
    udp_proxy: Option<std::sync::Arc<crate::udp_proxy::UdpProxy>>,
    cancel_token: Option<Arc<tokio_util::sync::CancellationToken>>,
}

pub type PerformanceMetrics = crate::metrics::SystemMetrics;

impl InstanceService {
    pub fn with_storage(storage: Arc<StorageManager>) -> Self {
        let service = Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            running_instances: Arc::new(RwLock::new(HashMap::new())),
            storage,
            metrics_manager: Arc::new(MetricsManager::new()),
        };

        service
    }

    pub async fn create_instance(&self, request: CreateInstanceRequest) -> Result<ProxyInstance> {
        let config = request.to_config();
        config.validate()?;

        let instance = ProxyInstance::new(request.name, config, request.auto_start);

        let mut instances = self.instances.write().await;
        instances.insert(instance.id, instance.clone());

        // Register with metrics manager
        self.metrics_manager.register_instance(instance.id).await;

        // Save to persistent storage
        if let Err(e) = self.storage.add_instance(&instance).await {
            error!("Failed to save instance to storage: {}", e);
        }

        info!("Created proxy instance: {}", instance.name);

        if request.auto_start {
            self.start_instance(instance.id).await?;
        }

        Ok(instance)
    }

    pub async fn restore_instance(&self, instance: ProxyInstance) -> Result<()> {
        let mut instances = self.instances.write().await;
        instances.insert(instance.id, instance.clone());

        info!("Restored proxy instance: {}", instance.name);
        Ok(())
    }

    pub async fn get_instances(&self) -> Vec<ProxyInstance> {
        let instances = self.instances.read().await;
        instances.values().cloned().collect()
    }

    pub async fn get_instance(&self, id: Uuid) -> Option<ProxyInstance> {
        let instances = self.instances.read().await;
        instances.get(&id).cloned()
    }

    pub async fn update_instance(
        &self,
        id: Uuid,
        request: UpdateInstanceRequest,
    ) -> Result<Option<ProxyInstance>> {
        let mut instances = self.instances.write().await;

        if let Some(instance) = instances.get_mut(&id) {
            let was_running = instance.status == crate::instance::InstanceStatus::Running;

            request.apply_to(instance);
            instance.config.validate()?;

            // Save to persistent storage
            if let Err(e) = self.storage.update_instance(instance).await {
                error!("Failed to update instance in storage: {}", e);
            }

            if was_running {
                self.stop_instance_internal(id).await?;
                self.start_instance_internal(id).await?;
            }

            info!("Updated proxy instance: {}", instance.name);
            Ok(Some(instance.clone()))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_instance(&self, id: Uuid) -> Result<bool> {
        self.stop_instance_internal(id).await?;

        let mut instances = self.instances.write().await;
        let removed = instances.remove(&id).is_some();

        if removed {
            // Unregister from metrics manager
            self.metrics_manager.unregister_instance(&id).await;

            // Remove from persistent storage
            if let Err(e) = self.storage.remove_instance(id).await {
                error!("Failed to remove instance from storage: {}", e);
            }
            info!("Deleted proxy instance: {}", id);
        }

        Ok(removed)
    }

    pub async fn start_instance(&self, id: Uuid) -> Result<bool> {
        self.start_instance_internal(id).await
    }

    async fn start_instance_internal(&self, id: Uuid) -> Result<bool> {
        let mut instances = self.instances.write().await;

        if let Some(instance) = instances.get_mut(&id) {
            if instance.status == crate::instance::InstanceStatus::Running {
                return Ok(true);
            }

            instance.start();
            let config = Arc::new(instance.config.clone());

            let cancel_token = Arc::new(tokio_util::sync::CancellationToken::new());
            let (tcp_handle, tcp_proxy) = if matches!(
                config.proxy.protocol,
                crate::config::Protocol::Tcp | crate::config::Protocol::Both
            ) {
                let instances = self.instances.clone();
                let tcp_proxy = std::sync::Arc::new(TcpProxy::new(config.clone(), id, instances));
                let token_clone = cancel_token.clone();
                let handle = Some(tokio::spawn({
                    let tcp_proxy_clone = tcp_proxy.clone();
                    async move {
                        if let Err(e) = tcp_proxy_clone.run_with_token(token_clone).await {
                            error!("TCP proxy error for instance {}: {}", id, e);
                        }
                    }
                }));
                (handle, Some(tcp_proxy))
            } else {
                (None, None)
            };

            let (udp_handle, udp_proxy) = if matches!(
                config.proxy.protocol,
                crate::config::Protocol::Udp | crate::config::Protocol::Both
            ) {
                let instances = self.instances.clone();
                let udp_proxy = std::sync::Arc::new(UdpProxy::new(config.clone(), id, instances));
                let token_clone = cancel_token.clone();
                let handle = Some(tokio::spawn({
                    let udp_proxy_clone = udp_proxy.clone();
                    async move {
                        if let Err(e) = udp_proxy_clone.run_with_token(token_clone).await {
                            error!("UDP proxy error for instance {}: {}", id, e);
                        }
                    }
                }));
                (handle, Some(udp_proxy))
            } else {
                (None, None)
            };

            let mut running_instances = self.running_instances.write().await;
            running_instances.insert(
                id,
                InstanceHandle {
                    tcp_handle,
                    udp_handle,
                    tcp_proxy,
                    udp_proxy,
                    cancel_token: Some(cancel_token.clone()),
                },
            );

            instance.set_running();
            info!("Started proxy instance: {}", instance.name);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn stop_instance(&self, id: Uuid) -> Result<bool> {
        self.stop_instance_internal(id).await
    }

    async fn stop_instance_internal(&self, id: Uuid) -> Result<bool> {
        let mut instances = self.instances.write().await;

        if let Some(instance) = instances.get_mut(&id) {
            if instance.status != crate::instance::InstanceStatus::Running {
                return Ok(true);
            }

            instance.stop();

            let mut running_instances = self.running_instances.write().await;
            if let Some(handle) = running_instances.remove(&id) {
                // Cancel the tasks first
                if let Some(cancel_token) = handle.cancel_token {
                    cancel_token.cancel();
                }

                // Give tasks a moment to clean up gracefully
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

                // Then abort the tasks if they haven't stopped
                if let Some(tcp_handle) = handle.tcp_handle {
                    tcp_handle.abort();
                }
                if let Some(udp_handle) = handle.udp_handle {
                    udp_handle.abort();
                }
            }

            instance.set_stopped();
            info!("Stopped proxy instance: {}", instance.name);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn start_auto_instances(&self) -> Result<()> {
        let instances = self.instances.read().await;
        let auto_start_instances: Vec<Uuid> = instances
            .values()
            .filter(|instance| instance.auto_start)
            .map(|instance| instance.id)
            .collect();

        for id in auto_start_instances {
            if let Err(e) = self.start_instance_internal(id).await {
                error!("Failed to start auto-start instance {}: {}", id, e);
            }
        }

        Ok(())
    }

    pub async fn get_instance_stats(&self) -> HashMap<Uuid, InstanceStats> {
        let instances = self.instances.read().await;
        let running_instances = self.running_instances.read().await;

        let mut stats = HashMap::new();
        let mut started_times = HashMap::new();

        for (id, instance) in instances.iter() {
            let is_running = running_instances.contains_key(id);
            started_times.insert(*id, instance.started_at);

            // Get metrics directly from the instance
            let instance_metrics = instance.metrics.get_stats(instance.started_at).await;

            // Debug log
            if instance_metrics.bytes_sent > 0 || instance_metrics.bytes_received > 0 {
                debug!(
                    "Instance {} ({}) - Sent: {} bytes, Received: {} bytes",
                    id, instance.name, instance_metrics.bytes_sent, instance_metrics.bytes_received
                );
            }

            stats.insert(
                *id,
                InstanceStats {
                    id: *id,
                    name: instance.name.clone(),
                    status: instance.status,
                    is_running,
                    uptime: instance.started_at.map(|started| {
                        started
                            .signed_duration_since(chrono::Utc::now())
                            .num_seconds()
                            .abs()
                    }),
                    bytes_sent: instance_metrics.bytes_sent,
                    bytes_received: instance_metrics.bytes_received,
                    connections_active: instance_metrics.connections_active,
                    bytes_sent_per_sec: instance_metrics.bytes_sent_per_sec,
                    bytes_received_per_sec: instance_metrics.bytes_received_per_sec,
                    error_rate: instance_metrics.error_rate,
                },
            );
        }

        debug!(
            "get_instance_stats returning stats for {} instances",
            stats.len()
        );
        stats
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstanceStats {
    pub id: Uuid,
    pub name: String,
    pub status: crate::instance::InstanceStatus,
    pub is_running: bool,
    pub uptime: Option<i64>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connections_active: u32,
    pub bytes_sent_per_sec: f64,
    pub bytes_received_per_sec: f64,
    pub error_rate: f64,
}

impl InstanceService {
    pub async fn export_config(&self) -> Result<String> {
        self.storage.export_config().await
    }

    pub async fn import_config(&self, config_content: &str) -> Result<()> {
        // Clear existing instances
        let current_instances = self.get_instances().await;
        for instance in current_instances {
            self.stop_instance_internal(instance.id).await?;
            let mut instances = self.instances.write().await;
            instances.remove(&instance.id);
        }

        // Import new configuration
        self.storage.import_config(config_content).await?;

        // Load the imported instances
        match self.storage.load().await {
            Ok(imported_instances) => {
                let count = imported_instances.len();
                for instance in imported_instances {
                    let mut instances_map = self.instances.write().await;
                    instances_map.insert(instance.id, instance.clone());
                }
                info!("Imported {} instances", count);
            }
            Err(e) => {
                return Err(e);
            }
        }

        Ok(())
    }

    pub async fn create_backup(&self) -> Result<std::path::PathBuf> {
        self.storage.create_backup().await
    }

    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.metrics_manager.get_system_metrics().await
    }

    pub async fn get_instance_session_metrics(
        &self,
        instance_id: &Uuid,
    ) -> Option<crate::metrics::SessionMetrics> {
        let running_instances = self.running_instances.read().await;
        if let Some(handle) = running_instances.get(instance_id) {
            if let Some(ref udp_proxy) = handle.udp_proxy {
                return Some(udp_proxy.get_session_metrics().await);
            }
            if let Some(ref _tcp_proxy) = handle.tcp_proxy {
                // Pour TCP, on retourne des metrics de session basées sur les connexions actives depuis les metrics
                let instances = self.instances.read().await;
                if let Some(instance) = instances.get(instance_id) {
                    let instance_metrics = instance.metrics.get_stats(instance.started_at).await;
                    return Some(crate::metrics::SessionMetrics {
                        session_timeout_seconds: 300, // Timeout TCP par défaut
                        cleanup_interval_seconds: 60,
                        active_sessions: instance_metrics.connections_active as usize,
                    });
                }
            }
        }
        None
    }
}
