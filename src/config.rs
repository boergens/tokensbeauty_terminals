use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub pool_target_size: usize,
    pub pool_max_size: usize,
    pub workspace_base: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
            pool_target_size: env::var("POOL_TARGET_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            pool_max_size: env::var("POOL_MAX_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            workspace_base: env::var("WORKSPACE_BASE")
                .unwrap_or_else(|_| "/tmp/terminals-workspaces".into()),
        }
    }
}
