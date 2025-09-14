use crate::instance::{CreateInstanceRequestStrings, UpdateInstanceRequest};
use crate::instance_manager::InstanceService;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl ErrorResponse {
    fn new(error: String, message: String) -> Self {
        Self { error, message }
    }
}

pub fn create_routes(instance_service: Arc<InstanceService>) -> Router {
    Router::new()
        .route("/api/instances", get(get_instances).post(create_instance))
        .route(
            "/api/instances/:id",
            get(get_instance)
                .put(update_instance)
                .delete(delete_instance),
        )
        .route("/api/instances/:id/start", post(start_instance))
        .route("/api/instances/:id/stop", post(stop_instance))
        .route("/api/instances/:id/stats", get(get_instance_stats))
        .route("/api/stats", get(get_all_stats))
        .route("/api/config/export", get(export_config))
        .route("/api/config/import", post(import_config))
        .route("/api/config/backup", post(create_backup))
        .route("/api/performance", get(get_performance_metrics))
        .route(
            "/api/instances/:id/session-metrics",
            get(get_instance_session_metrics),
        )
        .with_state(instance_service)
}

#[derive(Deserialize, Debug)]
pub struct InstanceQuery {
    pub status: Option<String>,
}

async fn get_instances(
    State(service): State<Arc<InstanceService>>,
    Query(params): Query<InstanceQuery>,
) -> Result<Json<Vec<crate::instance::ProxyInstance>>, StatusCode> {
    debug!("Getting instances with query: {:?}", params);

    let instances = service.get_instances().await;

    let filtered_instances = if let Some(status_filter) = &params.status {
        instances
            .into_iter()
            .filter(|instance| {
                format!("{:?}", instance.status).to_lowercase() == status_filter.to_lowercase()
            })
            .collect()
    } else {
        instances
    };

    Ok(Json(filtered_instances))
}

async fn get_instance(
    State(service): State<Arc<InstanceService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::instance::ProxyInstance>, StatusCode> {
    debug!("Getting instance: {}", id);

    match service.get_instance(id).await {
        Some(instance) => Ok(Json(instance)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn create_instance(
    State(service): State<Arc<InstanceService>>,
    Json(request): Json<CreateInstanceRequestStrings>,
) -> Result<Json<crate::instance::ProxyInstance>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Creating instance: {}", request.name);

    match request.to_typed() {
        Ok(typed_request) => match service.create_instance(typed_request).await {
            Ok(instance) => {
                info!("Created instance: {}", instance.name);
                Ok(Json(instance))
            }
            Err(e) => {
                error!("Failed to create instance: {}", e);
                let error_response =
                    ErrorResponse::new("CREATION_ERROR".to_string(), e.to_string());
                Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
            }
        },
        Err(e) => {
            error!("Invalid request data: {}", e);
            let error_response = ErrorResponse::new("VALIDATION_ERROR".to_string(), e);
            Err((StatusCode::BAD_REQUEST, Json(error_response)))
        }
    }
}

async fn update_instance(
    State(service): State<Arc<InstanceService>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateInstanceRequest>,
) -> Result<Json<crate::instance::ProxyInstance>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Updating instance: {}", id);

    match service.update_instance(id, request).await {
        Ok(Some(instance)) => {
            info!("Updated instance: {}", instance.name);
            Ok(Json(instance))
        }
        Ok(None) => {
            let error_response = ErrorResponse::new(
                "NOT_FOUND".to_string(),
                format!("Instance with ID {} not found", id),
            );
            Err((StatusCode::NOT_FOUND, Json(error_response)))
        }
        Err(e) => {
            error!("Failed to update instance {}: {}", id, e);
            let error_response = ErrorResponse::new("VALIDATION_ERROR".to_string(), e.to_string());
            Err((StatusCode::BAD_REQUEST, Json(error_response)))
        }
    }
}

async fn delete_instance(
    State(service): State<Arc<InstanceService>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    debug!("Deleting instance: {}", id);

    match service.delete_instance(id).await {
        Ok(true) => {
            info!("Deleted instance: {}", id);
            Ok(StatusCode::NO_CONTENT)
        }
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to delete instance {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn start_instance(
    State(service): State<Arc<InstanceService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::instance::ProxyInstance>, StatusCode> {
    debug!("Starting instance: {}", id);

    match service.start_instance(id).await {
        Ok(true) => {
            if let Some(instance) = service.get_instance(id).await {
                info!("Started instance: {}", instance.name);
                Ok(Json(instance))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to start instance {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn stop_instance(
    State(service): State<Arc<InstanceService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::instance::ProxyInstance>, StatusCode> {
    debug!("Stopping instance: {}", id);

    match service.stop_instance(id).await {
        Ok(true) => {
            if let Some(instance) = service.get_instance(id).await {
                info!("Stopped instance: {}", instance.name);
                Ok(Json(instance))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to stop instance {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_instance_stats(
    State(service): State<Arc<InstanceService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::instance_manager::InstanceStats>, StatusCode> {
    debug!("Getting stats for instance: {}", id);

    let stats = service.get_instance_stats().await;
    match stats.get(&id) {
        Some(stats) => Ok(Json(stats.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_all_stats(
    State(service): State<Arc<InstanceService>>,
) -> Json<std::collections::HashMap<Uuid, crate::instance_manager::InstanceStats>> {
    debug!("Getting all instance stats");

    let stats = service.get_instance_stats().await;
    Json(stats)
}

#[derive(Deserialize)]
pub struct ImportConfigRequest {
    pub config: String,
}

async fn export_config(
    State(service): State<Arc<InstanceService>>,
) -> Result<Json<ExportConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Exporting configuration");

    match service.export_config().await {
        Ok(config) => Ok(Json(ExportConfigResponse { config })),
        Err(e) => {
            error!("Failed to export configuration: {}", e);
            let error_response = ErrorResponse::new("EXPORT_ERROR".to_string(), e.to_string());
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

async fn import_config(
    State(service): State<Arc<InstanceService>>,
    Json(request): Json<ImportConfigRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("Importing configuration");

    match service.import_config(&request.config).await {
        Ok(_) => {
            info!("Configuration imported successfully");
            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!("Failed to import configuration: {}", e);
            let error_response = ErrorResponse::new("IMPORT_ERROR".to_string(), e.to_string());
            Err((StatusCode::BAD_REQUEST, Json(error_response)))
        }
    }
}

async fn create_backup(
    State(service): State<Arc<InstanceService>>,
) -> Result<Json<BackupResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Creating backup");

    match service.create_backup().await {
        Ok(backup_path) => {
            info!("Backup created: {:?}", backup_path);
            Ok(Json(BackupResponse {
                backup_path: backup_path.to_string_lossy().to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to create backup: {}", e);
            let error_response = ErrorResponse::new("BACKUP_ERROR".to_string(), e.to_string());
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

#[derive(Serialize)]
struct ExportConfigResponse {
    pub config: String,
}

#[derive(Serialize)]
struct BackupResponse {
    pub backup_path: String,
}

async fn get_performance_metrics(
    State(service): State<Arc<InstanceService>>,
) -> Json<crate::instance_manager::PerformanceMetrics> {
    debug!("Getting performance metrics");

    let metrics = service.get_performance_metrics().await;
    Json(metrics)
}

async fn get_instance_session_metrics(
    State(service): State<Arc<InstanceService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::metrics::SessionMetrics>, StatusCode> {
    debug!("Getting session metrics for instance: {}", id);

    match service.get_instance_session_metrics(&id).await {
        Some(metrics) => Ok(Json(metrics)),
        None => Err(StatusCode::NOT_FOUND),
    }
}
