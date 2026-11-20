//! Add the request-owned skill view to Qwen’s native discovery roots.
use agent_config::schema::AcpAgentConfig;

const ROOT_ENV: &str = "SHIJU_QWEN_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) {
        return Err(super::super::acp_option_error("conflicting Qwen skill directory environment"));
    }
    if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        acp_agent_registry::verify_reviewed_agent_executable("qwen", &config.command)
            .map_err(super::super::to_api_error)?;
        config.env.insert(ROOT_ENV.into(), view.0.root().to_string_lossy().into_owned());
    }
    Ok(())
}

#[cfg(test)]
#[path = "qwen_tests.rs"]
mod tests;
