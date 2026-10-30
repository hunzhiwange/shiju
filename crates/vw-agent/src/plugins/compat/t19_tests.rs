use crate::tools::mcp_tool::mcp_tool_id;

#[test]
fn codex_plugin_compat_t19_calls_exact_namespaced_tool() {
    let a = mcp_tool_id("package-a", "Read.File").unwrap();
    let b = mcp_tool_id("package-a", "read_file").unwrap();
    assert_ne!(a, b, "different raw names must remain independently callable");
}
#[test]
fn codex_plugin_compat_t19_rejects_invalid_input() {
    let id = mcp_tool_id("server", &"x".repeat(200)).unwrap();
    assert!(id.len() <= 64, "model callable identifier must fit the name limit");
}
#[test]
fn codex_plugin_compat_t19_preserves_isolation() {
    assert_ne!(
        mcp_tool_id("a-b", "tool").unwrap(),
        mcp_tool_id("a_b", "tool").unwrap(),
        "distinct source namespaces must not collapse"
    );
}

use super::{mcp_catalog, mcp_connections::McpSessionConnections};
use crate::tools::{
    Tool,
    mcp_tool::{McpTool, discover_mcp_tool_specs},
};
use agent_client_protocol::{McpServer, McpServerStdio};
use serde_json::json;
use std::sync::Arc;
fn server(mode: &str) -> McpServer {
    McpServer::Stdio(McpServerStdio::new("catalog", "python3").args(vec![
        format!("{}/testdata/plugin_compat/t19/server.py", env!("CARGO_MANIFEST_DIR")),
        mode.into(),
    ]))
}
#[tokio::test]
async fn codex_plugin_compat_t19_paged_names_route_to_original_wire_names() {
    let pool = Arc::new(McpSessionConnections::default());
    let server = server("normal");
    let specs = pool
        .with_peer(&server, None, |peer| {
            Box::pin(async move { discover_mcp_tool_specs(peer, "catalog", None, &[]).await })
        })
        .await
        .unwrap();
    assert_eq!(specs.len(), 2);
    assert_ne!(specs[0].id, specs[1].id);
    for (spec, raw) in specs.into_iter().zip(["Read.File", "read_file"]) {
        let tool = McpTool::from_session_server(server.clone(), raw, spec, pool.clone(), None);
        let result = tool.call(json!({})).await.unwrap();
        assert_eq!(result.model_text(), format!("{raw}:normal"));
    }
    pool.shutdown();
}
#[tokio::test]
async fn codex_plugin_compat_t19_rejects_loop_and_oversized_cursor() {
    for (mode, expected) in
        [("repeat", "repeated"), ("large", "65536"), ("pages", "100 pages"), ("items", "2048")]
    {
        let pool = McpSessionConnections::default();
        let error = pool
            .with_peer(&server(mode), None, |peer| Box::pin(mcp_catalog::list_tools(peer)))
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains(expected), "{error:#}");
        pool.shutdown();
    }
}
#[test]
fn codex_plugin_compat_t19_keeps_empty_allowlist_and_deny_precedence() {
    use super::{mcp_connection_settings as settings, mcp_legacy::parse_legacy_mcp};
    let root = tempfile::tempdir().unwrap();
    for (allow, expected) in [(json!([]), false), (json!(["read"]), true)] {
        let mut outcome=parse_legacy_mcp(json!({"catalog":{"command":"unused","enabled_tools":allow,"disabled_tools":["blocked"]}}),"fixture").unwrap();
        let mut entries = settings::prepare(&mut outcome, root.path()).unwrap();
        let mut server = outcome.into_runtime_servers("fixture").unwrap().remove(0);
        settings::bind(&mut server, entries.remove("catalog").unwrap()).unwrap();
        assert_eq!(settings::tool_allowed(&server, "read").unwrap(), expected);
        assert!(!settings::tool_allowed(&server, "blocked").unwrap());
    }
}

#[test]
fn codex_plugin_compat_t19_retains_same_named_servers_from_different_packages() {
    use crate::plugins::{PluginDiscoveryOptions, merge_enabled_plugin_mcp_servers};
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    for name in ["alpha", "beta"] {
        let package = workspace.join(".vibewindow/plugins").join(name);
        std::fs::create_dir_all(package.join(".codex-plugin")).unwrap();
        std::fs::write(
            package.join(".codex-plugin/plugin.json"),
            serde_json::to_vec(&json!({"name":name,"mcpServers":{"shared":{"command":name}}}))
                .unwrap(),
        )
        .unwrap();
    }
    let options =
        PluginDiscoveryOptions::new(workspace, root.path().join("config"), Default::default());
    let mut servers = vec![];
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 2, "no package may silently lose its same-named server");
    let names = servers.iter().map(crate::tools::mcp_common::server_name).collect::<Vec<_>>();
    assert_ne!(names[0], names[1]);
    assert!(matches!(&servers[1],McpServer::Stdio(s) if s.command.to_string_lossy()=="beta"));
}

