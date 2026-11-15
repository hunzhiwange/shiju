pub use agent_config::schema::config_validate::{normalize_wire_api, validate_config, validate_agent_profile_patch};

pub(crate) fn seed_agent_profile_patch(current: &super::Config, patch: &mut serde_json::Value) -> anyhow::Result<()> {
    let catalog = crate::agent::catalog::merged_agent_catalog(&current.agents);
    agent_config::schema::config_validate::seed_agent_profile_patch(current, patch, &catalog)
}
