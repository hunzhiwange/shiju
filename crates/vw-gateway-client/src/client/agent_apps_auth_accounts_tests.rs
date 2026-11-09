use super::test_support;
use crate::{
    AgentAppsAuthCredentials, AgentAppsAuthError, AgentAppsAuthLogout, AuthErrorCode, AuthService,
    GatewayAuth, GatewayClient,
};
use serde_json::{Value, json};
use vw_api_types::id::TenantScope;

const HOST: &str = "mac-local";
const ACCOUNT: &str = "account_a";
const CODEX: AuthService = AuthService::CodexOauth;
const DSH: AuthService = AuthService::DshAccount;
const KIMI: AuthService = AuthService::KimiOauth;
const CREDENTIALS: AgentAppsAuthCredentials<'_> = AgentAppsAuthCredentials {
    username: "admin",
    password: "fixture-private-password",
    principal_token: "fixture-private-principal",
};
const BASIC: &str = "Basic YWRtaW46Zml4dHVyZS1wcml2YXRlLXBhc3N3b3Jk";
const PRIVATE_ERROR: &str = "fixture-private-upstream-error";

#[tokio::test]
async fn auth6_sdk_accounts_commits_expected_scoped_result() {
    let scope = TenantScope::new("platform-a", "company-a").unwrap();
    let server = test_support::server_with_tenant_scope(vec![(204, Value::Null); 9], scope);
    let client = GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(GatewayAuth { skey: Some("unrelated\nbearer".into()) }),
    )
    .unwrap();

    client
        .agent_apps_auth_bind(HOST, CODEX, &CREDENTIALS, "supplier_a", Some(ACCOUNT))
        .await
        .expect("a scoped binding must confirm HTTP 204");
    client.agent_apps_auth_bind(HOST, CODEX, &CREDENTIALS, "supplier_a", None).await.unwrap();
    client
        .agent_apps_auth_bind(HOST, KIMI, &CREDENTIALS, "官方 ?#%2F", Some(ACCOUNT))
        .await
        .unwrap();
    client.agent_apps_auth_set_default(HOST, CODEX, &CREDENTIALS, ACCOUNT).await.unwrap();
    client
        .agent_apps_auth_logout(
            HOST,
            CODEX,
            &CREDENTIALS,
            AgentAppsAuthLogout::Account { account_id: ACCOUNT },
        )
        .await
        .unwrap();
    client
        .agent_apps_auth_logout(HOST, CODEX, &CREDENTIALS, AgentAppsAuthLogout::Default)
        .await
        .unwrap();
    client
        .agent_apps_auth_logout(HOST, CODEX, &CREDENTIALS, AgentAppsAuthLogout::All)
        .await
        .unwrap();
    client.agent_apps_auth_revoke(HOST, DSH, &CREDENTIALS, ACCOUNT).await.unwrap();
    client.agent_apps_auth_delete(HOST, CODEX, &CREDENTIALS, ACCOUNT).await.unwrap();

    for (method, resource, service, body) in [
        ("PUT", "bindings/supplier_a", "codex_oauth", json!({"account_id": ACCOUNT})),
        ("PUT", "bindings/supplier_a", "codex_oauth", json!({"account_id": null})),
        (
            "PUT",
            "bindings/%E5%AE%98%E6%96%B9%20%3F%23%252F",
            "kimi_oauth",
            json!({"account_id": ACCOUNT}),
        ),
        ("PUT", "accounts/account_a/default", "codex_oauth", Value::Null),
        ("POST", "logout", "codex_oauth", json!({"selection": "account", "account_id": ACCOUNT})),
        ("POST", "logout", "codex_oauth", json!({"selection": "default"})),
        ("POST", "logout", "codex_oauth", json!({"selection": "all"})),
        ("POST", "accounts/account_a/revoke", "dsh_account", Value::Null),
        ("DELETE", "accounts/account_a", "codex_oauth", Value::Null),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(
            request.path,
            format!("/v1/agent-apps/auth/{resource}?host_id={HOST}&service={service}")
        );
        assert_eq!(request.body, body);
        assert_eq!(request.header("authorization"), Some(BASIC));
        assert_eq!(request.header("x-agent-apps-token"), Some(CREDENTIALS.principal_token));
        assert_eq!(request.header("x-platform-id"), Some("platform-a"));
        assert_eq!(request.header("x-company-id"), Some("company-a"));
        assert_eq!(request.header("cache-control"), Some("no-cache, no-store"));
    }
    server.join();
}

