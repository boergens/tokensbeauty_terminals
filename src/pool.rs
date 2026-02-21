use crate::manager::InstanceManager;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info};

/// Spawn a background task that replenishes the warm pool.
/// Returns a JoinHandle that can be aborted for graceful shutdown.
pub fn spawn_pool_replenisher(
    manager: InstanceManager,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(
            target_size = manager.config.pool_target_size,
            max_size = manager.config.pool_max_size,
            "pool replenisher started"
        );

        let mut interval = time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            let warm = manager.warm_count();
            let total = manager.total_count();
            let target = manager.config.pool_target_size;
            let max = manager.config.pool_max_size;

            debug!(warm, total, target, max, "pool check");

            if warm >= target {
                continue;
            }

            let needed = (target - warm).min(max.saturating_sub(total));
            if needed == 0 {
                debug!("at max capacity, cannot add more warm instances");
                continue;
            }

            for _ in 0..needed {
                match manager.create_instance().await {
                    Ok(info) => {
                        info!(id = %info.id, "pool: created warm instance");
                    }
                    Err(e) => {
                        error!(%e, "pool: failed to create warm instance");
                        break;
                    }
                }
            }
        }
    })
}
