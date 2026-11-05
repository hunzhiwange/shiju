//! 验证工具目录响应只向调用方暴露稳定的工具标识列表。
//!
//! 先按新协议校验完整工具规格，再提取工具 ID；缺失或无效的元数据必须报错。

use serde_json::json;

use crate::client::test_support;

#[tokio::test]
async fn tools_list_maps_tool_specs_to_ids() {
    let server = test_support::server(vec![(
        200,
        json!({
            "items": [
                {
                    "identity": {"namespace": null, "name": "file_read"},
                    "source": "builtin",
                    "exposure": "direct",
                    "id": "file_read",
                    "display_name": "Read",
                    "description": "Read a file",
                    "input_schema": {"type": "object"},
                    "read_only": true
                },
                {
                    "identity": {"namespace": null, "name": "shell"},
                    "source": "builtin",
                    "exposure": "deferred",
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

#[tokio::test]
async fn tools_list_reads_ids_with_new_execution_metadata() {
    let server = test_support::server(vec![(
        200,
        json!({"items": [{
            "id": "file_read",
            "identity": {"namespace": null, "name": "file_read"},
            "source": "builtin",
            "exposure": "deferred",
            "display_name": "Read",
            "description": "Read a file",
            "input_schema": {"type": "object"}
        }]}),
    )]);

    assert_eq!(server.client().tools_list().await.unwrap(), vec!["file_read"]);
    server.take_request();
    server.join();
}

#[tokio::test]
async fn tools_list_rejects_catalog_items_without_stable_ids() {
    let server = test_support::server(vec![(
        200,
        json!({"items": [{
            "identity": {"namespace": null, "name": "file_read"},
            "source": "builtin",
            "exposure": "direct",
            "display_name": "Read",
            "description": "Read a file",
            "input_schema": {"type": "object"}
        }]}),
    )]);

    assert!(server.client().tools_list().await.is_err());
    server.take_request();
    server.join();
}

#[tokio::test]
async fn tools_list_rejects_missing_protocol_metadata() {
    for field in ["identity", "source", "exposure"] {
        let mut item = json!({
            "id": "file_read",
            "identity": {"namespace": null, "name": "file_read"},
            "source": "builtin",
            "exposure": "direct",
            "display_name": "Read",
            "description": "Read a file",
            "input_schema": {"type": "object"}
        });
        item.as_object_mut().unwrap().remove(field);
        let server = test_support::server(vec![(200, json!({"items": [item]}))]);

        assert!(server.client().tools_list().await.is_err(), "missing {field}");
        server.take_request();
        server.join();
    }
}

#[tokio::test]
async fn tools_list_rejects_invalid_protocol_metadata() {
    for (field, value) in [
        ("identity", json!({"namespace": null, "name": ""})),
        ("source", json!("unknown")),
        ("exposure", json!("unknown")),
    ] {
        let mut item = json!({
            "id": "file_read",
            "identity": {"namespace": null, "name": "file_read"},
            "source": "builtin",
            "exposure": "direct",
            "display_name": "Read",
            "description": "Read a file",
            "input_schema": {"type": "object"}
        });
        item[field] = value;
        let server = test_support::server(vec![(200, json!({"items": [item]}))]);

        assert!(server.client().tools_list().await.is_err(), "invalid {field}");
        server.take_request();
        server.join();
    }
}
