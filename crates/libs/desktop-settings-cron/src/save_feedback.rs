//! 保存回执在所属领域更新反馈状态；错误文本由配置边界规范化。

pub fn heartbeat_saved(
    state: &mut crate::app_init::state::HeartbeatSettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}

pub fn cron_saved(state: &mut crate::state::CronSettingsState, error: Option<String>) {
    state.save_error = error;
}

pub fn scheduler_saved(
    state: &mut crate::app_init::state::SchedulerSettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}
