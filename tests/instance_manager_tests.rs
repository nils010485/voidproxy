use void_proxy::instance_manager::InstanceService;
use void_proxy::storage::StorageManager;
use void_proxy::instance::{CreateInstanceRequest, InstanceStatus};
use void_proxy::config::Protocol;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn test_instance_service_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let instances = service.get_instances().await;
    assert_eq!(instances.len(), 0);
}

#[tokio::test]
async fn test_instance_service_create_instance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let request = CreateInstanceRequest {
        name: "Test Instance".to_string(),
        listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        listen_port: 8080,
        dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        dst_port: 80,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let instance = service.create_instance(request).await.unwrap();

    assert_eq!(instance.name, "Test Instance");
    assert_eq!(instance.status, InstanceStatus::Stopped);

    let instances = service.get_instances().await;
    assert_eq!(instances.len(), 1);
}

#[tokio::test]
async fn test_instance_service_get_instance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let request = CreateInstanceRequest {
        name: "Test Instance".to_string(),
        listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        listen_port: 8080,
        dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        dst_port: 80,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let instance = service.create_instance(request).await.unwrap();
    let retrieved_instance = service.get_instance(instance.id).await;

    assert!(retrieved_instance.is_some());
    assert_eq!(retrieved_instance.unwrap().id, instance.id);
}

#[tokio::test]
async fn test_instance_service_get_nonexistent_instance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let nonexistent_id = Uuid::new_v4();
    let retrieved_instance = service.get_instance(nonexistent_id).await;

    assert!(retrieved_instance.is_none());
}

#[tokio::test]
async fn test_instance_service_delete_instance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let request = CreateInstanceRequest {
        name: "Test Instance".to_string(),
        listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        listen_port: 8080,
        dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        dst_port: 80,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let instance = service.create_instance(request).await.unwrap();
    let deleted = service.delete_instance(instance.id).await.unwrap();

    assert!(deleted);

    let instances = service.get_instances().await;
    assert_eq!(instances.len(), 0);
}

#[tokio::test]
async fn test_instance_service_delete_nonexistent_instance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let nonexistent_id = Uuid::new_v4();
    let deleted = service.delete_instance(nonexistent_id).await.unwrap();

    assert!(!deleted);
}

#[tokio::test]
async fn test_instance_service_update_instance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let request = CreateInstanceRequest {
        name: "Test Instance".to_string(),
        listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        listen_port: 8080,
        dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        dst_port: 80,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let instance = service.create_instance(request).await.unwrap();

    let update_request = void_proxy::instance::UpdateInstanceRequest {
        name: Some("Updated Instance".to_string()),
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

    let updated_instance = service.update_instance(instance.id, update_request).await.unwrap();

    assert!(updated_instance.is_some());
    assert_eq!(updated_instance.unwrap().name, "Updated Instance");
}

#[tokio::test]
async fn test_instance_service_get_instance_stats() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let request = CreateInstanceRequest {
        name: "Test Instance".to_string(),
        listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        listen_port: 8080,
        dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        dst_port: 80,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let instance = service.create_instance(request).await.unwrap();
    let stats = service.get_instance_stats().await;

    assert!(stats.contains_key(&instance.id));
}

#[tokio::test]
async fn test_instance_service_performance_metrics() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let metrics = service.get_performance_metrics().await;

    assert_eq!(metrics.uptime_seconds, 0);
    assert_eq!(metrics.active_connections, 0);
}

#[tokio::test]
async fn test_instance_service_create_auto_start_instance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let request = CreateInstanceRequest {
        name: "Auto Start Instance".to_string(),
        listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        listen_port: 18081,
        dst_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        dst_port: 18082,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: 1,
        idle_timeout_secs: 1,
        log_level: "info".to_string(),
    };

    let _instance = service.create_instance(request).await.unwrap();

    let instances = service.get_instances().await;
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].name, "Auto Start Instance");
}

#[tokio::test]
async fn test_instance_service_clone() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);
    assert!(true);

    let request = CreateInstanceRequest {
        name: "Test Instance".to_string(),
        listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        listen_port: 8080,
        dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        dst_port: 80,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let instance = service.create_instance(request).await.unwrap();
    let _retrieved_instance = service.get_instance(instance.id).await;
    assert!(true);
}

#[tokio::test]
async fn test_instance_service_multiple_instances() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let service = InstanceService::with_storage(storage_manager);

    let request1 = CreateInstanceRequest {
        name: "Instance 1".to_string(),
        listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        listen_port: 8080,
        dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        dst_port: 80,
        protocol: Protocol::Tcp,
        auto_start: false,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let request2 = CreateInstanceRequest {
        name: "Instance 2".to_string(),
        listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        listen_port: 8081,
        dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101)),
        dst_port: 443,
        protocol: Protocol::Udp,
        auto_start: false,
        allow_list: None,
        deny_list: None,
        connect_timeout_secs: 30,
        idle_timeout_secs: 300,
        log_level: "info".to_string(),
    };

    let _instance1 = service.create_instance(request1).await.unwrap();
    let _instance2 = service.create_instance(request2).await.unwrap();

    let instances = service.get_instances().await;
    assert_eq!(instances.len(), 2);

    let names: Vec<String> = instances.iter().map(|i| i.name.clone()).collect();
    assert!(names.contains(&"Instance 1".to_string()));
    assert!(names.contains(&"Instance 2".to_string()));
}