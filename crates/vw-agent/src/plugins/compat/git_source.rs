//! Compose Git extraction with the existing audited host installer.
use anyhow::Result;
use std::path::Path;
use crate::plugins::PluginInstallResult;
pub(crate) fn install(
    url: &str,
    path: Option<&str>,
    reference: Option<&str>,
    sha: Option<&str>,
    destination: &Path,
    marketplace: Option<&str>,
) -> Result<PluginInstallResult> {
    plugin_install::git_source::install(url, path, reference, sha, destination, marketplace,
        crate::plugins::load_plugin_manifest,
        super::transaction_install::install_package_with_source)
}
