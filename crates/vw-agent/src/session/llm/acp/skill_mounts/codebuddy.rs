//! CodeBuddy's reviewed native scanner reads only the request-owned skill view.
use crate::config::schema::AcpAgentConfig;

const ROOT_ENV: &str = "SHIJU_CODEBUDDY_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) {
        return Err(super::super::acp_option_error(
            "conflicting CodeBuddy skill directory environment",
        ));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        vw_acp::verify_reviewed_agent_executable("codebuddy", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        // An empty selection disables all native skill levels in the reviewed scanner.
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    config.env.insert("CODEBUDDY_FORCE_HEADLESS_BUNDLE".into(), "1".into());
    config.env.insert("CODEBUDDY_FORCE_LITE_WB_BUNDLE".into(), "0".into());
    Ok(())
}

#[cfg(test)]
#[path = "codebuddy_tests.rs"]
mod tests;
