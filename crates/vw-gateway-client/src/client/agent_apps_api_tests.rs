use serde_json::json;
use vw_api_types::{agent_apps::CustomIconEntry, id::TenantScope};

use super::{GatewayClient, test_support};
use crate::endpoint::GatewayAuth;

#[tokio::test]
async fn cc_base_icon_sdk_read_preserves_headers_and_chinese_icons() {
    let icon = CustomIconEntry {
        id: "icon-1".into(),
        name: "工作图标".into(),
        data: "data:image/png;base64,aGVsbG8=".into(),
        created_at: 1730000000123,
    };
    let server = test_support::server_with_tenant_scope(
        vec![(200, json!([icon])), (200, json!([]))],
        TenantScope::new("platform-a", "company-b").unwrap(),
    );
    let endpoint = server
        .client()
        .endpoint()
        .clone()
        .with_auth(GatewayAuth { skey: Some("test-icon-key".into()) });
    let client = GatewayClient::new(endpoint).unwrap();
    let icons: Vec<CustomIconEntry> = client.custom_icons_list().await.unwrap();
    assert_eq!(icons, vec![icon]);
    // Re-read after a disconnected HTTP connection observes the current list.
    assert!(client.custom_icons_list().await.unwrap().is_empty());
    for _ in 0..2 {
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/agent-apps/custom-icons");
        assert_eq!(request.header("authorization"), Some("Bearer test-icon-key"));
        assert_eq!(request.header("x-platform-id"), Some("platform-a"));
        assert_eq!(request.header("x-company-id"), Some("company-b"));
        assert_eq!(request.body, serde_json::Value::Null);
    }
    server.join();
}

#[tokio::test]
async fn cc_base_icon_sdk_read_surfaces_business_errors() {
    let server = test_support::server(vec![(
        503,
        json!({
            "code": "storage_unavailable", "message": "图标存储暂不可用"
        }),
    )]);
    let error = server.client().custom_icons_list().await.unwrap_err();
    assert!(error.contains("503"), "{error}");
    assert!(error.contains("storage_unavailable"), "{error}");
    assert!(error.contains("图标存储暂不可用"), "{error}");
    server.join();
}

#[tokio::test]
async fn cc_base_icon_sdk_read_rejects_invalid_success_payload() {
    let server = test_support::server(vec![(200, json!([{"id": "incomplete"}]))]);
    assert!(server.client().custom_icons_list().await.is_err());
    server.join();
}

#[tokio::test]
async fn cc_base_icon_sdk_write_add_preserves_body_and_existed() {
    use vw_api_types::agent_apps::{AddCustomIconRequest, AddCustomIconResult};
    let icon = json!({"id":"icon-1","name":"工作","data":"data:image/png;base64,aGVsbG8=","createdAt":123});
    let server = test_support::server(vec![
        (201, json!({"icon":icon,"existed":false})),
        (200, json!({"icon":icon,"existed":true})),
    ]);
    let input = AddCustomIconRequest {
        name: "工作".into(),
        data: "data:image/png;base64,aGVsbG8=".into(),
    };
    for existed in [false, true] {
        let result: AddCustomIconResult = server.client().custom_icon_add(&input).await.unwrap();
        assert_eq!(result.existed, existed);
        assert_eq!(serde_json::to_value(result.icon).unwrap(), icon);
        let request = server.take_request();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v1/agent-apps/custom-icons");
        assert_eq!(request.body, serde_json::to_value(&input).unwrap());
    }
    server.join();
}

#[tokio::test]
async fn cc_base_icon_sdk_write_rename_encodes_id_and_body() {
    use vw_api_types::agent_apps::RenameCustomIconRequest;
    let icon = json!({"id":"图 /?#%","name":"新名称","data":"data:image/png;base64,aGVsbG8=","createdAt":123});
    let server = test_support::server(vec![(200, icon.clone())]);
    let input = RenameCustomIconRequest { name: "新名称".into() };
    let result = server.client().custom_icon_rename("图 /?#%", &input).await.unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), icon);
    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.path, "/v1/agent-apps/custom-icons/%E5%9B%BE%20%2F%3F%23%25");
    assert_eq!(request.body, json!({"name":"新名称"}));
    server.join();
}

#[tokio::test]
async fn cc_base_icon_sdk_write_delete_accepts_empty_204_and_encodes_id() {
    let server = test_support::server_raw(vec![(204, String::new())]);
    server.client().custom_icon_delete("图 /?#%").await.unwrap();
    let request = server.take_request();
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, "/v1/agent-apps/custom-icons/%E5%9B%BE%20%2F%3F%23%25");
    assert_eq!(request.body, serde_json::Value::Null);
    server.join();
}

#[tokio::test]
async fn cc_base_icon_sdk_write_preserves_errors_without_retry() {
    use vw_api_types::agent_apps::{AddCustomIconRequest, RenameCustomIconRequest};
    for (status, code) in [(400, "invalid_input"), (404, "not_found"), (503, "storage_unavailable")]
    {
        for method in ["POST", "PATCH", "DELETE"] {
            let server =
                test_support::server(vec![(status, json!({"code":code,"message":"操作失败"}))]);
            let error = match method {
                "POST" => server
                    .client()
                    .custom_icon_add(&AddCustomIconRequest {
                        name: "工作".into(),
                        data: "invalid".into(),
                    })
                    .await
                    .unwrap_err(),
                "PATCH" => server
                    .client()
                    .custom_icon_rename("missing", &RenameCustomIconRequest { name: "工作".into() })
                    .await
                    .unwrap_err(),
                _ => server.client().custom_icon_delete("missing").await.unwrap_err(),
            };
            assert!(error.contains(&status.to_string()), "{error}");
            assert!(error.contains(code), "{error}");
            assert!(error.contains("操作失败"), "{error}");
            assert_eq!(server.take_request().method, method);
            server.join();
        }
    }
}
