//! Add workspace-backed session skills to the managed aionrs scanner.
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
    publish_links(directory, workspace, view.root()).map_err(to_api_error)?;
    view.validate()
}

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(
        target_os = "linux",
        target_env = "gnu",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ),
))]
fn publish_links(directory: &Path, workspace: &Path, view: &Path) -> anyhow::Result<()> {
    use std::{fs, os::unix::fs::symlink};

    let workspace = workspace.canonicalize()?;
    let sources = fs::read_dir(view)?
        .map(|entry| {
            let entry = entry?;
            let source = fs::read_link(entry.path())?;
            let resolved = entry.path().canonicalize()?;
            anyhow::ensure!(
                source == resolved
                    && resolved != workspace
                    && resolved.starts_with(&workspace)
                    && resolved.is_dir(),
                "aionrs selected skill is outside the sandbox workspace"
            );
            Ok((entry.file_name(), source))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let skills = directory.join(".aionrs/skills");
    fs::create_dir_all(&skills)?;
    for (name, source) in sources {
        symlink(source, skills.join(name))?;
    }
    Ok(())
}

#[cfg(not(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(
        target_os = "linux",
        target_env = "gnu",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ),
)))]
fn publish_links(_directory: &Path, _workspace: &Path, _view: &Path) -> anyhow::Result<()> {
    anyhow::bail!("aionrs selected skills are unavailable on this platform")
}

#[cfg(all(
    test,
    any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(
            target_os = "linux",
            target_env = "gnu",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
    )
))]
#[path = "aionrs_tests.rs"]
mod tests;
