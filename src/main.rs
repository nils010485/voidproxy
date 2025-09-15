mod buffer_pool;
mod config;
mod instance;
mod instance_manager;
mod ip_cache;
mod metrics;
mod storage;
mod tcp_proxy;
mod udp_proxy;
mod web_api;
mod web_ui;
use anyhow::Result;
use clap::Parser;
use instance_manager::InstanceService;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use web_api::create_routes as create_api_routes;
use web_ui::create_routes;
#[derive(Parser, Debug)]
#[command(
    name = "void_proxy",
    about = "High-performance TCP/UDP proxy with web management interface",
    version
)]
struct Args {
    #[arg(long, default_value = "127.0.0.1", help = "Web UI listen IP")]
    web_listen_ip: String,
    #[arg(long, default_value = "8080", help = "Web UI listen port")]
    web_listen_port: u16,
    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,
    #[arg(
        long,
        default_value = "instances.toml",
        help = "Configuration file path"
    )]
    config_path: std::path::PathBuf,
}
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(if args.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .init();
    info!("Starting VoidProxy with persistent configuration");
    info!(
        "Web UI: http://{}:{}",
        args.web_listen_ip, args.web_listen_port
    );
    info!("Config: {:?}", args.config_path);
    let storage_manager = Arc::new(storage::StorageManager::new(args.config_path.clone()));
    let instance_service = Arc::new(InstanceService::with_storage(storage_manager.clone()));

    // Load instances in background to avoid blocking startup
    let storage_manager_bg = storage_manager.clone();
    let instance_service_bg = instance_service.clone();
    tokio::spawn(async move {
        match storage_manager_bg.load().await {
            Ok(instances) => {
                let mut loaded_count = 0;
                for instance in instances {
                    if let Err(e) = instance_service_bg.restore_instance(instance).await {
                        error!("Failed to restore instance: {}", e);
                    } else {
                        loaded_count += 1;
                    }
                }
                info!("Loaded {} instances from storage", loaded_count);
            }
            Err(e) => {
                error!("Failed to load instances from storage: {}", e);
                info!("Starting with empty instance list");
            }
        }
    });
    instance_service.start_auto_instances().await?;
    let cors = CorsLayer::permissive();
    let app = axum::Router::new()
        .merge(create_routes(args.web_listen_port))
        .merge(create_api_routes(instance_service.clone()))
        .layer(ServiceBuilder::new().layer(cors));
    let addr = SocketAddr::new(args.web_listen_ip.parse()?, args.web_listen_port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Web interface listening on {}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            info!("Received terminate signal, shutting down...");
        }
    }
}
