//! 处理渠道设置子模块的状态变更、字段转换和持久化。

use crate::app::{App, Message};
use iced::Task;
use iced::widget::text_editor;

use super::super::messages::{ChannelsMessage, SettingsMessage};
use super::changes::{
    apply_bool_change, apply_number_change, apply_receive_mode_change, apply_text_change,
};
use super::persist::persist_channels_settings;
use super::toggles::{add_channel_instance, remove_channel_instance, toggle_enabled};

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub(super) fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Channels(message) = message else {
        return Task::none();
    };

    match message {
        ChannelsMessage::Refresh => {
            app.channels_settings.save_error = None;
            Task::none()
        }
        ChannelsMessage::TabSelected(tab) => {
            app.channels_settings.active_tab = tab;
            Task::none()
        }
        ChannelsMessage::CliToggled(value) => {
            app.channels_settings.cli = value;
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::ProjectDirChanged(value) => {
            app.channels_settings.project_dir_input = value;
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::MessageTimeoutSecsChanged(value) => {
            app.channels_settings.message_timeout_secs = value.max(1);
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::EnabledToggled(channel, enabled) => {
            toggle_enabled(app, &channel, enabled);
            if let Some(tab) = crate::app::state::ChannelsSettingsTab::from_channel_id(&channel) {
                app.channels_settings.active_tab = tab;
            }
            app.channels_settings.refresh_text_inputs();
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::InstanceAdded(channel) => {
            add_channel_instance(app, &channel);
            if let Some(tab) = crate::app::state::ChannelsSettingsTab::from_channel_id(&channel) {
                app.channels_settings.active_tab = tab;
            }
            app.channels_settings.refresh_text_inputs();
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::InstanceRemoved(channel, index) => {
            remove_channel_instance(app, &channel, index);
            app.channels_settings.refresh_text_inputs();
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::TextChanged(key, value) => {
            app.channels_settings.text_inputs.insert(key.clone(), value.clone());
            apply_text_change(app, &key, value);
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::TextEditorAction(key, action) => {
            let should_persist = matches!(action, text_editor::Action::Edit(_));
            if let Some(editor) = app.channels_settings.text_editors.get_mut(&key) {
                editor.perform(action);
                let value = editor.text();
                app.channels_settings.text_inputs.insert(key.clone(), value.clone());
                apply_text_change(app, &key, value);
            }
            if should_persist {
                persist_channels_settings(app).unwrap_or_else(Task::none)
            } else {
                Task::none()
            }
        }
        ChannelsMessage::BoolToggled(key, value) => {
            apply_bool_change(app, &key, value);
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::NumberChanged(key, value) => {
            apply_number_change(app, &key, value);
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
        ChannelsMessage::ReceiveModeChanged(key, value) => {
            apply_receive_mode_change(app, &key, value);
            persist_channels_settings(app).unwrap_or_else(Task::none)
        }
    }
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
