//! Provider 认证、hook 信任和实例释放的线协议测试。

use serde_json::json;

use crate::client::test_support;

#[tokio::test]
async fn control_api_uses_provider_hook_and_instance_routes() {
    let server = test_support::server(vec![
        (200, json!({"openai": [{"type": "oauth", "label": "Browser"}]})),
        (
            200,
            json!({
                "url": "https://example.test/oauth",
                "method": "browser",
                "instructions": "open the URL"
            }),
        ),
        (200, json!(true)),
        (200, json!(true)),
        (200, json!(true)),
        (200, json!([])),
        (200, json!({"plugin_id": "plugin-a", "hook_hash": "hash-a", "trusted": true})),
        (200, json!({"plugin_id": "plugin-a", "hook_hash": "hash-a", "trusted": false})),
        (200, json!([])),
        (200, json!(true)),
        (200, json!(true)),
    ]);
    let provider_id = "openai /合作";
    let encoded_provider_id = "openai%20%2F%E5%90%88%E4%BD%9C";

    assert!(
        server.client().provider_oauth_callback(provider_id, 1, "  ").await.is_err(),
        "blank callback code must fail before sending"
    );
    assert!(
        server.client().auth_set("  ", &json!({"type": "api", "key": "secret"})).await.is_err(),
        "blank provider id must fail before sending"
    );

    let methods = server.client().provider_auth_methods().await.expect("auth methods");
    assert_eq!(methods["openai"][0].label(), "Browser");
    let authorization = server
        .client()
        .provider_oauth_authorize(provider_id, 1)
        .await
        .expect("OAuth authorize")
        .expect("authorization payload");
    assert_eq!(authorization.method, "browser");
    assert!(
        server
            .client()
            .provider_oauth_callback(provider_id, 1, "callback-code")
            .await
            .expect("OAuth callback")
    );
    assert!(
        server
            .client()
            .auth_set(provider_id, &json!({"type": "api", "key": "secret-key"}))
            .await
            .expect("auth set")
    );
    assert!(server.client().auth_remove(provider_id).await.expect("auth remove"));
    assert!(server.client().hooks_get().await.expect("hooks").is_empty());
    assert!(server.client().hook_trust("plugin-a", "hash-a").await.expect("trust").trusted);
    assert!(!server.client().hook_revoke("plugin-a", "hash-a").await.expect("revoke").trusted);
    assert!(server.client().hook_audit_get().await.expect("audit").is_empty());
    assert!(server.client().global_dispose().await.expect("global dispose"));
    assert!(
        server
            .client()
            .instance_dispose(Some("/tmp/instance one"))
            .await
            .expect("instance dispose")
    );

    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/provider/auth");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("/v1/provider/{encoded_provider_id}/oauth/authorize"));
    assert_eq!(request.body, json!({"method": 1}));
    let request = server.take_request();
    assert_eq!(request.path, format!("/v1/provider/{encoded_provider_id}/oauth/callback"));
    assert_eq!(request.body, json!({"method": 1, "code": "callback-code"}));
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, format!("/v1/auth/{encoded_provider_id}"));
    assert_eq!(request.body, json!({"type": "api", "key": "secret-key"}));
    let request = server.take_request();
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, format!("/v1/auth/{encoded_provider_id}"));
    assert_eq!(request.body, serde_json::Value::Null);
    for (method, path) in [
        ("GET", "/v1/hooks"),
        ("POST", "/v1/hooks/trust"),
        ("POST", "/v1/hooks/revoke"),
        ("GET", "/v1/hooks/audit"),
        ("POST", "/v1/global/dispose"),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, path);
    }
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/instance/dispose?directory=%2Ftmp%2Finstance+one");
    server.join();
}
