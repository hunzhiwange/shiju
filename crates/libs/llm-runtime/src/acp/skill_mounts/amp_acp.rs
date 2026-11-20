//! Bind the reviewed Amp Worker to the project scanner and one request-owned view.
use super::super::host::SkillLease;
use super::super::model_source::profile::AcpLaunchProfile;
use super::Error;
use std::path::Path;

pub(super) const SKILL_VIEW: &str = "SHIJU_AMP_SKILL_VIEW";

pub(in crate::acp) fn configure<S: SkillLease>(
    profile: &mut AcpLaunchProfile,
    enabled: bool,
    view: Option<&S>,
) -> Result<(), Error> {
    let selection = selection(enabled, view).map_err(super::super::to_api_error)?;
    profile
        .overlay_environment([(SKILL_VIEW.into(), selection)].into())
        .map_err(super::super::to_api_error)
}

fn selection<S: SkillLease>(enabled: bool, view: Option<&S>) -> anyhow::Result<String> {
    anyhow::ensure!(enabled || view.is_none(), "Amp skills require enabled tool approvals");
    if !enabled {
        return Ok("null".into());
    }
    let mut roots = Vec::new();
    if let Some(view) = view {
        view.validate().map_err(|error| anyhow::anyhow!("{error}"))?;
        for entry in std::fs::read_dir(view.root())? {
            let link = entry?.path();
            let source = std::fs::read_link(&link)?;
            roots.push((utf8(&link)?.to_owned(), utf8(&source)?.to_owned()));
        }
        view.validate().map_err(|error| anyhow::anyhow!("{error}"))?;
        roots.sort();
    }
    Ok(serde_json::to_string(&roots)?)
}

fn utf8(path: &Path) -> anyhow::Result<&str> {
    path.to_str().ok_or_else(|| anyhow::anyhow!("Amp skill directory requires a UTF-8 path"))
}

#[cfg(test)]
#[path = "amp_acp_tests.rs"]
mod tests;
