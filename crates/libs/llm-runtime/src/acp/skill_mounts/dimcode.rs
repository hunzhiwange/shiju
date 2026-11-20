//! Project an active, validated skill view into the DimCode prompt metadata.
use super::super::{
    Error, host::SkillLease, model_source::profile::AcpLaunchProfile, to_api_error,
};
use std::collections::HashMap;

pub(in crate::acp) fn configure(
    profile: &mut AcpLaunchProfile,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    let mut environment = selection(view)?;
    environment.insert("SHIJU_ACP_DIMCODE_COEXISTENCE".into(), "1".into());
    profile.overlay_environment(environment).map_err(to_api_error)
}

fn selection(view: Option<&impl SkillLease>) -> Result<HashMap<String, String>, Error> {
    let Some(view) = view else {
        return Ok(HashMap::new());
    };
    view.validate()?;
    let root = view.root().canonicalize().map_err(to_api_error)?;
    let root = root
        .to_str()
        .ok_or_else(|| super::acp_option_error("DimCode skill view path must be UTF-8"))?;
    Ok(HashMap::from([("SHIJU_ACP_DIMCODE_SKILL_ROOT".into(), root.into())]))
}

#[cfg(test)]
#[path = "dimcode_tests.rs"]
mod tests;
