//! Register a validated selection with the reviewed Poolside session-model launcher.
use super::super::{host::SkillLease, model_source::profile::AcpLaunchProfile, to_api_error};
use super::Error;

pub(in crate::acp) fn configure(
    profile: &mut AcpLaunchProfile,
    enabled: bool,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    if let Some(root) = selection(enabled, view).map_err(to_api_error)? {
        profile
            .overlay_environment([("SHIJU_POOLSIDE_SKILL_VIEW".into(), root)].into())
            .map_err(to_api_error)?;
    }
    Ok(())
}

fn selection(enabled: bool, view: Option<&impl SkillLease>) -> anyhow::Result<Option<String>> {
    anyhow::ensure!(enabled || view.is_none(), "Poolside skills require enabled native tools");
    if !enabled {
        return Ok(None);
    }
    let Some(view) = view else {
        return Ok(Some(String::new()));
    };
    view.validate().map_err(|error| anyhow::anyhow!("{error}"))?;
    let root = view.root().canonicalize()?;
    Ok(Some(
        root.into_os_string()
            .into_string()
            .map_err(|_| anyhow::anyhow!("Poolside skill view requires a UTF-8 path"))?,
    ))
}

#[cfg(test)]
#[path = "poolside_tests.rs"]
mod tests;
