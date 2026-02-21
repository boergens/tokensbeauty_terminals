use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    Warm,
    Claimed,
    Destroying,
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub id: Uuid,
    pub status: InstanceStatus,
    pub workspace: PathBuf,
    pub tmux_socket: String,
    pub tmux_session: String,
    pub created_at: std::time::Instant,
    pub ttyd_port: Option<u16>,
    pub ttyd_pid: Option<u32>,
}

impl Instance {
    pub fn new(id: Uuid, workspace: PathBuf) -> Self {
        let tmux_socket = format!("inst-{}", id);
        let tmux_session = format!("sess-{}", id);
        Self {
            id,
            status: InstanceStatus::Warm,
            workspace,
            tmux_socket,
            tmux_session,
            created_at: std::time::Instant::now(),
            ttyd_port: None,
            ttyd_pid: None,
        }
    }

    pub fn info(&self) -> InstanceInfo {
        InstanceInfo {
            id: self.id,
            status: self.status.clone(),
            workspace: self.workspace.display().to_string(),
            uptime_secs: self.created_at.elapsed().as_secs(),
            ttyd_port: self.ttyd_port,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InstanceInfo {
    pub id: Uuid,
    pub status: InstanceStatus,
    pub workspace: String,
    pub uptime_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttyd_port: Option<u16>,
}
