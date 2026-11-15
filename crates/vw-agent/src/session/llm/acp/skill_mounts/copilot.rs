//! Reviewed Copilot ACP boot options select only the request-owned skill view.
use crate::config::schema::AcpAgentConfig;

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
        vw_acp::verify_reviewed_agent_executable("copilot", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    // Explicit roots remain independent from an ambient native CLI directory list.
    config.env.insert("COPILOT_SKILLS_DIRS".into(), String::new());
    Ok(())
}

#[cfg(test)]
#[path = "copilot_tests.rs"]
mod tests;
