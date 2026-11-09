use super::{
    agent_apps_auth_start::Runtime,
    agent_apps_auth_start_tests::{BASIC, CLAUDE, FLOWS, Gateway, OWNER},
};
use crate::global_database::{GlobalDatabase, test_support::PostgresTestDatabase};
use axum::http::{StatusCode, header};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};
use vw_agent_apps::auth::credential_file::{AccountCredential, CredentialFileStore};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{
        AuthCredentialState, AuthError, AuthErrorCode, AuthFlow, AuthLoginMethod, AuthPasteRequest,
        AuthService, AuthStartRequest, AuthStartResponse,
    },
    id::TenantScope,
};
use vw_gateway_client::{
    AgentAppsAuthCredentials, AgentAppsAuthError, GatewayClient, GatewayEndpoint,
};

const ACCESS: &str = "auth55-private-access";
const REFRESH: &str = "auth55-private-refresh";
const CODE: &str = "auth55-private-code";
const URL_SECRET: &str = "auth55-private-url";
const STATE: &str = "auth55-browser-state";
const SERVICE: AuthService = AuthService::ClaudeOfficial;
const CREDENTIALS: AgentAppsAuthCredentials<'_> =
    AgentAppsAuthCredentials { username: "test", password: "secret", principal_token: OWNER };
const INPUT: AuthStartRequest = AuthStartRequest {
    method: AuthLoginMethod::CopyLink,
    github_domain: None,
    region: None,
    target_account_id: None,
};

#[derive(Clone, Default)]
struct Capture {
    next_span: Arc<AtomicU64>,
    records: Arc<Mutex<Vec<String>>>,
}

impl Subscriber for Capture {
    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }

    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        attributes.record(&mut self.clone());
        Id::from_u64(self.next_span.fetch_add(1, Ordering::Relaxed) + 1)
    }

    fn record(&self, _: &Id, values: &Record<'_>) {
        values.record(&mut self.clone());
    }

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        // Keep each event together so service, code and correlation can be matched.
        let mut fields = Fields::default();
        event.record(&mut fields);
        self.records.lock().unwrap().push(fields.0);
    }

    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}

#[derive(Default)]
struct Fields(String);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write;
        write!(&mut self.0, "{}={value:?} ", field.name()).unwrap();
    }
}

impl Visit for Capture {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let mut fields = Fields::default();
        fields.record_debug(field, value);
        self.records.lock().unwrap().push(fields.0);
    }
}

fn assert_redacted(surface: &str, output: &str) {
    for (name, value) in [
        ("access token", ACCESS),
        ("refresh token", REFRESH),
        ("authorization code", CODE),
        ("unexpected URL parameter", URL_SECRET),
        ("principal credential", OWNER),
        ("Basic credential", BASIC),
    ] {
        assert!(!output.contains(value), "{surface} exposed the {name}");
    }
    assert!(!output.contains(STATE), "{surface} exposed browser state");
}

fn sdk(gateway: &Gateway) -> anyhow::Result<GatewayClient> {
    GatewayClient::new(
        GatewayEndpoint::new(gateway.addr.ip().to_string(), gateway.addr.port())
            .with_tenant_scope(TenantScope::new("p", "c")?),
    )
    .map_err(anyhow::Error::msg)
}

