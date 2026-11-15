//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::session_gateway::{
    gateway_hook_audit_async, gateway_hook_revoke_async, gateway_hook_trust_async,
    gateway_hooks_async,
};
use crate::app::{App, Message};
use iced::Task;

use super::messages::{HooksMessage, SettingsMessage};
fn catalog_refresh_task() -> Task<Message> {
    Task::perform(gateway_hooks_async(), |result| {
        Message::Settings(SettingsMessage::Hooks(HooksMessage::CatalogLoaded(result)))
    })
}

fn audit_refresh_task() -> Task<Message> {
    Task::perform(gateway_hook_audit_async(), |result| {
        Message::Settings(SettingsMessage::Hooks(HooksMessage::AuditLoaded(result)))
    })
}

fn trust_task(plugin_id: String, hook_hash: String) -> Task<Message> {
    Task::perform(async move { gateway_hook_trust_async(&plugin_id, &hook_hash).await }, |result| {
        Message::Settings(SettingsMessage::Hooks(HooksMessage::MutationCompleted(result)))
    })
}

fn revoke_task(plugin_id: String, hook_hash: String) -> Task<Message> {
    Task::perform(
        async move { gateway_hook_revoke_async(&plugin_id, &hook_hash).await },
        |result| Message::Settings(SettingsMessage::Hooks(HooksMessage::MutationCompleted(result))),
    )
}

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod hooks_tests;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
 let SettingsMessage::Hooks(message) = message else { return Task::none(); };
 use desktop_settings_runtime::hooks::{self, Effect};
 match hooks::update(&mut app.hooks_settings, message) {
 Effect::None => Task::none(), Effect::CatalogRefresh => catalog_refresh_task(), Effect::AuditRefresh => audit_refresh_task(),
 Effect::Trust { plugin_id, hook_hash } => trust_task(plugin_id, hook_hash),
 Effect::Revoke { plugin_id, hook_hash } => revoke_task(plugin_id, hook_hash),
 Effect::Save { enabled, command_logger, plugins_enabled } => crate::app::update_hooks_config_async(move |hooks| {
 hooks.enabled = enabled; hooks.builtin.command_logger = command_logger; hooks.plugins.enabled = plugins_enabled;
 }),
 Effect::Error { source, error } => app.show_localized_toast(crate::app::state::ToastKind::Error, source, vec![error.into()]),
 Effect::MutationSucceeded { source } => Task::batch([
 Task::done(Message::Settings(SettingsMessage::Hooks(HooksMessage::CatalogRefresh))),
 app.show_localized_toast(crate::app::state::ToastKind::Success, source, vec![]),
 ]),
 }
}
