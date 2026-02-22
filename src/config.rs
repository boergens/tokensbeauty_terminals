use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub pool_target_size: usize,
    pub pool_max_size: usize,
    pub workspace_base: String,
    pub claude_md_template: String,
    pub tmux_width: u16,
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
            claude_md_template: env::var("CLAUDE_MD_TEMPLATE")
                .unwrap_or_else(|_| "/home/kevin/project/tokensbeauty/CLAUDE.md.template".into()),
            tmux_width: env::var("TMUX_WIDTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(80),
        }
    }
}
