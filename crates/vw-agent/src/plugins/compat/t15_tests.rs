//! 便携 MCP 声明经真实文件、发现和运行准入入口验证。

use super::mcp_legacy::PluginMcpDeclaration;
use crate::plugins::PluginDiscoveryOptions;
use crate::plugins::mcp::{
    load_portable_mcp_declarations_from_path as load_plugin_mcp_declarations_from_path,
    merge_enabled_plugin_mcp_servers,
};
use agent_client_protocol::McpServer;
use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use vw_config_types::plugins::PluginsConfig;

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";

fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn plugin(workspace: &Path, name: &str) -> PathBuf {
    let root = workspace.join(".vibewindow/plugins").join(name);
    write(
        &root.join("plugin.json"),
        &json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":name}),
    );
    root
}

fn config(declaration: &PluginMcpDeclaration) -> (&str, &Map<String, Value>) {
    match declaration {
        PluginMcpDeclaration::Legacy { name, config }
        | PluginMcpDeclaration::Portable { name, config } => (name, config),
        _ => panic!("named configuration expected"),
    }
}

#[test]
fn codex_plugin_compat_t15_selects_declared_transport() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp.json");
    fs::write(&path, include_bytes!("../../../testdata/plugin_compat/t15/mcp.json")).unwrap();
    let before = fs::read(&path).unwrap();
    let outcome = load_plugin_mcp_declarations_from_path(&path).unwrap();
    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.servers.len(), 2);
    let (name, local) = config(&outcome.servers[0]);
    assert_eq!(name, "local");
    assert_eq!(
        local.get("cwd"),
        Some(&json!("${PLUGIN_ROOT}")),
        "stdio defaults must remain explicit for runtime resolution"
    );
    let (name, remote) = config(&outcome.servers[1]);
    assert_eq!(name, "remote");
    assert_eq!(
        remote.get("http_headers"),
        Some(&json!({"X-Plugin":"fixture"})),
        "portable headers must normalize after client-owned headers are removed"
    );
    assert!(!remote.contains_key("command"));
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn codex_plugin_compat_t15_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp.json");
    write(
        &path,
        &json!({"$schema":"https://agent-plugins.org/schemas/2.0.0/mcp.schema.json","mcpServers":{"remote":{"type":"streamable-http","url":"https://example.test/mcp"}}}),
    );
    let result = load_plugin_mcp_declarations_from_path(&path);
    assert!(result.is_err(), "unsupported portable schema must not fall through legacy parsing");
}

#[test]
fn codex_plugin_compat_t15_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    plugin(&workspace, "absent");
    let root = plugin(&workspace, "active");
    write(
        &root.join("mcp.json"),
        &json!({"$schema":SCHEMA,"mcpServers":{"remote":{"type":"streamable-http","url":"https://example.test/mcp"}}}),
    );
    write(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"name":"ignored-identity","mcpServers":{"remote":{"command":"must-not-execute"},"injected":{"command":"must-not-execute"}}}),
    );
    let disabled = plugin(&workspace, "disabled");
    write(
        &disabled.join("mcp.json"),
        &json!({"$schema":SCHEMA,"mcpServers":{"invalid":{"type":"sse","url":"https://example.test/events"}}}),
    );
    fs::write(workspace.join(".vibewindow/plugins/disabled.disabled"), b"").unwrap();
    let before = fs::read(root.join("mcp.json")).unwrap();
    let options = PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    );
    let mut servers = Vec::new();
    let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
    assert!(
        result.is_ok(),
        "absent optional portable MCP must not disable healthy siblings: {result:?}"
    );
    assert_eq!(servers.len(), 1);
    let McpServer::Http(server) = &servers[0] else {
        panic!("declared HTTP transport must remain authoritative")
    };
    assert_eq!(server.name, "remote");
    assert_eq!(fs::read(root.join("mcp.json")).unwrap(), before);
    assert!(!root.join("must-not-execute").exists());
}

