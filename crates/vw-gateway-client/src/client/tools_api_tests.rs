//! 验证工具目录响应只向调用方暴露稳定的工具标识列表。
//!
//! 展示名称、描述和安全元数据由网关返回，但不属于当前精简客户端方法的结果契约。

use serde_json::json;

use crate::client::test_support;

#[tokio::test]
async fn tools_list_maps_tool_specs_to_ids() {
    let server = test_support::server(vec![(
        200,
        json!({
            "items": [
                {
                    "id": "file_read",
                    "display_name": "Read",
                    "description": "Read a file",
                    "input_schema": {"type": "object"},
                    "read_only": true
                },
                {
                    "id": "shell",
                    "display_name": "Shell",
                    "description": "Run command",
                    "input_schema": {"type": "object"},
                    "destructive": true
                }
            ]
        }),
    )]);

    let tools = server.client().tools_list().await.expect("tools");

    assert_eq!(tools, vec!["file_read".to_string(), "shell".to_string()]);
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/tools");
    server.join();
}
