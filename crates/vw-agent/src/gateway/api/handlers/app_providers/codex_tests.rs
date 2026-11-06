#![cfg(target_os = "macos")]
use axum::{Router, middleware};
use vw_api_types::{agent_apps::AppId, app_providers::preview::*, id::TenantScope};
#[tokio::test]
async fn cc04_codex_sdk_api_profile_refusal_is_redacted_and_readonly() {
    use vw_gateway_client::{GatewayClient, GatewayEndpoint};
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join(".codex");
    std::fs::create_dir(&directory).unwrap();
    let config = directory.join("config.toml");
    let auth = directory.join("auth.json");
    std::fs::write(&auth, "{}").unwrap();
    let router = Router::new()
        .nest(
            "/v1",
            super::preview::router_with_policy(Some(super::preview_tests::policy(root.path()))),
        )
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("p", "a").unwrap()),
    )
    .unwrap();
    let mut request = ProviderPreviewRequest {
        target_host_id: "host".into(),
        directory_override: None,
        settings: serde_json::json!({"auth":{"OPENAI_API_KEY":"fake-route-key"},"config":"model_provider='vendor'\n[model_providers.vendor]\nbase_url='https://fixture.test/v1'\nwire_api='responses'\n"}),
        category: None,
        provider_id: None,
        meta: None,
        request_options: None,
    };
    const TOKEN: &str = "fixture-config-reader-token-over-32-chars";
    for live in [
        "# user comment\n[projects.fixture]\ntrust_level='trusted'\n",
        "profile='active'\n[profiles.active]\nmodel_provider='other'\n",
        "profile=' active '\n[profiles.active]\nmodel_provider='other'\n",
        "profile='active'\n[profiles.active]\nopenai_base_url='https://override.test'\n",
        "profile='active'\n[profiles.active]\nexperimental_bearer_token='fake-profile-key'\n",
    ] {
        std::fs::write(&config, live).unwrap();
        let response = client.app_provider_preview(AppId::Codex, &request, TOKEN).await.unwrap();
        if live.contains("profile=") {
            assert_eq!(response.unavailable, Some(PreviewUnavailable::Ownership));
            assert!(response.settings.is_null());
            assert!(response.changes.is_empty());
        } else {
            assert!(response.unavailable.is_none());
            assert!(!response.changes.is_empty());
            assert!(
                response.settings["config"]["model_providers"]["custom"]
                    .get("experimental_bearer_token")
                    .is_none()
            );
        }
        assert!(!serde_json::to_string(&response).unwrap().contains("fake-route-key"));
        assert_eq!(std::fs::read_to_string(&config).unwrap(), live);
        assert_eq!(std::fs::read_to_string(&auth).unwrap(), "{}");
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
    }
    request.target_host_id = "metadata-only".into();
    assert!(matches!(
        client.app_provider_preview(AppId::Codex, &request, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }
    ));
    server.abort();
    let _ = server.await;
}
