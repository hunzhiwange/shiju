//! 验证全局配置客户端的读取、局部更新与嵌套路径映射。
//!
//! 重点约束租户信息只通过请求头传递，并覆盖缺失值、编解码失败和网关错误。

use serde::Deserialize;
use serde_json::json;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};

use crate::client::test_support;

#[derive(Debug, Deserialize)]
struct AcpConfig {
    agent: String,
}

#[tokio::test]
async fn tenant_global_config_api_uses_endpoint_scope_headers_without_tenant_path() {
    let tenant = TenantScope::new("platform / east", "company#one").expect("tenant");
    let server = test_support::server_with_tenant_scope(
        vec![(200, json!({"tenant": true})), (200, json!({}))],
        tenant.clone(),
    );

    assert!(
        server.client().tenant_global_config_get(&tenant).await.expect("tenant config")["tenant"]
            .as_bool()
            .unwrap()
    );
    server
        .client()
        .tenant_global_config_patch(&tenant, &json!({"default_provider": "tenant-provider"}))
        .await
        .expect("tenant patch");

    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/global/config");
    assert_eq!(request.header(PLATFORM_ID_HEADER), Some("platform / east"));
    assert_eq!(request.header(COMPANY_ID_HEADER), Some("company#one"));
    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.path, "/v1/global/config");
    assert_eq!(request.body, json!({"default_provider": "tenant-provider"}));
    server.join();
}

#[tokio::test]
async fn tenant_global_config_api_rejects_scope_that_conflicts_with_gateway_endpoint() {
    let server = test_support::server_with_tenant_scope(
        Vec::new(),
        TenantScope::new("platform-a", "company-a").expect("endpoint tenant"),
    );
    let conflicting = TenantScope::new("platform-b", "company-b").expect("request tenant");

    let error = server
        .client()
        .tenant_global_config_get(&conflicting)
        .await
        .expect_err("conflicting tenant scope should be rejected before sending");

    assert!(error.contains("does not match gateway endpoint tenant scope"));
    server.join();
}

#[tokio::test]
async fn config_api_reads_patches_and_maps_nested_global_paths() {
    let server = test_support::server(vec![
        (200, json!({"model": "gpt"})),
        (200, json!({"model": "o3"})),
        (200, json!({"global": true})),
        (200, json!({})),
        (200, json!({"agent": "codex"})),
        (200, json!({"app_ui": {"system_settings": {"theme": "dark"}}})),
        (200, json!({})),
        (200, json!({"app_ui": {}})),
    ]);

    assert_eq!(
        server.client().config_get(Some("/tmp/project")).await.expect("config")["model"],
        "gpt"
    );
    assert_eq!(
        server
            .client()
            .config_patch(Some("/tmp/project"), &json!({"model": "o3"}))
            .await
            .expect("patch")["model"],
        "o3"
    );
    assert!(
        server.client().global_config_get().await.expect("global")["global"].as_bool().unwrap()
    );
    server.client().global_config_patch(&json!({"x": 1})).await.expect("global patch");
    let acp: AcpConfig = server.client().global_acp_config_get().await.expect("acp");
    assert_eq!(acp.agent, "codex");
    assert_eq!(
        server
            .client()
            .global_config_get_path(&["app_ui", "system_settings", "theme"])
            .await
            .expect("path"),
        Some(json!("dark"))
    );
    server
        .client()
        .global_config_patch_path(&["app_ui", "system_settings"], json!({"theme": "light"}))
        .await
        .expect("patch path");
    assert_eq!(
        server
            .client()
            .global_config_get_path(&["app_ui", "system_settings"])
            .await
            .expect("missing path"),
        None
    );

    assert!(server.take_request().path.contains("/v1/config?directory=%2Ftmp%2Fproject"));
    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert!(request.path.contains("/v1/config?directory=%2Ftmp%2Fproject"));
    assert_eq!(request.body["model"], "o3");
    assert_eq!(server.take_request().path, "/v1/global/config");
    assert_eq!(server.take_request().path, "/v1/global/config");
    assert_eq!(server.take_request().path, "/v1/global/config/acp");
    assert_eq!(server.take_request().path, "/v1/global/config");
    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.body, json!({"app_ui": {"system_settings": {"theme": "light"}}}));
    assert_eq!(server.take_request().path, "/v1/global/config");
    server.join();
}

#[tokio::test]
async fn config_patch_without_directory_uses_unscoped_endpoint() {
    let server = test_support::server(vec![(200, json!({"model": "o4-mini"}))]);

    let response =
        server.client().config_patch(None, &json!({"model": "o4-mini"})).await.expect("patch");

    assert_eq!(response["model"], "o4-mini");
    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.path, "/v1/config");
    assert_eq!(request.body, json!({"model": "o4-mini"}));
    server.join();
}

#[tokio::test]
async fn global_config_patch_path_wraps_values_and_allows_empty_path() {
    let server = test_support::server(vec![(200, json!({})), (200, json!({}))]);

    server
        .client()
        .global_config_patch_path(&["app_ui", "system_settings", "theme"], json!("dark"))
        .await
        .expect("nested patch");
    server
        .client()
        .global_config_patch_path(&[], json!({"raw": true}))
        .await
        .expect("empty path patch");

    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.path, "/v1/global/config");
    assert_eq!(request.body, json!({"app_ui": {"system_settings": {"theme": "dark"}}}));

    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.path, "/v1/global/config");
    assert_eq!(request.body, json!({"raw": true}));
    server.join();
}

#[tokio::test]
async fn global_config_get_path_reports_missing_and_propagates_read_errors() {
    let server =
        test_support::server(vec![(200, json!({"app_ui": {}})), (500, json!({"error": "nope"}))]);

    assert_eq!(
        server
            .client()
            .global_config_get_path(&["app_ui", "system_settings"])
            .await
            .expect("missing path"),
        None
    );
    let error = server.client().global_config_get_path(&["app_ui"]).await.expect_err("read error");

    assert!(error.contains("500"));
    assert!(error.contains("nope"));
    assert_eq!(server.take_request().path, "/v1/global/config");
    assert_eq!(server.take_request().path, "/v1/global/config");
    server.join();
}

#[tokio::test]
async fn global_acp_config_get_reports_deserialize_errors() {
    let server = test_support::server(vec![(200, json!({"agent": 123}))]);

    let error = server.client().global_acp_config_get::<AcpConfig>().await.expect_err("decode");

    assert!(error.contains("invalid type"));
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/global/config/acp");
    server.join();
}

#[tokio::test]
async fn global_config_patch_propagates_gateway_errors() {
    let server = test_support::server(vec![(500, json!({"error": "write failed"}))]);

    let error = server.client().global_config_patch(&json!({"x": 1})).await.expect_err("patch");

    assert!(error.contains("500"));
    assert!(error.contains("write failed"));
    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.path, "/v1/global/config");
    assert_eq!(request.body, json!({"x": 1}));
    server.join();
}
