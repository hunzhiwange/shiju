//! 组合原有 npm 下载线程与库内归档审计安装。
use anyhow::Result;
use std::path::Path;
use plugin_types::PluginInstallResult;
pub use agent_plugins::npm_archive::install_archive;

pub(crate) fn install(
    package: &str,
    version: Option<&str>,
    registry: Option<&str>,
    destination: &Path,
    marketplace: Option<&str>,
) -> Result<PluginInstallResult> {
    let package = package.to_owned();
    let version = version.map(str::to_owned);
    let registry = registry.map(str::to_owned);
    // The existing installation API is synchronous and may be invoked inside a Tokio runtime.
    let archive = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(
            super::npm_source::fetch_npm_plugin(&package, version.as_deref(), registry.as_deref()),
        )
    })
    .join()
    .map_err(|_| anyhow::anyhow!("npm download worker panicked"))??;
    install_archive(archive, destination, marketplace)
}
