use axum::routing::{get, post};
use axum::Router;

use crate::handlers;
use crate::manager::InstanceManager;

pub fn build_router(manager: InstanceManager) -> Router {
    Router::new()
        .route("/dashboard", get(handlers::dashboard))
        .route("/health", get(handlers::health))
        .route("/pool/status", get(handlers::pool_status))
        .route(
            "/instances",
            post(handlers::create_instance).get(handlers::list_instances),
        )
        .route(
            "/instances/{id}",
            get(handlers::get_instance).delete(handlers::destroy_instance),
        )
        .route("/instances/{id}/input", post(handlers::send_input))
        .route("/instances/{id}/screen", get(handlers::capture_screen))
        .with_state(manager)
}
