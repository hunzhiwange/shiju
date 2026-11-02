use crate::plugins::install::{remove_installed_plugin, set_plugin_enabled};
use crate::plugins::{PluginDiscoveryOptions, discover_plugins, install_plugin_from_marketplace};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn digest(p: &Path) -> Vec<(String, String)> {
    let mut files = walkdir::WalkDir::new(p)
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            (
                e.path().strip_prefix(p).unwrap().to_string_lossy().into_owned(),
                hex::encode(Sha256::digest(fs::read(e.path()).unwrap())),
            )
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}
fn package(p: &Path) {
    fs::create_dir_all(p.join(".codex-plugin")).unwrap();
    fs::write(p.join(".codex-plugin/plugin.json"), r#"{"name":"original","version":"1"}"#).unwrap();
    fs::write(p.join("payload"), b"unchanged original bytes").unwrap();
}
#[test]
fn codex_plugin_compat_t61_executes_original_package_without_mutation() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source");
    package(&source);
    let before = digest(&source);
    let config = t.path().join("config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "market").unwrap();
    assert_eq!(digest(&installed.installed_dir), before);
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    assert_eq!(discover_plugins(&options).unwrap().enabled.len(), 1);
    set_plugin_enabled(&plugins, "original@market", false).unwrap();
    assert!(discover_plugins(&options).unwrap().enabled.is_empty());
    set_plugin_enabled(&plugins, "original@market", true).unwrap();
    assert_eq!(discover_plugins(&options).unwrap().enabled.len(), 1);
    remove_installed_plugin(&plugins, "original@market").unwrap();
    assert!(discover_plugins(&options).unwrap().installed.is_empty());
    assert_eq!(digest(&source), before);
}
#[test]
fn codex_plugin_compat_t61_rejects_invalid_input() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source");
    package(&source);
    fs::write(source.join(".codex-plugin/plugin.json"), b"{broken").unwrap();
    assert!(
        install_plugin_from_marketplace(
            source.to_str().unwrap(),
            &t.path().join("plugins"),
            "market"
        )
        .is_err()
    );
    assert_eq!(fs::read(source.join("payload")).unwrap(), b"unchanged original bytes");
}
#[test]
fn codex_plugin_compat_t61_preserves_isolation() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source");
    package(&source);
    let plugins = t.path().join("plugins");
    let a = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    let b = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "beta").unwrap();
    remove_installed_plugin(&plugins, "original@alpha").unwrap();
    assert!(!a.installed_dir.exists());
    assert!(b.installed_dir.exists());
    assert_eq!(digest(&source), digest(&b.installed_dir));
}

#[tokio::test]
async fn codex_plugin_compat_t61_runs_installed_mcp_side_effect() {
    use crate::tools::{
        Tool,
        mcp_tool::{McpTool, discover_mcp_tool_specs},
    };
    use std::sync::Arc;
    let t = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/plugin_compat/t61/package");
    let before = digest(&source);
    let config = t.path().join("config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "market").unwrap();
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    let mut servers = vec![];
    crate::plugins::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 1);
    let server = servers.remove(0);
    let pool = Arc::new(super::mcp_connections::McpSessionConnections::default());
    let specs = pool
        .with_peer(&server, None, |peer| {
            Box::pin(async move { discover_mcp_tool_specs(peer, "writer", None, &[]).await })
        })
        .await
        .unwrap();
    assert_eq!(specs.len(), 1);
    let tool = McpTool::from_session_server(
        server,
        "write",
        specs.into_iter().next().unwrap(),
        pool.clone(),
        None,
    );
    let output = t.path().join("output.txt");
    let result = tool.call(serde_json::json!({"output":output})).await.unwrap();
    assert_eq!(result.model_text(), "written");
    assert_eq!(fs::read_to_string(&output).unwrap(), "original package executed\n");
    pool.shutdown();
    assert_eq!(digest(&source), before);
    assert_eq!(digest(&installed.installed_dir), before);
    set_plugin_enabled(&plugins, "original@market", false).unwrap();
    let mut disabled = vec![];
    crate::plugins::merge_enabled_plugin_mcp_servers(&mut disabled, &options).unwrap();
    assert!(disabled.is_empty());
    remove_installed_plugin(&plugins, "original@market").unwrap();
    assert!(output.exists());
}
