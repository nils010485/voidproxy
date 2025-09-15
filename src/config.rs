use serde::{Deserialize, Serialize};
use std::net::IpAddr;
#[derive(Debug, Clone, Serialize, Deserialize)]
/**
 * Main configuration structure for proxy instances.
 *
 * Contains all the necessary configuration for a proxy instance including
 * the proxy settings and optional IP filtering configuration.
 */
pub struct Config {
    pub proxy: ProxyConfig,
    pub ip_filter: Option<IpFilterConfig>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/**
 * Core proxy configuration settings.
 *
 * Defines the listening and destination addresses and ports for the proxy,
 * as well as the protocol to use and automatic startup behavior.
 */
pub struct ProxyConfig {
    pub listen_ip: IpAddr,
    pub listen_port: u16,
    pub dst_ip: IpAddr,
    pub dst_port: u16,
    pub protocol: Protocol,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub log_level: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/**
 * Supported proxy protocols.
 *
 * Defines which network protocols the proxy should handle.
 */
pub enum Protocol {
    Tcp,
    Udp,
    Both,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/**
 * Supported logging levels.
 *
 * Defines the verbosity level for logging output.
 */
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/**
 * IP filtering configuration for access control.
 *
 * Allows defining allow lists and deny lists to control which clients
 * can connect to the proxy. Only one of allow_list or deny_list can be used.
 */
pub struct IpFilterConfig {
    pub allow_list: Option<Vec<IpAddr>>,
    pub deny_list: Option<Vec<IpAddr>>,
}
impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.proxy.listen_port == 0 {
            return Err(anyhow::anyhow!("Listen port cannot be 0"));
        }
        if self.proxy.dst_port == 0 {
            return Err(anyhow::anyhow!("Destination port cannot be 0"));
        }
        if self.proxy.connect_timeout_secs == 0 {
            return Err(anyhow::anyhow!("Connect timeout must be greater than 0"));
        }
        if self.proxy.idle_timeout_secs == 0 {
            return Err(anyhow::anyhow!("Idle timeout must be greater than 0"));
        }
        if self.proxy.connect_timeout_secs > 300 {
            return Err(anyhow::anyhow!("Connect timeout cannot exceed 300 seconds"));
        }
        if self.proxy.idle_timeout_secs > 3600 {
            return Err(anyhow::anyhow!("Idle timeout cannot exceed 3600 seconds"));
        }
        let valid_log_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_log_levels.contains(&self.proxy.log_level.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid log level '{}'. Must be one of: {}",
                self.proxy.log_level,
                valid_log_levels.join(", ")
            ));
        }
        if self.proxy.listen_port == self.proxy.dst_port
            && self.proxy.listen_ip == self.proxy.dst_ip
        {
            return Err(anyhow::anyhow!(
                "Listen and destination cannot be the same address and port"
            ));
        }
        if self.proxy.listen_ip.is_loopback() && !self.proxy.dst_ip.is_loopback() {
            tracing::warn!(
                "Instance '{}' listens on loopback but forwards to non-loopback - this may create a security risk",
                std::any::type_name::<Self>()
            );
        }
        if let Some(ref ip_filter) = self.ip_filter {
            if let Some(ref allow_list) = ip_filter.allow_list {
                if allow_list.is_empty() {
                    return Err(anyhow::anyhow!("Allow list cannot be empty"));
                }
                let mut unique_ips = std::collections::HashSet::new();
                for ip in allow_list {
                    if !unique_ips.insert(ip) {
                        return Err(anyhow::anyhow!(
                            "Duplicate IP address in allow list: {}",
                            ip
                        ));
                    }
                }
            }
            if let Some(ref deny_list) = ip_filter.deny_list {
                if deny_list.is_empty() {
                    return Err(anyhow::anyhow!("Deny list cannot be empty"));
                }
                let mut unique_ips = std::collections::HashSet::new();
                for ip in deny_list {
                    if !unique_ips.insert(ip) {
                        return Err(anyhow::anyhow!("Duplicate IP address in deny list: {}", ip));
                    }
                }
            }
            if ip_filter.allow_list.is_some() && ip_filter.deny_list.is_some() {
                return Err(anyhow::anyhow!(
                    "Cannot specify both allow_list and deny_list"
                ));
            }
        }
        Ok(())
    }
    pub fn is_ip_allowed(&self, ip: &IpAddr) -> bool {
        match &self.ip_filter {
            Some(filter) => {
                if let Some(ref allow_list) = filter.allow_list {
                    allow_list.contains(ip)
                } else if let Some(ref deny_list) = filter.deny_list {
                    !deny_list.contains(ip)
                } else {
                    true
                }
            }
            None => true,
        }
    }
}
