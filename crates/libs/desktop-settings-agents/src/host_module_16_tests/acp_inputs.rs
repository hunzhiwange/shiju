pub(super) use crate::message::AcpMessage;
pub(super) use desktop_settings_mcp::page::McpMessage;
use iced::Element;

pub(super) struct AcpPageFixture {
    pub acp_settings: crate::acp_state::AcpSettingsState,
    pub mcp_settings: desktop_settings_mcp::state::State,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: iced::Theme,
}
#[derive(Debug, Clone)]
pub(super) enum Message {
    Acp(AcpMessage),
    Mcp(McpMessage),
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
        unreachable!("plain settings help has no chat selection")
    }
}
fn event(value: crate::message::Event) -> Message {
    match value {
        crate::message::Event::Acp(value) => Message::Acp(value),
        crate::message::Event::CopyText(value) => Message::CopyCode(value),
        crate::message::Event::OpenUrlExternal(value) => Message::OpenUrlExternal(value),
        crate::message::Event::AppendText(value) => Message::AppendText(value),
        crate::message::Event::None => Message::None,
        _ => unreachable!("ACP emits only its own events"),
    }
}
pub(super) fn view(app: &AcpPageFixture) -> Element<'_, Message> {
    crate::acp_view::view(crate::acp_view::Input {
        help: super::selectable_help,
        state: &app.acp_settings,
        locale: app.desktop_locale,
        theme: &app.app_theme,
        frame: 0,
        icon: |agent, size| desktop_ui_controls::icons::acp_agent_icon(agent, size),
    })
    .map(event)
}
fn help<'a>(
    i18n: desktop_i18n::I18n,
    theme: &'a iced::Theme,
    title: &'a str,
    label: iced::widget::Text<'a>,
    body: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, Message> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
        label,
        desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
            title,
            body,
            Some(theme.palette().text),
            i18n,
        ),
    )
}
pub(super) mod mcp {
    use super::*;
    pub fn view(app: &AcpPageFixture) -> Element<'_, Message> {
        // Independent management panels are external content slots, not part of the configuration form under test.
        desktop_settings_mcp::page::view(
            Message::Mcp,
            &app.mcp_settings,
            app.desktop_locale,
            &app.app_theme,
            std::array::from_fn(|_| iced::widget::Space::new().into()),
            help,
        )
    }
    pub fn view_overlays<'a>(
        app: &'a AcpPageFixture,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        desktop_settings_mcp::page::view_overlays(
            Message::Mcp,
            &app.mcp_settings,
            app.desktop_locale,
            &app.app_theme,
            help,
            base,
        )
    }
}
pub(super) mod settings {
    use super::*;
    pub fn update(app: &mut AcpPageFixture, event: Message) {
        match event {
            Message::Mcp(event) => {
                let _ = desktop_settings_mcp::configuration_update::update(
                    &mut app.mcp_settings,
                    update_message(event),
                );
            }
            Message::Acp(AcpMessage::CancelEditor) => {
                let mut agents = Vec::new();
                let mut selected = None;
                let mut input = crate::acp_update::Update {
                    state: &mut app.acp_settings,
                    acp_agents: &mut agents,
                    acp_agent: &mut selected,
                    access: desktop_config::ConfigAccess::new(
                        gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
                        None,
                    ),
                    client: Err("offline fixture".into()),
                    snapshot_applied: false,
                    toasts: Vec::new(),
                };
                let _ = crate::acp_update::update(
                    &mut input,
                    crate::acp_update_message::AcpMessage::CancelEditor,
                );
            }
            other => panic!("unexpected configuration edit {other:?}"),
        }
    }
}
fn update_message(event: McpMessage) -> desktop_settings_mcp::configuration_update::Message {
    use desktop_settings_mcp::configuration_update::Message as U;
    match event {
        McpMessage::TabSelected(v0) => U::TabSelected(v0),
        McpMessage::Refresh => U::Refresh,
        McpMessage::InspectRefresh => U::InspectRefresh,
        McpMessage::EditorAction(v0) => U::EditorAction(v0),
        McpMessage::Format => U::Format,
        McpMessage::InsertTemplate => U::InsertTemplate,
        McpMessage::NewServerPanelToggled => U::NewServerPanelToggled,
        McpMessage::NewServerNameChanged(v0) => U::NewServerNameChanged(v0),
        McpMessage::NewServerTransportChanged(v0) => U::NewServerTransportChanged(v0),
        McpMessage::NewServerCommandChanged(v0) => U::NewServerCommandChanged(v0),
        McpMessage::NewServerArgsChanged(v0) => U::NewServerArgsChanged(v0),
        McpMessage::NewServerUrlChanged(v0) => U::NewServerUrlChanged(v0),
        McpMessage::NewServerTimeoutChanged(v0) => U::NewServerTimeoutChanged(v0),
        McpMessage::NewServerEnabledToggled(v0) => U::NewServerEnabledToggled(v0),
        McpMessage::NewServerRequiredToggled(v0) => U::NewServerRequiredToggled(v0),
        McpMessage::AddServer => U::AddServer,
        McpMessage::ServerEditRequested(v0) => U::ServerEditRequested(v0),
        McpMessage::ServerEditCanceled => U::ServerEditCanceled,
        McpMessage::SaveServerEdit => U::SaveServerEdit,
        McpMessage::ServerEnabledToggled(v0, v1) => U::ServerEnabledToggled(v0, v1),
        McpMessage::RuntimeServerEnabledToggled(v0, v1) => U::RuntimeServerEnabledToggled(v0, v1),
        McpMessage::PreviewExpandedToggled(v0) => U::PreviewExpandedToggled(v0),
        McpMessage::RequestDelete(v0) => U::RequestDelete(v0),
        McpMessage::ConfirmDelete(v0) => U::ConfirmDelete(v0),
        McpMessage::CancelDelete => U::CancelDelete,
        McpMessage::Save => U::Save,
    }
}

impl Default for AcpPageFixture { fn default() -> Self { Self { acp_settings: Default::default(), mcp_settings: Default::default(), desktop_locale: Default::default(), app_theme: iced::Theme::Light } } }
