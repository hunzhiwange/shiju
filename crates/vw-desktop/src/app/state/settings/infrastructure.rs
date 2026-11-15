use super::*;

pub(crate) use desktop_settings_network::app_init::state::{HttpRequestSettingsState};

pub(crate) use desktop_settings_agents::acp_state::AcpSettingsState;

pub use desktop_settings_mcp::state::{McpNewServerState, McpNewServerTransport, McpSettingsTab};

#[derive(Debug, Clone)]
pub(crate) struct McpSettingsState {
    pub(crate) tools: crate::app::message::settings::mcp_management_tools::ToolsState,
    pub(crate) diagnostics: crate::app::message::agent_apps_diagnostics::McpState,
    pub(crate) catalog: crate::app::message::settings::mcp_management_list::CatalogState,
    pub(crate) resync: crate::app::message::settings::mcp_management_resync::ResyncState,
    pub(crate) bindings: crate::app::message::settings::mcp_management_bindings::BindingsState,
    pub(crate) import: Option<crate::app::message::settings::mcp_management_import::ImportState>,
    pub(crate) connection_form: Option<crate::app::message::settings::mcp_management_form::Draft>,
    pub(crate) page: desktop_settings_mcp::state::State,
}

impl Default for McpSettingsState {
    fn default() -> Self {
        Self {
            diagnostics: Default::default(),
            tools: Default::default(),
            catalog: Default::default(),
            bindings: Default::default(),
            resync: Default::default(),
            import: None,
            connection_form: None,
            page: Default::default(),
        }
    }
}

// Preserve existing callers until the update slices migrate; state has one owner.
impl std::ops::Deref for McpSettingsState {
    type Target = desktop_settings_mcp::state::State;
    fn deref(&self) -> &Self::Target { &self.page }
}
impl std::ops::DerefMut for McpSettingsState {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.page }
}

pub use desktop_settings_security::security_state::*;

pub(crate) use desktop_settings_security::app_init::state::{AutonomyListKind, AutonomyListEditorState, AutonomySettingsState, AutonomySettingsTab};

/// 存储设置面板状态
///
/// 管理持久化存储 provider、连接地址、schema、table 与 TLS 配置。
pub(crate) use desktop_settings_memory::state::StorageSettingsState;



/// 代理设置面板状态
///
/// 管理网络代理的配置，包括 HTTP/HTTPS 代理和作用域等。
pub(crate) use desktop_settings_network::app_init::state::{ProxySettingsState, TunnelSettingsState};

pub use desktop_settings_appearance::system_settings_themes::state::*;

#[cfg(test)]
#[path = "infrastructure_tests.rs"]
mod infrastructure_tests;

pub(crate) use desktop_settings_runtime::state::{MultimodalSettingsState, ObservabilitySettingsState};

#[cfg(test)]
pub(crate) const AUTONOMY_LIST_KINDS: [AutonomyListKind; 7] = [
    AutonomyListKind::AllowedCommands,
    AutonomyListKind::ForbiddenPaths,
    AutonomyListKind::ShellEnvPassthrough,
    AutonomyListKind::AutoApprove,
    AutonomyListKind::AlwaysAsk,
    AutonomyListKind::AllowedRoots,
    AutonomyListKind::NonCliExcludedTools,
];
