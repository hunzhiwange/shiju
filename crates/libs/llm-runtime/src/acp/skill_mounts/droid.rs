//! Publish the trusted session lease under Droid's private personal scanner root.
use super::super::{host::SkillLease, model_source::profile::AcpLaunchProfile};
use std::path::{Path, PathBuf};

pub(in crate::acp) fn sandbox(
    profile: &AcpLaunchProfile,
    cwd: &Path,
    view: Option<&impl SkillLease>,
) -> anyhow::Result<acp_client::DroidSandbox> {
    let workspace = cwd.canonicalize()?;
    let home = profile.directory().canonicalize()?;
    let executable = Path::new(&profile.config().command).canonicalize()?;
    anyhow::ensure!(
        workspace.parent().is_some()
            && !home.starts_with(&workspace)
            && !workspace.starts_with(&home)
            && !executable.starts_with(&workspace),
        "Droid workspace overlaps the private runtime"
    );
    let relay_port =
        profile.relay_address().ok_or_else(|| anyhow::anyhow!("Missing Droid relay"))?.port();
    let skill_roots = publish(&home, &workspace, view)?;
    Ok(acp_client::DroidSandbox { workspace, home, skill_roots, relay_port })
}

fn publish(
    home: &Path,
    workspace: &Path,
    view: Option<&impl SkillLease>,
) -> anyhow::Result<Vec<PathBuf>> {
    #[cfg(not(unix))]
    {
        let _ = (home, workspace, view);
        anyhow::bail!("Restricted Droid requires Darwin arm64");
    }
    #[cfg(unix)]
    {
        use std::{fs, os::unix::fs::symlink};
        let factory = workspace.join(".factory");
        let native = factory.join("skills");
        let settings = factory.join("settings.json");
        for (path, directory) in [(&factory, true), (&native, true), (&settings, false)] {
            match fs::symlink_metadata(path) {
                Ok(metadata) => anyhow::ensure!(
                    (if directory { metadata.is_dir() } else { metadata.is_file() })
                        && path.canonicalize()? == *path,
                    "Droid project configuration must be inside the workspace"
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let mut reads = vec![native];
        if let Some(view) = view {
            view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
            let root = view.root().canonicalize()?;
            reads.push(root.clone());
            for entry in fs::read_dir(&root)? {
                let target = entry?.path().canonicalize()?;
                anyhow::ensure!(
                    target.is_dir()
                        && !home.starts_with(&target)
                        && !workspace.starts_with(&target),
                    "Invalid Droid skill source"
                );
                reads.push(target);
            }
            fs::create_dir(home.join(".factory"))?;
            symlink(root, home.join(".factory/skills"))?;
            view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
        }
        Ok(reads)
    }
}

#[cfg(test)]
#[path = "droid_tests.rs"]
mod tests;
