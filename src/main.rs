mod config;
mod error;
mod handlers;
mod instance;
mod manager;
mod pool;
mod routes;
mod sandbox;

use config::Config;
use manager::InstanceManager;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "terminals=info".into()),
        )
        .init();

    let config = Config::from_env();
    info!(host = %config.host, port = config.port, "starting server");

    let manager = InstanceManager::new(config.clone());

    // Clean up any stale resources from previous runs
    manager.cleanup_stale();

    // Start pool replenisher
    let pool_handle = pool::spawn_pool_replenisher(manager.clone());

    // Build router
    let app = routes::build_router(manager.clone());

    // Bind listener
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("failed to bind address");
    info!(%addr, "listening");

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

    // Shutdown: abort pool task and destroy all instances
    info!("shutting down...");
    pool_handle.abort();
    manager.destroy_all().await;
    info!("shutdown complete");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