async fn commit(home: &Path, database: GlobalDatabase, capture: Capture) -> anyhow::Result<()> {
    let gateway = Gateway::start_with_runtime(home, database, Arc::new(Runtime::default())).await?;
    let sdk = sdk(&gateway)?;
    let link = format!(
        "https://claude.com/cai/oauth/authorize?code=true&client_id=fixture-client&response_type=code\
         &redirect_uri=http%3A%2F%2Flocalhost%3A54545%2Fcallback&scope=user%3Aprofile+user%3Ainference\
         &code_challenge=0123456789012345678901234567890123456789012&code_challenge_method=S256&state={STATE}"
    );
    let paste = AuthPasteRequest { pasted: format!("{CODE}#{STATE}") };
    fs::write(
        home.join("claude"),
        format!(
            r#"#!/bin/sh
set -eu
"$BROWSER" '{link}'
IFS= read -r code
test "$code" = '{pasted}'
umask 077
printf '%s' '{{"claudeAiOauth":{{"accessToken":"{ACCESS}","refreshToken":"{REFRESH}"}}}}' > .credentials.json
printf '%s' '{{"oauthAccount":{{"accountUuid":"auth55-account","organizationUuid":"auth55-org"}}}}' > .claude.json
printf '%s\n' '{ACCESS} {REFRESH} {CODE}' >&2
"#,
            pasted = paste.pasted,
        ),
    )?;

    let start = sdk.agent_apps_auth_start("host", SERVICE, &CREDENTIALS, &INPUT).await?;
    assert_redacted("start Debug", &format!("{start:?}"));
    assert_redacted("paste Debug", &format!("{paste:?}"));
    let AuthStartResponse::Pending { flow_id, verification_url, can_paste, .. } = start;
    assert_eq!(verification_url, link, "browser receives the required challenge");
    assert_eq!(can_paste, Some(true));
    sdk.agent_apps_auth_paste("host", &CREDENTIALS, &flow_id, &paste).await?;
    let flow = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let flow = sdk.agent_apps_auth_poll("host", SERVICE, &CREDENTIALS, &flow_id).await?;
            if !matches!(flow, AuthFlow::Pending { .. }) {
                return Ok::<_, AgentAppsAuthError>(flow);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await??;
    assert_redacted("flow Debug", &format!("{flow:?}"));
    assert_redacted("flow JSON", &serde_json::to_string(&flow)?);
    let AuthFlow::Approved { account, .. } = flow else {
        anyhow::bail!("Claude CLI did not complete authorization: {flow:?}");
    };

    let accounts = sdk.agent_apps_auth_accounts("host", SERVICE, &CREDENTIALS).await?;
    assert_redacted("accounts Debug", &format!("{accounts:?}"));
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account_id, account.account_id);
    assert_eq!(accounts[0].service, SERVICE);
    assert_eq!(accounts[0].credential_state, AuthCredentialState::Ready);

    let response = gateway
        .client
        .get(format!("http://{}/v1/agent-apps/auth/accounts?{CLAUDE}", gateway.addr))
        .header(header::AUTHORIZATION, BASIC)
        .header("x-platform-id", "p")
        .header("x-company-id", "c")
        .header("x-agent-apps-token", OWNER)
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_redacted("account response body", &response.text().await?);

    let tenant = TenantScope::new("p", "c")?;
    let credential = CredentialFileStore::new(&home.join("credentials"))
        .and_then(|store| store.for_owner("owner"))
        .and_then(|owner| owner.read(&tenant, "host", AppId::Claude, &account.account_id))
        .map_err(anyhow::Error::msg)?;
    let AccountCredential::ClaudeDirectory(directory) = credential else {
        anyhow::bail!("authorized account has no managed credential directory");
    };
    let material: Value =
        serde_json::from_slice(&fs::read(directory.path().join(".credentials.json"))?)?;
    assert!(material["claudeAiOauth"]["accessToken"] == ACCESS);
    assert!(material["claudeAiOauth"]["refreshToken"] == REFRESH);
    assert_redacted("success trace", &capture.records.lock().unwrap().join("\n"));
    Ok(())
}

