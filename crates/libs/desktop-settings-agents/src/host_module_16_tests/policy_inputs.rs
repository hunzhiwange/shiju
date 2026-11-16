pub(super) use crate::autonomy_message::Message as Auto;
pub(super) use config_security::*;
pub(super) use desktop_settings_security::app_init::state::{
    AutonomyListKind, AutonomySettingsState, AutonomySettingsTab,
};
pub(super) use desktop_settings_security::security_message::Message as Secure;
use iced::Element;
#[derive(Debug, Clone)]
pub(super) enum Message {
    Autonomy(Auto),
    Security(Secure),
    None,
    CopyCode(String),
    OpenUrlExternal(String),
    AppendText(String),
}
#[derive(Default)]
pub(super) struct PolicyFixture {
    pub autonomy_settings: AutonomySettingsState,
    pub agents_settings: crate::agents_state::AgentsSettingsState,
    pub security_settings: desktop_settings_security::security_state::SecuritySettingsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: iced::Theme,
}
fn auto_event(message: Auto) -> Message {
    match message {
        Auto::None => Message::None,
        Auto::CopyCode(text) => Message::CopyCode(text),
        other => Message::Autonomy(other),
    }
}
fn secure_event(message: Secure) -> Message {
    match message {
        Secure::None => Message::None,
        Secure::CopyCode(text) => Message::CopyCode(text),
        other => Message::Security(other),
    }
}
fn inputs(app: &PolicyFixture) -> crate::autonomy_inputs::View<'_> {
    crate::autonomy_inputs::View {
        autonomy_settings: &app.autonomy_settings,
        available_tools: &app.agents_settings.available_tools,
        desktop_locale: app.desktop_locale,
        app_theme: app.app_theme,
        last_copied_code_hash: None,
        help_text: |i18n, theme, title, label, body| {
            desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
                label,
                desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<
                    Message,
                >(title, body, theme.map(|theme| theme.palette().text), i18n)
                .map(|event| match event {
                    Message::CopyCode(text) => Auto::CopyCode(text),
                    _ => Auto::None,
                }),
            )
        },
    }
}
pub(super) fn view(app: &PolicyFixture) -> Element<'_, Message> {
    crate::system_settings_autonomy::view(&inputs(app)).map(auto_event)
}
pub(super) fn view_overlays<'a>(
    app: &'a PolicyFixture,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    crate::system_settings_autonomy::view_overlays(&inputs(app), base, auto_event)
}
pub(super) trait Dispatch {
    fn dispatch(self, app: &mut PolicyFixture) -> bool;
}
impl Dispatch for Auto {
    fn dispatch(self, app: &mut PolicyFixture) -> bool {
        crate::autonomy_update::update(&mut app.autonomy_settings, self)
    }
}
impl Dispatch for Secure {
    fn dispatch(self, app: &mut PolicyFixture) -> bool {
        desktop_settings_security::security_update::update(&mut app.security_settings, self)
    }
}
pub(super) fn update(app: &mut PolicyFixture, message: impl Dispatch) -> bool {
    message.dispatch(app)
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
        unreachable!("plain settings help has no chat selection")
    }
}
pub(super) mod security {
    use super::*;
    fn input(app: &PolicyFixture) -> desktop_settings_security::SecurityView<'_> {
        desktop_settings_security::SecurityView {
            security_settings: &app.security_settings,
            desktop_locale: app.desktop_locale,
            app_theme: app.app_theme,
            last_copied_code_hash: None,
            help_text: |title, body, color, i18n| {
                desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<Message>(title, body, color, i18n).map(|event| match event { Message::CopyCode(text) => Secure::CopyCode(text), _ => Secure::None })
            },
        }
    }
    pub fn view(app: &PolicyFixture) -> Element<'_, Message> {
        desktop_settings_security::system_settings_security::view(&input(app)).map(secure_event)
    }
    pub fn view_overlays<'a>(
        app: &'a PolicyFixture,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        desktop_settings_security::system_settings_security::host_overlays(
            &input(app),
            base,
            secure_event,
        )
    }
}
