use super::super::{Error, acp_option_error, host::SkillLease};
use agent_config::schema::AcpAgentConfig;

pub(super) fn configure(
    config: &AcpAgentConfig,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    let Some(view) = view else {
        return Ok(());
    };
    if !cfg!(all(target_os = "macos", target_arch = "aarch64"))
        || !config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
        || config.args != ["acp"]
    {
        return Err(acp_option_error(
            "Nova skill mounts require the reviewed Darwin arm64 session-model runtime",
        ));
    }

    // The ACP client supplies the validated view as shiju.skillMounts.
    view.validate()?;
    acp_agent_registry::verify_reviewed_agent_executable("nova", &config.command)
        .map_err(acp_option_error)
}

#[cfg(test)]
#[path = "nova_tests.rs"]
mod tests;
