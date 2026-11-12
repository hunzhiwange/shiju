//! One host maintenance loop over existing tenant-scoped managed registries.
use super::{Error, cleanup, registry};
use std::{collections::HashSet, sync::LazyLock, time::Duration};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

static CREATED: LazyLock<Notify> = LazyLock::new(Notify::new);
pub(super) fn created() {
    CREATED.notify_one();
}

pub struct HostCleanup {
    stop: CancellationToken,
}
impl Drop for HostCleanup {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
impl HostCleanup {
    /// OS ownership deduplicates gateways sharing a home, independent of UI connections.
    /// Dropping the owner cancels future passes; an in-flight removal finishes safely.
    pub fn start() -> Result<Self, Error> {
        let root = vw_config_types::paths::home_config_dir(&crate::global::paths().home)
            .join("storage")
            .join("worktree-cleanup");
        std::fs::create_dir_all(&root)?;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("host.lock"))?;
        let stop = CancellationToken::new();
        let stopped = stop.clone();
        tokio::spawn(async move {
            while lock.try_lock().is_err() {
                tokio::select! {
                    biased;
                    _ = stopped.cancelled() => return,
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {},
                }
            }
            let _owner = lock;
            let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    biased;
                    _ = stopped.cancelled() => break,
                    _ = interval.tick() => {},
                    _ = CREATED.notified() => {},
                }
                if maintain(&stopped).await.is_err() {
                    tracing::warn!("Managed worktree maintenance pass failed");
                }
            }
        });
        Ok(Self { stop })
    }
}
async fn maintain(stop: &CancellationToken) -> Result<(), Error> {
    let keys = crate::storage::list_strict(&["managed-worktrees"])
        .await
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let mut visited = HashSet::new();
    for key in keys {
        if stop.is_cancelled() {
            break;
        }
        let parts = key.iter().map(String::as_str).collect::<Vec<_>>();
        let records: Vec<registry::Record> = match crate::storage::read(&parts).await {
            Ok(records) => records,
            Err(_) => {
                tracing::warn!("Managed worktree registry unavailable for cleanup");
                continue;
            }
        };
        for record in records {
            if stop.is_cancelled() {
                break;
            }
            if !visited.insert(record.scope.clone()) {
                continue;
            }
            // Registry metadata never grants access to a project absent from its tenant.
            let scope = record.scope;
            let result = async {
                let project = crate::project::get(&scope)
                    .await
                    .map_err(|e| Error::Invalid(e.to_string()))?
                    .ok_or_else(|| Error::Invalid("Cleanup project unavailable".into()))?;
                let settings = crate::config::get_global_for_tenant(scope.tenant_scope())
                    .await
                    .map_err(|e| Error::Invalid(e.to_string()))?
                    .worktree_settings;
                cleanup::refresh(&scope, std::path::Path::new(&project.worktree), &settings, stop)
                    .await
            }
            .await;
            if result.is_err() {
                tracing::warn!("Managed worktree project maintenance failed");
            }
        }
    }
    Ok(())
}
