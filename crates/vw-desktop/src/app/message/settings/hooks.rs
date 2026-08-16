//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::{App, Message};
use crate::app::session_gateway::{
    gateway_hook_audit_async, gateway_hook_revoke_async, gateway_hook_trust_async,
    gateway_hooks_async,
};
use iced::Task;

use super::messages::{HooksMessage, SettingsMessage};
fn persist_hooks_settings(app: &mut App) -> Task<Message> {
    let s = &app.hooks_settings;
    let enabled = s.enabled;
    let command_logger = s.command_logger;
    let plugins_enabled = s.plugins_enabled;

    crate::app::update_hooks_config_async(move |hooks| {
        hooks.enabled = enabled;
        hooks.builtin.command_logger = command_logger;
        hooks.plugins.enabled = plugins_enabled;
    })
}

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
    Task::perform(
        async move { gateway_hook_trust_async(&plugin_id, &hook_hash).await },
        |result| {
            Message::Settings(SettingsMessage::Hooks(HooksMessage::MutationCompleted(result)))
        },
    )
}

fn revoke_task(plugin_id: String, hook_hash: String) -> Task<Message> {
    Task::perform(
        async move { gateway_hook_revoke_async(&plugin_id, &hook_hash).await },
        |result| {
            Message::Settings(SettingsMessage::Hooks(HooksMessage::MutationCompleted(result)))
        },
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
    match message {
        SettingsMessage::Hooks(hooks_message) => {
            match hooks_message {
                HooksMessage::EnabledToggled(value) => {
                    app.hooks_settings.enabled = value;
                }
                HooksMessage::CommandLoggerToggled(value) => {
                    app.hooks_settings.command_logger = value;
                }
                HooksMessage::PluginHooksToggled(value) => {
                    app.hooks_settings.plugins_enabled = value;
                }
                HooksMessage::CatalogRefresh => {
                    app.hooks_settings.catalog_loading = true;
                    app.hooks_settings.catalog_error = None;
                    return catalog_refresh_task();
                }
                HooksMessage::CatalogLoaded(Ok(catalog)) => {
                    app.hooks_settings.catalog = catalog;
                    app.hooks_settings.catalog_loading = false;
                    app.hooks_settings.catalog_error = None;
                    return Task::none();
                }
                HooksMessage::CatalogLoaded(Err(error)) => {
                    app.hooks_settings.catalog_loading = false;
                    app.hooks_settings.catalog_error = Some(error.clone());
                    return app.show_error_toast(format!("读取插件 hook 清单失败：{error}"));
                }
                HooksMessage::AuditRefresh => {
                    app.hooks_settings.audit_loading = true;
                    app.hooks_settings.audit_error = None;
                    return audit_refresh_task();
                }
                HooksMessage::AuditLoaded(Ok(audit)) => {
                    app.hooks_settings.audit = audit;
                    app.hooks_settings.audit_loading = false;
                    app.hooks_settings.audit_error = None;
                    return Task::none();
                }
                HooksMessage::AuditLoaded(Err(error)) => {
                    app.hooks_settings.audit_loading = false;
                    app.hooks_settings.audit_error = Some(error.clone());
                    return app.show_error_toast(format!("读取 hook 审计失败：{error}"));
                }
                HooksMessage::TrustRequested { plugin_id, hook_hash } => {
                    app.hooks_settings.mutation_loading = true;
                    app.hooks_settings.mutation_error = None;
                    return trust_task(plugin_id, hook_hash);
                }
                HooksMessage::RevokeRequested { plugin_id, hook_hash } => {
                    app.hooks_settings.mutation_loading = true;
                    app.hooks_settings.mutation_error = None;
                    return revoke_task(plugin_id, hook_hash);
                }
                HooksMessage::MutationCompleted(Ok(result)) => {
                    app.hooks_settings.mutation_loading = false;
                    app.hooks_settings.mutation_error = None;
                    if let Some(entry) = app.hooks_settings.catalog.iter_mut().find(|entry| {
                        entry.plugin_id == result.plugin_id && entry.hook_hash == result.hook_hash
                    }) {
                        entry.trusted = result.trusted;
                    }
                    let action = if result.trusted { "信任" } else { "撤销信任" };
                    return Task::batch([
                        Task::done(Message::Settings(SettingsMessage::Hooks(
                            HooksMessage::CatalogRefresh,
                        ))),
                        app.show_success_toast(format!("已{action}插件 hook")),
                    ]);
                }
                HooksMessage::MutationCompleted(Err(error)) => {
                    app.hooks_settings.mutation_loading = false;
                    app.hooks_settings.mutation_error = Some(error.clone());
                    return app.show_error_toast(format!("更新插件 hook 信任失败：{error}"));
                }
            }

            app.hooks_settings.save_error = None;
            persist_hooks_settings(app)
        }
        _ => Task::none(),
    }
}
