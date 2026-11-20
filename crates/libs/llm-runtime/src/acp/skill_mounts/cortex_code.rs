//! Register selected project skills in Cortex's private launch directory.
use super::super::{Error, host::SkillLease, to_api_error};
use std::path::Path;

pub(in crate::acp) fn publish(
    directory: &Path,
    workspace: &Path,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    let Some(view) = view else {
        return Ok(());
    };
    view.validate()?;
    publish_links(directory, workspace, view.root()).map_err(to_api_error)
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn publish_links(directory: &Path, workspace: &Path, root: &Path) -> anyhow::Result<()> {
    use std::{fs, os::unix::fs::symlink};

    let workspace = workspace.canonicalize()?;
    let selected = fs::read_dir(root)?
        .map(|entry| {
            let entry = entry?;
            let source = fs::read_link(entry.path())?;
            anyhow::ensure!(
                source != workspace && source.starts_with(&workspace),
                "Cortex selected skill is outside the sandbox workspace"
            );
            Ok((entry.file_name(), source))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let skills = directory.join("session-skills");
    fs::create_dir(&skills)?;
    for (name, source) in selected {
        symlink(source, skills.join(name))?;
    }
    Ok(())
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn publish_links(_directory: &Path, _workspace: &Path, _root: &Path) -> anyhow::Result<()> {
    anyhow::bail!("Cortex skill mounts require the reviewed Darwin arm64 runtime")
}

#[cfg(test)]
#[path = "cortex_code_tests.rs"]
mod tests;
