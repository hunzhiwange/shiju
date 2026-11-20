//! Register the validated selection in Corust's request-local HOME.
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
    let entries = fs::read_dir(root)?
        .map(|entry| {
            let entry = entry?;
            let source = fs::read_link(entry.path())?;
            anyhow::ensure!(
                source.starts_with(&workspace) && source != workspace,
                "Corust selected skill is outside the sandbox workspace"
            );
            Ok((entry.file_name(), source))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let config = directory.join(".agents");
    fs::create_dir(&config)?;
    let skills = config.join("skills");
    fs::create_dir(&skills)?;
    // The sandbox can resolve project targets directly. It cannot enumerate
    // the external lease directory, so mirror the lease entries individually.
    for (name, source) in entries {
        symlink(source, skills.join(name))?;
    }
    Ok(())
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn publish_links(_directory: &Path, _workspace: &Path, _root: &Path) -> anyhow::Result<()> {
    anyhow::bail!("Corust requires the reviewed Darwin arm64 skill scanner")
}

#[cfg(test)]
#[path = "corust_agent_tests.rs"]
mod tests;
