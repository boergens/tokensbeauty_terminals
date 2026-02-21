use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Sandbox(SandboxError),
    Internal(String),
}

#[derive(Debug)]
pub enum SandboxError {
    TmuxFailed(String),
    WorkspaceCreation(String),
    BwrapFailed(String),
}

impl fmt::Display for SandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxError::TmuxFailed(msg) => write!(f, "tmux error: {}", msg),
            SandboxError::WorkspaceCreation(msg) => write!(f, "workspace error: {}", msg),
            SandboxError::BwrapFailed(msg) => write!(f, "bwrap error: {}", msg),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "not found: {}", msg),
            AppError::BadRequest(msg) => write!(f, "bad request: {}", msg),
            AppError::Sandbox(err) => write!(f, "sandbox: {}", err),
            AppError::Internal(msg) => write!(f, "internal: {}", msg),
        }
    }
}

impl From<SandboxError> for AppError {
    fn from(err: SandboxError) -> Self {
        AppError::Sandbox(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Sandbox(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = axum::Json(json!({ "error": message }));
        (status, body).into_response()
    }
}
