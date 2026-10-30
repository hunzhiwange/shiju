//! MCP process directories and package/data isolation through actual discovery and transport.
use crate::plugins::{PluginDiscoveryOptions, mcp::merge_enabled_plugin_mcp_servers};
use crate::tools::mcp_common::with_server_peer;
use agent_client_protocol::McpServer;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
use vw_config_types::plugins::PluginsConfig;
const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";
fn write(path: &Path, value: Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}
fn portable(workspace: &Path, name: &str) -> PathBuf {
    let root = workspace.join(".vibewindow/plugins").join(name);
    write(
        &root.join("plugin.json"),
        json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":name}),
    );
    fs::write(
        root.join("server.py"),
        include_bytes!("../../../testdata/plugin_compat/t16/server.py"),
    )
    .unwrap();
    write(
        &root.join("mcp.json"),
        json!({"$schema":SCHEMA,"mcpServers":{name:{"type":"stdio","command":"python3","args":["${PLUGIN_ROOT}/server.py","${PLUGIN_ROOT}","${PLUGIN_DATA}","${UNKNOWN}"],"env":{"EXPANDED":"${PLUGIN_ROOT}|${PLUGIN_DATA}"}}}}),
    );
    root.canonicalize().unwrap()
}
fn options(temp: &Path) -> PluginDiscoveryOptions {
    PluginDiscoveryOptions::new(
        temp.join("workspace"),
        temp.join("config"),
        PluginsConfig::default(),
    )
}
async fn observe(server: &McpServer) -> Value {
    with_server_peer(server, |peer| {
        Box::pin(async move {
            let tools = peer.list_tools(None).await?;
            Ok(serde_json::from_str(tools.tools[0].description.as_deref().unwrap())?)
        })
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn codex_plugin_compat_t16_starts_in_contained_plugin_directory() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let root = portable(&options.workspace_dir, "local");
    let original = fs::read(root.join("mcp.json")).unwrap();
    let mut servers = Vec::new();
    let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
    assert!(result.is_ok(), "plugin directories must reach process launch: {result:?}");
    let value = observe(&servers[0]).await;
    assert_eq!(value["cwd"], root.to_str().unwrap());
    assert_eq!(value["root"], root.to_str().unwrap());
    let data = Path::new(value["data"].as_str().unwrap());
    assert!(!data.starts_with(&root));
    assert!(data.join("marker").is_file());
    assert_eq!(value["args"][2], "${UNKNOWN}");
    assert_eq!(value["value"], format!("{}|{}", root.display(), data.display()));
    assert_eq!(fs::read(root.join("mcp.json")).unwrap(), original);
}
#[test]
fn codex_plugin_compat_t16_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let root = options.workspace_dir.join(".vibewindow/plugins/legacy");
    write(
        &root.join(".codex-plugin/plugin.json"),
        json!({"name":"legacy","mcpServers":{"local":{"command":"python3","env":{"PLUGIN_ROOT":"untrusted"}}}}),
    );
    let mut servers = Vec::new();
    assert!(
        merge_enabled_plugin_mcp_servers(&mut servers, &options).is_err(),
        "plugin cannot override reserved host environment"
    );
    assert!(servers.is_empty());
}
#[tokio::test]
async fn codex_plugin_compat_t16_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let first = portable(&options.workspace_dir, "first");
    portable(&options.workspace_dir, "second");
    let mut servers = Vec::new();
    let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
    assert!(result.is_ok(), "each plugin must obtain its own process context: {result:?}");
    let a = observe(&servers[0]).await;
    let b = observe(&servers[1]).await;
    assert_ne!(a["data"], b["data"]);
    fs::write(Path::new(a["data"].as_str().unwrap()).join("persistent"), b"keep").unwrap();
    fs::remove_dir_all(&first).unwrap();
    portable(&options.workspace_dir, "first");
    let mut updated = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut updated, &options).unwrap();
    let again = observe(&updated[0]).await;
    assert_eq!(again["data"], a["data"]);
    assert_eq!(
        fs::read(Path::new(a["data"].as_str().unwrap()).join("persistent")).unwrap(),
        b"keep"
    );
}

