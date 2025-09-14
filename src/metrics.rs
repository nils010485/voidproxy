use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
/**
 * Metrics tracking for a single proxy instance.
 *
 * Tracks various performance and usage metrics for a proxy instance
 * including traffic statistics and error counts.
 */
pub struct InstanceMetrics {
    pub bytes_sent: Arc<AtomicU64>,
    pub bytes_received: Arc<AtomicU64>,
    pub connections_active: Arc<AtomicU32>,
    pub connections_total: Arc<AtomicU32>,
    pub errors: Arc<AtomicU32>,
    last_update: Arc<RwLock<Instant>>,
}

impl InstanceMetrics {
    pub fn new() -> Self {
        Self {
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            connections_active: Arc::new(AtomicU32::new(0)),
            connections_total: Arc::new(AtomicU32::new(0)),
            errors: Arc::new(AtomicU32::new(0)),
            last_update: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub fn add_bytes_sent(&self, bytes: u64) {
        // Protection contre l'overflow - on sature à la valeur maximale
        let current = self.bytes_sent.load(Ordering::Relaxed);
        if let Some(new_value) = current.checked_add(bytes) {
            self.bytes_sent.store(new_value, Ordering::Relaxed);
        } else {
            self.bytes_sent.store(u64::MAX, Ordering::Relaxed);
        }
        self.update_timestamp();
    }

    pub fn add_bytes_received(&self, bytes: u64) {
        // Protection contre l'overflow - on sature à la valeur maximale
        let current = self.bytes_received.load(Ordering::Relaxed);
        if let Some(new_value) = current.checked_add(bytes) {
            self.bytes_received.store(new_value, Ordering::Relaxed);
        } else {
            self.bytes_received.store(u64::MAX, Ordering::Relaxed);
        }
        self.update_timestamp();
    }

    fn update_timestamp(&self) {
        if let Ok(mut last_update) = self.last_update.try_write() {
            *last_update = Instant::now();
        }
    }

    pub async fn get_stats(&self, started_at: Option<DateTime<Utc>>) -> InstanceStats {
        let bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        let bytes_received = self.bytes_received.load(Ordering::Relaxed);
        let connections_active = self.connections_active.load(Ordering::Relaxed);
        let connections_total = self.connections_total.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);

        let (bytes_sent_per_sec, bytes_received_per_sec) = if let Some(started) = started_at {
            let duration = Utc::now().signed_duration_since(started);
            let seconds = duration.num_seconds().max(1) as f64;
            (bytes_sent as f64 / seconds, bytes_received as f64 / seconds)
        } else {
            (0.0, 0.0)
        };

        let error_rate = if connections_total > 0 {
            errors as f64 / connections_total as f64
        } else {
            0.0
        };

        InstanceStats {
            bytes_sent,
            bytes_received,
            connections_active,
            connections_total,
            errors,
            bytes_sent_per_sec,
            bytes_received_per_sec,
            error_rate,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
/**
 * Statistical summary of instance metrics.
 *
 * Contains computed statistics derived from raw metrics including
 * throughput rates and error percentages.
 */
pub struct InstanceStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connections_active: u32,
    pub connections_total: u32,
    pub errors: u32,
    pub bytes_sent_per_sec: f64,
    pub bytes_received_per_sec: f64,
    pub error_rate: f64,
}

/**
 * Manages metrics collection for all proxy instances.
 *
 * Provides centralized metrics management including instance registration,
 * system metrics collection, and performance monitoring.
 */
pub struct MetricsManager {
    instances: Arc<RwLock<std::collections::HashMap<Uuid, InstanceMetrics>>>,
    system_metrics: Arc<RwLock<SystemMetrics>>,
}

#[derive(Debug, Clone, serde::Serialize)]
/**
 * System-wide performance metrics.
 *
 * Tracks overall system performance including memory usage,
 * CPU utilization, and connection statistics.
 */
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub total_memory_mb: u64,
    pub used_memory_mb: u64,
    pub cpu_usage_percent: f64,
    pub active_connections: u32,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
/**
 * Session-specific metrics for UDP proxy operations.
 *
 * Tracks session management metrics including timeout settings
 * and active session counts.
 */
pub struct SessionMetrics {
    pub session_timeout_seconds: u64,
    pub cleanup_interval_seconds: u64,
    pub active_sessions: usize,
}

impl MetricsManager {
    pub fn new() -> Self {
        let manager = Self {
            instances: Arc::new(RwLock::new(std::collections::HashMap::new())),
            system_metrics: Arc::new(RwLock::new(SystemMetrics {
                uptime_seconds: 0,
                total_memory_mb: 0,
                used_memory_mb: 0,
                cpu_usage_percent: 0.0,
                active_connections: 0,
                last_updated: Utc::now(),
            })),
        };

        manager.start_system_metrics_collection();
        manager
    }

    fn start_system_metrics_collection(&self) {
        let system_metrics = self.system_metrics.clone();
        let instances = self.instances.clone();
        let start_time = Instant::now();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                let uptime = start_time.elapsed().as_secs();

                // Get memory info using sys-info
                let (total_memory, used_memory) = if let Ok(mem_info) = sys_info::mem_info() {
                    (
                        mem_info.total as u64 / (1024 * 1024),
                        (mem_info.total - mem_info.free) as u64 / (1024 * 1024),
                    )
                } else {
                    (0, 0)
                };

                // Count active connections from all instances
                let active_connections = {
                    let instances_guard = instances.read().await;
                    instances_guard
                        .values()
                        .map(|m| m.connections_active.load(Ordering::Relaxed))
                        .sum()
                };

                let mut metrics_guard = system_metrics.write().await;
                metrics_guard.uptime_seconds = uptime;
                metrics_guard.total_memory_mb = total_memory;
                metrics_guard.used_memory_mb = used_memory;
                metrics_guard.active_connections = active_connections;
                metrics_guard.last_updated = Utc::now();

                // CPU usage is complex to measure accurately without external crates
                // Using a placeholder for now
                metrics_guard.cpu_usage_percent = 0.0;
            }
        });
    }

    pub async fn register_instance(&self, instance_id: Uuid) {
        let mut instances = self.instances.write().await;
        instances.insert(instance_id, InstanceMetrics::new());
    }

    pub async fn unregister_instance(&self, instance_id: &Uuid) {
        let mut instances = self.instances.write().await;
        instances.remove(instance_id);
    }

    pub async fn get_system_metrics(&self) -> SystemMetrics {
        self.system_metrics.read().await.clone()
    }
}
