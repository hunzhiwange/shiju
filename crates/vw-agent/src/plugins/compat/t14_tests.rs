//! 旧版 MCP 原包经实际文件、发现和合并入口加载。

use crate::plugins::PluginDiscoveryOptions;
use crate::plugins::mcp::{
    load_plugin_mcp_declarations_from_path, load_plugin_mcp_servers_from_path,
    merge_enabled_plugin_mcp_servers,
};
use agent_client_protocol::McpServer;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use vw_config_types::plugins::PluginsConfig;

fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn plugin(workspace: &Path, name: &str, declaration: Option<Value>) -> PathBuf {
    let root = workspace.join(".vibewindow/plugins").join(name);
    let mut manifest = json!({"name":name});
    if let Some(declaration) = declaration {
        manifest["mcpServers"] = declaration;
    }
    write(&root.join(".codex-plugin/plugin.json"), &manifest);
    root
}

fn names(servers: &[McpServer]) -> Vec<&str> {
    servers
        .iter()
        .map(|server| match server {
            McpServer::Stdio(server) => server.name.as_str(),
            McpServer::Http(server) => server.name.as_str(),
            McpServer::Sse(server) => server.name.as_str(),
            _ => panic!("unexpected transport"),
        })
        .collect()
}

#[test]
fn codex_plugin_compat_t14_loads_wrapped_and_inline_servers() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let root = plugin(&workspace, "file-pack", Some(json!("./servers.json")));
    fs::write(
        root.join("servers.json"),
        include_bytes!("../../../testdata/plugin_compat/t14/wrapped.json"),
    )
    .unwrap();
    plugin(
        &workspace,
        "inline-pack",
        Some(json!({"inline":{"command":"inline-server","env":{"FIXTURE":" value "}}})),
    );
    let options = PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    );
    let mut servers = Vec::new();
    let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
    assert!(result.is_ok(), "wrapped map and inline declarations must reach runtime: {result:?}");
    assert_eq!(names(&servers), ["file", "inline"]);
    let McpServer::Stdio(inline) = &servers[1] else { panic!("stdio expected") };
    assert_eq!(inline.env[0].value, " value ");
}

#[test]
fn codex_plugin_compat_t14_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("invalid.json");
    write(&path, &json!({"mcpServers":{"first-bad":{"args":[]},"second-bad":false}}));
    let error = load_plugin_mcp_servers_from_path(&path).unwrap_err().to_string();
    assert!(
        error.contains("first-bad") && error.contains("second-bad"),
        "every rejected server must be diagnosed: {error}"
    );
}

#[test]
fn codex_plugin_compat_t14_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let root = plugin(&workspace, "default-pack", None);
    write(
        &root.join(".mcp.json"),
        &json!({"default":{"command":"not-executed"},"user":{"command":"plugin-command"}}),
    );
    plugin(&workspace, "disabled-pack", Some(json!({"bad":false})));
    fs::write(workspace.join(".vibewindow/plugins/disabled-pack.disabled"), b"").unwrap();
    let before = hex::encode(Sha256::digest(fs::read(root.join(".mcp.json")).unwrap()));
    let mut servers =
        vw_acp::parse_mcp_servers(&json!([{"name":"user","command":"user-command"}]), "fixture")
            .unwrap();
    let options = PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    );
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(
        names(&servers),
        ["user", "default"],
        "default .mcp.json must load without enabling disabled declarations"
    );
    assert_eq!(hex::encode(Sha256::digest(fs::read(root.join(".mcp.json")).unwrap())), before);
    let McpServer::Stdio(user) = &servers[0] else { panic!("stdio expected") };
    assert_eq!(user.command, Path::new("user-command"));
    assert!(!root.join("not-executed").exists());
}

