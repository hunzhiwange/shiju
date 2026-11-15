//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。


use super::messages::Message;
use desktop_config::ConfigAccess;
use iced::Task;

use super::messages::SettingsMessage;

fn persist_scheduler_settings(app: &mut Inputs<'_>) -> Task<Message> {
    let enabled = app.scheduler_settings.enabled;
    let max_tasks = app.scheduler_settings.max_tasks.clamp(1, 10_000) as usize;
    let max_concurrent = app.scheduler_settings.max_concurrent.clamp(1, 100) as usize;

    app.config.update_scheduler_config_async(move |scheduler| {
        scheduler.enabled = enabled;
        scheduler.max_tasks = max_tasks;
        scheduler.max_concurrent = max_concurrent;
    }).map(Message::ConfigSaved)
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut Inputs<'_>, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::SchedulerEnabledToggled(v) => {
            app.scheduler_settings.enabled = v;
            app.scheduler_settings.save_error = None;
            persist_scheduler_settings(app)
        }
        SettingsMessage::SchedulerMaxTasksChanged(v) => {
            app.scheduler_settings.max_tasks = v.clamp(1, 10_000);
            app.scheduler_settings.save_error = None;
            persist_scheduler_settings(app)
        }
        SettingsMessage::SchedulerMaxConcurrentChanged(v) => {
            app.scheduler_settings.max_concurrent = v.clamp(1, 100);
            app.scheduler_settings.save_error = None;
            persist_scheduler_settings(app)
        }
        SettingsMessage::SchedulerSave => {
            app.scheduler_settings.save_error = None;
            persist_scheduler_settings(app)
        }
        SettingsMessage::SchedulerHelpOpen => {
            app.scheduler_settings.show_help_modal = true;
            Task::none()
        }
        SettingsMessage::SchedulerHelpClose => {
            app.scheduler_settings.show_help_modal = false;
            Task::none()
        }
        _ => Task::none(),
    }
}


pub struct Inputs<'a> {
pub scheduler_settings: &'a mut crate::app_init::state::SchedulerSettingsState,
 pub config: &'a ConfigAccess,
}

#[cfg(test)] #[path = "scheduler_tests.rs"] mod tests;
