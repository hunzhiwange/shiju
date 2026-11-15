//! 桌面只负责知识工作台的状态注入与跨域消息映射。
use crate::app::{App, Message};
use desktop_expert_hub::views::knowledge_workspace::{self as workspace, ViewInput};
pub fn view(app: &App) -> iced::Element<'_, Message> {
    workspace::view(ViewInput {
        knowledge: &app.knowledge, desktop_locale: app.desktop_locale, app_theme: &app.app_theme,
        files_dragging: !app.file_tree_interaction.dragging_file_paths.is_empty()
            || !app.file_tree_interaction.pending_drop_file_paths.is_empty(),
        last_copied_code_hash: app.last_copied_code_hash,
    }).map(|message| match message {
        workspace::Message::Knowledge(value) => Message::Knowledge(value),
        workspace::Message::OpenMemorySettings => Message::View(crate::app::message::ViewMessage::OpenSystemSettingsTab(crate::app::components::system_settings::SystemTab::Memory)),
        workspace::Message::CopyCode(value) => Message::CopyCode(value),
        workspace::Message::AppendToInput(value) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(value)),
        workspace::Message::View(workspace::ViewMessage::OpenUrlExternal(url)) => Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url)),
        workspace::Message::None => Message::None,
        workspace::Message::Selection { session_id, message_idx, text_idx, text } => Message::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text },
    })
}
