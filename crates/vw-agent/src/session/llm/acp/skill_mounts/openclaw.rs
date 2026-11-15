//! The reviewed scanner runs in the owned Gateway, before the ACP bridge starts.
use crate::config::schema::AcpAgentConfig;

pub(super) const ROOT_ENV: &str = "SHIJU_OPENCLAW_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) || std::env::var_os(ROOT_ENV).is_some() {
        return Err(super::super::acp_option_error(
            "conflicting OpenClaw skill directory environment",
        ));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        vw_acp::verify_reviewed_agent_executable("openclaw", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    Ok(())
}
#[cfg(test)]
#[path = "openclaw_tests.rs"]
mod tests;
