//! 工作树快照的加载状态与刷新节流。
use crate::helpers::execution::TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS;
use desktop_task_executor::executor::state::WorktreePoolSnapshot;

pub struct SnapshotState<'a> {
    pub snapshot: &'a mut Option<WorktreePoolSnapshot>,
    pub loading: &'a mut bool,
    pub last_refresh: &'a mut u64,
}
impl SnapshotState<'_> {
    pub fn clear(&mut self) {
        *self.snapshot = None;
        *self.loading = false;
        *self.last_refresh = 0;
    }
    pub fn request(
        &mut self,
        project: Option<&str>,
        force: bool,
        now: u64,
    ) -> bool {
        if project.is_none() {
            self.clear();
            return false;
        }
        if *self.loading {
            return false;
        }
        if !force && now.saturating_sub(*self.last_refresh) < TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS {
            return false;
        }
        *self.loading = true;
        true
    }
    pub fn loaded(
        &mut self,
        result: Result<Option<WorktreePoolSnapshot>, String>,
        now: u64,
    ) {
        *self.loading = false;
        *self.last_refresh = now;
        match result {
            Ok(snapshot) => *self.snapshot = snapshot,
            Err(error) => {
                if gateway_http::is_gateway_unavailable_error(&error) {
                    tracing::debug!(error = %error, "worktree pool snapshot refresh skipped because gateway is unavailable");
                } else {
                    tracing::warn!(error = %error, "gateway worktree pool snapshot refresh failed");
                }
            }
        }
    }
    pub fn maintained(
        &mut self,
        in_flight: &mut bool,
        tasks: &mut [desktop_task_model::Task],
        result: Result<crate::message::WorktreePoolMaintenanceOutcome, String>,
        now: u64,
    ) -> MaintenanceEffect {
        *in_flight = false;
        match result {
            Ok(outcome) => {
                tracing::debug!(
                    idle_count = outcome.idle_count,
                    target_idle_count = outcome.target_idle_count,
                    "gateway worktree pool maintenance completed"
                );
                if let Some(snapshot) = outcome.snapshot {
                    *self.loading = false;
                    *self.last_refresh = now;
                    *self.snapshot = Some(snapshot);
                    MaintenanceEffect::None
                } else {
                    MaintenanceEffect::Refresh
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "gateway worktree pool maintenance failed");
                if crate::update::rules::log_worktree_maintenance_failure(tasks, &error) {
                    MaintenanceEffect::Persist
                } else {
                    MaintenanceEffect::None
                }
            }
        }
    }
}
pub fn begin_maintenance(
    in_flight: &mut bool,
    slot_recycle_in_flight: bool,
    desired: Option<u32>,
    max_concurrent: u32,
) -> Option<u32> {
    if *in_flight || slot_recycle_in_flight {
        return None;
    }
    *in_flight = true;
    Some(desired.unwrap_or(max_concurrent).max(1))
}

pub enum MaintenanceEffect {
    None,
    Refresh,
    Persist,
}
