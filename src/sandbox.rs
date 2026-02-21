use crate::error::SandboxError;
use std::path::Path;
use std::process::Command;
use tracing::{debug, error};

/// Build the bwrap command arguments for launching a sandboxed bash shell.
pub fn bwrap_args(workspace: &Path, instance_id: &str, server_url: &str) -> Vec<String> {
    let workspace_str = workspace.display().to_string();
    [
        "--ro-bind", "/usr", "/usr",
        "--ro-bind", "/lib", "/lib",
        "--ro-bind", "/lib64", "/lib64",
        "--ro-bind", "/bin", "/bin",
        "--ro-bind", "/sbin", "/sbin",
        "--ro-bind", "/etc", "/etc",
        "--ro-bind", "/home/kevin/.local", "/home/kevin/.local",
        "--dev", "/dev",
        "--proc", "/proc",
        "--tmpfs", "/tmp",
        "--tmpfs", "/run",
        "--bind", &workspace_str, "/home/sandbox",
        "--chdir", "/home/sandbox",
        "--setenv", "HOME", "/home/sandbox",
        "--setenv", "PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/home/kevin/.local/bin:/home/sandbox/.local/bin",
        "--setenv", "TERMINAL_SERVER_URL", server_url,
        "--setenv", "INSTANCE_ID", instance_id,
        "--unsetenv", "CLAUDECODE",
        "--unshare-pid",
        "--die-with-parent",
        "--new-session",
        "/bin/bash",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Build the full shell command string that tmux will run.
pub fn bwrap_shell_command(workspace: &Path, instance_id: &str, server_url: &str) -> String {
    let args = bwrap_args(workspace, instance_id, server_url);
    let mut cmd = String::from("bwrap");
    for arg in &args {
        cmd.push(' ');
        // Quote arguments that contain spaces
        if arg.contains(' ') || arg.contains(':') {
            cmd.push('\'');
            cmd.push_str(arg);
            cmd.push('\'');
        } else {
            cmd.push_str(arg);
        }
    }
    cmd
}

/// Create a new tmux session running bwrap.
pub fn tmux_new_session(
    socket: &str,
    session: &str,
    workspace: &Path,
    instance_id: &str,
    server_url: &str,
) -> Result<(), SandboxError> {
    let shell_cmd = bwrap_shell_command(workspace, instance_id, server_url);
    debug!(socket, session, %shell_cmd, "creating tmux session");

    let output = Command::new("tmux")
        .args(["-L", socket, "new-session", "-d", "-s", session, "-x", "200", "-y", "50"])
        .arg(&shell_cmd)
        .output()
        .map_err(|e| SandboxError::TmuxFailed(format!("failed to spawn tmux: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(%stderr, "tmux new-session failed");
        return Err(SandboxError::TmuxFailed(format!(
            "tmux new-session exited {}: {}",
            output.status, stderr
        )));
    }
    Ok(())
}

/// Send keys (text) to the tmux pane.
pub fn tmux_send_keys(socket: &str, session: &str, text: &str) -> Result<(), SandboxError> {
    debug!(socket, session, %text, "sending keys");

    let output = Command::new("tmux")
        .args(["-L", socket, "send-keys", "-t", session, "--", text, "Enter"])
        .output()
        .map_err(|e| SandboxError::TmuxFailed(format!("failed to spawn tmux send-keys: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SandboxError::TmuxFailed(format!(
            "tmux send-keys failed: {}",
            stderr
        )));
    }
    Ok(())
}

/// Send raw keys (without appending Enter) to the tmux pane.
pub fn tmux_send_keys_raw(
    socket: &str,
    session: &str,
    keys: &[&str],
) -> Result<(), SandboxError> {
    debug!(socket, session, ?keys, "sending raw keys");

    let mut args = vec!["-L", socket, "send-keys", "-t", session, "--"];
    args.extend(keys);

    let output = Command::new("tmux")
        .args(&args)
        .output()
        .map_err(|e| {
            SandboxError::TmuxFailed(format!("failed to spawn tmux send-keys: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SandboxError::TmuxFailed(format!(
            "tmux send-keys failed: {}",
            stderr
        )));
    }
    Ok(())
}

/// Capture the current tmux pane content.
pub fn tmux_capture_pane(socket: &str, session: &str) -> Result<String, SandboxError> {
    debug!(socket, session, "capturing pane");

    let output = Command::new("tmux")
        .args([
            "-L", socket,
            "capture-pane", "-t", session, "-p",
        ])
        .output()
        .map_err(|e| {
            SandboxError::TmuxFailed(format!("failed to spawn tmux capture-pane: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SandboxError::TmuxFailed(format!(
            "tmux capture-pane failed: {}",
            stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Kill a tmux session.
pub fn tmux_kill_session(socket: &str, session: &str) -> Result<(), SandboxError> {
    debug!(socket, session, "killing session");

    let output = Command::new("tmux")
        .args(["-L", socket, "kill-session", "-t", session])
        .output()
        .map_err(|e| {
            SandboxError::TmuxFailed(format!("failed to spawn tmux kill-session: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Not an error if session already gone
        if !stderr.contains("no server running")
            && !stderr.contains("session not found")
            && !stderr.contains("can't find session")
        {
            return Err(SandboxError::TmuxFailed(format!(
                "tmux kill-session failed: {}",
                stderr
            )));
        }
    }
    Ok(())
}

/// Kill the entire tmux server for a given socket (full cleanup).
pub fn tmux_kill_server(socket: &str) -> Result<(), SandboxError> {
    debug!(socket, "killing tmux server");

    let output = Command::new("tmux")
        .args(["-L", socket, "kill-server"])
        .output()
        .map_err(|e| {
            SandboxError::TmuxFailed(format!("failed to spawn tmux kill-server: {}", e))
        })?;

    // Ignore failures -- server may already be gone
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!(%stderr, "tmux kill-server returned non-zero (likely already gone)");
    }
    Ok(())
}
