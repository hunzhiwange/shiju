//! Publish a validated selection to the reviewed TRAE private user scanner.
use super::super::{Error, host::SkillLease, to_api_error};
use std::path::Path;

pub(in crate::acp) fn publish(
    directory: &Path,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    let Some(view) = view else {
        return Ok(());
    };
    view.validate()?;
    publish_links(directory, view.root()).map_err(to_api_error)
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn publish_links(directory: &Path, root: &Path) -> anyhow::Result<()> {
    std::os::unix::fs::symlink(root, directory.join("skills"))?;
    Ok(())
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn publish_links(_directory: &Path, _root: &Path) -> anyhow::Result<()> {
    anyhow::bail!("TRAE requires the reviewed Darwin arm64 skill scanner")
}

#[cfg(test)]
#[path = "trae_tests.rs"]
mod tests;
