use super::{GatewayClient, test_support};
use crate::{AgentAppsAuthCredentials, AgentAppsAuthReadError, GatewayAuth};
use serde_json::json;
use vw_api_types::{
    agent_apps_auth::{AuthCredentialState, AuthErrorCode, AuthService},
    id::TenantScope,
};

const HOST: &str = "mac-local";
const SERVICE: AuthService = AuthService::CodexOauth;
const TOKEN_A: &str = "principal-a-00000000000000000000";
const TOKEN_B: &str = "principal-b-00000000000000000000";
const CREDENTIALS: AgentAppsAuthCredentials<'_> =
    AgentAppsAuthCredentials { username: "admin", password: "fixture", principal_token: TOKEN_A };

#[tokio::test]
async fn auth6_sdk_read_commits_expected_scoped_result() {
    let account = |id, state| {
        json!({
            "accounts": [{
                "account_id": id,
                "service": "codex_oauth",
                "label": "Fixture account",
                "credential_state": state,
                "is_default": state == "ready"
            }]
        })
    };
    let server = test_support::server(vec![
        (200, account("account-a", "ready")),
        (
            200,
            json!({
                "service": "codex_oauth",
                "default_account_id": "account-a",
                "capabilities": {
                    "login_methods": ["device_code", "browser"],
                    "can_set_default": true,
                    "can_bind": false,
                    "can_revoke": false
                }
            }),
        ),
        (
            200,
            json!({
                "bindings": [{
                    "provider_id": "supplier-a",
                    "service": "codex_oauth",
                    "account_id": "account-a"
                }]
            }),
        ),
        (200, account("account-b", "ready")),
        (200, json!({"bindings": []})),
        (200, account("account-a", "signed_out")),
    ]);
    let client_for_company = |company, skey: &str| {
        GatewayClient::new(
            server
                .client()
                .endpoint()
                .clone()
                .with_tenant_scope(TenantScope::new("platform", company).unwrap())
                .with_auth(GatewayAuth { skey: Some(skey.into()) }),
        )
        .unwrap()
    };
    let company_a = client_for_company("company-a", "unrelated-skey");
    // A malformed bearer credential also has no effect on a Basic-authenticated request.
    let company_b = client_for_company("company-b", "unrelated\nskey");
    let credentials_b = AgentAppsAuthCredentials { principal_token: TOKEN_B, ..CREDENTIALS };
    let accounts = company_a
        .agent_apps_auth_accounts(HOST, SERVICE, &CREDENTIALS)
        .await
        .expect("authorized account list");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account_id, "account-a");
    assert_eq!(accounts[0].credential_state, AuthCredentialState::Ready);

    let status = company_a.agent_apps_auth_status(HOST, SERVICE, &CREDENTIALS).await.unwrap();
    assert_eq!(status.service, SERVICE);
    assert_eq!(status.default_account_id.as_deref(), Some("account-a"));
    assert!(status.capabilities.can_set_default);

    let bindings = company_a
        .agent_apps_auth_bindings(HOST, SERVICE, Some("account-a"), &CREDENTIALS)
        .await
        .unwrap();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].provider_id, "supplier-a");
    assert_eq!(bindings[0].account_id, "account-a");
    assert_eq!(bindings[0].service, SERVICE);

    let other = company_b.agent_apps_auth_accounts(HOST, SERVICE, &credentials_b).await.unwrap();
    assert_eq!(other.len(), 1);
    assert_eq!(other[0].account_id, "account-b");
    assert!(
        company_b
            .agent_apps_auth_bindings(HOST, SERVICE, None, &credentials_b)
            .await
            .unwrap()
            .is_empty()
    );
    let refreshed = company_a.agent_apps_auth_accounts(HOST, SERVICE, &CREDENTIALS).await.unwrap();
    assert_eq!(refreshed[0].account_id, "account-a");
    assert_eq!(refreshed[0].credential_state, AuthCredentialState::SignedOut);

    for (resource, filter, company, token) in [
        ("accounts", "", "company-a", TOKEN_A),
        ("status", "", "company-a", TOKEN_A),
        ("bindings", "&account_id=account-a", "company-a", TOKEN_A),
        ("accounts", "", "company-b", TOKEN_B),
        ("bindings", "", "company-b", TOKEN_B),
        ("accounts", "", "company-a", TOKEN_A),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.path,
            format!("/v1/agent-apps/auth/{resource}?host_id=mac-local&service=codex_oauth{filter}")
        );
        assert!(request.body.is_null());
        assert_eq!(request.header("cache-control"), Some("no-cache, no-store"));
        assert_eq!(request.header("authorization"), Some("Basic YWRtaW46Zml4dHVyZQ=="));
        assert_eq!(request.header("x-agent-apps-token"), Some(token));
        assert_eq!(request.header("x-platform-id"), Some("platform"));
        assert_eq!(request.header("x-company-id"), Some(company));
    }
    server.join();
}

#[tokio::test]
async fn auth6_sdk_read_rejects_invalid_or_stale_input() {
    let server = test_support::server_raw(vec![
        (
            400,
            json!({
                "code": "invalid_input",
                "message": "Invalid account query",
                "correlation_id": "invalid-query"
            })
            .to_string(),
        ),
        (
            404,
            json!({
                "code": "not_found",
                "message": "Account resource not found",
                "correlation_id": "stale-account"
            })
            .to_string(),
        ),
        (502, "upstream returned fixture-secret".into()),
    ]);
    let client = server.client();
    let result = client.agent_apps_auth_status("bad&host=other", SERVICE, &CREDENTIALS).await;
    assert!(
        matches!(
            &result,
            Err(AgentAppsAuthReadError::Http { status: 400, error: Some(error) })
            if error.code == AuthErrorCode::InvalidInput && error.correlation_id == "invalid-query"
        ),
        "expected structured HTTP 400, got {result:?}"
    );
    let stale =
        client.agent_apps_auth_bindings(HOST, SERVICE, Some("deleted-account"), &CREDENTIALS).await;
    assert!(matches!(
        stale,
        Err(AgentAppsAuthReadError::Http { status: 404, error: Some(error) })
        if error.code == AuthErrorCode::NotFound && error.correlation_id == "stale-account"
    ));

    let upstream = client.agent_apps_auth_accounts(HOST, SERVICE, &CREDENTIALS).await.unwrap_err();
    assert_eq!(upstream, AgentAppsAuthReadError::Http { status: 502, error: None });
    assert!(!format!("{upstream:?} {upstream}").contains("fixture-secret"));

    assert_eq!(
        server.take_request().path,
        "/v1/agent-apps/auth/status?host_id=bad%26host%3Dother&service=codex_oauth"
    );
    assert_eq!(
        server.take_request().path,
        "/v1/agent-apps/auth/bindings?host_id=mac-local&service=codex_oauth&account_id=deleted-account"
    );
    assert_eq!(
        server.take_request().path,
        "/v1/agent-apps/auth/accounts?host_id=mac-local&service=codex_oauth"
    );
    let client = server.client().clone();
    server.join();

    let malformed = AgentAppsAuthCredentials { principal_token: "invalid\nheader", ..CREDENTIALS };
    assert_eq!(
        client.agent_apps_auth_accounts(HOST, SERVICE, &malformed).await,
        Err(AgentAppsAuthReadError::InvalidPrincipalToken)
    );
}
