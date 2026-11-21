//! Reviewed Copilot ACP boot options append the request-owned skill view.
use agent_config::schema::AcpAgentConfig;
use crate::acp::model_source::{drivers::copilot::PROJECT_SKILLS_ENV, profile::REFERENCE_ENV};

const ROOT_ENV: &str = "SHIJU_COPILOT_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if [ROOT_ENV, "COPILOT_SKILLS_DIRS", "COPILOT_CLI_DIST_DIR"]
        .iter()
        .any(|key| config.env.contains_key(*key))
    {
        return Err(super::super::acp_option_error(
            "conflicting Copilot skill directory environment",
        ));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        acp_agent_registry::verify_reviewed_agent_executable("copilot", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    // The managed model driver has already projected the authorized settings.
    // Mask an inherited opt-in on launches without a host grant.
    if config.env.contains_key(REFERENCE_ENV) {
        config.env.entry(PROJECT_SKILLS_ENV.into()).or_default();
    } else {
        config.env.insert(PROJECT_SKILLS_ENV.into(), String::new());
    }
    // Explicit roots remain independent from an ambient native CLI directory list.
    config.env.insert("COPILOT_SKILLS_DIRS".into(), String::new());
    Ok(())
}

#[cfg(test)]
#[path = "copilot_tests.rs"]
mod tests;
