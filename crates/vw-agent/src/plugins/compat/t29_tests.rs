use super::mcp_apps::association;
use serde_json::json;

#[test]
fn codex_plugin_compat_t29_binds_ui_resource_to_tool_result() {
    let meta = json!({"ui":{"resourceUri":"ui://weather/view"}});
    let binding = association("weather", "forecast", Some(&meta)).unwrap().unwrap();
    assert_eq!(binding["resource_uri"], "ui://weather/view");
    assert_eq!(binding["server"], "weather");
    assert_eq!(binding["renderable"], false);
    let mut message = json!({"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"Weather ready"}],"structuredContent":{"temperature":12}}});
    super::mcp_content::preserve_mcp_wire_result(&mut message);
    let mut typed: rmcp::model::CallToolResult =
        serde_json::from_value(message["result"].clone()).unwrap();
    typed.meta.as_mut().unwrap().0.insert(super::mcp_apps::BINDING.to_owned(), binding);
    let result =
        super::mcp_content::normalize_mcp_tool_result("weather", "forecast", typed).unwrap();
    assert_eq!(result.data["mcp_app"]["resource_uri"], "ui://weather/view");
    assert_eq!(result.data["structured_content"], json!({"temperature":12}));
    assert_eq!(result.data["content"][0]["text"], "Weather ready");
    assert!(result.model_result.get("mcp_app").is_none());
}
#[test]
fn codex_plugin_compat_t29_rejects_invalid_input() {
    for uri in [json!("https://evil/view"), json!(12), json!("ui://")] {
        assert!(
            association("weather", "forecast", Some(&json!({"ui":{"resourceUri":uri}}))).is_err()
        );
    }
}
#[test]
fn codex_plugin_compat_t29_preserves_isolation() {
    let private = json!({"openai/outputTemplate":"ui://private/view"});
    assert!(association("private", "forecast", Some(&private)).unwrap().is_none());
    let meta = json!({"ui/resourceUri":"ui://same/view"});
    let a = association("a", "forecast", Some(&meta)).unwrap().unwrap();
    let b = association("b", "forecast", Some(&meta)).unwrap().unwrap();
    assert_ne!(a, b);
    assert!(association("a", "forecast", None).unwrap().is_none());
}

#[test]
fn codex_plugin_compat_t29_preserves_resource_security_metadata() {
    let resource = json!({"contents":[{"uri":"ui://weather/view","mimeType":"text/html;profile=mcp-app","text":"<p>天气</p>","_meta":{"ui":{"csp":{"connectDomains":["https://weather.example"]},"permissions":{"camera":{}}}}}]});
    let before = resource.clone();
    super::mcp_apps::validate_resource("ui://weather/view", &resource).unwrap();
    assert_eq!(resource, before);
    assert!(super::mcp_apps::validate_resource("ui://other/view", &resource).is_err());
    let mut bad = resource;
    bad["contents"][0]["mimeType"] = json!("text/html");
    assert!(super::mcp_apps::validate_resource("ui://weather/view", &bad).is_err());
}

#[test]
fn codex_plugin_compat_t29_wire_metadata_is_connection_local() {
    let a = super::mcp_resources::RawResponse::default();
    let b = super::mcp_resources::RawResponse::default();
    a.capture(&json!({"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"forecast","_meta":{"ui":{"resourceUri":"ui://weather/view"}}}]}}));
    assert!(a.tool_metadata("forecast").is_some());
    assert!(b.tool_metadata("forecast").is_none());
    a.capture(&json!({"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"forecast"}]}}));
    assert_eq!(a.tool_metadata("forecast"), Some(serde_json::Value::Null));
    let info = super::mcp_capabilities::client_info();
    let caps = serde_json::to_value(info.capabilities).unwrap();
    assert!(caps.pointer("/extensions/io.modelcontextprotocol~1ui").is_none());
}

#[tokio::test]
async fn codex_plugin_compat_t29_reads_resource_over_real_stdio() {
    use agent_client_protocol::{McpServer, McpServerStdio};
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("server.py");
    std::fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t29/server.py"))
        .unwrap();
    std::fs::write(
        root.path().join("resource.json"),
        include_bytes!("../../../testdata/plugin_compat/t29/resource.json"),
    )
    .unwrap();
    let server = McpServer::Stdio(
        McpServerStdio::new("weather", "python3").args(vec![script.display().to_string()]),
    );
    let pool = super::mcp_connections::McpSessionConnections::default();
    let result = pool
        .with_client(&server, Some(std::time::Duration::from_secs(5)), |client| {
            Box::pin(async move {
                let tools = client.service().catalogs.tools(client.peer()).await?;
                assert_eq!(tools.len(), 1);
                let meta = client.service().raw_responses.tool_metadata("forecast").unwrap();
                let mut app = association("weather", "forecast", Some(&meta))?.unwrap();
                super::mcp_apps::resource(client, &mut app).await?;
                Ok(app)
            })
        })
        .await
        .unwrap();
    assert_eq!(result["resource"]["contents"][0]["mimeType"], "text/html;profile=mcp-app");
    assert_eq!(
        result["resource"]["contents"][0]["_meta"]["ui"]["permissions"],
        json!({"camera":{}})
    );
    pool.shutdown();
}
