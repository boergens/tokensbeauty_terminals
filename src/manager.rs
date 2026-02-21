use crate::config::Config;
use crate::error::{AppError, SandboxError};
use crate::instance::{Instance, InstanceInfo, InstanceStatus};
use crate::sandbox;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task;
use tracing::{error, info, warn};
use uuid::Uuid;

pub type SharedState = Arc<Mutex<HashMap<Uuid, Instance>>>;

#[derive(Clone)]
pub struct InstanceManager {
    pub state: SharedState,
    pub config: Config,
}

impl InstanceManager {
    pub fn new(config: Config) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Create a new sandboxed instance. This spawns blocking subprocess work on
    /// a blocking thread so we don't block the tokio runtime.
    pub async fn create_instance(&self) -> Result<InstanceInfo, AppError> {
        let id = Uuid::new_v4();
        let workspace = std::path::PathBuf::from(&self.config.workspace_base).join(id.to_string());

        // Create workspace directory
        std::fs::create_dir_all(&workspace).map_err(|e| {
            SandboxError::WorkspaceCreation(format!(
                "failed to create {}: {}",
                workspace.display(),
                e
            ))
        })?;

        let instance = Instance::new(id, workspace.clone());
        let socket = instance.tmux_socket.clone();
        let session = instance.tmux_session.clone();
        let ws = workspace.clone();

        // Spawn tmux+bwrap on a blocking thread
        task::spawn_blocking(move || sandbox::tmux_new_session(&socket, &session, &ws))
            .await
            .map_err(|e| AppError::Internal(format!("join error: {}", e)))??;

        let info = instance.info();
        {
            let mut state = self.state.lock().unwrap();
            state.insert(id, instance);
        }

        info!(%id, "instance created");
        Ok(info)
    }

    /// Acquire a warm instance from the pool, or create a new one.
    pub async fn acquire_instance(&self) -> Result<InstanceInfo, AppError> {
        // Try to claim a warm instance
        let warm_id = {
            let mut state = self.state.lock().unwrap();
            let warm = state
                .values_mut()
                .find(|inst| inst.status == InstanceStatus::Warm);
            if let Some(inst) = warm {
                inst.status = InstanceStatus::Claimed;
                Some(inst.info())
            } else {
                None
            }
        };

        if let Some(info) = warm_id {
            info!(id = %info.id, "claimed warm instance");
            return Ok(info);
        }

        // No warm instances available -- check max size
        {
            let state = self.state.lock().unwrap();
            if state.len() >= self.config.pool_max_size {
                return Err(AppError::BadRequest(
                    "maximum instance limit reached".into(),
                ));
            }
        }

        // Create a new one
        let mut info = self.create_instance().await?;
        // Mark it as claimed
        {
            let mut state = self.state.lock().unwrap();
            if let Some(inst) = state.get_mut(&info.id) {
                inst.status = InstanceStatus::Claimed;
                info = inst.info();
            }
        }
        Ok(info)
    }

    /// Destroy an instance: kill tmux, remove workspace.
    pub async fn destroy_instance(&self, id: Uuid) -> Result<(), AppError> {
        let (socket, session, workspace) = {
            let mut state = self.state.lock().unwrap();
            let inst = state
                .get_mut(&id)
                .ok_or_else(|| AppError::NotFound(format!("instance {} not found", id)))?;
            inst.status = InstanceStatus::Destroying;
            (
                inst.tmux_socket.clone(),
                inst.tmux_session.clone(),
                inst.workspace.clone(),
            )
        };

        // Kill tmux on blocking thread
        let s = socket.clone();
        let sess = session.clone();
        task::spawn_blocking(move || {
            let _ = sandbox::tmux_kill_session(&s, &sess);
            let _ = sandbox::tmux_kill_server(&s);
        })
        .await
        .map_err(|e| AppError::Internal(format!("join error: {}", e)))?;

        // Remove workspace
        if workspace.exists() {
            if let Err(e) = std::fs::remove_dir_all(&workspace) {
                warn!(%id, %e, "failed to remove workspace");
            }
        }

        // Remove from state
        {
            let mut state = self.state.lock().unwrap();
            state.remove(&id);
        }

        info!(%id, "instance destroyed");
        Ok(())
    }

    /// Send text input to an instance's tmux pane.
    pub async fn send_input(&self, id: Uuid, text: &str) -> Result<(), AppError> {
        let (socket, session) = self.get_tmux_info(id)?;

        let text = text.to_string();
        task::spawn_blocking(move || sandbox::tmux_send_keys(&socket, &session, &text))
            .await
            .map_err(|e| AppError::Internal(format!("join error: {}", e)))??;

        Ok(())
    }

    /// Send raw keys to an instance's tmux pane (no Enter appended).
    pub async fn send_keys_raw(&self, id: Uuid, keys: Vec<String>) -> Result<(), AppError> {
        let (socket, session) = self.get_tmux_info(id)?;

        task::spawn_blocking(move || {
            let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
            sandbox::tmux_send_keys_raw(&socket, &session, &key_refs)
        })
        .await
        .map_err(|e| AppError::Internal(format!("join error: {}", e)))??;

        Ok(())
    }

    /// Capture the screen content of an instance.
    pub async fn capture_screen(&self, id: Uuid) -> Result<String, AppError> {
        let (socket, session) = self.get_tmux_info(id)?;

        let content = task::spawn_blocking(move || sandbox::tmux_capture_pane(&socket, &session))
            .await
            .map_err(|e| AppError::Internal(format!("join error: {}", e)))??;

        Ok(content)
    }

    /// List all instances.
    pub fn list_instances(&self) -> Vec<InstanceInfo> {
        let state = self.state.lock().unwrap();
        state.values().map(|inst| inst.info()).collect()
    }

    /// Get a single instance's info.
    pub fn get_instance(&self, id: Uuid) -> Result<InstanceInfo, AppError> {
        let state = self.state.lock().unwrap();
        state
            .get(&id)
            .map(|inst| inst.info())
            .ok_or_else(|| AppError::NotFound(format!("instance {} not found", id)))
    }

    /// Count warm instances.
    pub fn warm_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state
            .values()
            .filter(|inst| inst.status == InstanceStatus::Warm)
            .count()
    }

    /// Total instance count.
    pub fn total_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.len()
    }

    /// Destroy all instances (for shutdown).
    pub async fn destroy_all(&self) {
        let ids: Vec<Uuid> = {
            let state = self.state.lock().unwrap();
            state.keys().cloned().collect()
        };

        for id in ids {
            if let Err(e) = self.destroy_instance(id).await {
                error!(%id, %e, "failed to destroy instance during shutdown");
            }
        }
    }

    fn get_tmux_info(&self, id: Uuid) -> Result<(String, String), AppError> {
        let state = self.state.lock().unwrap();
        let inst = state
            .get(&id)
            .ok_or_else(|| AppError::NotFound(format!("instance {} not found", id)))?;

        if inst.status == InstanceStatus::Destroying {
            return Err(AppError::BadRequest(format!(
                "instance {} is being destroyed",
                id
            )));
        }

        Ok((inst.tmux_socket.clone(), inst.tmux_session.clone()))
    }
}
