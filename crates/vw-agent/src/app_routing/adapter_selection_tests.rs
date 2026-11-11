use crate::app_routing::adapter_selection::{
    Adapter, ProviderType, adapter, auth_headers, gemini_auth_headers,
};
use vw_agent_apps::auth::{
    credential_file::{AccountCredential, CredentialFileStore, GeminiCredential},
    gemini_refresh::{GeminiRefresh, GeminiScope},
};
use vw_api_types::{TimestampMs, agent_apps::AppId, id::TenantScope};

#[test]
fn cc_route_031_adapter_selection_expected_behavior() {
    use ProviderType::*;
    for (kind, expected, name) in [
        (Claude, Adapter::Claude, "x-api-key"),
        (ClaudeAuth, Adapter::Claude, "authorization"),
        (Codex, Adapter::Codex, "authorization"),
        (Gemini, Adapter::Gemini, "x-goog-api-key"),
        (GeminiCli, Adapter::Gemini, "authorization"),
        (OpenRouter, Adapter::Claude, "authorization"),
        (GitHubCopilot, Adapter::Claude, "authorization"),
        (CodexOAuth, Adapter::Claude, "authorization"),
        (XaiOAuth, Adapter::Claude, "authorization"),
    ] {
        assert_eq!(adapter(kind), expected);
        let headers = auth_headers(kind, "fixture-key").unwrap();
        assert!(headers[name].is_sensitive());
        assert_eq!(
            headers[name],
            if name == "authorization" { "Bearer fixture-key" } else { "fixture-key" }
        );
    }
}
#[test]
fn cc_route_031_crlf_returns_authentication_error() {
    for token in ["", "fixture\r\nx-injected: value", "fixture\n", "fixture\0", "测试"] {
        assert_eq!(
            auth_headers(ProviderType::GeminiCli, token).unwrap_err(),
            axum::http::StatusCode::UNAUTHORIZED
        );
    }
}
#[tokio::test]
async fn cc_route_031_gemini_reads_owner_credential_each_time() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::default();
    let scope = GeminiScope { tenant: &tenant, principal: "fixture-owner", host: "fixture-host" };
    let owner = store.for_owner(scope.principal).unwrap();
    let refresh = GeminiRefresh::new().unwrap();
    let database_root = tempfile::tempdir().unwrap();
    let database =
        crate::global_database::GlobalDatabase::new(database_root.path().join("fixture.sqlite"))
            .unwrap();
    let accounts =
        crate::agent_apps::auth_accounts::AuthAccountStore::new(database, root.path()).unwrap();
    for token in ["fixture-one", "fixture-two"] {
        let credential = AccountCredential::GeminiOauth(GeminiCredential {
            access_token: token.into(),
            refresh_token: "fixture-refresh".into(),
            client_id: "fixture-client".into(),
            client_secret: "fixture-secret".into(),
            expires_at: TimestampMs(i64::MAX),
            requires_reauth: false,
        });
        if token == "fixture-one" {
            let metadata = crate::agent_apps::auth_accounts::AuthAccountMetadata::new(
                "fixture-identity",
                TimestampMs(1),
            )
            .unwrap();
            accounts.import_gemini_grant(scope, "fixture-account", &metadata, &credential).unwrap();
        } else {
            owner
                .lease(&tenant, scope.host, AppId::Gemini, "fixture-account", false)
                .unwrap()
                .save(&credential)
                .unwrap();
        }
        let headers =
            gemini_auth_headers(&accounts, &refresh, scope, "fixture-account").await.unwrap();
        assert_eq!(headers["authorization"], format!("Bearer {token}"));
    }
    assert!(
        gemini_auth_headers(
            &accounts,
            &refresh,
            GeminiScope { principal: "other-owner", ..scope },
            "fixture-account"
        )
        .await
        .is_err()
    );
    assert!(
        gemini_auth_headers(
            &accounts,
            &refresh,
            GeminiScope { host: "other-host", ..scope },
            "fixture-account"
        )
        .await
        .is_err()
    );
}
#[test]
fn cc_route_031_targets_and_url_rejection() {
    use crate::app_routing::adapter_selection::target_url;
    for (kind, host) in [
        (ProviderType::Claude, "api.anthropic.com"),
        (ProviderType::ClaudeAuth, "api.anthropic.com"),
        (ProviderType::Codex, "api.openai.com"),
        (ProviderType::Gemini, "generativelanguage.googleapis.com"),
        (ProviderType::GeminiCli, "generativelanguage.googleapis.com"),
        (ProviderType::OpenRouter, "openrouter.ai"),
        (ProviderType::GitHubCopilot, "api.githubcopilot.com"),
        (ProviderType::CodexOAuth, "chatgpt.com"),
        (ProviderType::XaiOAuth, "api.x.ai"),
    ] {
        assert_eq!(target_url(kind, None, "/v1/messages").unwrap().host_str(), Some(host));
    }
    for endpoint in ["//attacker.example", "/../token", "/%2e%2e/token", "/v1/messages?key=fixture"]
    {
        assert!(target_url(ProviderType::Claude, None, endpoint).is_err());
    }
}
#[test]
fn cc_route_031_target_deduplicates_protocol_prefix() {
    let url = crate::app_routing::adapter_selection::target_url(
        ProviderType::Codex,
        Some("https://fixture.invalid/v1"),
        "/v1/chat/completions",
    )
    .unwrap();
    assert_eq!(url.path(), "/v1/chat/completions");
}

