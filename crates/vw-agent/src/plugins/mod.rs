//! 插件包的发现、校验、安装与运行时集成入口。
//!
//! 子模块分别维护路径审计、钩子信任、清单解析、MCP 与技能加载边界；
//! 本模块只重导出经过这些边界约束的稳定能力。

mod audit;
mod cli;
mod hook_handler;
mod hook_trust;
mod hooks;
mod install;
mod loader;
mod manifest;
mod mcp;
mod skills;
mod types;

mod compat;
pub(crate) use compat::mcp_auth::runtime_headers as plugin_mcp_http_headers;
pub(crate) use compat::mcp_capabilities::{
    client_info as mcp_client_info, validate_server_protocol as validate_mcp_server_protocol,
};
pub(crate) use compat::mcp_connection_sessions::{
    close_session as close_session_mcp_connections, connections as mcp_session_connections,
};
pub(crate) use compat::mcp_connection_settings::settings as plugin_mcp_connection_settings;
pub(crate) use compat::mcp_connections::McpSessionConnections;
pub(crate) use compat::mcp_content::{normalize_mcp_tool_result, preserve_mcp_wire_result};
pub(crate) use compat::mcp_elicitation;
pub(crate) use compat::mcp_notifications::{McpCatalogs, ToolCatalogBinding};
pub(crate) use compat::mcp_resources::{RawResponse as RawMcpResponse, call as call_mcp_resource};

/// 插件管理命令与命令行执行入口。
pub use crate::cli_commands::PluginCommands;
pub use cli::handle_command;
pub use compat::hook_manifest::{
    CodexHookDiagnostic, CodexHookEvent, CodexHookHandler, CodexHookMatcherGroup,
    CodexHooksManifest,
};
pub(crate) use compat::mcp_environment::runtime_cwd as plugin_mcp_runtime_cwd;
pub(crate) use compat::skill_invocation::reference_text as plugin_skill_reference_text;
pub(crate) use hook_handler::trusted_plugin_hook_handlers;
pub use hook_trust::{
    PluginHookTrustStatus, PluginHookTrustStore, plugin_hook_trust_file, plugin_hook_trust_hash,
};
pub use hooks::{
    PluginHook, PluginHookEvent, PluginHookMode, PluginHooksManifest, load_plugin_hook_source,
    load_plugin_hooks_manifest,
};
pub use install::{
    PluginInstallResult, install_plugin_from_marketplace, install_plugin_from_source,
    install_plugin_marketplace_entry,
};
pub(crate) use install::{install_dir_from_config, remove_installed_plugin, set_plugin_enabled};
pub use loader::{
    DiscoveredPlugin, PluginDiscovery, PluginDiscoveryDiagnostic, PluginDiscoveryOptions,
    discover_plugins,
};
pub use manifest::load_plugin_manifest;
pub(crate) use mcp::{merge_enabled_plugin_mcp_servers, plugin_mcp_discovery_options};
pub(crate) use skills::load_plugin_skills;
/// 已完成路径审计和字段规范化的插件清单类型。
pub use types::{
    PluginAuthor, PluginAuthorDetails, PluginHookSource, PluginIdentity, PluginInstallMetadata,
    PluginInterface, PluginManifest, PluginMcpServers, PluginPath,
};

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod manifest_tests;

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod hooks_tests;

#[cfg(test)]
#[path = "hook_trust_tests.rs"]
mod hook_trust_tests;

#[cfg(test)]
#[path = "hook_handler_tests.rs"]
mod hook_handler_tests;

#[cfg(test)]
#[path = "install_tests.rs"]
mod install_tests;

#[cfg(test)]
#[path = "cli_tests.rs"]
mod cli_tests;

#[cfg(test)]
#[path = "skills_tests.rs"]
mod skills_tests;

pub(crate) use compat::mcp_catalog::list_tools as list_mcp_catalog_tools;

pub(crate) use compat::mcp_connection_settings::tool_allowed as plugin_mcp_tool_allowed;

pub use compat::mcp_auth::discover_oauth as discover_plugin_mcp_oauth;
pub use compat::mcp_oauth_discovery::OAuthDiscovery;
pub use compat::mcp_oauth_flow::authorize as authorize_plugin_mcp;

pub use compat::mcp_oauth_tokens::logout as logout_plugin_mcp;

pub(crate) use compat::mcp_oauth_tokens::access as plugin_mcp_oauth_access;

pub(crate) use compat::mcp_auth::runtime_oauth_headers as plugin_mcp_oauth_headers;

pub(crate) use compat::mcp_apps;

pub(crate) use compat::mcp_app_execution;

pub(crate) use compat::hook_async::AsyncHooks;

pub(crate) use compat::hook_compact;
pub(crate) use compat::hook_model;
pub(crate) use compat::hook_prompt;
pub(crate) use compat::hook_session;
pub(crate) use compat::hook_subagent;

#[cfg(test)]
pub(crate) use compat::t41_tests;

#[cfg(test)]
pub(crate) use compat::t42_tests;

pub(crate) use compat::hook_stop;

#[cfg(test)]
pub(crate) use compat::t44_tests;

pub use compat::marketplace::{
    AuthenticationPolicy, InstallationPolicy, MarketplaceEntry, MarketplacePolicy,
    MarketplaceSource, PluginMarketplace, load_plugin_marketplace,
};

pub use compat::npm_source::{NpmPluginArchive, fetch_npm_plugin};

pub use compat::upgrade::{PluginUpgradePreview, activate_plugin_upgrade, prepare_plugin_upgrade};

pub use compat::marketplace::{
    MarketplaceRegistration, add_plugin_marketplace, list_plugin_marketplaces,
    read_registered_marketplace, registered_plugin_entry,
};
pub use compat::upgrade::prepare_plugin_marketplace_upgrade;
