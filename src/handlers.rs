use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::instance::InstanceInfo;
use crate::manager::InstanceManager;

// --- Request/Response types ---

#[derive(Deserialize)]
pub struct InputRequest {
    pub text: Option<String>,
    pub keys: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct ExecRequest {
    pub command: String,
    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
}

fn default_delay_ms() -> u64 {
    500
}

#[derive(Serialize)]
pub struct ScreenResponse {
    pub content: String,
}

#[derive(Serialize)]
pub struct PoolStatus {
    pub warm: usize,
    pub total: usize,
    pub target_size: usize,
    pub max_size: usize,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

// --- Handlers ---

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn pool_status(State(mgr): State<InstanceManager>) -> Json<PoolStatus> {
    Json(PoolStatus {
        warm: mgr.warm_count(),
        total: mgr.total_count(),
        target_size: mgr.config.pool_target_size,
        max_size: mgr.config.pool_max_size,
    })
}

pub async fn create_instance(
    State(mgr): State<InstanceManager>,
) -> Result<Json<InstanceInfo>, AppError> {
    let info = mgr.acquire_instance().await?;
    Ok(Json(info))
}

pub async fn list_instances(State(mgr): State<InstanceManager>) -> Json<Vec<InstanceInfo>> {
    Json(mgr.list_instances())
}

pub async fn get_instance(
    State(mgr): State<InstanceManager>,
    Path(id): Path<Uuid>,
) -> Result<Json<InstanceInfo>, AppError> {
    Ok(Json(mgr.get_instance(id)?))
}

pub async fn destroy_instance(
    State(mgr): State<InstanceManager>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    mgr.destroy_instance(id).await?;
    Ok(Json(serde_json::json!({ "status": "destroyed" })))
}

pub async fn send_input(
    State(mgr): State<InstanceManager>,
    Path(id): Path<Uuid>,
    Json(req): Json<InputRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let has_text = req.text.is_some();
    let has_keys = req.keys.is_some();
    if let Some(text) = &req.text {
        mgr.send_input(id, text).await?;
    }
    if let Some(keys) = req.keys {
        mgr.send_keys_raw(id, keys).await?;
    }
    if !has_text && !has_keys {
        return Err(AppError::BadRequest(
            "must provide 'text' or 'keys'".into(),
        ));
    }
    Ok(Json(serde_json::json!({ "status": "sent" })))
}

pub async fn capture_screen(
    State(mgr): State<InstanceManager>,
    Path(id): Path<Uuid>,
) -> Result<Json<ScreenResponse>, AppError> {
    let content = mgr.capture_screen(id).await?;
    Ok(Json(ScreenResponse { content }))
}

pub async fn exec_command(
    State(mgr): State<InstanceManager>,
    Path(id): Path<Uuid>,
    Json(req): Json<ExecRequest>,
) -> Result<Json<ScreenResponse>, AppError> {
    mgr.send_input(id, &req.command).await?;
    tokio::time::sleep(std::time::Duration::from_millis(req.delay_ms)).await;
    let content = mgr.capture_screen(id).await?;
    Ok(Json(ScreenResponse { content }))
}

pub async fn start_ttyd(
    State(mgr): State<InstanceManager>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let port = mgr.start_ttyd(id).await?;
    Ok(Json(serde_json::json!({ "port": port })))
}

pub async fn stop_ttyd(
    State(mgr): State<InstanceManager>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    mgr.stop_ttyd(id)?;
    Ok(Json(serde_json::json!({ "status": "stopped" })))
}
