//! MCP directory declaration validation through actual discovery and path auditing.
use crate::PluginDiscoveryOptions;
use agent_client_protocol::McpServer;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
use config_plugins::PluginsConfig;
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
        include_bytes!("../../../vw-agent/testdata/plugin_compat/t16/server.py"),
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
fn merge_enabled_plugin_mcp_servers(
    servers: &mut Vec<McpServer>,
    options: &PluginDiscoveryOptions,
) -> anyhow::Result<()> {
    crate::mcp::merge_enabled_plugin_mcp_servers(servers, options, &|options| {
        Ok((crate::discover_plugins(options)?, options.clone()))
    })
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
