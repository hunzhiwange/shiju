#[cfg(not(target_arch = "wasm32"))]
mod channel_reload;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use channel_reload::ChannelConfigWatcher;

pub mod automation;
pub mod channels;
pub mod provider;
pub mod proxy;
pub mod security;
pub mod skills;
pub use config_tools as tools;
pub use config_ui as ui;

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
#[path = "../../../../libs/agent-config/src/schema/config_load_tests.rs"]
mod config_load_tests;
#[cfg(test)]
#[path = "../../../../libs/agent-config/src/schema/config_save_tests.rs"]
mod config_save_tests;
#[cfg(test)]
#[path = "../../../../libs/agent-config/src/schema/config_tests.rs"]
mod config_tests;
#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;

// Re-export domain-owned configuration types for backward compatibility.
pub use config_agent as agent;
pub use config_gateway as gateway;
pub use config_hooks as hooks;
pub use config_memory as memory;
pub use config_observability as observability;
pub use config_reliability as reliability;
pub use config_routing as routing;
pub use config_runtime as runtime;

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
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use config_load::TenantConfigSource;
pub(crate) use config_load::load_or_init_config;
pub use config_load::load_or_init_config_for_tenant;
pub(crate) use config_load::load_or_init_config_for_tenant_without_env;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use config_save::encode_config_for_storage;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use config_save::save_config_for_tenant;
pub(crate) use config_save::{
    save_config, update_project_config_for_tenant, update_project_config_from_configs,
};
pub(crate) use config_secrets::{decrypt_config_secrets, encrypt_config_secrets};
pub(crate) use config_validate::{
    seed_agent_profile_patch, validate_agent_profile_patch, validate_config,
};
pub use workspace::default_config_and_workspace_dirs;
pub(crate) use workspace::resolve_runtime_config_dirs;

pub use agent_config::schema::CONFIG_AGENT_KEY;
