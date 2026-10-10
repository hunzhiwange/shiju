//! 验证通用 JSON 请求辅助函数的状态码、解码与重试语义。
//!
//! 覆盖各 HTTP 动词、租户请求头、404 可选结果，以及可重试和不可重试错误的边界。

use serde_json::json;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};

use crate::client::test_support;

fn assert_tenant_headers(
    request: &test_support::RecordedRequest,
    platform_id: &str,
    company_id: &str,
) {
    assert_eq!(request.header(PLATFORM_ID_HEADER), Some(platform_id));
    assert_eq!(request.header(COMPANY_ID_HEADER), Some(company_id));
}

#[tokio::test]
async fn request_helpers_cover_http_verbs_success_404_decode_and_error_paths() {
    let tenant_scope = TenantScope::new("platform-helpers", "company-helpers").expect("tenant");
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, json!({"ok": "get"})),
            (200, json!({"ok": "post"})),
            (200, json!({"ok": "patch"})),
            (200, json!({"ok": "put"})),
            (204, json!(null)),
            (200, json!({"ok": "delete_json"})),
            (200, json!({"ok": "optional"})),
            (404, json!({"message": "missing"})),
            (500, json!({"message": "boom"})),
            (200, json!("not a number")),
        ],
        tenant_scope,
    );

    let get: serde_json::Value = server
        .client()
        .get_json("/helper/get", &[("q".to_string(), "one".to_string())])
        .await
        .expect("get");
    assert_eq!(get["ok"], "get");
    let post: serde_json::Value =
        server.client().post_json("/helper/post", &[], &json!({"a": 1})).await.expect("post");
    assert_eq!(post["ok"], "post");
    let patch: serde_json::Value =
        server.client().patch_json("/helper/patch", &[], &json!({"b": 2})).await.expect("patch");
    assert_eq!(patch["ok"], "patch");
    let put: serde_json::Value =
        server.client().put_json("/helper/put", &[], &json!({"c": 3})).await.expect("put");
    assert_eq!(put["ok"], "put");
    server.client().delete_empty("/helper/delete-empty", &[]).await.expect("delete empty");
    let deleted: serde_json::Value = server
        .client()
        .delete_json("/helper/delete-json", &[], &json!({"d": 4}))
        .await
        .expect("delete json");
    assert_eq!(deleted["ok"], "delete_json");
    let optional: Option<serde_json::Value> =
        server.client().get_json_with_404("/helper/optional", &[]).await.expect("optional");
    assert_eq!(optional.expect("some")["ok"], "optional");
    let missing: Option<serde_json::Value> =
        server.client().get_json_with_404("/helper/missing", &[]).await.expect("missing");
    assert!(missing.is_none());
    let status_error = server
        .client()
        .get_json::<serde_json::Value>("/helper/error", &[])
        .await
        .expect_err("status error");
    assert!(status_error.contains("gateway request failed: 500"));
    let decode_error = server
        .client()
        .get_json::<u64>("/helper/decode-error", &[])
        .await
        .expect_err("decode error");
    assert!(!decode_error.is_empty());

    let requests = (0..10).map(|_| server.take_request()).collect::<Vec<_>>();
    for request in &requests {
        assert_tenant_headers(request, "platform-helpers", "company-helpers");
    }
    assert_eq!(requests[0].path, "/helper/get?q=one");
    assert_eq!(requests[1].method, "POST");
    assert_eq!(requests[1].path, "/helper/post");
    assert_eq!(requests[1].body, json!({"a": 1}));
    assert_eq!(requests[2].method, "PATCH");
    assert_eq!(requests[2].body, json!({"b": 2}));
    assert_eq!(requests[3].method, "PUT");
    assert_eq!(requests[3].body, json!({"c": 3}));
    assert_eq!(requests[4].method, "DELETE");
    assert_eq!(requests[5].method, "DELETE");
    assert_eq!(requests[5].body, json!({"d": 4}));
    assert_eq!(requests[6].path, "/helper/optional");
    assert_eq!(requests[7].path, "/helper/missing");
    assert_eq!(requests[8].path, "/helper/error");
    assert_eq!(requests[9].path, "/helper/decode-error");
    server.join();
}





#[tokio::test]
async fn delete_empty_returns_status_and_response_body_on_failure() {
    let server = test_support::server(vec![(405, json!({"message": "delete denied"}))]);

    let error = server
        .client()
        .delete_empty("/helper/delete-denied", &[("force".to_string(), "false".to_string())])
        .await
        .expect_err("delete status error");

    assert!(error.contains("gateway request failed: 405"));
    assert!(error.contains("delete denied"));
    let request = server.take_request();
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, "/helper/delete-denied?force=false");
    server.join();
}

#[tokio::test]
async fn get_json_with_404_maps_decode_and_non_not_found_errors() {
    let server = test_support::server_raw(vec![
        (200, "not-json".to_string()),
        (500, json!({"message": "gateway failed"}).to_string()),
    ]);

    let decode_error = server
        .client()
        .get_json_with_404::<serde_json::Value>("/helper/optional-decode", &[])
        .await
        .expect_err("optional response decode error");
    assert!(!decode_error.is_empty());

    let status_error = server
        .client()
        .get_json_with_404::<serde_json::Value>("/helper/optional-error", &[])
        .await
        .expect_err("non-404 status error");
    assert!(status_error.contains("gateway request failed: 500"));
    assert!(status_error.contains("gateway failed"));

    let requests = (0..2).map(|_| server.take_request()).collect::<Vec<_>>();
    assert_eq!(requests[0].path, "/helper/optional-decode");
    assert_eq!(requests[1].path, "/helper/optional-error");
    server.join();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn get_json_blocking_maps_success_status_error_and_decode_error() {
    let tenant_scope = TenantScope::new("platform-blocking", "company-blocking").expect("tenant");
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, json!({"ok": true})),
            (500, json!({"message": "boom"})),
            (200, json!("not a bool")),
        ],
        tenant_scope,
    );

    let ok: serde_json::Value = super::get_json_blocking(
        server.client().endpoint(),
        "/blocking/ok",
        &[("mode".to_string(), "sync".to_string())],
    )
    .expect("blocking ok");
    assert_eq!(ok["ok"], true);
    let status_error = super::get_json_blocking::<serde_json::Value>(
        server.client().endpoint(),
        "/blocking/error",
        &[],
    )
    .expect_err("blocking status error");
    assert!(status_error.contains("gateway request failed: 500"));
    let decode_error =
        super::get_json_blocking::<bool>(server.client().endpoint(), "/blocking/decode", &[])
            .expect_err("blocking decode error");
    assert!(!decode_error.is_empty());

    let requests = (0..3).map(|_| server.take_request()).collect::<Vec<_>>();
    for request in &requests {
        assert_tenant_headers(request, "platform-blocking", "company-blocking");
    }
    assert_eq!(requests[0].path, "/blocking/ok?mode=sync");
    assert_eq!(requests[1].path, "/blocking/error");
    assert_eq!(requests[2].path, "/blocking/decode");
    server.join();
}
