//! 验证识矩应用目录、安装状态、API 密钥绑定与启动接口。
//!
//! 固件同时约束当前字段形状、固定网关路径，以及敏感值不得出现在调试输出中。

use serde_json::json;

use super::{
    SHIJU_CREDENTIAL_HEADER, ShijuAppApiKeyBindingDto, ShijuAppApiKeyBindingSaveRequest,
    ShijuAppCatalogDto, ShijuAppLaunchDto, ShijuAppListDto, ShijuCredential, ShijuIamApiKeyListDto,
};
use crate::client::test_support;

fn credential() -> ShijuCredential {
    ShijuCredential::new("fixture-private-app-secret").expect("credential")
}

#[test]
fn shiju_app_binding_accepts_upstream_string_key_id() {
    let fixture = json!({
        "$schema": "https://gateway.shiju.cloud/schemas/BindingRecord.json",
        "personal": {
            "apiKeyId": "101001",
            "keyName": "fixture key",
            "apiKeyMask": "sk_****8395",
            "status": "enabled",
            "expiresAt": "2041-10-03 08:00:00",
            "configured": true,
            "canEdit": true,
            "updatedAt": "2026-08-17T10:46:00.20508+08:00"
        },
        "unattended": {
            "apiKeyId": "",
            "configured": false,
            "canEdit": true
        }
    });
    let binding: ShijuAppApiKeyBindingDto =
        serde_json::from_value(fixture).expect("upstream binding");
    assert_eq!(binding.personal.api_key_id, Some(101001));
    assert!(binding.personal.configured);
}

#[test]
fn shiju_app_binding_accepts_empty_and_numeric_key_ids() {
    for (raw, expected) in [(json!(""), None), (json!(null), None), (json!(42), Some(42))] {
        let binding: ShijuAppApiKeyBindingDto = serde_json::from_value(json!({
            "personal": {
                "configured": false,
                "canEdit": true,
                "apiKeyId": raw,
                "keyName": "",
                "apiKeyMask": "",
                "status": "",
                "expiresAt": "",
                "updatedAt": ""
            }
        }))
        .expect("binding key ID");
        assert_eq!(binding.personal.api_key_id, expected);
    }
}

#[test]
fn shiju_app_contract_preserves_catalog_and_installed_shapes() {
    let catalog_fixture = json!({
        "apps": [{
            "appId": "knowledge-assistant",
            "name": "知识助手",
            "description": "企业知识检索",
            "iconUrl": null,
            "version": "1.2.3",
            "installed": true,
            "installationId": "01K1APP0000000000000000001",
            "userId": "101027",
            "installedAt": "2026-07-30T09:30:00+08:00",
            "permissions": ["knowledge:read", "chat:write"]
        }, {
            "appId": "meeting-notes",
            "name": "会议纪要",
            "description": null,
            "iconUrl": null,
            "version": "0.9.0",
            "installed": false,
            "installationId": null,
            "userId": null,
            "installedAt": null,
            "permissions": []
        }]
    });
    let installed_fixture = json!({
        "apps": [{
            "installationId": "01K1APP0000000000000000001",
            "appId": "knowledge-assistant",
            "name": "知识助手",
            "description": "企业知识检索",
            "iconUrl": null,
            "version": "1.2.3",
            "source": "catalog",
            "reconcileStatus": "ready",
            "runtimeStatus": "ready",
            "runtimeUrl":
                "https://app.shiju.cloud/app/01K1APP0000000000000000001/?session=private",
            "createdAt": "2026-07-30T09:30:00+08:00",
            "updatedAt": "2026-07-30T09:31:00+08:00"
        }]
    });

    let catalog: ShijuAppCatalogDto =
        serde_json::from_value(catalog_fixture.clone()).expect("catalog contract");
    let installed: ShijuAppListDto =
        serde_json::from_value(installed_fixture.clone()).expect("installed contract");

    assert_eq!(catalog.apps[0].permissions, ["knowledge:read", "chat:write"]);
    assert_eq!(catalog.apps[0].installation_id.as_deref(), Some("01K1APP0000000000000000001"));
    assert!(catalog.apps[1].installation_id.is_none());
    assert!(catalog.apps[1].user_id.is_none());
    assert!(catalog.apps[1].installed_at.is_none());
    assert!(catalog.apps[1].icon_url.is_none());
    assert_eq!(installed.apps[0].reconcile_status, "ready");
    assert_eq!(installed.apps[0].runtime_status, "ready");
    assert!(installed.apps[0].icon_url.is_none());
    let installed_debug = format!("{installed:?}");
    assert!(!installed_debug.contains("session=private"));
    assert!(installed_debug.contains("<redacted>"));
    assert_eq!(serde_json::to_value(catalog).unwrap(), catalog_fixture);
    assert_eq!(serde_json::to_value(installed).unwrap(), installed_fixture);

    let not_installed: ShijuAppCatalogDto = serde_json::from_value(json!({
        "apps": [{
            "appId": "not-installed",
            "name": "未安装应用",
            "description": null,
            "version": "1.0.0",
            "installed": false
        }]
    }))
    .expect("optional catalog fields");
    assert!(not_installed.apps[0].installation_id.is_none());
    assert!(not_installed.apps[0].user_id.is_none());
    assert!(not_installed.apps[0].installed_at.is_none());
    assert!(not_installed.apps[0].icon_url.is_none());
    assert!(not_installed.apps[0].permissions.is_empty());
}

