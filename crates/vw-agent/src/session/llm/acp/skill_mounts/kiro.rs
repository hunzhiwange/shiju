//! Kiro's reviewed native scanner reads only the request-owned skill view.
use crate::config::schema::AcpAgentConfig;

const ROOT_ENV: &str = "SHIJU_KIRO_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if [ROOT_ENV, "KIRO_KAS_SERVER_PATH", "KIRO_KAS_NODE_PATH", "KAS_BUNDLE_PATH"]
        .iter()
        .any(|key| config.env.contains_key(*key))
    {
        return Err(super::super::acp_option_error("conflicting Kiro skill directory environment"));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        vw_acp::verify_reviewed_agent_executable("kiro", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        // An empty selection disables all native skill levels in the reviewed scanner.
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    Ok(())
}

#[cfg(test)]
#[path = "kiro_tests.rs"]
mod tests;
