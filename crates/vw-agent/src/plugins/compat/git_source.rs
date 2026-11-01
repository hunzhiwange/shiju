//! Isolated Git object extraction: never checks out hooks or submodules.
use crate::plugins::{PluginIdentity, PluginInstallResult, load_plugin_manifest};
use anyhow::{Context, Result, bail};
use std::path::{Component, Path};

pub(crate) fn install(
    url: &str,
    path: Option<&str>,
    reference: Option<&str>,
    sha: Option<&str>,
    destination: &Path,
    marketplace: Option<&str>,
) -> Result<PluginInstallResult> {
    if let Ok(parsed) = url::Url::parse(url) {
        if parsed.password().is_some()
            || (matches!(parsed.scheme(), "http" | "https") && !parsed.username().is_empty())
            || parsed.query().is_some()
        {
            bail!("Git source must not contain embedded credentials or query parameters");
        }
    }
    let revision = sha.or(reference).context("Git plugin source requires an explicit revision")?;
    if revision.is_empty() {
        bail!("Git revision cannot be empty");
    }
    let path = Path::new(path.unwrap_or(""));
    if path.components().any(|c| !matches!(c, Component::Normal(_)))
        || path.to_string_lossy().contains('\\')
    {
        bail!("invalid Git plugin subpath");
    }
    let temporary = tempfile::tempdir()?;
    let repo = git2::build::RepoBuilder::new()
        .bare(true)
        .clone(url, &temporary.path().join("repository"))?;
    let commit = repo.revparse_single(revision)?.peel_to_commit()?;
    if let Some(sha) = sha {
        if sha.len() != 40 || commit.id().to_string() != sha.to_ascii_lowercase() {
            bail!("Git SHA must be a full commit identifier");
        }
    }
    let tree = commit.tree()?;
    let selected = if path.as_os_str().is_empty() {
        tree
    } else {
        tree.get_path(path)?.to_object(&repo)?.peel_to_tree()?
    };
    let payload = temporary.path().join("payload");
    extract(&repo, &selected, &payload)?;
    let manifest = load_plugin_manifest(&payload)?;
    let identity = match marketplace {
        Some(m) => PluginIdentity::marketplace(&manifest.name, m)?,
        None => PluginIdentity::local(&manifest.name),
    };
    let provenance =
        serde_json::json!({"git":url,"path":path,"commit":commit.id().to_string()}).to_string();
    super::transaction_install::install_package_with_source(
        &manifest,
        identity,
        destination,
        Some(&provenance),
    )
}
fn extract(repo: &git2::Repository, tree: &git2::Tree<'_>, destination: &Path) -> Result<()> {
    std::fs::create_dir(destination)?;
    for entry in tree {
        let name = entry.name().context("non UTF-8 Git filename")?;
        if name == "." || name == ".." || name == ".git" || name.contains(['/', '\\']) {
            bail!("unsafe Git entry");
        }
        let target = destination.join(name);
        match entry.filemode() {
            0o040000 => extract(repo, &repo.find_tree(entry.id())?, &target)?,
            0o100644 | 0o100755 => {
                std::fs::write(&target, repo.find_blob(entry.id())?.content())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(
                        &target,
                        std::fs::Permissions::from_mode(entry.filemode() as u32 & 0o777),
                    )?;
                }
            }
            _ => bail!("Git plugin contains a symlink, submodule or unsupported entry"),
        }
    }
    Ok(())
}