#[test]
fn shiju_app_api_key_contract_preserves_safe_shapes() {
    let configured_fixture = json!({
        "personal": {
            "configured": true,
            "canEdit": true,
            "apiKeyId": 42,
            "keyName": "Desktop automation",
            "apiKeyMask": "sk-****cafe",
            "status": "enabled",
            "expiresAt": null,
            "updatedAt": "2026-08-22T09:30:00+08:00"
        }
    });
    let unconfigured_fixture = json!({
        "personal": {
            "configured": false,
            "canEdit": true,
            "apiKeyId": null,
            "keyName": null,
            "apiKeyMask": null,
            "status": null,
            "expiresAt": null,
            "updatedAt": null
        }
    });
    let keys_fixture = json!({
        "apiKeys": [{
            "apiKeyId": 42,
            "keyName": "Desktop automation",
            "apiKeyMask": "sk-****cafe",
            "status": "enabled",
            "expiresAt": null,
            "updatedAt": "2026-08-21T09:30:00+08:00"
        }, {
            "apiKeyId": 43,
            "keyName": "Expired integration",
            "apiKeyMask": "sk-****dead",
            "status": "disabled",
            "expiresAt": "2026-08-20T00:00:00+08:00",
            "updatedAt": "2026-08-20T00:00:00+08:00"
        }]
    });

    let configured: ShijuAppApiKeyBindingDto =
        serde_json::from_value(configured_fixture.clone()).expect("configured binding");
    let unconfigured: ShijuAppApiKeyBindingDto =
        serde_json::from_value(unconfigured_fixture.clone()).expect("unconfigured binding");
    let keys: ShijuIamApiKeyListDto =
        serde_json::from_value(keys_fixture.clone()).expect("IAM API key list");

    assert!(configured.personal.configured);
    assert!(configured.personal.can_edit);
    assert_eq!(configured.personal.api_key_id, Some(42));
    assert_eq!(configured.personal.api_key_mask.as_deref(), Some("sk-****cafe"));
    assert!(!unconfigured.personal.configured);
    assert!(unconfigured.personal.api_key_id.is_none());
    assert_eq!(keys.api_keys.len(), 2);
    assert_eq!(keys.api_keys[0].status, "enabled");
    assert_eq!(keys.api_keys[1].expires_at.as_deref(), Some("2026-08-20T00:00:00+08:00"));
    assert_eq!(serde_json::to_value(configured).unwrap(), configured_fixture);
    assert_eq!(serde_json::to_value(unconfigured).unwrap(), unconfigured_fixture);
    assert_eq!(serde_json::to_value(keys).unwrap(), keys_fixture);

    let save = ShijuAppApiKeyBindingSaveRequest { api_key_id: 42 };
    assert_eq!(serde_json::to_value(save).unwrap(), json!({"apiKeyId": 42}));
    assert!(
        serde_json::from_value::<ShijuAppApiKeyBindingSaveRequest>(json!({
            "apiKeyId": 42,
            "keyName": "must not be accepted"
        }))
        .is_err()
    );
}

