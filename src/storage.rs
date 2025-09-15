use crate::instance::{InstanceStatus, ProxyInstance};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
/**
 * Persistent data structure for storing proxy instance configurations.
 *
 * Contains all instances along with metadata about the configuration
 * including version information and timestamps.
 */
pub struct PersistentData {
    pub instances: Vec<PersistentInstance>,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/**
 * Persistent representation of a proxy instance for storage.
 *
 * Stores the essential configuration and state information for a proxy
 * instance that can be serialized to and deserialized from storage.
 */
pub struct PersistentInstance {
    pub id: Uuid,
    pub name: String,
    pub config: crate::config::Config,
    pub status: InstanceStatus,
    pub created_at: String,
    pub started_at: Option<String>,
    pub auto_start: bool,
}
impl From<ProxyInstance> for PersistentInstance {
    fn from(instance: ProxyInstance) -> Self {
        Self {
            id: instance.id,
            name: instance.name,
            config: instance.config,
            status: instance.status,
            created_at: instance.created_at.to_rfc3339(),
            started_at: instance.started_at.map(|dt| dt.to_rfc3339()),
            auto_start: instance.auto_start,
        }
    }
}
impl TryFrom<PersistentInstance> for ProxyInstance {
    type Error = anyhow::Error;
    fn try_from(persistent: PersistentInstance) -> Result<Self> {
        let instance = Self {
            id: persistent.id,
            name: persistent.name,
            config: persistent.config,
            status: persistent.status,
            created_at: chrono::DateTime::parse_from_rfc3339(&persistent.created_at)?
                .with_timezone(&chrono::Utc),
            started_at: persistent
                .started_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()?
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            auto_start: persistent.auto_start,
            metrics: Arc::new(crate::metrics::InstanceMetrics::new()),
        };
        Ok(instance)
    }
}
/**
 * Manages persistent storage of proxy instance configurations.
 *
 * Handles loading, saving, and managing proxy instance configurations
 * in a persistent storage format with backup capabilities.
 */
pub struct StorageManager {
    config_path: PathBuf,
    data: RwLock<PersistentData>,
}
impl StorageManager {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            data: RwLock::new(PersistentData {
                instances: Vec::new(),
                version: "1.0".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            }),
        }
    }
    pub async fn load(&self) -> Result<Vec<ProxyInstance>> {
        if !self.config_path.exists() {
            info!("No existing configuration file found, starting fresh");
            return Ok(Vec::new());
        }
        debug!("Loading configuration from: {:?}", self.config_path);
        let content = fs::read_to_string(&self.config_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;
        let persistent_data: PersistentData = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?;
        let mut data = self.data.write().await;
        *data = persistent_data.clone();
        let instances: Result<Vec<ProxyInstance>> = persistent_data
            .instances
            .into_iter()
            .map(TryInto::try_into)
            .collect();
        info!(
            "Loaded {} instances from configuration",
            instances.as_ref().map_or(0, |v| v.len())
        );
        instances
    }
    pub async fn add_instance(&self, instance: &ProxyInstance) -> Result<()> {
        let mut data = self.data.write().await;
        data.instances.push(instance.clone().into());
        data.updated_at = chrono::Utc::now().to_rfc3339();
        let content = toml::to_string_pretty(&*data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize configuration: {}", e))?;
        fs::write(&self.config_path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;
        debug!("Added instance {} to configuration", instance.name);
        Ok(())
    }
    pub async fn update_instance(&self, instance: &ProxyInstance) -> Result<()> {
        let mut data = self.data.write().await;
        data.instances.retain(|i| i.id != instance.id);
        data.instances.push(instance.clone().into());
        data.updated_at = chrono::Utc::now().to_rfc3339();
        let content = toml::to_string_pretty(&*data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize configuration: {}", e))?;
        fs::write(&self.config_path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;
        debug!("Updated instance {} in configuration", instance.name);
        Ok(())
    }
    pub async fn remove_instance(&self, instance_id: Uuid) -> Result<()> {
        let mut data = self.data.write().await;
        let initial_len = data.instances.len();
        data.instances.retain(|i| i.id != instance_id);
        if data.instances.len() < initial_len {
            data.updated_at = chrono::Utc::now().to_rfc3339();
            let content = toml::to_string_pretty(&*data)
                .map_err(|e| anyhow::anyhow!("Failed to serialize configuration: {}", e))?;
            fs::write(&self.config_path, content)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;
            debug!("Removed instance {} from configuration", instance_id);
        }
        Ok(())
    }
    pub async fn export_config(&self) -> Result<String> {
        let data = self.data.read().await;
        let content = toml::to_string_pretty(&*data)
            .map_err(|e| anyhow::anyhow!("Failed to export configuration: {}", e))?;
        Ok(content)
    }
    pub async fn import_config(&self, config_content: &str) -> Result<()> {
        let persistent_data: PersistentData = toml::from_str(config_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse imported configuration: {}", e))?;
        let mut data = self.data.write().await;
        *data = persistent_data;
        data.updated_at = chrono::Utc::now().to_rfc3339();
        let content = toml::to_string_pretty(&*data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize imported configuration: {}", e))?;
        fs::write(&self.config_path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write imported configuration: {}", e))?;
        info!(
            "Imported configuration with {} instances",
            data.instances.len()
        );
        Ok(())
    }
    pub async fn get_backup_path(&self) -> PathBuf {
        let mut backup_path = self.config_path.clone();
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        backup_path.set_extension(format!("backup_{}.toml", timestamp));
        backup_path
    }
    pub async fn create_backup(&self) -> Result<PathBuf> {
        let backup_path = self.get_backup_path().await;
        let content = self.export_config().await?;
        fs::write(&backup_path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create backup: {}", e))?;
        info!("Created backup at: {:?}", backup_path);
        Ok(backup_path)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Protocol;
    use crate::instance::CreateInstanceRequest;
    use std::net::{IpAddr, Ipv4Addr};
    #[tokio::test]
    async fn test_storage_manager_save_and_load() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let storage = StorageManager::new(config_path.clone());
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
        let instance = ProxyInstance::new(
            request.name.clone(),
            request.to_config(),
            request.auto_start,
        );
        storage.add_instance(&instance).await.unwrap();
        let loaded_instances = storage.load().await.unwrap();
        assert_eq!(loaded_instances.len(), 1);
        assert_eq!(loaded_instances[0].name, instance.name);
        assert_eq!(loaded_instances[0].id, instance.id);
    }
    #[tokio::test]
    async fn test_storage_manager_export_import() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let storage = StorageManager::new(config_path.clone());
        let request1 = CreateInstanceRequest {
            name: "Instance 1".to_string(),
            listen_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 8080,
            dst_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: 80,
            protocol: Protocol::Tcp,
            auto_start: true,
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
            allow_list: Some(vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10))]),
            deny_list: None,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
            log_level: "info".to_string(),
        };
        let instance1 = ProxyInstance::new(
            request1.name.clone(),
            request1.to_config(),
            request1.auto_start,
        );
        let instance2 = ProxyInstance::new(
            request2.name.clone(),
            request2.to_config(),
            request2.auto_start,
        );
        storage.add_instance(&instance1).await.unwrap();
        storage.add_instance(&instance2).await.unwrap();
        let exported_config = storage.export_config().await.unwrap();
        let config_path2 = temp_dir.path().join("test_config2.toml");
        let storage2 = StorageManager::new(config_path2.clone());
        storage2.import_config(&exported_config).await.unwrap();
        let imported_instances = storage2.load().await.unwrap();
        assert_eq!(imported_instances.len(), 2);
        let names: Vec<String> = imported_instances.iter().map(|i| i.name.clone()).collect();
        assert!(names.contains(&"Instance 1".to_string()));
        assert!(names.contains(&"Instance 2".to_string()));
    }
    #[tokio::test]
    async fn test_storage_manager_backup() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let storage = StorageManager::new(config_path.clone());
        let request = CreateInstanceRequest {
            name: "Backup Test Instance".to_string(),
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
        let instance = ProxyInstance::new(
            request.name.clone(),
            request.to_config(),
            request.auto_start,
        );
        storage.add_instance(&instance).await.unwrap();
        let backup_path = storage.create_backup().await.unwrap();
        assert!(backup_path.exists());
        let backup_content = fs::read_to_string(&backup_path).await.unwrap();
        assert!(backup_content.contains("Backup Test Instance"));
    }
}
