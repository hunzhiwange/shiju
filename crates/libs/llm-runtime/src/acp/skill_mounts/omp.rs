//! Register the request's skill view in the pinned OMP native scanner.
use super::super::{
    Error, host::SkillLease, model_source::profile::AcpLaunchProfile, to_api_error,
};
use agent_config::schema::AcpAgentConfig;
use serde_json::json;
use std::{fs, path::Path};

pub(in crate::acp) const SETTINGS_ENV: &str = "SHIJU_OMP_SKILL_SETTINGS";
pub(in crate::acp) const SANDBOX_ENV: &str = "SHIJU_OMP_SANDBOX";
pub(in crate::acp) const POLICY_FILE: &str = "omp.sb";

pub(in crate::acp) fn settings(root: Option<&Path>) -> anyhow::Result<Vec<u8>> {
    let directories = serde_json::to_value(root.into_iter().collect::<Vec<_>>())?;
    Ok(serde_json::to_vec(&json!({
        "skills": { "customDirectories": directories },
    }))?)
}

pub(in crate::acp) fn configure(
    profile: &AcpLaunchProfile,
    config: &mut AcpAgentConfig,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    let Some(view) = view else {
        return Ok(());
    };
    view.validate()?;
    let result = (|| -> anyhow::Result<()> {
        let root = view.root().canonicalize()?;
        let settings_path =
            profile.write_config("omp-selected-skills.yml", &settings(Some(&root))?)?;
        let policy = fs::read_to_string(profile.directory().join(POLICY_FILE))?;
        let policy = allow_selection(policy, &settings_path.canonicalize()?, &root)?;
        let policy_path = profile.write_config("omp-selected.sb", policy.as_bytes())?;
        config.env.insert(SETTINGS_ENV.into(), settings_path.to_string_lossy().into_owned());
        config.env.insert(SANDBOX_ENV.into(), policy_path.to_string_lossy().into_owned());
        Ok(())
    })();
    result.map_err(to_api_error)
}

fn allow_selection(mut policy: String, settings: &Path, root: &Path) -> anyhow::Result<String> {
    policy.push_str(&format!("\n(allow file-read-data (literal {}))\n", quote(settings)?));
    policy.push_str(&format!("(allow file-read-data (subpath {}))\n", quote(root)?));
    // Seatbelt checks the resolved paths as well as the links in the view.
    for entry in fs::read_dir(root)? {
        let source = fs::read_link(entry?.path())?;
        anyhow::ensure!(source.is_absolute(), "OMP skill source must be absolute");
        policy.push_str(&format!("(allow file-read-data (subpath {}))\n", quote(&source)?));
    }
    Ok(policy)
}

fn quote(path: &Path) -> anyhow::Result<String> {
    let text = path.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 OMP skill path"))?;
    anyhow::ensure!(!text.chars().any(char::is_control), "Invalid OMP skill path");
    Ok(serde_json::to_string(text)?)
}

#[cfg(all(test, unix))]
#[path = "omp_tests.rs"]
mod tests;
