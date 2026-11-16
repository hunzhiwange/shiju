use crate::tools::mcp_tool::mcp_tool_id;
use serde_json::json;

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
