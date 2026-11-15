//! 看板配置消息更新及刷新截止时间维护。
use desktop_task_model::TaskBoardSettings;
use crate::message::TaskBoardMessage;
use crate::helpers::settings::*;

pub struct SettingsState<'a> {
    pub settings: &'a mut TaskBoardSettings,
    pub worktree_desired_capacity: &'a mut Option<u32>,
    pub next_refresh_at_ms: &'a mut u64,
    pub next_scheduler_tick_at_ms: &'a mut u64,
    pub next_auto_promote_tick_at_ms: &'a mut u64,
}

pub fn update_settings(state: SettingsState<'_>, message: TaskBoardMessage) -> Result<(), TaskBoardMessage> {
    match message {
        TaskBoardMessage::SetAutoPromoteDelay(seconds) => {
            (*state.settings).auto_promote_delay_seconds = seconds;
        }
        TaskBoardMessage::SetMaxConcurrent(count) => {
            (*state.settings).max_concurrent = count.clamp(1, 10);
            (*state.worktree_desired_capacity) = None;
        }
        TaskBoardMessage::ToggleAutoRefresh(enabled) => {
            (*state.settings).auto_refresh = enabled;
        }
        TaskBoardMessage::SetRefreshIntervalSeconds(seconds) => {
            (*state.settings).refresh_interval_seconds = seconds.clamp(1, 3600);
            (*state.next_refresh_at_ms) =
                next_deadline_ms(task_board_refresh_interval_secs(state.settings));
        }
        TaskBoardMessage::SetSchedulerTickIntervalSeconds(seconds) => {
            (*state.settings).scheduler_tick_interval_seconds = seconds.clamp(1, 60);
            (*state.next_scheduler_tick_at_ms) =
                next_deadline_ms(scheduler_tick_interval_secs(state.settings));
        }
        TaskBoardMessage::SetAutoPromoteTickIntervalSeconds(seconds) => {
            (*state.settings).auto_promote_tick_interval_seconds = seconds.clamp(1, 3600);
            (*state.next_auto_promote_tick_at_ms) =
                next_deadline_ms(auto_promote_tick_interval_secs(state.settings));
        }
        TaskBoardMessage::SetFailedRetryMinutes(minutes) => {
            (*state.settings).failed_retry_minutes = minutes.clamp(1, 1440);
        }
        TaskBoardMessage::SetFailedRetryMaxAttempts(attempts) => {
            (*state.settings).failed_retry_max_attempts = attempts.clamp(0, 10);
        }
        TaskBoardMessage::SetRunningTimeoutMinutes(minutes) => {
            (*state.settings).running_timeout_minutes = minutes.clamp(1, 1440);
        }
        TaskBoardMessage::ToggleRecycleWorktreeOnTaskFinish(enabled) => {
            (*state.settings).recycle_worktree_on_task_finish = enabled;
        }
        TaskBoardMessage::SetPrSubmittedStallTimeoutSeconds(seconds) => {
            (*state.settings).pr_submitted_stall_timeout_seconds = seconds.clamp(5, 3600);
        }
        TaskBoardMessage::SetAgentSchedulerDispatchMode(dispatch_mode) => {
            (*state.settings).agent_scheduler_dispatch_mode = dispatch_mode;
        }
        TaskBoardMessage::SettingsUpdated(mut settings) => {
            settings = settings.sanitized();
            (*state.settings) = settings;
            (*state.worktree_desired_capacity) = None;
            (*state.next_refresh_at_ms) =
                next_deadline_ms(task_board_refresh_interval_secs(state.settings));
            (*state.next_scheduler_tick_at_ms) =
                next_deadline_ms(scheduler_tick_interval_secs(state.settings));
            (*state.next_auto_promote_tick_at_ms) =
                next_deadline_ms(auto_promote_tick_interval_secs(state.settings));
        }
        other => return Err(other),
    }
    Ok(())
}
