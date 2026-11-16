use super::mcp_apps::association;
use serde_json::json;

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
    let pool = super::mcp_connections::McpSessionConnections::new(crate::plugins::compat::mcp_connections::host());
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

#[tokio::test]
#[ignore = "requires public Chess MCP network service"]
async fn codex_plugin_compat_t29_public_chess_resource() {
    use agent_client_protocol::{McpServer, McpServerHttp};
    let server = McpServer::Http(McpServerHttp::new("chess", "https://mcp.chess.niklas.sh/mcp"));
    let pool = super::mcp_connections::McpSessionConnections::new(crate::plugins::compat::mcp_connections::host());
    let result = pool
        .with_client(&server, Some(std::time::Duration::from_secs(45)), |client| {
            Box::pin(async move {
                let tools = client.service().catalogs.tools(client.peer()).await?;
                assert!(tools.iter().any(|t| t.name == "get-daily-puzzle"));
                let metadata =
                    client.service().raw_responses.tool_metadata("get-daily-puzzle").unwrap();
                let mut app = association("chess", "get-daily-puzzle", Some(&metadata))?.unwrap();
                // The same tool also declares a private template; select the public ext-apps URI.
                assert!(app["resource_uri"].as_str().unwrap().contains("/ext-apps/"));
                let mut result = client
                    .peer()
                    .call_tool(rmcp::model::CallToolRequestParam {
                        name: "get-daily-puzzle".into(),
                        arguments: Some(Default::default()),
                    })
                    .await?;
                super::mcp_apps::resource(client, &mut app).await?;
                assert!(!app["resource"]["contents"][0]["text"].as_str().unwrap().is_empty());
                result
                    .meta
                    .get_or_insert_with(Default::default)
                    .0
                    .insert(super::mcp_apps::BINDING.to_owned(), app);
                super::mcp_content::normalize_mcp_tool_result("chess", "get-daily-puzzle", result)
            })
        })
        .await
        .unwrap();
    assert_eq!(result.data["is_error"], false);
    assert!(result.data["structured_content"].is_object());
    assert!(!result.data["content"].as_array().unwrap().is_empty());
    assert_eq!(
        result.data["mcp_app"]["resource"]["contents"][0]["mimeType"],
        "text/html;profile=mcp-app"
    );
    assert_eq!(result.data["mcp_app"]["renderable"], false);
    pool.shutdown();
}
