//! Authentication declaration rejection through actual plugin discovery and merge.
use crate::PluginDiscoveryOptions;
use agent_client_protocol::McpServer;
use config_plugins::PluginsConfig;
use serde_json::{Value, json};
use std::{fs, path::{Path, PathBuf}};

fn write(path: &Path, value: Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

fn options(temp: &Path) -> PluginDiscoveryOptions {
    PluginDiscoveryOptions::new(
        temp.join("workspace"),
        temp.join("config"),
        PluginsConfig::default(),
    )
}

fn plugin(options: &PluginDiscoveryOptions, name: &str, server: Value) -> PathBuf {
    let root = options.workspace_dir.join(".vibewindow/plugins").join(name);
    write(
        &root.join(".codex-plugin/plugin.json"),
        json!({"name":name,"mcpServers":{"shared":server}}),
    );
    root
}

// Synchronous fixtures have no tool context or turn snapshot. Match the host's
// context-free selection branch with real discovery, without mocking policy.
fn merge_enabled_plugin_mcp_servers(
    servers: &mut Vec<McpServer>,
    options: &PluginDiscoveryOptions,
) -> anyhow::Result<()> {
    crate::mcp::merge_enabled_plugin_mcp_servers(servers, options, &|options| {
        Ok((crate::discover_plugins(options)?, options.clone()))
    })
}

#[test]
fn codex_plugin_compat_t17_rejects_invalid_public_auth_without_values() {
    for (index, auth) in [
        json!({"type":"none","clientSecret":"fixture-only-secret"}),
        json!({"type":"hosted"}),
        json!({"type":"oauth","client":{"mode":"provided"}}),
        json!({"type":"oauth","client":{"mode":"provided","clientId":" ","tokenEndpointAuthMethod":"none"}}),
        json!({"type":"oauth","client":{"mode":"provided","clientId":"public","tokenEndpointAuthMethod":"unknown"}}),
        json!({"type":"oauth","client":{"mode":"dcr","tokenEndpointAuthMethod":"none"}}),
        json!({"type":"oauth","defaultScopes":[" "]}),
        json!({"type":"oauth","baseScopes":false}),
        json!({"type":"oauth","tokenUrl":"https://fixture-only-secret@example.test/token"}),
        json!({"type":"oauth","authorizationUrl":"https://example.test#fragment"}),
        json!({"type":"oauth","registrationUrl":"relative"}),
        json!({"type":"oauth","oidcEnabled":"fixture-only-secret"}),
        json!({"type":"oauth","defaultScopes":null}),
        json!({"type":"api_key","headerScheme":"custom_header"}),
        json!({"type":"api_key","headerScheme":"custom_header","headerName":"Host"}),
        json!({"type":"api_key","headerScheme":"custom_header","headerName":"invalid name"}),
        json!({"type":"api_key","headerScheme":"bearer","headerName":"X-Fixture"}),
        json!({"type":"api_key","headerScheme":"unknown"}),
        json!({"type":"api_key","headerScheme":"basic","token":"fixture-only-secret"}),
    ].into_iter().enumerate() {
        let temp = tempfile::tempdir().unwrap();
        let options = options(temp.path());
        plugin(
            &options,
            "local",
            json!({"url":"https://example.test/mcp","extensions":{"com.openai":{"auth":auth}}}),
        );
        let mut servers = Vec::new();
        let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
        assert!(result.is_err(), "public auth rejection case {index} must fail");
        let error = result.unwrap_err();
        assert!(!format!("{error:#}").contains("fixture-only-secret"));
        assert!(servers.is_empty());
    }
}

#[test]
fn codex_plugin_compat_t17_rejects_invalid_oauth_and_private_authentication() {
    for fields in [
        json!({"auth":"ema_auth"}),
        json!({"auth":"chatgpt"}),
        json!({"oauth":{"client_secret":"fixture-only-secret"}}),
        json!({"oauth":{"client_id":"public-client","client_secret":" "}}),
        json!({"oauth":{"callback_port":65536}}),
        json!({"oauth":{"token_endpoint_auth_method":"client_secret_basic"}}),
        json!({"oauth":{"ema_registration":{}}}),
        json!({"oauth":{"authorization_server_issuer":"https://example.test"}}),
        json!({"scopes":false}),
        json!({"oauth_resource":false}),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let options = options(temp.path());
        let mut config = fields;
        config["url"] = json!("https://example.test/mcp");
        plugin(&options, "local", config);
        let mut servers = Vec::new();
        let error = merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap_err();
        assert!(!format!("{error:#}").contains("fixture-only-secret"));
        assert!(servers.is_empty());
    }
}
