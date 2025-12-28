use crate::config::{Config, LogLevel, Protocol};
use crate::metrics::InstanceMetrics;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
/**
 * Represents a proxy instance with its configuration and runtime state.
 *
 * Each proxy instance has a unique ID, name, configuration, current status,
 * timestamps for creation and startup, and associated metrics.
 */
pub struct ProxyInstance {
    pub id: Uuid,
    pub name: String,
    pub config: Config,
    pub status: InstanceStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub auto_start: bool,
    #[serde(skip)]
    pub metrics: Arc<InstanceMetrics>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/**
 * Runtime status of a proxy instance.
 *
 * Tracks the current operational state of a proxy instance through its lifecycle.
 */
pub enum InstanceStatus {
    Stopped,
    Running,
    Error,
    Starting,
    Stopping,
}
impl ProxyInstance {
    pub fn new(name: String, config: Config, auto_start: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            config,
            status: InstanceStatus::Stopped,
            created_at: Utc::now(),
            started_at: None,
            auto_start,
            metrics: Arc::new(InstanceMetrics::new()),
        }
    }
    pub fn start(&mut self) {
        self.status = InstanceStatus::Starting;
        self.started_at = Some(Utc::now());
    }
    pub fn set_running(&mut self) {
        self.status = InstanceStatus::Running;
    }
    pub fn stop(&mut self) {
        self.status = InstanceStatus::Stopping;
        self.started_at = None;
    }
    pub fn set_stopped(&mut self) {
        self.status = InstanceStatus::Stopped;
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/**
 * Request structure for creating a new proxy instance.
 *
 * Contains all the necessary parameters to create and configure a proxy instance.
 */
pub struct CreateInstanceRequest {
    pub name: String,
    pub listen_ip: IpAddr,
    pub listen_port: u16,
    pub dst_ip: IpAddr,
    pub dst_port: u16,
    pub protocol: Protocol,
    pub auto_start: bool,
    pub allow_list: Option<Vec<IpAddr>>,
    pub deny_list: Option<Vec<IpAddr>>,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub log_level: LogLevel,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/**
 * String-based request structure for creating a proxy instance.
 *
 * Used for API requests where IP addresses are provided as strings
 * and need to be parsed and validated.
 */
pub struct CreateInstanceRequestStrings {
    pub name: String,
    pub listen_ip: String,
    pub listen_port: u16,
    pub dst_ip: String,
    pub dst_port: u16,
    pub protocol: Protocol,
    pub auto_start: bool,
    pub allow_list: Option<Vec<String>>,
    pub deny_list: Option<Vec<String>>,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub log_level: String,
}
impl CreateInstanceRequestStrings {
    pub fn to_typed(&self) -> Result<CreateInstanceRequest, String> {
        let listen_ip = self
            .listen_ip
            .parse()
            .map_err(|e| format!("Invalid listen IP: {}", e))?;
        let dst_ip = self
            .dst_ip
            .parse()
            .map_err(|e| format!("Invalid destination IP: {}", e))?;
        let allow_list = self
            .allow_list
            .as_ref()
            .map(|list| {
                list.iter()
                    .map(|s| {
                        s.parse()
                            .map_err(|e| format!("Invalid allow IP {}: {}", s, e))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
            .map_err(|e| format!("Invalid allow list: {}", e))?;
        let deny_list = self
            .deny_list
            .as_ref()
            .map(|list| {
                list.iter()
                    .map(|s| {
                        s.parse()
                            .map_err(|e| format!("Invalid deny IP {}: {}", s, e))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
            .map_err(|e| format!("Invalid deny list: {}", e))?;
        let log_level = self.log_level.to_lowercase();
        let log_level = match log_level.as_str() {
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => return Err(format!("Invalid log level: {}", self.log_level)),
        };
        Ok(CreateInstanceRequest {
            name: self.name.clone(),
            listen_ip,
            listen_port: self.listen_port,
            dst_ip,
            dst_port: self.dst_port,
            protocol: self.protocol,
            auto_start: self.auto_start,
            allow_list,
            deny_list,
            connect_timeout_secs: self.connect_timeout_secs,
            idle_timeout_secs: self.idle_timeout_secs,
            log_level,
        })
    }
}
impl CreateInstanceRequest {
    pub fn to_config(&self) -> Config {
        Config {
            proxy: crate::config::ProxyConfig {
                listen_ip: self.listen_ip,
                listen_port: self.listen_port,
                dst_ip: self.dst_ip,
                dst_port: self.dst_port,
                protocol: self.protocol,
                connect_timeout_secs: self.connect_timeout_secs,
                idle_timeout_secs: self.idle_timeout_secs,
                log_level: self.log_level,
            },
            ip_filter: if self.allow_list.is_some() || self.deny_list.is_some() {
                Some(crate::config::IpFilterConfig {
                    allow_list: self.allow_list.clone(),
                    deny_list: self.deny_list.clone(),
                })
            } else {
                None
            },
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/**
 * Request structure for updating an existing proxy instance.
 *
 * Contains optional fields for updating specific aspects of a proxy instance.
 * Only provided fields will be updated.
 */
pub struct UpdateInstanceRequest {
    pub name: Option<String>,
    pub listen_ip: Option<IpAddr>,
    pub listen_port: Option<u16>,
    pub dst_ip: Option<IpAddr>,
    pub dst_port: Option<u16>,
    pub protocol: Option<Protocol>,
    pub auto_start: Option<bool>,
    pub allow_list: Option<Vec<IpAddr>>,
    pub deny_list: Option<Vec<IpAddr>>,
    pub connect_timeout_secs: Option<u64>,
    pub idle_timeout_secs: Option<u64>,
    pub log_level: Option<LogLevel>,
}
impl UpdateInstanceRequest {
    pub fn apply_to(&self, instance: &mut ProxyInstance) {
        if let Some(name) = &self.name {
            instance.name = name.clone();
        }
        if let Some(listen_ip) = self.listen_ip {
            instance.config.proxy.listen_ip = listen_ip;
        }
        if let Some(listen_port) = self.listen_port {
            instance.config.proxy.listen_port = listen_port;
        }
        if let Some(dst_ip) = self.dst_ip {
            instance.config.proxy.dst_ip = dst_ip;
        }
        if let Some(dst_port) = self.dst_port {
            instance.config.proxy.dst_port = dst_port;
        }
        if let Some(protocol) = self.protocol {
            instance.config.proxy.protocol = protocol;
        }
        if let Some(auto_start) = self.auto_start {
            instance.auto_start = auto_start;
        }
        if self.allow_list.is_some() || self.deny_list.is_some() {
            instance.config.ip_filter = Some(crate::config::IpFilterConfig {
                allow_list: self.allow_list.clone(),
                deny_list: self.deny_list.clone(),
            });
        }
        if let Some(connect_timeout_secs) = self.connect_timeout_secs {
            instance.config.proxy.connect_timeout_secs = connect_timeout_secs;
        }
        if let Some(idle_timeout_secs) = self.idle_timeout_secs {
            instance.config.proxy.idle_timeout_secs = idle_timeout_secs;
        }
        if let Some(log_level) = self.log_level {
            instance.config.proxy.log_level = log_level;
        }
    }
}
pub type InstanceManager = Arc<RwLock<HashMap<Uuid, ProxyInstance>>>;
