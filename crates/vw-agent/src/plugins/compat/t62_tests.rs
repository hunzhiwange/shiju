use crate::plugins::install::{remove_installed_plugin, set_plugin_enabled};
use crate::plugins::{PluginDiscoveryOptions, discover_plugins, install_plugin_from_marketplace};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn public_original(root: &Path, name: &str) -> std::path::PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/plugin_compat/t61/public")
        .join(format!("{name}.tar.gz"));
    let archive = fs::File::open(fixture).unwrap();
    tar::Archive::new(flate2::read::GzDecoder::new(archive)).unpack(root).unwrap();
    root.join(name)
}
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
#[tokio::test]
async fn codex_plugin_compat_t62_calls_actual_public_readonly_service() {
    use crate::tools::{
        Tool,
        mcp_tool::{McpTool, discover_mcp_tool_specs},
    };
    use std::sync::Arc;
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "context7");
    let before = digest(&source);
    let config = t.path().join("config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "public").unwrap();
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    let mut servers = vec![];
    crate::plugins::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 1);
    let server = servers.remove(0);
    let pool = Arc::new(super::mcp_connections::McpSessionConnections::default());
    let specs = pool
        .with_peer(&server, None, |peer| {
            Box::pin(async move { discover_mcp_tool_specs(peer, "context7", None, &[]).await })
        })
        .await
        .unwrap();
    let id = crate::tools::mcp_tool::mcp_tool_id("context7", "resolve-library-id").unwrap();
    let spec =
        specs.into_iter().find(|s| s.id == id).expect("public service exposes documented resolver");
    let tool = McpTool::from_session_server(
        server,
        "resolve-library-id",
        spec,
        pool.clone(),
        Some(std::time::Duration::from_secs(30)),
    );
    let result = tool.call(serde_json::json!({"libraryName":"tokio","query":"Rust Tokio async runtime task spawn API"})).await.unwrap();
    assert!(
        result.model_text().to_lowercase().contains("tokio"),
        "public resolver must return requested library"
    );
    pool.shutdown();
    set_plugin_enabled(&plugins, "context7@public", false).unwrap();
    let mut disabled = vec![];
    crate::plugins::merge_enabled_plugin_mcp_servers(&mut disabled, &options).unwrap();
    assert!(disabled.is_empty());
    remove_installed_plugin(&plugins, "context7@public").unwrap();
    assert!(!installed.installed_dir.exists());
    assert_eq!(digest(&source), before);
}

#[tokio::test]
async fn codex_plugin_compat_t62_rejects_invalid_input() {
    use agent_client_protocol::{McpServer, McpServerHttp};
    let pool = super::mcp_connections::McpSessionConnections::default();
    let server = McpServer::Http(McpServerHttp::new("unavailable", "http://127.0.0.1:0/mcp"));
    let result = pool.with_peer(&server, None, |_peer| Box::pin(async { Ok(()) })).await;
    assert!(result.is_err(), "unreachable endpoint must fail visibly");
    pool.shutdown();
}
#[test]
fn codex_plugin_compat_t62_preserves_isolation() {
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "context7");
    let before = digest(&source);
    let plugins = t.path().join("plugins");
    let a = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    let b = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "beta").unwrap();
    set_plugin_enabled(&plugins, "context7@alpha", false).unwrap();
    remove_installed_plugin(&plugins, "context7@alpha").unwrap();
    assert!(!a.installed_dir.exists());
    assert_eq!(digest(&b.installed_dir), before);
    assert_eq!(digest(&source), before);
}
