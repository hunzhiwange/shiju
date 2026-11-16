use desktop_task_model::TaskBoardSettings;
pub fn now_ms() -> u64 {
    runtime_time::now_ms()
}
pub fn next_deadline_ms(interval_secs: u64) -> u64 {
    now_ms().saturating_add(interval_secs.saturating_mul(1000))
}
pub fn sanitized_task_board_settings(settings: TaskBoardSettings) -> TaskBoardSettings {
    settings.sanitized()
}
pub fn task_board_refresh_interval_secs(settings: &TaskBoardSettings) -> u64 {
    settings.refresh_interval_seconds.clamp(1, 3600)
}
pub fn scheduler_tick_interval_secs(settings: &TaskBoardSettings) -> u64 {
    settings.scheduler_tick_interval_seconds.clamp(1, 60)
}
pub fn auto_promote_tick_interval_secs(settings: &TaskBoardSettings) -> u64 {
    settings.auto_promote_tick_interval_seconds.clamp(1, 3600)
}
pub fn pr_submitted_stall_timeout_secs(settings: &TaskBoardSettings) -> u64 {
    settings.pr_submitted_stall_timeout_seconds.clamp(300, 3600) as u64
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;

pub fn scheduler_deadline(deadline: &mut u64, settings: &TaskBoardSettings) {
    if *deadline <= now_ms() { *deadline = next_deadline_ms(scheduler_tick_interval_secs(settings)); }
}
pub fn fixed_deadline(deadline: &mut u64, interval_seconds: u64) {
    *deadline = next_deadline_ms(interval_seconds);
}

pub fn refresh_deadline(deadline: &mut u64, settings: &TaskBoardSettings) {
    fixed_deadline(deadline, task_board_refresh_interval_secs(settings));
}