#[test]
fn codex_plugin_compat_t15_rejects_document_shape_and_missing_package_schema() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp.json");
    for value in [
        json!({"$schema":SCHEMA}),
        json!({"$schema":SCHEMA,"mcpServers":[]}),
        json!({"$schema":SCHEMA,"mcpServers":{},"extensions":{"com.openai":{"auth":"private-payload"}}}),
        json!({"$schema":null,"mcpServers":{}}),
    ] {
        write(&path, &value);
        let error = match load_plugin_mcp_declarations_from_path(&path) {
            Ok(_) => panic!("invalid document accepted"),
            Err(error) => error.to_string(),
        };
        assert!(!error.contains("private-payload"));
    }
    let workspace = temp.path().join("workspace");
    let root = plugin(&workspace, "pack");
    write(
        &root.join("mcp.json"),
        &json!({"mcpServers":{"remote":{"type":"http","url":"https://example.test/mcp"}}}),
    );
    let manifest = crate::plugins::load_plugin_manifest(&root).unwrap();
    assert!(
        crate::plugins::mcp::load_plugin_mcp_declarations(&manifest).is_err(),
        "selected portable package must not fall back to legacy MCP parsing"
    );
}

#[test]
fn codex_plugin_compat_t15_retains_siblings_and_reports_transport_fields() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp.json");
    write(
        &path,
        &json!({"$schema":SCHEMA,"mcpServers":{
            "good":{"type":"streamable-http","url":"https://example.test/mcp"},
            "events":{"type":"sse","url":"https://example.test/events"},
            "unknown":{"type":"private-payload","url":"https://example.test/mcp"},
            "conflict":{"type":"stdio","command":"fixture","url":"https://example.test/mcp"},
            "missing":{"command":"fixture"}
        }}),
    );
    let outcome = load_plugin_mcp_declarations_from_path(&path).unwrap();
    assert_eq!(outcome.servers.len(), 1);
    assert_eq!(config(&outcome.servers[0]).0, "good");
    assert_eq!(outcome.errors.len(), 4);
    assert!(outcome.errors.iter().any(|error| error.name == "events"
        && error.message.contains("sse")
        && error.message.contains("not supported")));
    assert!(
        outcome
            .errors
            .iter()
            .any(|error| error.name == "conflict" && error.message.contains("url"))
    );
    let error = outcome.into_runtime_servers("fixture").unwrap_err().to_string();
    assert!(error.contains("unknown") && error.contains("type"));
    assert!(!error.contains("private-payload"));
}

#[test]
fn codex_plugin_compat_t15_validates_stdio_fields_and_preserves_raw_arguments() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp.json");
    for value in [
        json!({"type":"stdio","command":"/private-payload"}),
        json!({"type":"stdio","command":"./../private-payload"}),
        json!({"type":"stdio","command":"C:\\private-payload"}),
        json!({"type":"stdio","command":"fixture","cwd":null}),
        json!({"type":"stdio","command":"fixture","cwd":"${PLUGIN_DATA}/../private-payload"}),
        json!({"type":"stdio","command":"fixture","args":null}),
        json!({"type":"stdio","command":"fixture","env":null}),
        json!({"type":"stdio","command":"fixture","env":{"PLUGIN_ROOT":"private-payload"}}),
        json!({"type":"stdio","command":"fixture","env":{"PLUGIN_DATA":"private-payload"}}),
        json!({"type":"stdio","command":"fixture","env":{"A":false}}),
        json!({"type":"stdio","command":"fixture","args":[false]}),
    ] {
        write(&path, &json!({"$schema":SCHEMA,"mcpServers":{"invalid":value}}));
        let outcome = load_plugin_mcp_declarations_from_path(&path).unwrap();
        assert!(outcome.servers.is_empty());
        assert_eq!(outcome.errors.len(), 1);
        assert!(!outcome.errors[0].message.contains("private-payload"));
    }
    write(
        &path,
        &json!({"$schema":SCHEMA,"mcpServers":{"local":{"type":"stdio","command":"./bin/server","args":["", " value ","${PLUGIN_ROOT}:${PLUGIN_DATA}"],"env":{"EMPTY":"","CACHE":"${PLUGIN_DATA}/cache"},"cwd":"${PLUGIN_DATA}/work"}}}),
    );
    let outcome = load_plugin_mcp_declarations_from_path(&path).unwrap();
    assert!(outcome.errors.is_empty());
    let (_, config) = config(&outcome.servers[0]);
    assert_eq!(config["args"], json!(["", " value ", "${PLUGIN_ROOT}:${PLUGIN_DATA}"]));
    assert_eq!(config["env"]["CACHE"], "${PLUGIN_DATA}/cache");
    assert_eq!(config["cwd"], "${PLUGIN_DATA}/work");
}

