//! Pi uses --no-skills with an explicit view; the ACP catalog uses the same root.
use crate::config::schema::AcpAgentConfig;

pub(super) const ROOT_ENV: &str = "SHIJU_PI_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) || std::env::var_os(ROOT_ENV).is_some() {
        return Err(super::super::acp_option_error("conflicting Pi skill directory environment"));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        vw_acp::verify_reviewed_agent_executable("pi", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    Ok(())
}
#[cfg(test)]
#[path = "pi_tests.rs"]
mod tests;
