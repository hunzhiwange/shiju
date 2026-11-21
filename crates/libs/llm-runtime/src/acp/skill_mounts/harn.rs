//! Publish only the request-owned Harn skill selection.
use super::super::host::SkillLease;
use super::super::model_source::profile::AcpLaunchProfile;
use super::Error;
use std::collections::HashMap;
use std::path::Path;

pub(in crate::acp) fn configure<S: SkillLease>(
    profile: &mut AcpLaunchProfile,
    enabled: bool,
    view: Option<&S>,
) -> Result<(), Error> {
    let env = selection(profile.directory(), enabled, view).map_err(super::super::to_api_error)?;
    profile.overlay_environment(env).map_err(super::super::to_api_error)
}

fn selection<S: SkillLease>(
    launch_dir: &Path,
    enabled: bool,
    view: Option<&S>,
) -> anyhow::Result<HashMap<String, String>> {
    anyhow::ensure!(enabled || view.is_none(), "Harn skills require enabled tool approvals");
    if !enabled {
        return Ok(HashMap::new());
    }
    let root = launch_dir.canonicalize()?.join("skill-roots");
    std::fs::create_dir(&root)?;
    let utf8 = |path: &Path| {
        path.to_str()
            .map(str::to_owned)
            .ok_or_else(|| anyhow::anyhow!("Harn skill path must be UTF-8"))
    };
    let mut env = HashMap::from([("SHIJU_HARN_SKILL_ROOT".into(), utf8(&root)?)]);
    if let Some(view) = view {
        view.snapshot(&root.join("selected")).map_err(|error| anyhow::anyhow!("{error}"))?;
        env.insert("SHIJU_HARN_SELECTED_VIEW".into(), utf8(view.root())?);
    }
    Ok(env)
}

#[cfg(test)]
#[path = "harn_tests.rs"]
mod tests;
