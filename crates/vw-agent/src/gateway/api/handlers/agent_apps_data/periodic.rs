//! Gateway-owned polling; dropped with the server, independent of desktop pages.
use std::{path::PathBuf, time::Duration};
use crate::global_database::GlobalDatabase;
use vw_agent_apps::data_management::{periodic::periodic_backup_if_needed, safety_snapshot::SafetySnapshotError};

struct BackupFailure;
impl From<SafetySnapshotError> for BackupFailure {
    fn from(_: SafetySnapshotError) -> Self { Self }
}

pub(crate) async fn run(policy: PathBuf) {
    let mut ticks = tokio::time::interval(Duration::from_secs(60));
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        ticks.tick().await;
        let policy = policy.clone();
        // Await every blocking worker: no detached recurring work or overlaps.
        let result = tokio::task::spawn_blocking(move || -> Result<(), BackupFailure> {
            let targets = super::super::agent_apps::periodic_data_backup_targets(&policy)
                .map_err(|_| BackupFailure)?;
            if targets.is_empty() { return Ok(()); }
            let database = GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| BackupFailure)?;
            let mut failed = false;
            for (root, scope, hours) in targets {
                let target = vw_api_types::agent_apps::data_management::DataManagementTarget {
                    scope: scope.project.clone(), target_host_id: scope.host_id.clone(), principal_id: scope.principal_id.clone(),
                };
                let settings = match super::data_aux_api::load_settings(&database, &target) {
                    Ok(value) => value,
                    Err(_) => { failed = true; continue; }
                };
                let hours = settings.as_ref().map_or(hours, |s| s.interval_hours);
                let result = periodic_backup_if_needed::<BackupFailure>(
                    &root, &scope, hours, chrono::Utc::now().timestamp_millis(), || {
                        super::backup_capture::capture_preferences(
                            &root, &scope, &database, "Periodic backup",
                        ).map_err(|_| BackupFailure)
                    },
                );
                match result {
                    Err(_) => failed = true,
                    Ok(Some(_)) => {
                        if let Some(settings) = settings {
                            use vw_agent_apps::data_management::{backup_api, retention};
                            let cleanup = (|| {
                                let _guard = backup_api::lock_directory(&root).map_err(|_| BackupFailure)?;
                                let policy = retention::RetentionPolicy::new(i64::from(settings.retain_count)).map_err(|_| BackupFailure)?;
                                retention::cleanup_local_backups(&root, &scope, policy).map_err(|_| BackupFailure)?;
                                Ok::<(), BackupFailure>(())
                            })();
                            if cleanup.is_err() { failed = true; }
                        }
                    }
                    Ok(None) => {}
                }
            }
            if failed { Err(BackupFailure) } else { Ok(()) }
        }).await;
        if !matches!(result, Ok(Ok(()))) {
            tracing::warn!("Scheduled data backup failed");
        }
    }
}
