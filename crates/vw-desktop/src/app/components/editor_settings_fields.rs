//! 编辑器设置视图的桌面状态投影和跨域消息映射。
use crate::app::{App, Message, message::editor::{EditorMessage, metrics}};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::message::PreviewMessage;
use desktop_editor::components::editor_settings_fields::{self as fields, Message as Action, SettingsView};

pub(crate) fn view(app: &App, control_width: f32) -> iced::Element<'_, Message> {
    let metrics = metrics::displayed(app);
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let color = app.appearance.app_theme.palette().text;
    fields::view(SettingsView {
        locale: app.general_settings.desktop_locale,
        editor_theme: app.effective_editor_theme(), follow_system_theme: app.appearance.editor_follow_system_theme,
        font_size: metrics.font_size, line_height: metrics.line_height, auto_line_height: metrics.auto_line_height,
        language: app.preview_state.current_language,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_disabled: app.preview_state.lsp_disabled,
    }, control_width, move |title, description| {
        super::chat_panel::message_view::selectable_titled_text(title, description, Some(color), i18n)
    }).map(map_message)
}

pub(crate) fn map_message(action: Action<Message>) -> Message {
    let editor = match action {
        Action::Help(message) => return message,
        Action::ThemeChanged(value) => EditorMessage::ThemeChanged(value),
        Action::ToggleFollowSystemTheme(value) => EditorMessage::ToggleFollowSystemTheme(value),
        Action::FontSizePreview(value) => EditorMessage::FontSizePreview(value),
        Action::LineHeightPreview(value) => EditorMessage::LineHeightPreview(value),
        Action::ToggleAutoLineHeight(value) => EditorMessage::ToggleAutoLineHeight(value),
        Action::CommitMetrics => EditorMessage::CommitMetrics,
        Action::LanguageChanged(value) => EditorMessage::LanguageChanged(value),
        #[cfg(not(target_arch = "wasm32"))]
        Action::LspToggleDisabled(value) => return Message::Preview(PreviewMessage::LspToggleDisabled(value)),
        #[cfg(not(target_arch = "wasm32"))]
        Action::CopyLspCommand(server) => EditorMessage::CopyLspCommand(server),
        #[cfg(not(target_arch = "wasm32"))]
        Action::InstallLsp(server) => EditorMessage::InstallLsp(server),
    };
    Message::Editor(editor)
}
