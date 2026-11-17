//! Executor-owned project profile persistence.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use agent_gateway::agent_apps::profile_store as profile_store;

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use agent_gateway::agent_apps::profile_operations as profile_operations;
#[cfg(target_os = "macos")]
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use agent_gateway::agent_apps::profile_skills as profile_skills;