#[test]
fn codex_plugin_compat_t14_retains_valid_siblings_and_normalized_aliases() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("servers.json");
    write(
        &path,
        &json!({
            "good":{"type":"streamable-http","name":"display-only","url":"https://example.test/mcp","scopes":[],"oauth":{"clientId":"legacy-client","client_id":"preferred-client","clientSecret":"fixture-only-secret","callbackUrl":"http://127.0.0.1/callback","callbackPort":4321}},
            "bad-one":{"args":[]},"bad-two":false
        }),
    );
    let before = fs::read(&path).unwrap();
    let outcome = load_plugin_mcp_declarations_from_path(&path).unwrap();
    assert_eq!(
        outcome.errors.iter().map(|error| error.name.as_str()).collect::<Vec<_>>(),
        ["bad-one", "bad-two"]
    );
    assert_eq!(outcome.servers.len(), 1, "invalid siblings must not erase normalized declarations");
    let super::mcp_legacy::PluginMcpDeclaration::Legacy { name, config } = &outcome.servers[0]
    else {
        panic!("named declaration expected")
    };
    assert_eq!(name, "good");
    assert!(!config.contains_key("name") && !config.contains_key("type"));
    assert_eq!(config["scopes"], json!([]));
    assert_eq!(
        config["oauth"],
        json!({"client_id":"preferred-client","client_secret":"fixture-only-secret","callback_url":"http://127.0.0.1/callback","callback_port":4321})
    );
    let error = outcome.into_runtime_servers("fixture").unwrap_err().to_string();
    assert!(error.contains("bad-one") && error.contains("bad-two") && error.contains("good"));
    assert!(!error.contains("fixture-only-secret") && !error.contains("preferred-client"));
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn codex_plugin_compat_t14_preserves_acp_array_shapes_and_per_entry_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("servers.json");
    for wrap in [false, true] {
        let servers = json!([{"name":"stdio","command":"fixture","args":["serve"],"env":[{"name":"A","value":"B"}]},{"name":"events","type":"sse","url":"https://example.test/events"}]);
        write(&path, &if wrap { json!({"mcpServers":servers}) } else { servers });
        let servers = load_plugin_mcp_servers_from_path(&path).unwrap();
        assert_eq!(names(&servers), ["stdio", "events"]);
        assert!(
            matches!(servers[1], McpServer::Sse(_)),
            "existing ACP array transport stays intact"
        );
    }
    write(&path, &json!([{"name":"ok","command":"fixture"},{"name":"bad-one"},false]));
    let outcome = load_plugin_mcp_declarations_from_path(&path).unwrap();
    assert_eq!(outcome.servers.len(), 1);
    assert_eq!(
        outcome.errors.iter().map(|error| error.name.as_str()).collect::<Vec<_>>(),
        ["bad-one", "#2"]
    );
    assert!(load_plugin_mcp_servers_from_path(&path).is_err());
}

#[test]
fn codex_plugin_compat_t14_infers_transport_without_treating_labels_as_capabilities() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("servers.json");
    for label in ["http", "streamable_http", "streamable-http", "stdio", "sse", "unknown"] {
        write(
            &path,
            &json!({"remote":{"type":label,"name":"not-an-identity","url":"https://example.test/mcp","oauth":{}}}),
        );
        let servers = load_plugin_mcp_servers_from_path(&path).unwrap();
        assert_eq!(names(&servers), ["remote"]);
        assert!(
            matches!(servers[0], McpServer::Http(_)),
            "legacy label {label} is not an SSE declaration"
        );
    }
    write(
        &path,
        &json!({"stdio":{"type":"http","command":"fixture","args":["", " x "],"env":{"EMPTY":"","SPACED":" x "}}}),
    );
    let servers = load_plugin_mcp_servers_from_path(&path).unwrap();
    let McpServer::Stdio(server) = &servers[0] else { panic!("command chooses stdio") };
    assert_eq!(server.args, ["", " x "]);
    assert_eq!(
        server
            .env
            .iter()
            .map(|entry| (entry.name.as_str(), entry.value.as_str()))
            .collect::<Vec<_>>(),
        [("EMPTY", ""), ("SPACED", " x ")]
    );
}

