//! 系统状态与发现客户端的线协议测试。

use serde_json::{Value, json};
use vw_api_types::id::TenantScope;

use crate::client::test_support;

#[tokio::test]
async fn system_api_uses_documented_paths() {
    let tenant_scope = TenantScope::new("platform-a", "company-b").expect("tenant scope");
    let server = test_support::server_raw_with_tenant_scope(
        vec![
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, json!({"ok": true}).to_string()),
            (200, "gateway_requests_total 7\n".to_string()),
            (500, json!({"error": "unavailable"}).to_string()),
            (500, "metrics unavailable\n".to_string()),
        ],
        tenant_scope,
    );

    let client = server.client();
    let responses: Vec<Result<Value, String>> = vec![
        client.system_health().await,
        client.system_status().await,
        client.system_tools().await,
        client.system_mcp_inspect().await,
        client.system_api_health().await,
        client.system_global_health().await,
        client.system_models().await,
        client.system_path(Some("/tmp/workspace")).await,
        client.system_vcs(Some("/tmp/workspace")).await,
        client.system_commands(Some("/tmp/workspace")).await,
        client.system_agents(Some("/tmp/workspace")).await,
        client.system_skills(Some("/tmp/workspace")).await,
        client.system_cli_tools().await,
    ];
    let metrics = client.system_metrics().await.expect("metrics response");

    assert!(responses.iter().all(Result::is_ok));
    assert_eq!(metrics, "gateway_requests_total 7\n");

    let expected_paths = [
        "/v1/health",
        "/v1/status",
        "/v1/tools",
        "/v1/mcp/inspect",
        "/v1/api-health",
        "/v1/global/health",
        "/v1/models",
        "/v1/path?directory=%2Ftmp%2Fworkspace",
        "/v1/vcs?directory=%2Ftmp%2Fworkspace",
        "/v1/command?directory=%2Ftmp%2Fworkspace",
        "/v1/agent?directory=%2Ftmp%2Fworkspace",
        "/v1/skill?directory=%2Ftmp%2Fworkspace",
        "/v1/cli-tools",
        "/v1/metrics",
    ];
    for expected_path in expected_paths {
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, expected_path);
        assert_eq!(request.header("x-platform-id"), Some("platform-a"));
        assert_eq!(request.header("x-company-id"), Some("company-b"));
        assert_eq!(request.body, Value::Null);
    }

    let json_error = client.system_status().await.expect_err("JSON status must surface errors");
    assert!(json_error.contains("500"));
    let text_error = client.system_metrics().await.expect_err("metrics must surface errors");
    assert!(text_error.contains("500"));
    server.join();
}
