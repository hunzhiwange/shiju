use super::{GatewayClient, test_support};
use crate::{
    AgentAppsAuthCredentials, AgentAppsAuthFlowError, AuthErrorCode, AuthFlow, AuthFlowId,
    AuthLoginMethod, AuthPasteRequest, AuthService, AuthStartRequest, AuthStartResponse,
    GatewayAuth,
};
use serde_json::{Value, json};
use tracing::field::Field;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};
use vw_api_types::id::TenantScope;

const HOST: &str = "mac-local";
const CLAUDE: AuthService = AuthService::ClaudeOfficial;
const CODEX: AuthService = AuthService::CodexOauth;
const TOKEN: &str = "fixture-private-principal";
const PASSWORD: &str = "fixture-private-password";
const BASIC: &str = "Basic YWRtaW46Zml4dHVyZS1wcml2YXRlLXBhc3N3b3Jk";
const CREDENTIALS: AgentAppsAuthCredentials<'_> =
    AgentAppsAuthCredentials { username: "admin", password: PASSWORD, principal_token: TOKEN };
const FLOWS: &str = "/v1/agent-apps/auth/flows";
const CLAUDE_ID: &str = "4fa1c610-3637-4782-b631-dba73bda3022";
const CODEX_ID: &str = "809ceda4-2e4e-4ca3-b987-b44eb87f94c3";
const CHALLENGE: &str = "https://auth.example.test/authorize?state=fixture-private-state";
const USER_CODE: &str = "ABCD-9876";
const PASTE: &str = "http://localhost:45381/callback?code=fixture-private-paste";
const PRIVATE_STATE: &str = "fixture-private-state";
const PRIVATE_PASTE: &str = "fixture-private-paste";
const RAW_ERROR: &str = "fixture-private-upstream-error";
const EXPIRES: u64 = 1_797_000_000_000;

