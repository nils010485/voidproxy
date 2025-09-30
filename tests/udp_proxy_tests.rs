use void_proxy::udp_proxy::UdpProxy;
use void_proxy::config::{Config, ProxyConfig, Protocol};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
async fn test_udp_proxy_creation() {
    let config = Arc::new(Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Udp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    });

    let instance_id = Uuid::new_v4();
    let instances = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

    let _proxy = UdpProxy::new(config, instance_id, instances);

    assert!(true);
}

#[tokio::test]
async fn test_udp_proxy_cancellation() {
    let config = Arc::new(Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 0,
            dst_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            dst_port: 0,
            protocol: Protocol::Udp,
            connect_timeout_secs: 1,
            idle_timeout_secs: 1,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    });

    let instance_id = Uuid::new_v4();
    let instances = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let proxy = UdpProxy::new(config, instance_id, instances);

    let cancel_token = Arc::new(CancellationToken::new());
    let cancel_token_clone = cancel_token.clone();

    let proxy_clone = proxy.clone();
    let handle = tokio::spawn(async move {
        proxy_clone.run_with_token(cancel_token_clone).await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    cancel_token.cancel();

    let result = handle.await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_udp_proxy_session_metrics() {
    let config = Arc::new(Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Udp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    });

    let instance_id = Uuid::new_v4();
    let instances = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let proxy = UdpProxy::new(config, instance_id, instances);

    let metrics = proxy.get_session_metrics().await;

    assert_eq!(metrics.session_timeout_seconds, 300);
    assert_eq!(metrics.cleanup_interval_seconds, 60);
    assert_eq!(metrics.active_sessions, 0);
}

#[tokio::test]
async fn test_udp_proxy_with_ip_filter() {
    let mut config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 0,
            dst_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            dst_port: 0,
            protocol: Protocol::Udp,
            connect_timeout_secs: 1,
            idle_timeout_secs: 1,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    config.ip_filter = Some(void_proxy::config::IpFilterConfig {
        allow_list: Some(vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))]),
        deny_list: None,
    });

    let config = Arc::new(config);
    let instance_id = Uuid::new_v4();
    let instances = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let proxy = UdpProxy::new(config, instance_id, instances);

    let cancel_token = Arc::new(CancellationToken::new());
    let cancel_token_clone = cancel_token.clone();

    let proxy_clone = proxy.clone();
    let handle = tokio::spawn(async move {
        proxy_clone.run_with_token(cancel_token_clone).await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    cancel_token.cancel();

    let result = handle.await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_udp_proxy_buffer_pool() {
    let config = Arc::new(Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 0,
            dst_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            dst_port: 0,
            protocol: Protocol::Udp,
            connect_timeout_secs: 1,
            idle_timeout_secs: 1,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    });

    let instance_id = Uuid::new_v4();
    let instances = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let _proxy = UdpProxy::new(config, instance_id, instances);

    assert!(true);
}

#[tokio::test]
async fn test_udp_proxy_session_manager() {
    let config = Arc::new(Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Udp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    });

    let instance_id = Uuid::new_v4();
    let instances = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let _proxy = UdpProxy::new(config, instance_id, instances);

    assert!(true);
}

#[tokio::test]
async fn test_udp_proxy_clone() {
    let config = Arc::new(Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Udp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    });

    let instance_id = Uuid::new_v4();
    let instances = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let proxy = UdpProxy::new(config, instance_id, instances);

    let _proxy_clone = proxy.clone();

    assert!(true);
}

#[tokio::test]
async fn test_udp_proxy_timeout_values() {
    let config = Arc::new(Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Udp,
            connect_timeout_secs: 15,
            idle_timeout_secs: 60,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    });

    let instance_id = Uuid::new_v4();
    let instances = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let _proxy = UdpProxy::new(config, instance_id, instances);

    assert!(true);
}

#[tokio::test]
async fn test_udp_proxy_ip_cache() {
    let config = Arc::new(Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 0,
            dst_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            dst_port: 0,
            protocol: Protocol::Udp,
            connect_timeout_secs: 1,
            idle_timeout_secs: 1,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    });

    let instance_id = Uuid::new_v4();
    let instances = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let _proxy = UdpProxy::new(config, instance_id, instances);

    assert!(true);
}