use super::digest;
use crate::{install_plugin_from_marketplace, PluginDiscoveryOptions};
use std::{path::{Path, PathBuf}, fs, future::Future};
fn package(fixtures: &Path, root: &Path) -> PathBuf {
    let source = root.join("插件 original package");
    fs::create_dir_all(&source).unwrap();
    for name in ["plugin.json", "mcp.json", "server.py"] {
        fs::copy(fixtures.join("package").join(name), source.join(name)).unwrap();
    }
    source
}
/// Preserve Unicode path and package integrity assertions around real host MCP execution.
pub async fn unicode_paths<F, Fut>(fixtures: &Path, invoke: F,
    merge: &dyn Fn(&mut Vec<agent_client_protocol::McpServer>, &PluginDiscoveryOptions) -> anyhow::Result<()>,
    close_plugin: &dyn Fn(&Path))
where F: FnOnce(agent_client_protocol::McpServer, PathBuf) -> Fut,
      Fut: Future<Output = anyhow::Result<tool_types::ToolCallResult>> {
    let t = tempfile::tempdir().unwrap();
    let source = package(fixtures, t.path());
    let before = digest(&source);
    let config = t.path().join("配置 config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "market").unwrap();
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    let mut servers = vec![];
    merge(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 1);
    let server = servers.remove(0);
    let output = t.path().join("输出 output.txt");
    let result = invoke(server, output.clone()).await.unwrap();
    assert_eq!(result.model_text(), "written");
    assert_eq!(fs::read_to_string(&output).unwrap(), "original package executed\n");
    assert_eq!(digest(&source), before);
    assert_eq!(digest(&installed.installed_dir), before);
    crate::install::set_plugin_enabled(&plugins, "original@market", false, close_plugin).unwrap();
    let mut disabled = vec![];
    merge(&mut disabled, &options).unwrap();
    assert!(disabled.is_empty());
    crate::install::remove_installed_plugin(&plugins, "original@market", close_plugin).unwrap();
    assert!(output.exists());
}