#[test]
fn codex_plugin_compat_t14_rejects_malformed_documents_and_unsupported_runtime_settings() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("servers.json");
    for value in
        [json!(true), json!("private-fixture-payload"), json!(null), json!({"mcpServers":42})]
    {
        write(&path, &value);
        let error = load_plugin_mcp_servers_from_path(&path).unwrap_err().to_string();
        assert!(error.contains("expected array or object"));
        assert!(!error.contains("private-fixture-payload"));
    }
    for value in [
        json!({"command":"fixture","env":{"A":false}}),
        json!({"command":"fixture","args":[1]}),
        json!({"command":"fixture","url":"https://example.test/mcp"}),
        json!({"url":"https://example.test/mcp","env":{"A":"B"}}),
        json!({"command":"fixture","enabled":"no"}),
    ] {
        write(&path, &json!({"bad":value}));
        assert!(load_plugin_mcp_servers_from_path(&path).is_err());
    }
    for field in [
        "cwd",
        "oauth",
        "auth",
        "bearer_token",
        "required",
        "env_vars",
        "enabled_tools",
        "http_headers",
    ] {
        let mut config = json!({"command":"fixture"});
        config[field] = if field == "oauth" {
            json!({"clientSecret":"private-fixture-payload"})
        } else {
            json!("private-fixture-payload")
        };
        write(&path, &json!({"restricted":config}));
        let error = load_plugin_mcp_servers_from_path(&path).unwrap_err().to_string();
        assert!(error.contains(field) && error.contains("restricted"));
        assert!(!error.contains("private-fixture-payload"));
    }
    for empty in [json!({}), json!([]), json!({"mcpServers":{}}), json!({"mcpServers":[]})] {
        write(&path, &empty);
        assert!(load_plugin_mcp_servers_from_path(&path).unwrap().is_empty());
    }
}

#[test]
fn codex_plugin_compat_t14_uses_explicit_declarations_before_legacy_defaults() {
    for legacy in ["codex", "claude", "cursor"] {
        for declaration in
            [None, Some(json!("./explicit.json")), Some(json!({"inline":{"command":"fixture"}}))]
        {
            let temp = tempfile::tempdir().unwrap();
            let workspace = temp.path().join("workspace");
            let root = plugin(&workspace, "pack", declaration.clone());
            if legacy != "codex" {
                fs::rename(root.join(".codex-plugin"), root.join(format!(".{legacy}-plugin")))
                    .unwrap();
            }
            write(
                &root.join(".mcp.json"),
                &if declaration.is_none() {
                    json!({"default":{"command":"fixture"}})
                } else {
                    json!(false)
                },
            );
            write(&root.join("explicit.json"), &json!({"explicit":{"command":"fixture"}}));
            let options = PluginDiscoveryOptions::new(
                workspace,
                temp.path().join("config"),
                PluginsConfig::default(),
            );
            let mut servers = Vec::new();
            merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
            let expected = match declaration {
                None => "default",
                Some(Value::String(_)) => "explicit",
                _ => "inline",
            };
            assert_eq!(names(&servers), [expected], "{legacy}");
        }
    }
}

#[test]
fn codex_plugin_compat_t14_keeps_rejected_inline_configs_out_of_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    plugin(&workspace, "inline-pack", Some(json!({"good":{"command":"fixture"},"bad":false})));
    let options = PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    );
    let mut servers =
        vw_acp::parse_mcp_servers(&json!([{"name":"user","command":"fixture"}]), "fixture")
            .unwrap();
    let error = merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap_err();
    assert!(format!("{error:#}").contains("server 'bad'"));
    assert_eq!(names(&servers), ["user"], "failed config must not partially install good siblings");
}

#[test]
fn codex_plugin_compat_t14_skips_absent_defaults_and_disabled_named_servers() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    plugin(&workspace, "absent", None);
    plugin(
        &workspace,
        "disabled-server",
        Some(json!({"muted":{"command":"must-not-run","enabled":false}})),
    );
    let options = PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    );
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert!(servers.is_empty());
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t14_reaudits_implicit_default_links() {
    for dangling in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let root = plugin(&workspace, "pack", None);
        let outside = temp.path().join("outside.json");
        if !dangling {
            fs::write(&outside, b"private-fixture-payload").unwrap();
        }
        std::os::unix::fs::symlink(&outside, root.join(".mcp.json")).unwrap();
        let options = PluginDiscoveryOptions::new(
            workspace,
            temp.path().join("config"),
            PluginsConfig::default(),
        );
        let mut servers = Vec::new();
        let error =
            merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap_err().to_string();
        assert!(!error.contains("private-fixture-payload"));
        assert!(servers.is_empty());
        if !dangling {
            assert_eq!(fs::read(outside).unwrap(), b"private-fixture-payload");
        }
    }
}
