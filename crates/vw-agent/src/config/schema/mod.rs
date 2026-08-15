pub mod automation;
pub mod channels;
pub mod provider;
pub mod proxy;
pub mod security;
pub mod skills;
pub use vw_config_types::tools;
pub use vw_config_types::ui;

mod config;
mod config_env;
mod config_helpers;
mod config_io;
pub(crate) mod config_load;
mod config_save;
mod config_secrets;
mod config_validate;
pub(crate) mod workspace;

#[cfg(test)]
#[path = "config_env_tests.rs"]
mod config_env_tests;
#[cfg(test)]
#[path = "config_helpers_tests.rs"]
mod config_helpers_tests;
#[cfg(test)]
#[path = "config_io_tests.rs"]
mod config_io_tests;
#[cfg(test)]
#[path = "config_load_tests.rs"]
mod config_load_tests;
#[cfg(test)]
#[path = "config_save_tests.rs"]
mod config_save_tests;
#[cfg(test)]
#[path = "config_secrets_tests.rs"]
mod config_secrets_tests;
#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
#[cfg(test)]
#[path = "config_validate_tests.rs"]
mod config_validate_tests;
#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;

// Tier-1 types migrated to vw-config-types; re-export for backward compatibility
pub use vw_config_types::agent;
pub use vw_config_types::gateway;
pub use vw_config_types::hooks;
pub use vw_config_types::memory;
pub use vw_config_types::observability;
pub use vw_config_types::reliability;
pub use vw_config_types::routing;
pub use vw_config_types::runtime;

// Re-export types for backward compatibility and ease of use
pub use agent::*;
pub use automation::*;
pub use channels::*;
pub use gateway::*;
pub use hooks::*;
pub use memory::*;
pub use observability::*;
pub use provider::*;
pub use proxy::*;
pub use reliability::*;
pub use routing::*;
pub use runtime::*;
pub use security::*;
pub use skills::*;
pub use tools::*;
pub use ui::*;

pub use config::{AcpAgentConfig, Config, ConfigExt};
pub(crate) use config_env::apply_env_overrides;
pub(crate) use config_io::parse_config_contents;
pub(crate) use config_load::load_or_init_config;
pub use config_load::load_or_init_config_for_tenant;
pub(crate) use config_load::load_or_init_config_for_tenant_without_env;
pub(crate) use config_save::{
    save_config, save_config_for_tenant, update_project_config_for_tenant,
    update_project_config_from_configs,
};
pub(crate) use config_secrets::{decrypt_config_secrets, encrypt_config_secrets};
pub(crate) use config_validate::validate_config;
pub use workspace::default_config_and_workspace_dirs;
pub(crate) use workspace::resolve_runtime_config_dirs;

/// 旧用户配置迁移来源文件名。
const LEGACY_CONFIG_JSON_FILENAME: &str = "vibewindow.json";
pub const CONFIG_AGENT_KEY: &str = "agent";
