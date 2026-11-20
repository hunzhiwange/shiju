//! Register only the host-validated selection in the managed siGit scanner.
use super::super::{
    Error, host::SkillLease, model_source::profile::AcpLaunchProfile, to_api_error,
};
use std::{collections::HashMap, fs};

pub(in crate::acp) fn configure(
    profile: &mut AcpLaunchProfile,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    let roots = selection(view).map_err(to_api_error)?;
    profile
        .overlay_environment(HashMap::from([("SIGIT_MANAGED_SKILL_DIRS".into(), roots)]))
        .map_err(to_api_error)
}

fn selection(view: Option<&impl SkillLease>) -> anyhow::Result<String> {
    let Some(view) = view else {
        return Ok("[]".into());
    };
    view.validate()?;
    let root = view.root().canonicalize()?;
    let mut sources = fs::read_dir(root)?
        .map(|entry| {
            let link = entry?.path();
            let target = fs::read_link(&link)?;
            Ok((link, target))
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    sources.sort();
    view.validate()?;
    Ok(serde_json::to_string(&sources)?)
}

#[cfg(test)]
#[path = "sigit_tests.rs"]
mod tests;