#[test]
fn codex_plugin_compat_t15_validates_http_before_filtering_client_headers() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp.json");
    for value in [
        json!({"type":"streamable-http","url":"http://example.test/private-payload"}),
        json!({"type":"streamable-http","url":"https://user:private-payload@example.test/mcp"}),
        json!({"type":"streamable-http","url":"https://example.test/mcp#private-payload"}),
        json!({"type":"streamable-http","url":"file:///private-payload"}),
        json!({"type":"streamable-http","url":"https://example.test/mcp","headers":null}),
        json!({"type":"streamable-http","url":"https://example.test/mcp","headers":{"X-Fixture":"a","x-fixture":"private-payload"}}),
        json!({"type":"streamable-http","url":"https://example.test/mcp","headers":{"Authorization":"private-payload\r\nInjected: true"}}),
        json!({"type":"streamable-http","url":"https://example.test/mcp","headers":{"Invalid Header":"private-payload"}}),
    ] {
        write(&path, &json!({"$schema":SCHEMA,"mcpServers":{"invalid":value}}));
        let outcome = load_plugin_mcp_declarations_from_path(&path).unwrap();
        assert!(outcome.servers.is_empty());
        assert_eq!(outcome.errors.len(), 1);
        assert!(!outcome.errors[0].message.contains("private-payload"));
    }
    for url in [
        "http://localhost/mcp",
        "http://127.0.0.1/mcp",
        "http://[::1]/mcp",
        "https://example.test/mcp",
    ] {
        write(
            &path,
            &json!({"$schema":SCHEMA,"mcpServers":{"remote":{"type":"streamable-http","url":url,"headers":{"aUtHoRiZaTiOn":"fixture","Host":"example.test","mcp-session-id":"fixture","X-Fixture":"café"}}}}),
        );
        let outcome = load_plugin_mcp_declarations_from_path(&path).unwrap();
        assert!(outcome.errors.is_empty());
        assert_eq!(config(&outcome.servers[0]).1["http_headers"], json!({"X-Fixture":"café"}));
    }
}

