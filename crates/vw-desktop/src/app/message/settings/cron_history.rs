//! 桌面壳：运行记录可见性与项目上下文协调。
use crate::app::{App, Message};
use crate::app::state::CronSettingsTab;
use iced::Task;
pub use desktop_settings_cron::message::HistoryMessage as CronHistoryMessage;
use super::cron::mapping;
pub(crate) fn visible(app: &App) -> bool {
 app.navigation_panels.show_system_settings && app.navigation_panels.system_settings_tab == crate::app::components::system_settings::SystemTab::Cron && app.cron_settings.active_tab == CronSettingsTab::History
}
fn task(app: &mut App, event: Option<CronHistoryMessage>) -> Task<Message> {
 use desktop_settings_cron::update::{cron, history};
 let delivery = crate::app::components::system_settings_cron::availability(app);
 let config = crate::app::config::agent_config_access();
 let mut notices = Vec::new();
 let mut inputs = cron::Inputs { cron_settings: &mut app.cron_settings, desktop_locale: app.general_settings.desktop_locale, delivery, client: crate::app::config::gateway_client(), config: &config, notices: &mut notices };
 let task = match event { Some(event) => history::update(&mut inputs, event), None => history::refresh(&mut inputs) }.map(mapping::message);
 let mut tasks = vec![task];
 for notice in notices { tasks.push(match notice { cron::Notice::Error(text) => app.show_error_toast(text), cron::Notice::Success(text) => app.show_success_toast(text) }); }
 Task::batch(tasks)
}
pub(crate) fn reconcile(app: &mut App) -> Task<Message> {
 let visible = visible(app);
 if app.cron_settings.history.sync_context(visible, app.navigation_tabs.project_path.clone()) { task(app, None) } else { Task::none() }
}
pub(crate) fn update(app: &mut App, event: CronHistoryMessage) -> Task<Message> {
 let context = reconcile(app);
 Task::batch([context, task(app, Some(event))])
}
