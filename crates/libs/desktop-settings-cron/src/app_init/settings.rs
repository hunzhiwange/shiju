//! 从已加载配置构造本领域设置；不执行网络或宿主操作。

pub fn build_cron_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::CronSettingsState {
    let cron_cfg = &full_agent_cfg.cron;

    crate::state::CronSettingsState {
        enabled: cron_cfg.enabled,
        max_run_history: cron_cfg.max_run_history.clamp(1, 10_000),
        ..Default::default()
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
