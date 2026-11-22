//! Publish the verified view to the managed GLM scanner without extending the native reader's scope.
use super::super::{
    Error, host::SkillLease, model_source::profile::AcpLaunchProfile, to_api_error,
};
use std::{collections::HashMap, fs, path::Path};

/// Targets that can use the same workspace view with the reviewed v3 package.
/// The host still decides whether to expose the capability.
pub(in crate::acp) const fn supports_session_view_target() -> bool {
    cfg!(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(
            target_os = "linux",
            target_env = "gnu",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))
}

pub(in crate::acp) fn configure(
    profile: &mut AcpLaunchProfile,
    workspace: &Path,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    if !supports_session_view_target() {
        return Err(super::super::acp_option_error(
            "GLM session skill view is unavailable on this target",
        ));
    }
    let sources = selection(workspace, view).map_err(to_api_error)?;
    profile
        .overlay_environment(HashMap::from([("SHIJU_GLM_SKILL_VIEW".into(), sources)]))
        .map_err(to_api_error)
}

fn selection(workspace: &Path, view: Option<&impl SkillLease>) -> anyhow::Result<String> {
    let Some(view) = view else {
        return Ok("[]".into());
    };
    view.validate()?;
    let workspace = workspace.canonicalize()?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(view.root().canonicalize()?)? {
        let entry = entry?;
        let link = entry.path();
        let target = fs::read_link(&link)?;
        anyhow::ensure!(
            target.starts_with(&workspace) && target != workspace,
            "GLM selected skill must be inside the native reader's workspace"
        );
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("GLM selected skill name is not Unicode"))?;
        entries.push((name, link, target));
    }
    entries.sort();
    view.validate()?;
    Ok(serde_json::to_string(&entries)?)
}

#[cfg(test)]
#[path = "glm_acp_agent_tests.rs"]
mod tests;