#[tokio::test]
async fn cc_route_031_listener_uses_current_managed_credential_on_wire() {
    use crate::app_routing::{
        RoutingListener, SharedTransport, adapter_selection::ManagedCredential,
        dispatch::JsonTarget,
    };
    use axum::{Json, Router, http::HeaderMap, routing::post};
    use std::{os::unix::fs::PermissionsExt, sync::Arc, time::Duration};
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let database_root = tempfile::tempdir().unwrap();
    let database =
        crate::global_database::GlobalDatabase::new(database_root.path().join("fixture.sqlite"))
            .unwrap();
    let accounts = Arc::new(
        crate::agent_apps::auth_accounts::AuthAccountStore::new(database, root.path()).unwrap(),
    );
    let tenant = TenantScope::default();
    let scope = GeminiScope { tenant: &tenant, principal: "fixture-owner", host: "fixture-host" };
    let credential = |token: &str| {
        AccountCredential::GeminiOauth(GeminiCredential {
            access_token: token.into(),
            refresh_token: "fixture-refresh".into(),
            client_id: "fixture-client".into(),
            client_secret: "fixture-secret".into(),
            expires_at: TimestampMs(i64::MAX),
            requires_reauth: false,
        })
    };
    let metadata = crate::agent_apps::auth_accounts::AuthAccountMetadata::new(
        "fixture-identity",
        TimestampMs(1),
    )
    .unwrap();
    accounts
        .import_gemini_grant(scope, "fixture-account", &metadata, &credential("fixture-one"))
        .unwrap();
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base: reqwest::Url = format!("http://{}", upstream.local_addr().unwrap()).parse().unwrap();
    let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
    let app = Router::new().route(
        "/fixture",
        post(move |headers: HeaderMap| async move {
            sent.send(headers).unwrap();
            Json(serde_json::json!({"id":"fixture"}))
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(upstream, app).await.unwrap();
    });
    let listener = RoutingListener::default();
    let address = listener.start(0).unwrap();
    let binding = super::VerifiedRouteBinding::new(
        vw_api_types::id::ProjectScope::new(tenant.clone(), "fixture-project".into()).unwrap(),
        "fixture-host".into(),
        AppId::Codex,
        "fixture-account".into(),
        1,
    );
    let active = listener
        .auth
        .issue_with_target(
            binding,
            Some(JsonTarget {
                url: base.join("/fixture").unwrap(),
                authorization: "Bearer obsolete-fixture".parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
                managed: Some(Arc::new(ManagedCredential {
                    owner: accounts.clone(),
                    account_adapter: crate::app_routing::adapter_selection::AccountAdapter::new(
                        ProviderType::GeminiCli,
                    )
                    .unwrap(),
                    tenant: tenant.clone(),
                    principal: scope.principal.into(),
                    host: scope.host.into(),
                    account: "fixture-account".into(),
                    endpoint: "/fixture".into(),
                    base: Some(base),
                })),
            }),
        )
        .unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    for token in ["fixture-one", "fixture-two"] {
        if token == "fixture-two" {
            let store =
                CredentialFileStore::new(root.path()).unwrap().for_owner(scope.principal).unwrap();
            store
                .lease(&tenant, scope.host, AppId::Gemini, "fixture-account", false)
                .unwrap()
                .save(&credential(token))
                .unwrap();
        }
        let response = client
            .post(format!("http://{address}/v1/chat/completions"))
            .bearer_auth(&active.token)
            .json(&serde_json::json!({"messages":[]}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let headers =
            tokio::time::timeout(Duration::from_secs(5), received.recv()).await.unwrap().unwrap();
        assert_eq!(headers["authorization"], format!("Bearer {token}"));
        assert_eq!(headers["x-goog-api-client"], "GeminiCLI/1.0");
    }
    listener.stop().await.unwrap();
    server.abort();
    let _ = server.await;
}
