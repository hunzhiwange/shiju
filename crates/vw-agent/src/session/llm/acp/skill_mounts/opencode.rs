//! OpenCode's reviewed native scanner reads only the request-owned skill view.
use crate::config::schema::AcpAgentConfig;

const ROOT_ENV: &str = "SHIJU_OPENCODE_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) || std::env::var_os(ROOT_ENV).is_some() {
        return Err(super::super::acp_option_error(
            "conflicting OpenCode skill directory environment",
        ));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        vw_acp::verify_reviewed_agent_executable("opencode", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        // An empty selection disables all native skill levels in the reviewed scanner.
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    config.env.insert("OPENCODE_PURE".into(), "1".into());
    Ok(())
}

#[cfg(test)]
#[path = "opencode_tests.rs"]
mod tests;
