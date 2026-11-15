use super::mcp_connections::McpSessionConnections;
use crate::plugins::{PluginDiscoveryOptions, merge_enabled_plugin_mcp_servers};
use crate::tools::{Tool, mcp_resource::McpResourceTool};
use serde_json::json;
use std::sync::Arc;

fn config(servers: Vec<agent_client_protocol::McpServer>) -> vw_acp::ResolvedAcpxConfig {
    vw_acp::ResolvedAcpxConfig {
        default_agent: "main".into(),
        default_permissions: vw_acp::PermissionMode::ApproveAll,
        non_interactive_permissions: vw_acp::NonInteractivePermissionPolicy::Deny,
        auth_policy: vw_acp::AuthPolicy::Skip,
        ttl_ms: 300_000,
        timeout_ms: None,
        queue_max_depth: 16,
        format: vw_acp::OutputFormat::Text,
        agents: Default::default(),
        auth: Default::default(),
        disable_exec: false,
        mcp_servers: servers,
        mcp_server_management: vec![],
        global_path: "global.json".into(),
        project_path: ".vwacprc.json".into(),
        has_global_config: false,
        has_project_config: false,
    }
}
fn fixture(
    root: &std::path::Path,
    names: &[&str],
    mode: &str,
) -> (McpResourceTool, Arc<McpSessionConnections>, Vec<String>) {
    let workspace = root.join("workspace");
    for name in names {
        let package = workspace.join(".vibewindow/plugins").join(name);
        std::fs::create_dir_all(package.join(".codex-plugin")).unwrap();
        std::fs::copy(
            format!("{}/testdata/plugin_compat/t22/server.py", env!("CARGO_MANIFEST_DIR")),
            package.join("server.py"),
        )
        .unwrap();
        std::fs::write(package.join(".codex-plugin/plugin.json"),serde_json::to_vec(&json!({
            "name":name,"mcpServers":{"shared":{"command":"python3","args":[package.join("server.py"),name,root.join(format!("{name}.jsonl")),mode]}}
        })).unwrap()).unwrap();
    }
    let mut servers = vec![];
    merge_enabled_plugin_mcp_servers(
        &mut servers,
        &PluginDiscoveryOptions::new(workspace, root.join("config"), Default::default()),
    )
    .unwrap();
    let names =
        servers.iter().map(|s| crate::tools::mcp_common::server_name(s).to_owned()).collect();
    let pool = Arc::new(McpSessionConnections::default());
    pool.synchronize(&servers).unwrap();
    (McpResourceTool::new(config(servers), pool.clone()), pool, names)
}
#[tokio::test]
async fn codex_plugin_compat_t22_preserves_resource_origin_and_prompt_roles() {
    let root = tempfile::tempdir().unwrap();
    let (tool, pool, names) = fixture(root.path(), &["alpha"], "normal");
    let name = &names[0];
    let read = tool
        .call(json!({"server":name,"action":"read_resource","uri":"fixture:///first"}))
        .await
        .unwrap();
    assert_eq!(read.data["server"], *name, "resource must retain host-bound server origin");
    assert_eq!(read.data["mcp_result"]["contents"][0]["_meta"]["marker"], "host-only");
    let prompt=tool.call(json!({"server":name,"action":"get_prompt","name":"first","arguments":{"question":"hello"}})).await.unwrap();
    assert_eq!(prompt.data["mcp_result"]["messages"][0]["role"], "user");
    assert_eq!(prompt.data["mcp_result"]["messages"][1]["role"], "assistant");
    assert!(!prompt.model_text().contains("host-only"));
    pool.shutdown();
}
#[tokio::test]
async fn codex_plugin_compat_t22_rejects_invalid_input() {
    let root = tempfile::tempdir().unwrap();
    let (tool, pool, names) = fixture(root.path(), &["alpha"], "normal");
    assert!(tool.call(json!({"server":names[0],"action":"get_prompt","name":"first","arguments":{"question":42}})).await.is_err(),"prompt arguments must remain protocol strings");
    pool.shutdown();
}
#[tokio::test]
async fn codex_plugin_compat_t22_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let (tool, pool, names) = fixture(root.path(), &["alpha", "beta"], "normal");
    let mut texts = vec![];
    for name in names {
        let result = tool
            .call(json!({"server":name,"action":"read_resource","uri":"file:///not-a-host-read"}))
            .await
            .unwrap();
        assert_eq!(result.data["server"], name, "server payload cannot replace host identity");
        texts.push(result.data["mcp_result"]["contents"][0]["text"].as_str().unwrap().to_owned());
    }
    assert_ne!(texts[0], texts[1]);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t22_lists_all_pages_and_preserves_metadata() {
    let root = tempfile::tempdir().unwrap();
    let (tool, pool, names) = fixture(root.path(), &["alpha"], "normal");
    for (action, field) in [
        ("list_resources", "resources"),
        ("list_resource_templates", "resourceTemplates"),
        ("list_prompts", "prompts"),
    ] {
        let result = tool.call(json!({"server":names[0],"action":action})).await.unwrap();
        let rows = result.data["mcp_result"][field].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "first");
        assert_eq!(rows[1]["name"], "second");
        assert_eq!(rows[0]["_meta"]["source"], "alpha");
        assert!(!result.model_text().contains("forged"));
        if field == "resources" {
            assert_eq!(rows[0]["size"], 1_u64 << 40);
        }
    }
    let requests = std::fs::read_to_string(root.path().join("alpha.jsonl")).unwrap();
    assert_eq!(requests.lines().filter(|s| s.contains("initialize")).count(), 1);
    assert_eq!(requests.lines().filter(|s| s.contains("\"cursor\": \"next\"")).count(), 3);
    pool.shutdown();
}
#[tokio::test]
async fn codex_plugin_compat_t22_rejects_invalid_actions_uris_and_parameter_shapes() {
    let root = tempfile::tempdir().unwrap();
    let (tool, pool, names) = fixture(root.path(), &["alpha"], "normal");
    for mut input in [
        json!({"action":"subscribe","uri":"fixture:///first"}),
        json!({"action":"read_resource","uri":"/tmp/local"}),
        json!({"action":"get_prompt","name":""}),
        json!({"action":"list_resources","uri":"fixture:///first"}),
        json!({"action":"get_prompt","name":"first","arguments":{"question":false}}),
        json!({"action":"list_resources","host_context":true}),
    ] {
        input["server"] = json!(names[0]);
        assert!(tool.call(input).await.is_err());
    }
    assert!(tool.call(json!({"server":"missing","action":"list_resources"})).await.is_err());
    assert!(
        !root.path().join("alpha.jsonl").exists(),
        "invalid input must fail before starting a process"
    );
    pool.shutdown();
}
#[tokio::test]
async fn codex_plugin_compat_t22_rejects_pagination_cycles_and_limits() {
    for (mode, expected) in [
        ("repeat", "repeated"),
        ("large-cursor", "65536"),
        ("items", "2048"),
        ("pages", "100 pages"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (tool, pool, names) = fixture(root.path(), &["alpha"], mode);
        for action in ["list_resources", "list_resource_templates", "list_prompts"] {
            let error = tool.call(json!({"server":names[0],"action":action})).await.unwrap_err();
            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }
        pool.shutdown();
    }
}
#[tokio::test]
async fn codex_plugin_compat_t22_rejects_remote_malformed_content_and_roles() {
    let root = tempfile::tempdir().unwrap();
    let (tool, pool, names) = fixture(root.path(), &["alpha"], "normal");
    for input in [
        json!({"server":names[0],"action":"read_resource","uri":"fixture:///invalid"}),
        json!({"server":names[0],"action":"get_prompt","name":"invalid","arguments":{"question":"hello"}}),
    ] {
        let error = tool.call(input).await.unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("MCP"));
        assert!(!message.contains("host-only"));
    }
    pool.shutdown();
}
#[tokio::test]
async fn codex_plugin_compat_t22_keeps_package_and_host_files_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let (tool, pool, names) = fixture(root.path(), &["alpha"], "normal");
    let package = root.path().join("workspace/.vibewindow/plugins/alpha");
    let script = std::fs::read(package.join("server.py")).unwrap();
    let manifest = std::fs::read(package.join(".codex-plugin/plugin.json")).unwrap();
    let host = root.path().join("host.txt");
    std::fs::write(&host, "host-secret").unwrap();
    let uri = url::Url::from_file_path(&host).unwrap().to_string();
    let result =
        tool.call(json!({"server":names[0],"action":"read_resource","uri":uri})).await.unwrap();
    assert_eq!(result.data["mcp_result"]["contents"][0]["uri"], uri);
    assert_eq!(result.data["mcp_result"]["contents"][0]["text"], format!("alpha:{uri}"));
    assert!(!result.model_text().contains("host-secret"));
    assert_eq!(std::fs::read_to_string(host).unwrap(), "host-secret");
    assert_eq!(std::fs::read(package.join("server.py")).unwrap(), script);
    assert_eq!(std::fs::read(package.join(".codex-plugin/plugin.json")).unwrap(), manifest);
    assert_eq!(result.data["mcp_result"]["contents"][1]["blob"], "AAEC");
    assert!(result.data["source_root"].as_str().unwrap().ends_with("/alpha"));
    pool.shutdown();
}
#[tokio::test]
async fn codex_plugin_compat_t22_disabled_and_cancelled_sessions_reject_before_connecting() {
    let root = tempfile::tempdir().unwrap();
    let (tool, pool, names) = fixture(root.path(), &["alpha"], "normal");
    pool.synchronize(&[]).unwrap();
    assert!(
        tool.call(json!({"server":names[0],"action":"read_resource","uri":"fixture:///first"}))
            .await
            .is_err()
    );
    assert!(!root.path().join("alpha.jsonl").exists());
    pool.shutdown();
}
#[tokio::test]
async fn codex_plugin_compat_t22_reads_and_gets_over_http_json_and_sse() {
    use tokio::{
        io::{AsyncBufReadExt, BufReader},
        process::Command,
    };
    for mode in ["json", "http-sse"] {
        let root = tempfile::tempdir().unwrap();
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/plugin_compat/t22/http_server.py");
        let mut child = Command::new("python3")
            .arg(script)
            .arg(mode)
            .arg(root.path().join("requests.jsonl"))
            .stdout(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut port = String::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            BufReader::new(child.stdout.take().unwrap()).read_line(&mut port),
        )
        .await
        .unwrap()
        .unwrap();
        let server =
            agent_client_protocol::McpServer::Http(agent_client_protocol::McpServerHttp::new(
                "http",
                format!("http://127.0.0.1:{}/mcp", port.trim()),
            ));
        let pool = Arc::new(McpSessionConnections::default());
        let tool = McpResourceTool::new(config(vec![server]), pool.clone());
        for input in [
            json!({"server":"http","action":"list_resources"}),
            json!({"server":"http","action":"read_resource","uri":"fixture:///first"}),
            json!({"server":"http","action":"get_prompt","name":"first","arguments":{"question":"hello"}}),
        ] {
            let result = tokio::time::timeout(std::time::Duration::from_secs(5), tool.call(input))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(result.data["server"], "http");
            assert!(!result.model_text().contains("host-only"));
            assert!(!result.data["mcp_result"].is_null());
        }
        pool.shutdown();
        child.kill().await.unwrap();
    }
}
#[test]
fn codex_plugin_compat_t22_registers_resource_tool_in_actual_runtime() {
    use crate::tools::{ToolRuntimeContext, execute_tool_call};
    let root = tempfile::tempdir().unwrap();
    let (_tool, pool, names) = fixture(root.path(), &["alpha"], "normal");
    pool.shutdown();
    let workspace = root.path().join("workspace");
    let manifest = workspace.join(".vibewindow/plugins/alpha/.codex-plugin/plugin.json");
    let mut declaration: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    declaration["mcpServers"]["shared"]["required"] = json!(true);
    std::fs::write(&manifest, serde_json::to_vec(&declaration).unwrap()).unwrap();
    let ctx =
        ToolRuntimeContext::new("t22-runtime", Some(workspace.to_string_lossy().into_owned()));
    let result=execute_tool_call("McpResource",&json!({"server":names[0],"action":"get_prompt","name":"first","arguments":{"question":"from runtime"}}).to_string(),&ctx).unwrap();
    assert_eq!(result.data["server"], names[0]);
    assert_eq!(result.data["mcp_result"]["messages"][0]["content"]["text"], "from runtime");
    crate::plugins::close_session_mcp_connections("t22-runtime", &workspace);
}

