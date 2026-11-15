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
    Edit(message::edit::EditMessage),
    Icons(message::icons::Event),
    Endpoints(message::endpoints::Event),
    Env(message::env::Event),
    Desktop(message::desktop::Event),
    Binding,
    EndpointDiagnostics,
}

pub mod tray_switch;
