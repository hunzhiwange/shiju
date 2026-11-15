//! 从已加载配置构造本领域设置；不执行网络或宿主操作。

pub fn build_cron_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::CronSettingsState {
    let cron_cfg = &full_agent_cfg.cron;

    crate::state::CronSettingsState {
        show_add_modal: false,
        show_cron_templates: false,
        date_time_picker: None,
        enabled: cron_cfg.enabled,
        max_run_history: cron_cfg.max_run_history.clamp(1, 10_000),
        active_tab: crate::state::CronSettingsTab::default(),
        history: crate::state::CronHistoryState::default(),
        jobs_loading: false,
        jobs: Vec::new(),
        list_filter: crate::state::CronListFilter::default(),
        batch_mode: false,
        selected_job_ids: Vec::new(),
        expanded_job_ids: Vec::new(),
        editing_job_id: None,
        edit_draft: crate::state::CronJobDraft::default(),
        add_draft: crate::state::CronJobDraft::default(),
        runs_modal_job_id: None,
        runs_modal_loading: false,
        runs_modal_error: None,
        runs_modal: Vec::new(),
        runs_modal_editor: iced::widget::text_editor::Content::new(),
        show_help_modal: false,
        save_error: None,
        save_error_translation: None,
        action_status: None,
    }
}


pub fn build_heartbeat_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::app_init::state::HeartbeatSettingsState {
    let heartbeat_cfg = &full_agent_cfg.heartbeat;

    crate::app_init::state::HeartbeatSettingsState {
        enabled: heartbeat_cfg.enabled,
        interval_minutes: heartbeat_cfg.interval_minutes.clamp(1, 1440),
        message_input: heartbeat_cfg.message.clone().unwrap_or_default(),
        target_input: heartbeat_cfg.target.clone().unwrap_or_default(),
        to_input: heartbeat_cfg.to.clone().unwrap_or_default(),
        show_help_modal: false,
        save_error: None,
    }
}

pub fn build_scheduler_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::app_init::state::SchedulerSettingsState {
    let scheduler_cfg = &full_agent_cfg.scheduler;

    crate::app_init::state::SchedulerSettingsState {
        enabled: scheduler_cfg.enabled,
        max_tasks: scheduler_cfg.max_tasks.clamp(1, 10_000) as u32,
        max_concurrent: scheduler_cfg.max_concurrent.clamp(1, 100) as u32,
        show_help_modal: false,
        save_error: None,
    }
}
