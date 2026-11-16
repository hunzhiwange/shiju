//! 桌面壳：注入配置入口并映射领域结果。
use crate::app::{App, Message};
use super::messages::SettingsMessage;
use desktop_settings_cron::update::cron as domain;
use iced::Task;
#[path = "cron_mapping.rs"] pub(super) mod mapping;
pub fn update(app: &mut App, event: SettingsMessage) -> Task<Message> {
 let delivery = crate::app::components::system_settings_cron::availability(app);
 let mut notices = Vec::new();
 let config = crate::app::config::agent_config_access();
 let task = domain::update(&mut domain::Inputs { cron_settings: &mut app.cron_settings, config: &config, desktop_locale: app.general_settings.desktop_locale, delivery, client: crate::app::config::gateway_client(), notices: &mut notices, }, mapping::event(event)).map(mapping::message);
 let mut tasks = vec![task];
 for notice in notices { tasks.push(match notice { domain::Notice::Error(text) => app.show_error_toast(text), domain::Notice::Success(text) => app.show_success_toast(text) }); }
 Task::batch(tasks)
}
#[cfg(test)]
#[path = "cron_tests.rs"] mod cron_tests;
pub(crate) fn refresh_run_history_locale(app: &mut App) {
 domain::refresh_run_history_locale(&mut app.cron_settings, app.general_settings.desktop_locale);
}
pub(crate) fn validate_delivery(app: &App, draft: &crate::app::state::CronJobDraft) -> Result<(), String> {
 desktop_settings_cron::delivery::validate(draft, crate::app::components::system_settings_cron::availability(app))
}
