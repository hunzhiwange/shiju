
pub(crate) use desktop_settings_network::app_init::state::{HttpRequestSettingsState};

pub(crate) use desktop_settings_agents::acp_state::AcpSettingsState;

pub use desktop_settings_mcp::state::McpSettingsTab;

pub(crate) use desktop_settings_mcp::management_state::McpSettingsState;

pub use desktop_settings_security::security_state::*;

pub(crate) use desktop_settings_security::app_init::state::AutonomySettingsState;

/// 存储设置面板状态
///
/// 管理持久化存储 provider、连接地址、schema、table 与 TLS 配置。
pub(crate) use desktop_settings_memory::state::StorageSettingsState;



/// 代理设置面板状态
///
/// 管理网络代理的配置，包括 HTTP/HTTPS 代理和作用域等。
pub(crate) use desktop_settings_network::app_init::state::{ProxySettingsState, TunnelSettingsState};

pub use desktop_settings_appearance::system_settings_themes::state::*;



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

#[cfg(test)]
use super::*;
#[cfg(test)]
pub use desktop_settings_mcp::state::McpNewServerState;
#[cfg(test)]
pub use desktop_settings_mcp::state::McpNewServerTransport;
#[cfg(test)]
pub(crate) use desktop_settings_security::app_init::state::AutonomyListEditorState;
#[cfg(test)]
pub(crate) use desktop_settings_security::app_init::state::AutonomyListKind;
#[cfg(test)]
pub(crate) use desktop_settings_security::app_init::state::AutonomySettingsTab;
