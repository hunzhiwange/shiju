//! Whole-page domain assertions migrated from the desktop.
mod acp_fixture;
mod acp_view;
mod catalog_fixture;
mod catalog_view;
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage
    for crate::message::Event
{
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyText(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        match effect {
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => {
                Self::CopyText(text)
            }
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url) => {
                Self::OpenUrlExternal(url)
            }
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text) => {
                Self::AppendText(text)
            }
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        unreachable!("plain help has no chat selection")
    }
}
fn selectable_help<'a>(
    i18n: desktop_i18n::I18n,
    theme: Option<&iced::Theme>,
    title: &'a str,
    label: iced::widget::Text<'a>,
    body: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, crate::message::Event> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
        label,
        desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
            title,
            body,
            theme.map(|theme| theme.palette().text),
            i18n,
        ),
    )
}
mod acp_inputs;
mod acp_ui;
mod agents_controller;
mod agents_controller_tests;
mod basic_ui;
mod catalog_transactions;
mod defaults;
mod dialogue_inputs;
mod dialogue_rollout;
mod dialogue_ui;
mod dialogue_update;
mod mcp;
mod page_states;
mod policy_inputs;
mod policy_ui;
mod profile_inputs;
mod profile_render;
mod profile_ui;
mod profile_update;
mod skills;
