// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::state::*;

#[test]
fn cron_tabs_job_types_and_draft_defaults_match_api_contract() {
    assert_eq!(CronSettingsTab::default(), CronSettingsTab::Jobs);
    assert_eq!(CronSettingsTab::Scheduler, CronSettingsTab::Scheduler);
    assert_eq!(CronAddJobType::default(), CronAddJobType::Agent);
    assert_eq!(CronAddJobType::Shell.as_api_value(), "shell");
    assert_eq!(CronAddJobType::Agent.as_api_value(), "agent");
    assert_eq!(CronAddScheduleKind::default(), CronAddScheduleKind::Cron);
    assert_eq!(CronAddScheduleKind::Cron.as_api_value(), "cron");
    assert_eq!(CronAddScheduleKind::At.as_api_value(), "at");
    assert_eq!(CronAddScheduleKind::Every.as_api_value(), "every");

    let draft = CronJobDraft::default();
    assert!(draft.name.is_empty());
    assert_eq!(draft.job_type, CronAddJobType::Agent);
    assert_eq!(draft.schedule_kind, CronAddScheduleKind::Cron);
    assert_eq!(draft.session_target, "isolated");
    assert_eq!(draft.agent, "main");
    assert_eq!(draft.schedule, "0 9 * * *");
    assert_eq!(draft.frequency, CronFrequency::Daily);
    assert!(draft.at.is_empty());
    assert!(draft.every_ms.is_empty());
    assert!(draft.command.is_empty());
    assert!(draft.command_editor.text().is_empty());
    assert!(draft.prompt.is_empty());
    assert!(draft.prompt_editor.text().is_empty());
    assert!(draft.acp_agent.is_empty());
    assert!(draft.project_path.is_empty());
    assert!(draft.model_provider.is_empty());
    assert!(draft.model.is_empty());
    assert!(!draft.wake);
    assert!(draft.fallbacks.is_empty());
    assert!(!draft.full_access);
    assert!(!draft.task_pool);
    assert!(!draft.delivery_enabled);
    assert!(draft.delivery_channel.is_empty());
    assert!(draft.delivery_to.is_empty());
    assert!(draft.delivery_best_effort);
    assert!(!draft.delete_after_run);

    let state = CronSettingsState::default();
    assert!(state.enabled);
    assert_eq!(state.max_run_history, 50);
    assert_eq!(state.active_tab, CronSettingsTab::Jobs);
    assert!(!state.jobs_loading);
    assert!(state.jobs.is_empty());
    assert!(state.selected_job_ids.is_empty());
    assert!(state.editing_job_id.is_none());
    assert!(state.runs_modal_job_id.is_none());
    assert!(!state.runs_modal_loading);
    assert!(state.runs_modal_error.is_none());
    assert!(state.runs_modal.is_empty());
    assert!(state.runs_modal_editor.text().is_empty());
    assert!(!state.show_help_modal);
    assert!(state.save_error.is_none());
    assert!(state.action_status.is_none());
}
}

mod behavior_1 {
use crate::app_init::state::SchedulerSettingsState;
#[test]
fn scheduler_defaults_match_safe_runtime_limits() {
    let scheduler = SchedulerSettingsState::default();
    assert!(scheduler.enabled);
    assert_eq!(scheduler.max_tasks, 64);
    assert_eq!(scheduler.max_concurrent, 4);
    assert!(!scheduler.show_help_modal);
    assert!(scheduler.save_error.is_none());
}
}

mod behavior_2 {
use crate::app_init::state::HeartbeatSettingsState;
#[test]
fn route_classification_and_heartbeat_defaults_are_ready_to_edit_3() {
    let heartbeat = HeartbeatSettingsState::default();
    assert!(!heartbeat.enabled);
    assert_eq!(heartbeat.interval_minutes, 30);
    assert!(heartbeat.message_input.is_empty());
    assert!(heartbeat.target_input.is_empty());
    assert!(heartbeat.to_input.is_empty());
    assert!(!heartbeat.show_help_modal);
    assert!(heartbeat.save_error.is_none());
}
}
