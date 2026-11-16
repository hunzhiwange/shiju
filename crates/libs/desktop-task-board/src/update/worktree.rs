//! 工作树快照的加载状态与刷新节流。
use super::super::helpers::execution::{
    TASK_BOARD_WORKTREE_SNAPSHOT_EXPANDED_INTERVAL_MS, TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS,
};
use desktop_task_executor::executor::state::WorktreePoolSnapshot;

pub struct SnapshotState<'a> {
    pub snapshot: &'a mut Option<WorktreePoolSnapshot>,
    pub loading: &'a mut bool,
    pub last_refresh: &'a mut u64,
}
pub fn refresh_interval(expanded: bool) -> u64 {
    if expanded {
        TASK_BOARD_WORKTREE_SNAPSHOT_EXPANDED_INTERVAL_MS
    } else {
        TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS
    }
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
        expanded: bool,
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
        if !force && now.saturating_sub(*self.last_refresh) < refresh_interval(expanded) {
            return false;
        }
        *self.loading = true;
        true
    }
    pub fn loaded(
        &mut self,
        is_git: bool,
        result: Result<Option<WorktreePoolSnapshot>, String>,
        now: u64,
    ) {
        if !is_git {
            self.clear();
            return;
        }
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
        is_git: bool,
        has_project: bool,
        result: Result<crate::message::WorktreePoolMaintenanceOutcome, String>,
        now: u64,
    ) -> MaintenanceEffect {
        *in_flight = false;
        if !is_git {
            self.clear();
            return MaintenanceEffect::None;
        }
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
                if has_project
                    && crate::update::rules::log_worktree_maintenance_failure(tasks, &error)
                {
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
    manual_action: Option<&str>,
    desired: Option<u32>,
    max_concurrent: u32,
) -> Option<u32> {
    if *in_flight || manual_action.is_some() {
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

/// 清理动作只针对污染或已失效的受管槽位。
pub fn cleanup_slot_ids(
    snapshot: &desktop_task_executor::executor::state::WorktreePoolSnapshot,
) -> Vec<String> {
    use desktop_task_executor::executor::state::WorktreeState;
    snapshot
        .slots
        .iter()
        .filter(|slot| matches!(slot.state, WorktreeState::Tainted | WorktreeState::Dead))
        .map(|slot| slot.id.clone())
        .collect()
}