#[test]
fn codex_plugin_compat_t15_limits_independent_overlay_to_local_env_references() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let root = plugin(&workspace, "pack");
    write(
        &root.join("plugin.json"),
        &json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"pack","extensions":{"com.openai":{"interface":{"displayName":"Fixture"},"mcpServers":{"local":{"command":"inline-must-not-run","env_vars":["INLINE"]}}}}}),
    );
    write(
        &root.join("mcp.json"),
        &json!({"$schema":SCHEMA,"mcpServers":{
            "local":{"type":"stdio","command":"original","args":["original"],"env":{"TOKEN":"${TOKEN}","FIXTURE":"keep"}},
            "remote":{"type":"streamable-http","url":"https://example.test/mcp"}
        }}),
    );
    write(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"name":"ignored","mcpServers":"./overlay.json"}),
    );
    write(
        &root.join("overlay.json"),
        &json!({"local":{"command":"must-not-replace","env_vars":["TOKEN",{"name":"LOCAL","source":"local"},{"name":"REMOTE","source":"remote"}]},"remote":{"command":"must-not-replace","env_vars":["HTTP_INJECTION"]},"injected":{"command":"must-not-run","env_vars":["INJECTED"]}}),
    );
    let before =
        [root.join("mcp.json"), root.join("overlay.json")].map(|path| fs::read(path).unwrap());
    let manifest = crate::plugins::load_plugin_manifest(&root).unwrap();
    let (outcome, _) = crate::plugins::mcp::load_plugin_mcp_declarations(&manifest).unwrap();
    assert_eq!(outcome.servers.len(), 2);
    let (_, local) = config(&outcome.servers[0]);
    assert_eq!(local["command"], "original");
    assert_eq!(local["args"], json!(["original"]));
    assert_eq!(local["env_vars"], json!(["TOKEN",{"name":"LOCAL","source":"local"}]));
    assert_eq!(local["env"], json!({"FIXTURE":"keep"}));
    assert!(!config(&outcome.servers[1]).1.contains_key("env_vars"));
    assert_eq!(
        [root.join("mcp.json"), root.join("overlay.json")].map(|path| fs::read(path).unwrap()),
        before
    );
    assert!(!root.join("must-not-run").exists());
}

#[test]
fn codex_plugin_compat_t15_keeps_pending_runtime_settings_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp.json");
    for value in [
        json!({"type":"stdio","command":"fixture"}),
        json!({"type":"streamable-http","url":"https://example.test/mcp","headers":{"X-Fixture":"private-payload"}}),
    ] {
        write(&path, &json!({"$schema":SCHEMA,"mcpServers":{"pending":value}}));
        let outcome = load_plugin_mcp_declarations_from_path(&path).unwrap();
        assert!(outcome.errors.is_empty());
        let error = outcome.into_runtime_servers("fixture").unwrap_err().to_string();
        assert!(error.contains("not supported by the runtime yet"));
        assert!(!error.contains("private-payload"));
    }
}

#[test]
fn codex_plugin_compat_t15_keeps_manifest_format_authoritative() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let root = workspace.join(".vibewindow/plugins/legacy");
    write(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"name":"legacy","mcpServers":"./servers.json"}),
    );
    write(
        &root.join("servers.json"),
        &json!({"$schema":SCHEMA,"mcpServers":{"legacy":{"type":"sse","url":"https://example.test/mcp"}}}),
    );
    let options = PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    );
    let mut servers = Vec::new();
    let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
    assert!(
        result.is_ok(),
        "legacy package must retain command/url inference even with schema metadata: {result:?}"
    );
    assert!(matches!(&servers[0], McpServer::Http(_)));
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t15_reaudits_portable_files_and_overlay_links() {
    for overlay in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = plugin(temp.path(), "pack");
        write(
            &root.join("mcp.json"),
            &json!({"$schema":SCHEMA,"mcpServers":{"remote":{"type":"streamable-http","url":"https://example.test/mcp"}}}),
        );
        let manifest = crate::plugins::load_plugin_manifest(&root).unwrap();
        let outside = temp.path().join("outside.json");
        fs::write(&outside, b"private-payload").unwrap();
        let path = if overlay {
            fs::create_dir(root.join(".codex-plugin")).unwrap();
            root.join(".codex-plugin/plugin.json")
        } else {
            fs::remove_file(root.join("mcp.json")).unwrap();
            root.join("mcp.json")
        };
        std::os::unix::fs::symlink(&outside, path).unwrap();
        let error = match crate::plugins::mcp::load_plugin_mcp_declarations(&manifest) {
            Ok(_) => panic!("symlink accepted"),
            Err(error) => format!("{error:#}"),
        };
        assert!(!error.contains("private-payload"));
        assert_eq!(fs::read(outside).unwrap(), b"private-payload");
    }
}