#[tokio::test]
async fn auth6_sdk_accounts_rejects_invalid_or_stale_input() {
    let gateway_error = |code: &str| json!({"code": code, "message": PRIVATE_ERROR, "correlation_id": "fixture-correlation"});
    let server = test_support::server(vec![
        (409, gateway_error("reauth_required")),
        (404, gateway_error("not_found")),
        (409, gateway_error("account_in_use")),
        (501, gateway_error("unsupported")),
        (502, gateway_error("remote_revocation_unconfirmed")),
        (200, Value::Null),
    ]);
    let client = server.client().clone();

    let reauth = client
        .agent_apps_auth_bind(HOST, CODEX, &CREDENTIALS, "supplier_a", Some(ACCOUNT))
        .await
        .unwrap_err();
    assert_http(reauth, 409, AuthErrorCode::ReauthRequired);
    assert_http(
        client.agent_apps_auth_set_default(HOST, CODEX, &CREDENTIALS, ACCOUNT).await.unwrap_err(),
        404,
        AuthErrorCode::NotFound,
    );
    assert_http(
        client.agent_apps_auth_delete(HOST, CODEX, &CREDENTIALS, ACCOUNT).await.unwrap_err(),
        409,
        AuthErrorCode::AccountInUse,
    );
    assert_http(
        client.agent_apps_auth_revoke(HOST, CODEX, &CREDENTIALS, ACCOUNT).await.unwrap_err(),
        501,
        AuthErrorCode::Unsupported,
    );
    assert_http(
        client.agent_apps_auth_revoke(HOST, DSH, &CREDENTIALS, ACCOUNT).await.unwrap_err(),
        502,
        AuthErrorCode::RemoteRevocationUnconfirmed,
    );
    assert_eq!(
        client
            .agent_apps_auth_logout(HOST, CODEX, &CREDENTIALS, AgentAppsAuthLogout::Default)
            .await,
        Err(AgentAppsAuthError::Http { status: 200, code: None, correlation_id: None })
    );
    server.join();

    let invalid = Err(AgentAppsAuthError::Request(AuthErrorCode::InvalidInput));
    assert_eq!(
        client.agent_apps_auth_delete(HOST, CODEX, &CREDENTIALS, "../logout").await,
        invalid
    );
    assert_eq!(client.agent_apps_auth_bind(HOST, KIMI, &CREDENTIALS, "..", None).await, invalid);
    assert_eq!(
        client
            .agent_apps_auth_logout(
                HOST,
                CODEX,
                &CREDENTIALS,
                AgentAppsAuthLogout::Account { account_id: "" }
            )
            .await,
        invalid
    );
    assert_eq!(client.agent_apps_auth_set_default("", CODEX, &CREDENTIALS, ACCOUNT).await, invalid);

    let malformed = AgentAppsAuthCredentials { principal_token: "invalid\nheader", ..CREDENTIALS };
    assert_eq!(
        client.agent_apps_auth_delete(HOST, CODEX, &malformed, ACCOUNT).await,
        Err(AgentAppsAuthError::InvalidPrincipalToken)
    );
    assert_eq!(
        client.agent_apps_auth_delete(HOST, CODEX, &CREDENTIALS, ACCOUNT).await,
        Err(AgentAppsAuthError::Transport)
    );
}

fn assert_http(error: AgentAppsAuthError, status: u16, code: AuthErrorCode) {
    assert_eq!(
        error,
        AgentAppsAuthError::Http {
            status,
            code: Some(code),
            correlation_id: Some("fixture-correlation".into())
        },
        "the SDK must preserve the Gateway error"
    );
    let rendered = format!("{error:?} {error}");
    for secret in [
        PRIVATE_ERROR,
        "fixture-correlation",
        CREDENTIALS.password,
        CREDENTIALS.principal_token,
        BASIC,
    ] {
        assert!(!rendered.contains(secret), "authorization data was exposed");
    }
}