#[tokio::test]
async fn auth6_sdk_flow_commits_expected_scoped_result() {
    let _logs = tracing::subscriber::set_default(RedactedLogs);
    let scope = TenantScope::new("platform-a", "company-a").unwrap();
    let server = test_support::server_with_tenant_scope(
        vec![
            (
                201,
                json!({
                    "state": "pending", "flow_id": CLAUDE_ID, "expires_at": EXPIRES,
                    "retry_after_seconds": 4, "verification_url": CHALLENGE, "can_paste": true
                }),
            ),
            (202, Value::Null),
            (
                200,
                json!({
                    "state": "pending", "flow_id": CLAUDE_ID, "expires_at": EXPIRES,
                    "retry_after_seconds": 13
                }),
            ),
            (
                200,
                json!({
                    "state": "approved", "flow_id": CLAUDE_ID, "expires_at": EXPIRES,
                    "account": {
                        "account_id": "account-a", "service": "claude_official", "label": "Fixture",
                        "credential_state": "ready", "is_default": false
                    }
                }),
            ),
            (
                201,
                json!({
                    "state": "pending", "flow_id": CODEX_ID, "expires_at": EXPIRES,
                    "retry_after_seconds": 7, "verification_url": CHALLENGE, "user_code": USER_CODE
                }),
            ),
            (204, Value::Null),
            (200, json!({"state": "cancelled", "flow_id": CODEX_ID, "expires_at": EXPIRES})),
        ],
        scope,
    );
    let client = GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(GatewayAuth { skey: Some("unrelated\nbearer".into()) }),
    )
    .unwrap();

    let start = client
        .agent_apps_auth_start(HOST, CLAUDE, &CREDENTIALS, &start_input(AuthLoginMethod::CopyLink))
        .await
        .expect("Claude start must return the scoped challenge");
    assert_redacted(&format!("{start:?}"));
    let AuthStartResponse::Pending {
        flow_id,
        retry_after_seconds,
        verification_url,
        can_paste,
        ..
    } = start;
    assert_eq!(flow_id.0, CLAUDE_ID);
    assert_eq!(retry_after_seconds, 4);
    assert_eq!(verification_url, CHALLENGE);
    assert_eq!(can_paste, Some(true));

    let paste = AuthPasteRequest { pasted: PASTE.into() };
    assert_redacted(&format!("{paste:?}"));
    client.agent_apps_auth_paste(HOST, &CREDENTIALS, &flow_id, &paste).await.unwrap();
    let pending = client.agent_apps_auth_poll(HOST, CLAUDE, &CREDENTIALS, &flow_id).await.unwrap();
    assert!(matches!(pending, AuthFlow::Pending { retry_after_seconds: 13, .. }));
    let approved = client.agent_apps_auth_poll(HOST, CLAUDE, &CREDENTIALS, &flow_id).await.unwrap();
    assert!(
        matches!(approved, AuthFlow::Approved { account, .. } if account.account_id == "account-a")
    );

    let start = client
        .agent_apps_auth_start(HOST, CODEX, &CREDENTIALS, &start_input(AuthLoginMethod::DeviceCode))
        .await
        .unwrap();
    assert_redacted(&format!("{start:?}"));
    let AuthStartResponse::Pending { flow_id, user_code, retry_after_seconds, .. } = start;
    assert_eq!(flow_id.0, CODEX_ID);
    assert_eq!(user_code.as_deref(), Some(USER_CODE));
    assert_eq!(retry_after_seconds, 7);
    client.agent_apps_auth_cancel(HOST, CODEX, &CREDENTIALS, &flow_id).await.unwrap();
    let cancelled = client.agent_apps_auth_poll(HOST, CODEX, &CREDENTIALS, &flow_id).await.unwrap();
    assert!(matches!(cancelled, AuthFlow::Cancelled { .. }));

    for (method, suffix, service, body) in [
        ("POST", "".into(), "claude_official", json!({"method": "copy_link"})),
        ("POST", format!("/{CLAUDE_ID}/paste"), "claude_official", json!({"pasted": PASTE})),
        ("GET", format!("/{CLAUDE_ID}"), "claude_official", Value::Null),
        ("GET", format!("/{CLAUDE_ID}"), "claude_official", Value::Null),
        ("POST", "".into(), "codex_oauth", json!({"method": "device_code"})),
        ("DELETE", format!("/{CODEX_ID}"), "codex_oauth", Value::Null),
        ("GET", format!("/{CODEX_ID}"), "codex_oauth", Value::Null),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, format!("{FLOWS}{suffix}?host_id=mac-local&service={service}"));
        assert_eq!(request.body, body);
        assert_eq!(request.header("authorization"), Some(BASIC));
        assert_eq!(request.header("x-agent-apps-token"), Some(TOKEN));
        assert_eq!(request.header("x-platform-id"), Some("platform-a"));
        assert_eq!(request.header("x-company-id"), Some("company-a"));
        assert_eq!(request.header("cache-control"), Some("no-cache, no-store"));
    }
    server.join();
}

