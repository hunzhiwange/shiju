use serde_json::json;

use super::{
    SHIJU_CREDENTIAL_HEADER, ShijuAppCatalogDto, ShijuAppLaunchDto, ShijuAppListDto,
    ShijuCredential,
};
use crate::client::test_support;

fn credential() -> ShijuCredential {
    ShijuCredential::new("fixture-private-app-secret").expect("credential")
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
