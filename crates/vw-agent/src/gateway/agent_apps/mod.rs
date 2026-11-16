//! Executor-owned project profile persistence.
pub(crate) use agent_gateway::agent_apps::profile_store as profile_store;

pub(crate) use agent_gateway::agent_apps::profile_operations as profile_operations;
#[cfg(target_os = "macos")]
pub(crate) use agent_gateway::agent_apps::profile_skills as profile_skills;