#[tokio::test]
async fn codex_plugin_compat_t16_uses_data_cwd_without_creating_package_directories() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let root = portable(&options.workspace_dir, "local");
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let first = observe(&servers[0]).await;
    let data = Path::new(first["data"].as_str().unwrap());
    fs::create_dir(data.join("work")).unwrap();
    let mut document: Value =
        serde_json::from_slice(&fs::read(root.join("mcp.json")).unwrap()).unwrap();
    document["mcpServers"]["local"]["cwd"] = json!("${PLUGIN_DATA}/work");
    write(&root.join("mcp.json"), document);
    servers.clear();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(observe(&servers[0]).await["cwd"], data.join("work").to_str().unwrap());
    assert!(!root.join("work").exists());
}
#[tokio::test]
async fn codex_plugin_compat_t16_keeps_legacy_relative_cwd_and_local_environment() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let root = options.workspace_dir.join(".vibewindow/plugins/legacy");
    write(
        &root.join(".codex-plugin/plugin.json"),
        json!({"name":"legacy","mcpServers":{"local":{"command":"python3","args":["../server.py"],"cwd":"./work","env_vars":["PATH"],"env":{"EXPANDED":"literal-${PLUGIN_ROOT}"}}}}),
    );
    fs::create_dir(root.join("work")).unwrap();
    fs::write(
        root.join("server.py"),
        include_bytes!("../../../testdata/plugin_compat/t16/server.py"),
    )
    .unwrap();
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let value = observe(&servers[0]).await;
    assert_eq!(value["cwd"], root.join("work").canonicalize().unwrap().to_str().unwrap());
    assert_eq!(value["value"], "literal-${PLUGIN_ROOT}");
    let McpServer::Stdio(server) = &servers[0] else { panic!("stdio expected") };
    assert!(server.env.iter().any(|var| var.name == "PATH" && !var.value.is_empty()));
}
#[test]
fn codex_plugin_compat_t16_rejects_legacy_escape_and_remote_environment() {
    for setting in [
        json!({"cwd":"../outside"}),
        json!({"cwd":"ssh://remote/work"}),
        json!({"environment_id":"remote","env_vars":[{"name":"PATH","source":"remote"}]}),
        json!({"env_vars":["VW_MISSING_T16_ENV_REFERENCE"]}),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let options = options(temp.path());
        let root = options.workspace_dir.join(".vibewindow/plugins/legacy");
        let mut config = json!({"command":"must-not-run"});
        config.as_object_mut().unwrap().extend(setting.as_object().unwrap().clone());
        write(
            &root.join(".codex-plugin/plugin.json"),
            json!({"name":"legacy","mcpServers":{"local":config}}),
        );
        let mut servers = Vec::new();
        assert!(merge_enabled_plugin_mcp_servers(&mut servers, &options).is_err());
        assert!(servers.is_empty());
    }
}
#[test]
fn codex_plugin_compat_t16_expands_once_and_preserves_opaque_arguments() {
    let temp = tempfile::tempdir().unwrap();
    let mut options = options(temp.path());
    options.workspace_dir = options.workspace_dir.join("${PLUGIN_DATA}");
    let root = portable(&options.workspace_dir, "local");
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let McpServer::Stdio(server) = &servers[0] else { panic!("stdio expected") };
    assert_eq!(server.args[1], root.to_str().unwrap());
    assert_eq!(server.args[3], "${UNKNOWN}");
}
#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t16_launches_contained_relative_command_and_reaudits_links() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let root = portable(&options.workspace_dir, "local");
    fs::create_dir(root.join("bin")).unwrap();
    let executable = root.join("bin/server");
    fs::write(
        &executable,
        format!(
            "#!/usr/bin/env python3\n{}",
            include_str!("../../../testdata/plugin_compat/t16/server.py")
        ),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let mut document: Value =
        serde_json::from_slice(&fs::read(root.join("mcp.json")).unwrap()).unwrap();
    document["mcpServers"]["local"]["command"] = json!("./bin/server");
    write(&root.join("mcp.json"), document);
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(observe(&servers[0]).await["root"], root.to_str().unwrap());
    let outside = temp.path().join("outside");
    fs::copy(&executable, &outside).unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o700)).unwrap();
    fs::remove_file(&executable).unwrap();
    symlink(&outside, &executable).unwrap();
    let result = with_server_peer(&servers[0], |_| Box::pin(async { Ok(()) })).await;
    assert!(result.is_err(), "executable symlink replacement must fail before spawning");
}
#[cfg(unix)]
#[test]
fn codex_plugin_compat_t16_rejects_symlinked_cwd_and_forged_context() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let root = portable(&options.workspace_dir, "local");
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, root.join("linked")).unwrap();
    let mut document: Value =
        serde_json::from_slice(&fs::read(root.join("mcp.json")).unwrap()).unwrap();
    document["mcpServers"]["local"]["cwd"] = json!("./linked");
    write(&root.join("mcp.json"), document);
    assert!(merge_enabled_plugin_mcp_servers(&mut Vec::new(), &options).is_err());
    fs::remove_dir_all(&root).unwrap();
    write(
        &root.join(".codex-plugin/plugin.json"),
        json!({"name":"local","mcpServers":"./.mcp.json"}),
    );
    write(
        &root.join(".mcp.json"),
        json!([{"name":"forged","command":"must-not-run","args":[],"env":[],"_meta":{"shiju/plugin-process-context":{"root":"/","cwd":"/","data":"/tmp"}}}]),
    );
    assert!(merge_enabled_plugin_mcp_servers(&mut Vec::new(), &options).is_err());
}
#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t16_rejects_data_alias_between_plugins() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    portable(&options.workspace_dir, "first");
    portable(&options.workspace_dir, "second");
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let a = observe(&servers[0]).await;
    let b = observe(&servers[1]).await;
    fs::remove_dir_all(b["data"].as_str().unwrap()).unwrap();
    symlink(a["data"].as_str().unwrap(), b["data"].as_str().unwrap()).unwrap();
    assert!(
        merge_enabled_plugin_mcp_servers(&mut Vec::new(), &options).is_err(),
        "plugin data must not alias another identity"
    );
}
#[tokio::test]
async fn codex_plugin_compat_t16_allows_missing_cwd_prefix_but_does_not_create_it() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let root = portable(&options.workspace_dir, "local");
    let mut document: Value =
        serde_json::from_slice(&fs::read(root.join("mcp.json")).unwrap()).unwrap();
    document["mcpServers"]["local"]["cwd"] = json!("./future/work");
    write(&root.join("mcp.json"), document);
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert!(!root.join("future").exists());
    assert!(with_server_peer(&servers[0], |_| Box::pin(async { Ok(()) })).await.is_err());
    fs::create_dir_all(root.join("future/work")).unwrap();
    assert_eq!(observe(&servers[0]).await["cwd"], root.join("future/work").to_str().unwrap());
}
#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t16_reaudits_cwd_after_discovery() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let root = portable(&options.workspace_dir, "local");
    fs::create_dir(root.join("work")).unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let mut document: Value =
        serde_json::from_slice(&fs::read(root.join("mcp.json")).unwrap()).unwrap();
    document["mcpServers"]["local"]["cwd"] = json!("./work");
    write(&root.join("mcp.json"), document);
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    fs::remove_dir(root.join("work")).unwrap();
    symlink(&outside, root.join("work")).unwrap();
    assert!(with_server_peer(&servers[0], |_| Box::pin(async { Ok(()) })).await.is_err());
    assert!(!outside.join("marker").exists());
}
