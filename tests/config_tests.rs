use void_proxy::config::{Config, ProxyConfig, Protocol};

#[tokio::test]
async fn test_config_creation() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: "127.0.0.1".parse().unwrap(),
            listen_port: 8080,
            dst_ip: "127.0.0.1".parse().unwrap(),
            dst_port: 8081,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    assert_eq!(config.proxy.listen_port, 8080);
    assert_eq!(config.proxy.dst_port, 8081);
    assert_eq!(config.proxy.protocol, Protocol::Tcp);
    assert_eq!(config.proxy.connect_timeout_secs, 30);
    assert_eq!(config.proxy.idle_timeout_secs, 300);
    assert_eq!(config.proxy.log_level, "info");
}

#[tokio::test]
async fn test_config_validation() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: "127.0.0.1".parse().unwrap(),
            listen_port: 8080,
            dst_ip: "127.0.0.1".parse().unwrap(),
            dst_port: 8081,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    // Test validation - this should not panic
    let result = config.validate();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_config_with_timeouts() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: "127.0.0.1".parse().unwrap(),
            listen_port: 8080,
            dst_ip: "127.0.0.1".parse().unwrap(),
            dst_port: 8081,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 10,
            idle_timeout_secs: 60,
            log_level: "debug".to_string(),
        },
        ip_filter: None,
    };

    assert_eq!(config.proxy.connect_timeout_secs, 10);
    assert_eq!(config.proxy.idle_timeout_secs, 60);
    assert_eq!(config.proxy.log_level, "debug");
}

#[tokio::test]
async fn test_config_log_levels() {
    let log_levels = vec!["error", "warn", "info", "debug", "trace"];

    for level in log_levels {
        let config = Config {
            proxy: ProxyConfig {
                listen_ip: "127.0.0.1".parse().unwrap(),
                listen_port: 8080,
                dst_ip: "127.0.0.1".parse().unwrap(),
                dst_port: 8081,
                protocol: Protocol::Tcp,
                connect_timeout_secs: 30,
                idle_timeout_secs: 300,
                log_level: level.to_string(),
            },
            ip_filter: None,
        };

        assert_eq!(config.proxy.log_level, level);
        // Test that validation passes for valid log levels
        let result = config.validate();
        assert!(result.is_ok(), "Log level {} should be valid", level);
    }
}

#[tokio::test]
async fn test_config_timeout_bounds() {
    let config = Config {
        proxy: ProxyConfig {
            listen_ip: "127.0.0.1".parse().unwrap(),
            listen_port: 8080,
            dst_ip: "127.0.0.1".parse().unwrap(),
            dst_port: 8081,
            protocol: Protocol::Tcp,
            connect_timeout_secs: 1,  // Minimum value
            idle_timeout_secs: 3600, // Maximum value
            log_level: "info".to_string(),
        },
        ip_filter: None,
    };

    assert_eq!(config.proxy.connect_timeout_secs, 1);
    assert_eq!(config.proxy.idle_timeout_secs, 3600);
}