#[tokio::test]
async fn auth6_sdk_flow_rejects_invalid_or_stale_input() {
    let _logs = tracing::subscriber::set_default(RedactedLogs);
    let server = test_support::server_raw(vec![
        (
            409,
            json!({"code": "conflict", "message": RAW_ERROR, "correlation_id": "flow-conflict"})
                .to_string(),
        ),
        (
            404,
            json!({"code": "not_found", "message": RAW_ERROR, "correlation_id": "flow-missing"})
                .to_string(),
        ),
        (502, RAW_ERROR.into()),
        (200, RAW_ERROR.into()),
        (200, String::new()),
    ]);
    let client = server.client().clone();
    let flow_id = AuthFlowId(CLAUDE_ID.into());
    let paste = AuthPasteRequest { pasted: PASTE.into() };

    let error =
        client.agent_apps_auth_paste(HOST, &CREDENTIALS, &flow_id, &paste).await.unwrap_err();
    assert_eq!(
        error,
        AgentAppsAuthFlowError::Http {
            status: 409,
            code: Some(AuthErrorCode::Conflict),
            correlation_id: Some("flow-conflict".into())
        },
        "terminal flow must preserve the gateway conflict"
    );
    assert_redacted(&format!("{error:?} {error}"));

    let stale =
        client.agent_apps_auth_poll(HOST, CLAUDE, &CREDENTIALS, &flow_id).await.unwrap_err();
    assert_eq!(
        stale,
        AgentAppsAuthFlowError::Http {
            status: 404,
            code: Some(AuthErrorCode::NotFound),
            correlation_id: Some("flow-missing".into())
        }
    );

    let upstream =
        client.agent_apps_auth_cancel(HOST, CLAUDE, &CREDENTIALS, &flow_id).await.unwrap_err();
    assert_eq!(
        upstream,
        AgentAppsAuthFlowError::Http { status: 502, code: None, correlation_id: None }
    );
    assert_redacted(&format!("{upstream:?} {upstream}"));
    let decode =
        client.agent_apps_auth_poll(HOST, CLAUDE, &CREDENTIALS, &flow_id).await.unwrap_err();
    assert_eq!(decode, AgentAppsAuthFlowError::Decode);
    assert_redacted(&format!("{decode:?} {decode}"));

    // A successful HTTP status alone is insufficient to confirm local cancellation.
    assert_eq!(
        client.agent_apps_auth_cancel(HOST, CLAUDE, &CREDENTIALS, &flow_id).await,
        Err(AgentAppsAuthFlowError::Http { status: 200, code: None, correlation_id: None })
    );
    server.join();

    let malformed = AgentAppsAuthCredentials { principal_token: "invalid\nheader", ..CREDENTIALS };
    assert_eq!(
        client.agent_apps_auth_poll(HOST, CLAUDE, &malformed, &flow_id).await,
        Err(AgentAppsAuthFlowError::InvalidPrincipalToken)
    );
    let unsafe_id = AuthFlowId("../accounts?scope=other".into());
    assert_eq!(
        client.agent_apps_auth_cancel(HOST, CLAUDE, &CREDENTIALS, &unsafe_id).await,
        Err(AgentAppsAuthFlowError::Request(AuthErrorCode::InvalidInput))
    );
    assert_eq!(
        client
            .agent_apps_auth_start(
                HOST,
                CLAUDE,
                &CREDENTIALS,
                &start_input(AuthLoginMethod::DeviceCode)
            )
            .await,
        Err(AgentAppsAuthFlowError::Request(AuthErrorCode::Unsupported))
    );

    let transport =
        client.agent_apps_auth_paste(HOST, &CREDENTIALS, &flow_id, &paste).await.unwrap_err();
    assert_eq!(transport, AgentAppsAuthFlowError::Transport);
    assert_redacted(&format!("{transport:?} {transport}"));
}

fn start_input(method: AuthLoginMethod) -> AuthStartRequest {
    AuthStartRequest { method, github_domain: None, region: None, target_account_id: None }
}

fn assert_redacted(value: &str) {
    for secret in [
        TOKEN,
        PASSWORD,
        BASIC,
        CLAUDE_ID,
        CODEX_ID,
        CHALLENGE,
        PRIVATE_STATE,
        USER_CODE,
        PASTE,
        PRIVATE_PASTE,
        RAW_ERROR,
    ] {
        assert!(!value.contains(secret), "authorization data was exposed");
    }
}

struct RedactedLogs;

impl Subscriber for RedactedLogs {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        span.record(&mut assert_redacted_field);
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, record: &Record<'_>) {
        record.record(&mut assert_redacted_field);
    }

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        event.record(&mut assert_redacted_field);
    }

    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}

fn assert_redacted_field(_: &Field, value: &dyn std::fmt::Debug) {
    assert_redacted(&format!("{value:?}"));
}
