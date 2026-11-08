//! 处理聊天快捷消息的本地编辑与系统设置持久化。

use crate::app::config::update_system_settings_config_result_async;
use crate::app::{App, Message};
use iced::Task;
use vw_config_types::ui::{QuickMessageConfig, QuickMessageGroup, QuickMessageKind};

use super::{QuickMessagesMessage, SettingsMessage};

/// 更新快捷消息设置草稿，并在保存时写入系统设置。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::QuickMessages(message) = message else {
        return Task::none();
    };

    #[cfg(not(target_arch = "wasm32"))]
    if !matches!(&message, QuickMessagesMessage::Saved(_)) {
        app.note_startup_setting_edit("quick_messages");
    }
    match message {
        QuickMessagesMessage::Add => {
            app.quick_messages.push(QuickMessageConfig {
                text: "新快捷消息".to_string(),
                group: QuickMessageGroup::General,
                kind: QuickMessageKind::FillInput,
            });
            Task::none()
        }
        QuickMessagesMessage::Remove(index) => {
            if index < app.quick_messages.len() {
                app.quick_messages.remove(index);
            }
            Task::none()
        }
        QuickMessagesMessage::TextChanged(index, text) => {
            if let Some(item) = app.quick_messages.get_mut(index) {
                item.text = text;
            }
            Task::none()
        }
        QuickMessagesMessage::GroupChanged(index, group) => {
            if let Some(item) = app.quick_messages.get_mut(index) {
                item.group = group;
            }
            Task::none()
        }
        QuickMessagesMessage::KindChanged(index, kind) => {
            if let Some(item) = app.quick_messages.get_mut(index) {
                item.kind = kind;
            }
            Task::none()
        }
        QuickMessagesMessage::MoveUp(index) => {
            if index > 0 && index < app.quick_messages.len() {
                app.quick_messages.swap(index, index - 1);
            }
            Task::none()
        }
        QuickMessagesMessage::MoveDown(index) => {
            if let Some(next_index) = index.checked_add(1)
                && next_index < app.quick_messages.len()
            {
                app.quick_messages.swap(index, next_index);
            }
            Task::none()
        }
        QuickMessagesMessage::Save => {
            if app.quick_messages.iter().any(|item| item.text.trim().is_empty()) {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "快捷消息内容不能为空",
                    vec![],
                );
            }

            let quick_messages = app.quick_messages.clone();
            Task::perform(
                async move {
                    update_system_settings_config_result_async(move |config| {
                        config.quick_messages = quick_messages;
                    })
                    .await
                },
                |result| {
                    Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::Saved(
                        result,
                    )))
                },
            )
        }
        QuickMessagesMessage::Saved(Ok(())) => app.show_localized_toast(
            crate::app::state::ToastKind::Success,
            "快捷消息已保存",
            vec![],
        ),
        QuickMessagesMessage::Saved(Err(error)) => app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "保存快捷消息失败：%s",
            vec![error.into()],
        ),
    }
}

#[cfg(test)]
#[path = "quick_messages_tests.rs"]
mod quick_messages_tests;
