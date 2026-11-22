//! ACP 代理注册、受审运行时验证与安装。

pub mod agent_registry;
#[cfg(not(target_arch = "wasm32"))]
mod install;
mod managed_runtime;

pub use agent_registry::{
    AgentCommandSpec, DEFAULT_AGENT_NAME, EMPLOYEE_ACP_BACKEND, EMPLOYEE_ACP_PROFILE,
    EMPLOYEE_ACP_CREDENTIAL_ENV, built_in_agent_registry, built_in_agent_specs,
    list_built_in_agents, merge_agent_registry, merge_agent_specs, normalize_agent_name,
    resolve_agent_command, resolve_agent_spec, resolve_agent_spec_with_overrides,
};
#[cfg(not(target_arch = "wasm32"))]
pub use install::install_builtin_agent;
pub use managed_runtime::{reviewed_agent_command, verify_auggie_restricted_executable, verify_droid_restricted_executable, verify_openclaw_mcp_executable, verify_openclaw_restricted_executable, verify_reviewed_agent_executable, verify_vtcode_mcp_executable, verify_vtcode_skill_executable};

#[cfg(all(test, not(target_arch = "wasm32")))]
static TEST_HOME_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