async fn reject(home: &Path, database: GlobalDatabase, capture: Capture) -> anyhow::Result<()> {
    let gateway = Gateway::start_with_runtime(home, database, Arc::new(Runtime::default())).await?;
    let sdk = sdk(&gateway)?;
    for query in [
        format!("token={ACCESS}&extra={URL_SECRET}"),
        format!("redirect_uri=http%3A%2F%2Flocalhost%3A54545%2Fcallback%3Fcode%3D{CODE}"),
        format!("code_challenge={REFRESH:0<43}"),
    ] {
        let link =
            format!("https://claude.com/cai/oauth/authorize?code=true&state={STATE}&{query}");
        fs::write(
            home.join("claude"),
            format!(
                r#"#!/bin/sh
set -eu
"$BROWSER" '{link}'
printf '%s\n' 'token={ACCESS} refresh={REFRESH} code={CODE} extra={URL_SECRET}' >&2
sleep 1
exit 1
"#,
            ),
        )?;

        let (status, body) =
            gateway.post(FLOWS, CLAUDE, OWNER, json!({"method":"copy_link"})).await?;
        assert_redacted("rejected start response", &body.to_string());
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        let error: AuthError = serde_json::from_value(body)?;
        assert_eq!(error.code, AuthErrorCode::UpstreamFailure);
        assert_eq!(error.message, "Authorization provider unavailable");
        let raw_id = error.correlation_id.clone();
        assert!(uuid::Uuid::parse_str(&raw_id).is_ok());
        assert_redacted("gateway error Debug", &format!("{error:?}"));

        let error =
            sdk.agent_apps_auth_start("host", SERVICE, &CREDENTIALS, &INPUT).await.unwrap_err();
        assert_redacted("SDK error", &format!("{error:?} {error}"));
        let AgentAppsAuthError::Http {
            status: 502,
            code: Some(AuthErrorCode::UpstreamFailure),
            correlation_id: Some(id),
        } = error
        else {
            anyhow::bail!("SDK did not retain the gateway failure: {error:?}");
        };
        assert!(uuid::Uuid::parse_str(&id).is_ok());
        assert_ne!(id, raw_id);
        assert_diagnostic(&capture, &id)?;
        assert_diagnostic(&capture, &raw_id)?;
    }
    assert!(sdk.agent_apps_auth_accounts("host", SERVICE, &CREDENTIALS).await?.is_empty());
    assert_redacted("failure trace", &capture.records.lock().unwrap().join("\n"));
    Ok(())
}

fn assert_diagnostic(capture: &Capture, id: &str) -> anyhow::Result<()> {
    let records = capture.records.lock().unwrap();
    let event = records
        .iter()
        .find(|event| event.contains(id))
        .ok_or_else(|| anyhow::anyhow!("missing diagnostic event for the SDK error"))?;
    assert!(event.contains("service=ClaudeOfficial"));
    assert!(event.contains("code=UpstreamFailure"));
    assert!(event.contains("operation=\"start\""));
    assert_redacted("diagnostic event", event);
    Ok(())
}

fn runtime() -> anyhow::Result<tokio::runtime::Runtime> {
    // Keep the listener, CLI supervisor and SDK futures under the test subscriber.
    Ok(tokio::runtime::Builder::new_current_thread().enable_all().build()?)
}

#[test]
fn auth6_redaction_commits_expected_scoped_result() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let db = GlobalDatabase::for_home(home.path())?;
    let capture = Capture::default();
    let _guard = tracing::subscriber::set_default(capture.clone());
    runtime()?.block_on(commit(home.path(), db, capture))
}

#[test]
fn auth6_redaction_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let db = GlobalDatabase::for_home(home.path())?;
    let capture = Capture::default();
    let _guard = tracing::subscriber::set_default(capture.clone());
    runtime()?.block_on(reject(home.path(), db, capture))
}

#[test]
fn auth6_redaction_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let db = PostgresTestDatabase::from_environment()?;
    let capture = Capture::default();
    let _guard = tracing::subscriber::set_default(capture.clone());
    runtime()?.block_on(commit(home.path(), db.database().clone(), capture))
}

#[test]
fn auth6_redaction_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let db = PostgresTestDatabase::from_environment()?;
    let capture = Capture::default();
    let _guard = tracing::subscriber::set_default(capture.clone());
    runtime()?.block_on(reject(home.path(), db.database().clone(), capture))
}
