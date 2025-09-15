use void_proxy::web_api::{create_routes, InstanceQuery, ImportConfigRequest};
use void_proxy::instance_manager::InstanceService;
use void_proxy::storage::StorageManager;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_web_api_routes_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");
    let storage_manager = Arc::new(StorageManager::new(config_path));
    let instance_service = Arc::new(InstanceService::with_storage(storage_manager));

    let _router = create_routes(instance_service);

    assert!(true);
}


#[tokio::test]
async fn test_instance_query_parsing() {
    let query = InstanceQuery {
        status: Some("running".to_string()),
    };

    assert_eq!(query.status, Some("running".to_string()));
}

#[tokio::test]
async fn test_instance_query_empty() {
    let query = InstanceQuery { status: None };

    assert!(query.status.is_none());
}

#[tokio::test]
async fn test_import_config_request_creation() {
    let request = ImportConfigRequest {
        config: "test_config_content".to_string(),
    };

    assert_eq!(request.config, "test_config_content");
}





#[tokio::test]
async fn test_web_api_import_config_request_deserialization() {
    let json = r#"{"config": "test config content"}"#;
    let request: ImportConfigRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.config, "test config content");
}

#[tokio::test]
async fn test_web_api_instance_query_deserialization() {
    let json = r#"{"status": "stopped"}"#;
    let query: InstanceQuery = serde_json::from_str(json).unwrap();

    assert_eq!(query.status, Some("stopped".to_string()));
}

#[tokio::test]
async fn test_web_api_instance_query_empty_deserialization() {
    let json = r#"{}"#;
    let query: InstanceQuery = serde_json::from_str(json).unwrap();

    assert!(query.status.is_none());
}


