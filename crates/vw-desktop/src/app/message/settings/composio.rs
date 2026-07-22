//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::{App, Message};
use iced::Task;
use vw_config_types::tools::ComposioConfig;

use super::messages::SettingsMessage;

fn apply_composio_inputs(
    composio: &mut ComposioConfig,
    enabled: bool,
    api_key: String,
    entity_id: String,
) {
    let api_key = api_key.trim().to_string();
    let entity_id = entity_id.trim().to_string();

    composio.enabled = enabled;
    composio.api_key = if api_key.is_empty() { None } else { Some(api_key) };
    composio.entity_id = if entity_id.is_empty() { "default".to_string() } else { entity_id };
}

fn persist_composio_settings(app: &mut App) -> Task<Message> {
    let s = &app.composio_settings;
    let enabled = s.enabled;
    let api_key = s.api_key_input.clone();
    let entity_id = s.entity_id_input.clone();

    crate::app::update_composio_config_async(move |composio| {
        apply_composio_inputs(composio, enabled, api_key, entity_id);
    })
}

#[cfg(test)]
#[path = "composio_tests.rs"]
mod composio_tests;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::ComposioEnabledToggled(value) => {
            app.composio_settings.enabled = value;
            app.composio_settings.save_error = None;
            persist_composio_settings(app)
        }
        SettingsMessage::ComposioApiKeyChanged(value) => {
            app.composio_settings.api_key_input = value;
            app.composio_settings.save_error = None;
            persist_composio_settings(app)
        }
        SettingsMessage::ComposioEntityIdChanged(value) => {
            app.composio_settings.entity_id_input = value;
            app.composio_settings.save_error = None;
            persist_composio_settings(app)
        }
        _ => Task::none(),
    }
}