#[tokio::test]
async fn shiju_app_client_uses_binding_key_list_and_save_routes() {
    let binding_fixture = json!({
        "personal": {
            "configured": false,
            "canEdit": true,
            "apiKeyId": null,
            "keyName": null,
            "apiKeyMask": null,
            "status": null,
            "expiresAt": null,
            "updatedAt": null
        }
    });
    let keys_fixture = json!({"apiKeys": []});
    let server =
        test_support::server(vec![(200, binding_fixture), (200, keys_fixture), (204, json!(null))]);
    let credential = credential();

    server
        .client()
        .shiju_app_api_key_binding(&credential, "01K1APP0000000000000000001")
        .await
        .unwrap();
    server.client().shiju_app_api_keys(&credential).await.unwrap();
    server
        .client()
        .shiju_app_save_api_key_binding(
            &credential,
            "01K1APP0000000000000000001",
            &ShijuAppApiKeyBindingSaveRequest { api_key_id: 42 },
        )
        .await
        .unwrap();

    let binding = server.take_request();
    assert_eq!(binding.method, "GET");
    assert_eq!(binding.path, "/v1/shiju/apps/01K1APP0000000000000000001/api-key-binding");
    assert_eq!(binding.body, serde_json::Value::Null);

    let keys = server.take_request();
    assert_eq!(keys.method, "GET");
    assert_eq!(keys.path, "/v1/shiju/apps/api-keys");
    assert_eq!(keys.body, serde_json::Value::Null);

    let save = server.take_request();
    assert_eq!(save.method, "PUT");
    assert_eq!(save.path, "/v1/shiju/apps/01K1APP0000000000000000001/api-key-binding");
    assert_eq!(save.body, json!({"apiKeyId": 42}));

    for request in [binding, keys, save] {
        assert_eq!(request.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
        assert!(!request.path.contains(credential.expose()));
        assert!(!request.body.to_string().contains(credential.expose()));
    }
    server.join();
}

#[tokio::test]
async fn shiju_app_client_uses_fixed_gateway_paths() {
    let server = test_support::server(vec![
        (200, json!({"apps": []})),
        (200, json!({"apps": []})),
        (
            200,
            json!({
                "installationId": "01K1APP0000000000000000001",
                "name": "知识助手",
                "runtimeUrl": "https://app.shiju.cloud/app/01K1APP0000000000000000001/",
                "runtimeSetCookie": "app_runtime_session=fixture; Path=/; HttpOnly; Secure; SameSite=None"
            }),
        ),
    ]);
    let credential = credential();

    server.client().shiju_app_catalog(&credential).await.unwrap();
    server.client().shiju_apps(&credential).await.unwrap();
    server
        .client()
        .shiju_app_prepare_launch(&credential, "01K1APP0000000000000000001")
        .await
        .unwrap();

    let catalog = server.take_request();
    assert_eq!(catalog.method, "GET");
    assert_eq!(catalog.path, "/v1/shiju/apps/catalog");
    assert_eq!(catalog.body, serde_json::Value::Null);

    let installed = server.take_request();
    assert_eq!(installed.method, "GET");
    assert_eq!(installed.path, "/v1/shiju/apps");
    assert_eq!(installed.body, serde_json::Value::Null);

    let launch = server.take_request();
    assert_eq!(launch.method, "POST");
    assert_eq!(launch.path, "/v1/shiju/apps/01K1APP0000000000000000001/launch");
    assert_eq!(launch.body, serde_json::Value::Null);

    for request in [catalog, installed, launch] {
        assert_eq!(request.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
        assert!(!request.path.contains(credential.expose()));
        assert!(!request.body.to_string().contains(credential.expose()));
    }
    server.join();
}

#[tokio::test]
async fn shiju_app_sensitive_values_remain_redacted() {
    let runtime_url =
        "https://app.shiju.cloud/app/01K1APP0000000000000000001/?binding=runtime-private";
    let launch: ShijuAppLaunchDto = serde_json::from_value(json!({
        "installationId": "01K1APP0000000000000000001",
        "name": "知识助手",
        "runtimeUrl": runtime_url,
        "runtimeSessionToken": "runtime-session-private"
    }))
    .expect("launch contract");
    let launch_debug = format!("{launch:?}");
    assert!(!launch_debug.contains(runtime_url));
    assert!(!launch_debug.contains("runtime-private"));
    assert!(!launch_debug.contains("runtime-session-private"));
    assert!(launch_debug.contains("<redacted>"));

    let credential = credential();
    let server = test_support::server(vec![(
        400,
        json!({
            "message": runtime_url,
            "credential": credential.expose()
        }),
    )]);
    let error = server
        .client()
        .shiju_app_prepare_launch(&credential, "01K1APP0000000000000000001")
        .await
        .expect_err("non-success response");
    assert!(!error.contains(runtime_url));
    assert!(!error.contains("runtime-private"));
    assert!(!error.contains(credential.expose()));

    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
    server.join();
}
