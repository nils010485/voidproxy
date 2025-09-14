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
        // Validate listen port
        if self.proxy.listen_port == 0 {
            return Err(anyhow::anyhow!("Listen port cannot be 0"));
        }

        // Validate destination port
        if self.proxy.dst_port == 0 {
            return Err(anyhow::anyhow!("Destination port cannot be 0"));
        }

        // Note: u16 type already enforces port range (1-65535), but we validate port 0 explicitly above

        // Prevent port collision
        if self.proxy.listen_port == self.proxy.dst_port
            && self.proxy.listen_ip == self.proxy.dst_ip
        {
            return Err(anyhow::anyhow!(
                "Listen and destination cannot be the same address and port"
            ));
        }

        // Validate IP addresses
        if self.proxy.listen_ip.is_loopback() && !self.proxy.dst_ip.is_loopback() {
            // Warn about potential security implications but allow it
            tracing::warn!(
                "Instance '{}' listens on loopback but forwards to non-loopback - this may create a security risk",
                std::any::type_name::<Self>()
            );
        }

        // Validate IP filter configuration
        if let Some(ref ip_filter) = self.ip_filter {
            if let Some(ref allow_list) = ip_filter.allow_list {
                if allow_list.is_empty() {
                    return Err(anyhow::anyhow!("Allow list cannot be empty"));
                }

                // Check for duplicate IPs in allow list
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

                // Check for duplicate IPs in deny list
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_config_validation_valid_config() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 80,
                protocol: Protocol::Tcp,
            },
            ip_filter: None,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_zero_listen_port() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 0,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 80,
                protocol: Protocol::Tcp,
            },
            ip_filter: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_dst_port() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 0,
                protocol: Protocol::Tcp,
            },
            ip_filter: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_port_collision() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                dst_port: 8080,
                protocol: Protocol::Tcp,
            },
            ip_filter: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_different_ports_same_ip() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                dst_port: 8081,
                protocol: Protocol::Tcp,
            },
            ip_filter: None,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_same_port_different_ip() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 8080,
                protocol: Protocol::Tcp,
            },
            ip_filter: None,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ip_filtering_allow_list() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 80,
                protocol: Protocol::Tcp,
            },
            ip_filter: Some(IpFilterConfig {
                allow_list: Some(vec![
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
                ]),
                deny_list: None,
            }),
        };

        assert!(config.is_ip_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10))));
        assert!(config.is_ip_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))));
        assert!(!config.is_ip_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 30))));
    }

    #[test]
    fn test_ip_filtering_deny_list() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 80,
                protocol: Protocol::Tcp,
            },
            ip_filter: Some(IpFilterConfig {
                allow_list: None,
                deny_list: Some(vec![
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
                ]),
            }),
        };

        assert!(!config.is_ip_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10))));
        assert!(!config.is_ip_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))));
        assert!(config.is_ip_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 30))));
    }

    #[test]
    fn test_ip_filtering_no_filter() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 80,
                protocol: Protocol::Tcp,
            },
            ip_filter: None,
        };

        assert!(config.is_ip_allowed(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10))));
        assert!(config.is_ip_allowed(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }

    #[test]
    fn test_ip_filtering_empty_allow_list() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 80,
                protocol: Protocol::Tcp,
            },
            ip_filter: Some(IpFilterConfig {
                allow_list: Some(vec![]),
                deny_list: None,
            }),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ip_filtering_empty_deny_list() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 80,
                protocol: Protocol::Tcp,
            },
            ip_filter: Some(IpFilterConfig {
                allow_list: None,
                deny_list: Some(vec![]),
            }),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ip_filtering_both_lists() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 80,
                protocol: Protocol::Tcp,
            },
            ip_filter: Some(IpFilterConfig {
                allow_list: Some(vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10))]),
                deny_list: Some(vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))]),
            }),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ip_filtering_duplicate_ips_allow_list() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 80,
                protocol: Protocol::Tcp,
            },
            ip_filter: Some(IpFilterConfig {
                allow_list: Some(vec![
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
                ]),
                deny_list: None,
            }),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ip_filtering_duplicate_ips_deny_list() {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                listen_port: 8080,
                dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                dst_port: 80,
                protocol: Protocol::Tcp,
            },
            ip_filter: Some(IpFilterConfig {
                allow_list: None,
                deny_list: Some(vec![
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
                ]),
            }),
        };

        assert!(config.validate().is_err());
    }
}
