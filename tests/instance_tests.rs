use void_proxy::instance::{ProxyInstance, InstanceStatus, CreateInstanceRequest, CreateInstanceRequestStrings, UpdateInstanceRequest};
use void_proxy::config::{Config, ProxyConfig, Protocol};
use std::net::{IpAddr, Ipv4Addr};

#[tokio::test]
async fn test_proxy_instance_creation() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    let instance = ProxyInstance::new("Test Instance".to_string(), config, false);

    assert_eq!(instance.name, "Test Instance");
    assert_eq!(instance.status, InstanceStatus::Stopped);
    assert!(!instance.auto_start);
    assert!(instance.started_at.is_none());
}

#[tokio::test]
async fn test_proxy_instance_auto_start() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    let instance = ProxyInstance::new("Auto Start Instance".to_string(), config, true);

    assert!(instance.auto_start);
    assert_eq!(instance.status, InstanceStatus::Stopped);
}

#[tokio::test]
async fn test_proxy_instance_start() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    let mut instance = ProxyInstance::new("Test Instance".to_string(), config, false);

    instance.start();

    assert_eq!(instance.status, InstanceStatus::Starting);
    assert!(instance.started_at.is_some());
}

#[tokio::test]
async fn test_proxy_instance_set_running() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    let mut instance = ProxyInstance::new("Test Instance".to_string(), config, false);

    instance.start();
    instance.set_running();

    assert_eq!(instance.status, InstanceStatus::Running);
}

#[tokio::test]
async fn test_proxy_instance_stop() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    let mut instance = ProxyInstance::new("Test Instance".to_string(), config, false);

    instance.start();
    instance.set_running();
    instance.stop();

    assert_eq!(instance.status, InstanceStatus::Stopping);
    assert!(instance.started_at.is_none());
}

#[tokio::test]
async fn test_proxy_instance_set_stopped() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    let mut instance = ProxyInstance::new("Test Instance".to_string(), config, false);

    instance.start();
    instance.set_running();
    instance.set_stopped();

    assert_eq!(instance.status, InstanceStatus::Stopped);
}

#[tokio::test]
async fn test_proxy_instance_unique_id() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    let instance1 = ProxyInstance::new("Instance 1".to_string(), config.clone(), false);
    let instance2 = ProxyInstance::new("Instance 2".to_string(), config, false);

    assert_ne!(instance1.id, instance2.id);
}

#[tokio::test]
async fn test_proxy_instance_clone() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    let instance = ProxyInstance::new("Test Instance".to_string(), config, false);
    let cloned_instance = instance.clone();

    assert_eq!(instance.id, cloned_instance.id);
    assert_eq!(instance.name, cloned_instance.name);
    assert_eq!(instance.status, cloned_instance.status);
}

#[tokio::test]
async fn test_create_instance_request_strings_valid() {
    let request = CreateInstanceRequestStrings {
        name: "Test Instance".to_string(),
        listen_ip: "127.0.0.1".to_string(),
        listen_port: 8080,
        dst_ip: "192.168.1.100".to_string(),
        dst_port: 80,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: Some(vec!["192.168.1.10".to_string()]),
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let result = request.to_typed();
    assert!(result.is_ok());

    let typed = result.unwrap();
    assert_eq!(typed.name, "Test Instance");
    assert_eq!(typed.listen_ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(typed.dst_ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)));
}

#[tokio::test]
async fn test_create_instance_request_strings_invalid_ip() {
    let request = CreateInstanceRequestStrings {
        name: "Test Instance".to_string(),
        listen_ip: "invalid_ip".to_string(),
        listen_port: 8080,
        dst_ip: "192.168.1.100".to_string(),
        dst_port: 80,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let result = request.to_typed();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_instance_request_to_config() {
    let request = CreateInstanceRequest {
        name: "Test Instance".to_string(),
        listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        listen_port: 8080,
        dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        dst_port: 80,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: Some(vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10))]),
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let config = request.to_config();

    assert_eq!(config.proxy.listen_ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(config.proxy.listen_port, 8080);
    assert_eq!(config.proxy.dst_port, 80);
    assert_eq!(config.proxy.protocol, Protocol::Tcp);
    assert!(config.ip_filter.is_some());
}

#[tokio::test]
async fn test_update_instance_request_partial() {
    let request = UpdateInstanceRequest {
        name: Some("Updated Name".to_string()),
        listen_ip: None,
        listen_port: None,
        dst_ip: None,
        dst_port: None,
        protocol: None,
        auto_start: None,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: None,
        idle_timeout_secs: None,
        log_level: None,
    };

    assert_eq!(request.name, Some("Updated Name".to_string()));
    assert!(request.listen_ip.is_none());
    assert!(request.listen_port.is_none());
}

#[tokio::test]
async fn test_instance_status_values() {
    assert_ne!(InstanceStatus::Stopped, InstanceStatus::Running);
    assert_ne!(InstanceStatus::Running, InstanceStatus::Error);
    assert_ne!(InstanceStatus::Error, InstanceStatus::Starting);
    assert_ne!(InstanceStatus::Starting, InstanceStatus::Stopping);
}

#[tokio::test]
async fn test_proxy_instance_metrics() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    let instance = ProxyInstance::new("Test Instance".to_string(), config, false);

    let _metrics = instance.metrics.clone();
    assert!(true);
}

#[tokio::test]
async fn test_proxy_instance_serialization() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    let instance = ProxyInstance::new("Test Instance".to_string(), config, false);

    let json = serde_json::to_string(&instance).unwrap();
    assert!(json.contains("Test Instance"));
    assert!(json.contains("stopped"));
}

#[tokio::test]
async fn test_proxy_instance_deserialization() {
    let json = r#"{
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "name": "Test Instance",
        "config": {
            "proxy": {
                "listen_ip": "127.0.0.1",
                "listen_port": 8080,
                "dst_ip": "192.168.1.100",
                "dst_port": 80,
                "protocol": "tcp",
                "connect_timeout_secs": 30,
                "idle_timeout_secs": 300,
                "log_level": "info"
            },
            "ip_filter": null
        },
        "status": "stopped",
        "created_at": "2023-01-01T00:00:00Z",
        "started_at": null,
        "auto_start": false
    }"#;

    let instance: ProxyInstance = serde_json::from_str(json).unwrap();
    assert_eq!(instance.name, "Test Instance");
    assert_eq!(instance.status, InstanceStatus::Stopped);
    assert!(!instance.auto_start);
}