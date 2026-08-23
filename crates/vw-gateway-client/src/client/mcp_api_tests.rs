//! 验证 MCP 检查请求的目录查询参数与 JSON 响应透传。
//!
//! 固件用于锁定客户端调用的网关路径，不模拟 MCP 服务自身的运行行为。

use serde_json::json;

use crate::client::test_support;

#[tokio::test]
async fn mcp_inspect_gets_gateway_inspection_response() {
    let server = test_support::server(vec![(
        200,
        json!({
            "servers": [{
                "name": "docs",
                "transport": "http",
                "enabled": true,
                "required": false,
                "connection_status": "connected",
                "tool_count": 1,
                "resource_count": 1,
                "prompt_count": 1,
                "tools": [{
                    "id": "mcp__docs__search",
                    "display_name": "docs: search",
                    "description": "Search docs",
                    "input_schema": {"type": "object"}
                }],
                "resources": [{"uri": "file:///docs/readme.md", "name": "README"}],
                "prompts": [{"name": "review", "description": "Review docs"}]
            }]
        }),
    )]);

    let response = server.client().mcp_inspect().await.expect("mcp inspect");

    assert_eq!(response.servers[0].name, "docs");
    assert_eq!(response.servers[0].tools[0].id.0, "mcp__docs__search");
    assert_eq!(response.servers[0].resources[0]["uri"], "file:///docs/readme.md");
    assert_eq!(response.servers[0].prompts[0]["name"], "review");
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/mcp/inspect");
    server.join();
}
