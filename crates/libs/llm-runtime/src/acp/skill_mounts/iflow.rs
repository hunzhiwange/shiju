//! Add a reviewed request-owned view to iFlow's native skill discovery.
use agent_config::schema::AcpAgentConfig;
use crate::acp::host::SkillLease;

const ROOT_ENV: &str = "SHIJU_IFLOW_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) {
        return Err(super::super::acp_option_error(
            "conflicting iFlow skill directory environment",
        ));
    }
    let root = if let Some(view) = view {
        view.validate()?;
        acp_agent_registry::verify_reviewed_agent_executable("iflow", &config.command)
            .map_err(super::super::to_api_error)?;
        view.root().to_string_lossy().into_owned()
    } else {
        // An explicit empty value masks inherited views while preserving native roots.
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    Ok(())
}

#[cfg(test)]
#[path = "iflow_tests.rs"]
mod tests;
