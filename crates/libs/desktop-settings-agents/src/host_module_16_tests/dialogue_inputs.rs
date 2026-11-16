pub(super) use crate::dialogue_flow_message::Message as Local;
use iced::Element;
#[derive(Default)]
pub(super) struct DialogueFixture {
    pub dialogue_flow_show_reasoning_summary: bool,
    pub dialogue_flow_expand_shell_tool_section: bool,
    pub dialogue_flow_expand_edit_tool_section: bool,
    pub dialogue_flow_settings_save_message: Option<String>,
    pub dialogue_flow_settings_translation: Option<desktop_i18n::format::Message>,
    pub quick_messages: Vec<config_ui::QuickMessageConfig>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: iced::Theme,
}
#[derive(Debug, Clone)]
pub(super) enum Message {
    Dialogue(Local),
    Quick(desktop_settings_appearance::quick_messages::Message),
    None,
    CopyCode(String),
    OpenUrlExternal(String),
    AppendText(String),
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect::*;
        match effect {
            Copy(text) => Self::CopyCode(text),
            Append(text) => Self::AppendText(text),
            OpenUrl(url) => Self::OpenUrlExternal(url),
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        unreachable!("settings help has no chat selection")
    }
}
pub(super) fn view(app: &DialogueFixture) -> Element<'_, Message> {
    crate::system_settings_dialogue_flow::view(&crate::dialogue_flow_inputs::View {
        dialogue_flow_show_reasoning_summary: app.dialogue_flow_show_reasoning_summary,
        dialogue_flow_expand_shell_tool_section: app.dialogue_flow_expand_shell_tool_section,
        dialogue_flow_expand_edit_tool_section: app.dialogue_flow_expand_edit_tool_section,
        dialogue_flow_settings_save_message: &app.dialogue_flow_settings_save_message,
        dialogue_flow_settings_translation: &app.dialogue_flow_settings_translation,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        help_text: |i18n, theme, title, label, body| {
            desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
                label,
                desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<
                    Message,
                >(title, body, theme.map(|theme| theme.palette().text), i18n)
                .map(|event| match event {
                    Message::CopyCode(text) => Local::CopyCode(text),
                    _ => Local::None,
                }),
            )
        },
    })
    .map(|event| match event {
        Local::None => Message::None,
        Local::CopyCode(text) => Message::CopyCode(text),
        other => Message::Dialogue(other),
    })
}
pub(super) mod quick {
    use super::*;
    use desktop_settings_appearance::quick_messages as domain;
    pub fn view(app: &DialogueFixture) -> Element<'_, Message> {
        domain::view(&domain::View { quick_messages: &app.quick_messages, desktop_locale: app.desktop_locale, app_theme: &app.app_theme },
            |title, body, color, i18n| desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<Message>(title, body, color, i18n).map(|event| match event { Message::CopyCode(text) => domain::Message::CopyCode(text), _ => domain::Message::None }))
            .map(|event| match event { domain::Message::None => Message::None, domain::Message::CopyCode(text) => Message::CopyCode(text), other => Message::Quick(other) })
    }
    pub fn apply(app: &mut DialogueFixture, message: domain::Message) {
        let domain::Message::TextChanged(index, value) = message else {
            panic!("expected the edited template event")
        };
        let _ = desktop_settings_appearance::quick_messages_update::update(
            &mut app.quick_messages,
            desktop_settings_appearance::quick_messages_update::QuickMessagesMessage::TextChanged(
                index, value,
            ),
        );
    }
}
