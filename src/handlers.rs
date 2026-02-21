use axum::extract::{Path, State};
use axum::response::Html;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::error;
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
    let info = mgr.acquire_instance().await.map_err(|e| {
        error!(%e, "failed to create instance");
        e
    })?;
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
    mgr.destroy_instance(id).await.map_err(|e| {
        error!(%id, %e, "failed to destroy instance");
        e
    })?;
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
        mgr.send_input(id, text).await.map_err(|e| {
            error!(%id, %text, %e, "failed to send input");
            e
        })?;
    }
    if let Some(keys) = req.keys {
        mgr.send_keys_raw(id, keys.clone()).await.map_err(|e| {
            error!(%id, ?keys, %e, "failed to send raw keys");
            e
        })?;
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
    let content = mgr.capture_screen(id).await.map_err(|e| {
        error!(%id, %e, "failed to capture screen");
        e
    })?;
    Ok(Json(ScreenResponse { content }))
}



pub async fn dashboard(
    State(mgr): State<InstanceManager>,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let instances = mgr.list_instances();
    let base_host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.split(':').next())
        .unwrap_or("localhost");

    let mut rows = String::new();
    for inst in &instances {
        let status_class = match inst.status {
            crate::instance::InstanceStatus::Warm => "warm",
            crate::instance::InstanceStatus::Claimed => "claimed",
            crate::instance::InstanceStatus::Destroying => "destroying",
        };
        let ttyd_cell = match inst.ttyd_port {
            Some(port) => format!(
                r#"<a href="http://{}:{}" target="_blank">port {}</a>"#,
                base_host, port, port
            ),
            None => "not running".to_string(),
        };
        rows.push_str(&format!(
            r#"<tr>
                <td><code>{}</code></td>
                <td><span class="status {}">{:?}</span></td>
                <td>{}</td>
                <td>{}s</td>
                <td><code>{}</code></td>
            </tr>"#,
            inst.id, status_class, inst.status, ttyd_cell, inst.uptime_secs, inst.workspace,
        ));
    }

    let pool = mgr.warm_count();
    let total = mgr.total_count();

    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Terminals Dashboard</title>
<meta http-equiv="refresh" content="5">
<style>
  body {{ font-family: system-ui, sans-serif; margin: 2rem; background: #1a1a2e; color: #e0e0e0; }}
  h1 {{ color: #fff; }}
  .stats {{ margin-bottom: 1.5rem; color: #aaa; }}
  .stats span {{ color: #fff; font-weight: bold; }}
  table {{ border-collapse: collapse; width: 100%; }}
  th, td {{ padding: 0.6rem 1rem; text-align: left; border-bottom: 1px solid #333; }}
  th {{ color: #888; font-weight: 600; font-size: 0.85rem; text-transform: uppercase; }}
  code {{ background: #2a2a3e; padding: 0.15rem 0.4rem; border-radius: 3px; font-size: 0.85rem; }}
  a {{ color: #6ec6ff; text-decoration: none; }}
  a:hover {{ text-decoration: underline; }}
  .status {{ padding: 0.2rem 0.6rem; border-radius: 4px; font-size: 0.85rem; font-weight: 600; }}
  .warm {{ background: #1b5e20; color: #a5d6a7; }}
  .claimed {{ background: #0d47a1; color: #90caf9; }}
  .destroying {{ background: #b71c1c; color: #ef9a9a; }}
  .empty {{ color: #666; text-align: center; padding: 2rem; }}
</style>
</head>
<body>
<h1>Terminals Dashboard</h1>
<div class="stats">
  <span>{total}</span> instances (<span>{pool}</span> warm) &middot; auto-refreshes every 5s
</div>
{body}
</body>
</html>"#,
        total = total,
        pool = pool,
        body = if instances.is_empty() {
            r#"<p class="empty">No instances running.</p>"#.to_string()
        } else {
            format!(
                r#"<table>
<tr><th>Instance</th><th>Status</th><th>ttyd</th><th>Uptime</th><th>Workspace</th></tr>
{rows}
</table>"#,
                rows = rows
            )
        }
    ))
}
