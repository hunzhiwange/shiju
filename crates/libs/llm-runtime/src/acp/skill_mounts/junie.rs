//! Publish the validated skill view to the reviewed Junie launcher.
use super::super::{
    Error, host::SkillLease, model_source::profile::AcpLaunchProfile, to_api_error,
};
use std::{collections::BTreeMap, fs, path::PathBuf};

pub(in crate::acp) fn publish(
    profile: &AcpLaunchProfile,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    let Some(selection) = selection(view).map_err(to_api_error)? else {
        return Ok(());
    };
    profile.write_config("junie-skills.json", &selection).map_err(to_api_error)?;
    Ok(())
}

fn selection(view: Option<&impl SkillLease>) -> anyhow::Result<Option<Vec<u8>>> {
    let Some(view) = view else {
        return Ok(None);
    };
    anyhow::ensure!(
        cfg!(all(target_os = "macos", target_arch = "aarch64")),
        "Junie requires its reviewed Darwin arm64 skill scanner"
    );
    view.validate()?;
    let root = view.root().canonicalize()?;
    let sources: BTreeMap<String, PathBuf> = fs::read_dir(&root)?
        .map(|entry| {
            let entry = entry?;
            let name = entry.file_name().into_string().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid Junie skill name")
            })?;
            Ok((name, fs::read_link(entry.path())?))
        })
        .collect::<std::io::Result<_>>()?;
    view.validate()?;
    Ok(Some(serde_json::to_vec(&serde_json::json!({
        "root": root,
        "sources": sources,
    }))?))
}

#[cfg(test)]
#[path = "junie_tests.rs"]
mod tests;
