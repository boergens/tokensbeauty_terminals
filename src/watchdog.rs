use crate::manager::InstanceManager;
use crate::sandbox;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

const CHECK_INTERVAL_SECS: u64 = 30;
const STUCK_THRESHOLD_SECS: u64 = 300;

/// Spawn a background task that watches for stuck Claimed instances.
/// Returns a JoinHandle that can be aborted for graceful shutdown.
pub fn spawn_watchdog(manager: InstanceManager) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(
            check_interval_secs = CHECK_INTERVAL_SECS,
            stuck_threshold_secs = STUCK_THRESHOLD_SECS,
            "watchdog started"
        );

        let mut interval = time::interval(Duration::from_secs(CHECK_INTERVAL_SECS));

        loop {
            interval.tick().await;

            let claimed = manager.claimed_instances();
            if claimed.is_empty() {
                continue;
            }

            debug!(count = claimed.len(), "watchdog checking claimed instances");

            for (id, socket, session) in claimed {
                // Capture screen on a blocking thread (tmux is a subprocess call)
                let s = socket.clone();
                let sess = session.clone();
                let capture = tokio::task::spawn_blocking(move || {
                    sandbox::tmux_capture_pane(&s, &sess)
                })
                .await;

                let content = match capture {
                    Ok(Ok(c)) => c,
                    Ok(Err(e)) => {
                        warn!(%id, %e, "watchdog: failed to capture screen");
                        continue;
                    }
                    Err(e) => {
                        error!(%id, %e, "watchdog: join error capturing screen");
                        continue;
                    }
                };

                // Hash the screen content
                let mut hasher = DefaultHasher::new();
                content.hash(&mut hasher);
                let hash = hasher.finish();

                // Compare with previous hash and update state
                let should_nudge = {
                    let mut state = manager.state.lock().unwrap();
                    if let Some(inst) = state.get_mut(&id) {
                        if inst.last_screen_hash == Some(hash) {
                            // Screen unchanged — check how long
                            inst.last_screen_change.elapsed().as_secs() >= STUCK_THRESHOLD_SECS
                        } else {
                            // Screen changed — update tracking
                            inst.last_screen_hash = Some(hash);
                            inst.last_screen_change = std::time::Instant::now();
                            false
                        }
                    } else {
                        false
                    }
                };

                if should_nudge {
                    info!(%id, "watchdog: instance appears stuck, nudging");
                    match manager.nudge_instance(id).await {
                        Ok(()) => {
                            // Reset the timer after nudging so we don't immediately re-nudge
                            let mut state = manager.state.lock().unwrap();
                            if let Some(inst) = state.get_mut(&id) {
                                inst.last_screen_change = std::time::Instant::now();
                                inst.last_screen_hash = None;
                            }
                            info!(%id, "watchdog: nudge sent successfully");
                        }
                        Err(e) => {
                            error!(%id, %e, "watchdog: failed to nudge instance");
                        }
                    }
                }
            }
        }
    })
}
