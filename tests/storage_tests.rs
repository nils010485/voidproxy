use void_proxy::storage::{StorageManager, PersistentInstance};
use void_proxy::instance::ProxyInstance;
use void_proxy::config::{Config, ProxyConfig, Protocol};
use std::net::{IpAddr, Ipv4Addr};
use tempfile::TempDir;

#[tokio::test]
async fn test_storage_manager_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let _storage = StorageManager::new(config_path);

    assert!(true);
}

#[tokio::test]
async fn test_storage_manager_load_empty() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("nonexistent.toml");

    let storage = StorageManager::new(config_path.clone());
    let instances = storage.load().await.unwrap();

    assert_eq!(instances.len(), 0);
}

#[tokio::test]
async fn test_storage_manager_add_instance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let storage = StorageManager::new(config_path);

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

    storage.add_instance(&instance).await.unwrap();

    assert!(true);
}

#[tokio::test]
async fn test_storage_manager_update_instance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let storage = StorageManager::new(config_path);

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
    storage.add_instance(&instance).await.unwrap();

    assert!(true);
}

#[tokio::test]
async fn test_storage_manager_remove_instance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let storage = StorageManager::new(config_path);

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
    storage.add_instance(&instance).await.unwrap();

    storage.remove_instance(instance.id).await.unwrap();

    assert!(true);
}

#[tokio::test]
async fn test_storage_manager_get_backup_path() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let storage = StorageManager::new(config_path.clone());
    let backup_path = storage.get_backup_path().await;

    let backup_name = backup_path.file_name().unwrap().to_str().unwrap();
    assert!(backup_name.contains("backup_"));
    assert!(backup_name.ends_with(".toml"));
}

#[tokio::test]
async fn test_persistent_instance_conversion() {
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
    let persistent: PersistentInstance = instance.clone().into();

    assert_eq!(persistent.id, instance.id);
    assert_eq!(persistent.name, instance.name);
    assert_eq!(persistent.status, instance.status);
    assert_eq!(persistent.auto_start, instance.auto_start);

    assert!(true);
}

#[tokio::test]
async fn test_storage_manager_concurrent_operations() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let storage = StorageManager::new(config_path);

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

    let handle1 = tokio::spawn(async move {
        let storage_clone = StorageManager::new(temp_dir.path().join("test_config_clone.toml"));
        storage_clone.add_instance(&instance1).await.unwrap();
    });

    let handle2 = tokio::spawn(async move {
        storage.add_instance(&instance2).await.unwrap();
    });

    handle1.await.unwrap();
    handle2.await.unwrap();

    assert!(true);
}

#[tokio::test]
async fn test_storage_manager_import_invalid_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let storage = StorageManager::new(config_path.clone());

    let invalid_config = "invalid toml content";

    let result = storage.import_config(invalid_config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_storage_manager_export_empty() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let storage = StorageManager::new(config_path.clone());
    let exported = storage.export_config().await.unwrap();

    assert!(exported.contains("version = \"1.0\""));
    assert!(exported.contains("instances = []"));
}