#[tokio::test]
async fn codex_plugin_compat_t22_inspection_retains_resource_and_prompt_metadata() {
    let root = tempfile::tempdir().unwrap();
    let (_tool, pool, _) = fixture(root.path(), &["alpha"], "normal");
    pool.shutdown();
    let mut servers = vec![];
    merge_enabled_plugin_mcp_servers(
        &mut servers,
        &PluginDiscoveryOptions::new(
            root.path().join("workspace"),
            root.path().join("config"),
            Default::default(),
        ),
    )
    .unwrap();
    let result = crate::tools::mcp_inspect::inspect_mcp_servers(&config(servers)).await;
    assert_eq!(
        result.servers[0].resources.len(),
        2,
        "inspection must use the same lossless resource boundary"
    );
    assert_eq!(result.servers[0].resources[0]["_meta"]["source"], "alpha");
    assert_eq!(result.servers[0].prompts[0]["_meta"]["source"], "alpha");
}

#[tokio::test]
async fn codex_plugin_compat_t22_preserves_admission_timeouts_and_sanitizes_remote_errors() {
    use agent_client_protocol::{McpServer, McpServerStdio};
    for mode in ["normal", "no-cap", "disabled"] {
        let root = tempfile::tempdir().unwrap();
        let journal = root.path().join("requests.jsonl");
        let server = McpServer::Stdio(McpServerStdio::new("local", "python3").args(vec![
            format!("{}/testdata/plugin_compat/t22/server.py", env!("CARGO_MANIFEST_DIR")),
            "local".into(),
            journal.to_string_lossy().into_owned(),
            mode.into(),
        ]));
        let mut config = config(vec![server]);
        let mut management = vw_acp::McpServerManagement::default_for_server("local");
        management.enabled = mode != "disabled";
        management.timeout_ms = Some(250);
        config.mcp_server_management.push(management);
        let pool = Arc::new(McpSessionConnections::default());
        let tool = McpResourceTool::new(config, pool.clone());
        if mode == "normal" {
            let error = tool
                .call(json!({"server":"local","action":"get_prompt","name":"first","arguments":{}}))
                .await
                .unwrap_err();
            assert!(format!("{error:#}").contains("-32602"));
            assert!(!format!("{error:#}").contains("host-only-error-detail"));
            let error = tokio::time::timeout(
                std::time::Duration::from_secs(4),
                tool.call(
                    json!({"server":"local","action":"read_resource","uri":"fixture:///slow"}),
                ),
            )
            .await
            .unwrap()
            .unwrap_err();
            assert!(format!("{error:#}").contains("timed out"));
        } else {
            let error = tool
                .call(json!({"server":"local","action":"read_resource","uri":"fixture:///first"}))
                .await
                .unwrap_err();
            assert!(format!("{error:#}").contains(if mode == "disabled" {
                "disabled"
            } else {
                "advertise resources"
            }));
            if journal.exists() {
                assert!(!std::fs::read_to_string(&journal).unwrap().contains("resources/read"));
            } else {
                assert_eq!(mode, "disabled");
            }
        }
        pool.shutdown();
    }
}