#[tokio::test]
async fn codex_plugin_compat_t19_denied_tool_cannot_bypass_catalog_filter() {
    let pool = Arc::new(McpSessionConnections::default());
    let mut server = server("normal");
    super::mcp_connection_settings::bind(
        &mut server,
        super::mcp_connection_settings::ConnectionSettings {
            enabled_tools: Some(vec![]),
            ..Default::default()
        },
    )
    .unwrap();
    let spec =
        crate::tools::ToolSpec::new("mcp__catalog__read_file", "denied", json!({"type":"object"}));
    let tool = McpTool::from_session_server(server, "read_file", spec, pool.clone(), None);
    let error = tool.call(json!({})).await.unwrap_err();
    assert!(format!("{error:#}").contains("disabled by its declaration"));
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t19_same_named_plugin_tools_call_their_own_process() {
    use crate::plugins::{PluginDiscoveryOptions, merge_enabled_plugin_mcp_servers};
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    for name in ["alpha", "beta"] {
        let package = workspace.join(".vibewindow/plugins").join(name);
        std::fs::create_dir_all(package.join(".codex-plugin")).unwrap();
        let args = vec![
            format!("{}/testdata/plugin_compat/t19/server.py", env!("CARGO_MANIFEST_DIR")),
            name.into(),
        ];
        std::fs::write(
            package.join(".codex-plugin/plugin.json"),
            serde_json::to_vec(
                &json!({"name":name,"mcpServers":{"shared":{"command":"python3","args":args}}}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    let options =
        PluginDiscoveryOptions::new(workspace, root.path().join("config"), Default::default());
    let mut servers = vec![];
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let pool = Arc::new(McpSessionConnections::default());
    let mut ids = vec![];
    for (server, expected) in servers.into_iter().zip(["alpha", "beta"]) {
        let label = crate::tools::mcp_common::server_name(&server).to_owned();
        let specs = pool
            .with_peer(&server, None, move |peer| {
                let label = label.clone();
                Box::pin(async move { discover_mcp_tool_specs(peer, &label, None, &[]).await })
            })
            .await
            .unwrap();
        let spec = specs[1].clone();
        ids.push(spec.id.clone());
        let tool = McpTool::from_session_server(server, "read_file", spec, pool.clone(), None);
        assert_eq!(
            tool.call(json!({})).await.unwrap().model_text(),
            format!("read_file:{expected}")
        );
    }
    assert_ne!(ids[0], ids[1]);
    pool.shutdown();
}

#[test]
fn codex_plugin_compat_t19_runtime_registry_applies_plugin_filters_and_parallel_limit() {
    use crate::tools::{ToolRuntimeContext, tool_specs_for_context};
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    let package = workspace.join(".vibewindow/plugins/alpha");
    std::fs::create_dir_all(package.join(".codex-plugin")).unwrap();
    std::fs::write(workspace.join(".vwacprc.json"), r#"{"mcpServers":[]}"#).unwrap();
    let args = vec![
        format!("{}/testdata/plugin_compat/t19/server.py", env!("CARGO_MANIFEST_DIR")),
        "normal".into(),
    ];
    std::fs::write(package.join(".codex-plugin/plugin.json"),serde_json::to_vec(&json!({"name":"alpha","mcpServers":{"catalog":{"command":"python3","args":args,"enabled_tools":["read_file"],"supports_parallel_tool_calls":false}}})).unwrap()).unwrap();
    let config = crate::app::agent::config::Config {
        workspace_dir: workspace.clone(),
        config_path: root.path().join("config/config.toml"),
        ..Default::default()
    };
    let context = ToolRuntimeContext::new("t19-policy", Some(workspace.display().to_string()))
        .with_test_config(config);
    let specs = tool_specs_for_context(&context);
    assert!(
        specs.iter().any(|spec| spec.id == "mcp__catalog__read_file" && !spec.concurrency_safe)
    );
    let excluded = mcp_tool_id("catalog", "Read.File").unwrap();
    assert!(specs.iter().all(|spec| spec.id != excluded));
    crate::plugins::close_session_mcp_connections("t19-policy", &workspace);
}
