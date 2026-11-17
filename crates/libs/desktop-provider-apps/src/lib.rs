//! Provider application configuration state and views.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentAppsContext {
    pub gateway: String,
    pub tenant: vw_api_transport::TenantScope,
    pub project: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub context: AgentAppsContext,
    pub host: String,
    pub app: vw_api_app_identity::AppId,
}

pub mod state;
pub mod message;
pub mod aggregation_desktop_types;
pub mod components;

#[derive(Clone)]
pub enum Message {
    ListEditor(components::list::EditorMessage),
    None,
    Mode(components::mode::Event),
    Paths(components::paths::Event),
    Preview(components::preview::Event),
    Snippets(components::snippets::Event),
    Universal(components::universal::Event),
    CopyCode(String),
    HelpContext(desktop_ui_controls::text_editor_context_menu::TextContextEffect),
    Edit(message::edit::EditMessage),
    Icons(message::icons::Event),
    Endpoints(message::endpoints::Event),
    Env(message::env::Event),
    Desktop(message::desktop::Event),
    Binding,
    EndpointDiagnostics,
}

pub mod tray_switch;

/// Feedback returned to the host for presentation.
pub enum Notice { Error(String), Success(String) }
fn error<E>(notice: &mut Option<Notice>, text: impl Into<String>) -> iced::Task<E> {
    *notice = Some(Notice::Error(text.into())); iced::Task::none()
}
fn success<E>(notice: &mut Option<Notice>, text: impl Into<String>) -> iced::Task<E> {
    *notice = Some(Notice::Success(text.into())); iced::Task::none()
}
pub mod auth;

#[cfg(test)]
mod host_module_28_tests;

/// Explicit native acceptance fixtures.
pub mod acceptance;
