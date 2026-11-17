use super::{agent_apps_auth_start::Runtime, auth_router as agent_apps_auth_read};
use axum::{Extension, Form, Json, Router, extract::State, routing::post};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    net::SocketAddr,
    os::unix::fs::PermissionsExt,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{net::TcpListener, task::JoinHandle};
use vw_agent_apps::{
    auth::credential_file::{AccountCredential, CredentialFileStore},
    providers::codex_catalog,
};
use {
    agent_app_services::auth_accounts::AuthAccountScope,
    agent_database::{GlobalDatabase, run_sync_database, test_support::PostgresTestDatabase},
    app_provider_services::AppProviderStore,
};
use {
    gateway_endpoint::GatewayEndpoint,
    {
        gateway_app_auth::AgentAppsAuthCredentials, gateway_app_auth::AgentAppsAuthError,
        gateway_app_auth::AgentAppsAuthLogout, vw_gateway_client::GatewayClient,
    },
};
use {
    vw_api_app_auth::AuthCredentialState, vw_api_app_auth::AuthErrorCode,
    vw_api_app_auth::AuthFlow, vw_api_app_auth::AuthFlowId, vw_api_app_auth::AuthLoginMethod,
    vw_api_app_auth::AuthService, vw_api_app_auth::AuthStartRequest,
    vw_api_app_auth::AuthStartResponse, vw_api_transport::id::TenantScope,
};

const HOST: &str = "desktop-local";
const SERVICE: AuthService = AuthService::CodexOauth;
const PROVIDER: &str = "official";
const OWNER: AgentAppsAuthCredentials<'_> = AgentAppsAuthCredentials {
    username: "test",
    password: "secret",
    principal_token: "fixture-auth56-owner-token-aaaaaaaa",
};
const INPUT: AuthStartRequest = AuthStartRequest {
    method: AuthLoginMethod::DeviceCode,
    github_domain: None,
    region: None,
    target_account_id: None,
};
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const DEVICE_ID: &str = "fixture-auth56-private-device";
const USER_CODE: &str = "ABCD-EFGH";
const CODE: &str = "fixture-auth56-private-code";
const VERIFIER: &str = "fixture-auth56-private-verifier";
const REFRESH: &str = "fixture-auth56-private-refresh";

struct Server {
    address: SocketAddr,
    task: JoinHandle<()>,
}

impl Server {
    async fn start(router: Router) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let task = tokio::spawn(async move {
            axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        });
        Ok(Self { address, task })
    }

    async fn stop(mut self) {
        self.task.abort();
        assert!((&mut self.task).await.unwrap_err().is_cancelled());
    }

    fn sdk(&self, tenant: &TenantScope) -> anyhow::Result<GatewayClient> {
        GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", self.address.port())
                .with_tenant_scope(tenant.clone()),
        )
        .map_err(anyhow::Error::msg)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Clone, Default)]
struct OAuth(Arc<Mutex<Vec<&'static str>>>);

impl OAuth {
    async fn serve(&self) -> anyhow::Result<Server> {
        Server::start(
            Router::new()
                .route("/api/accounts/deviceauth/usercode", post(Self::start))
                .route("/api/accounts/deviceauth/token", post(Self::poll))
                .route("/oauth/token", post(Self::exchange))
                .with_state(self.clone()),
        )
        .await
    }

    async fn start(State(oauth): State<Self>, Json(request): Json<Value>) -> Json<Value> {
        assert_eq!(request, json!({"client_id":CLIENT_ID}));
        oauth.0.lock().unwrap().push("start");
        Json(
            json!({"device_auth_id":DEVICE_ID,"user_code":USER_CODE,"interval":"1","expires_in":60}),
        )
    }

    async fn poll(State(oauth): State<Self>, Json(request): Json<Value>) -> Json<Value> {
        assert_eq!(request, json!({"device_auth_id":DEVICE_ID,"user_code":USER_CODE}));
        oauth.0.lock().unwrap().push("poll");
        Json(json!({"authorization_code":CODE,"code_verifier":VERIFIER}))
    }

    async fn exchange(State(oauth): State<Self>, Form(request): Form<Value>) -> Json<Value> {
        assert_eq!(
            request,
            json!({
                "grant_type":"authorization_code",
                "code":CODE,
                "redirect_uri":"https://auth.openai.com/deviceauth/callback",
                "client_id":CLIENT_ID,
                "code_verifier":VERIFIER
            })
        );
        oauth.0.lock().unwrap().push("exchange");
        Json(json!({
            "access_token":Self::access_token(),
            "refresh_token":REFRESH,
            "id_token":jwt(json!({"sub":"fixture-auth56-user"}))
        }))
    }

    fn access_token() -> String {
        jwt(
            json!({"https://api.openai.com/auth":{"chatgpt_account_id":"fixture-auth56-workspace"}}),
        )
    }

    fn assert_requests(&self, expected: &[&str]) {
        assert_eq!(*self.0.lock().unwrap(), expected);
    }
}

fn jwt(claims: Value) -> String {
    format!(
        "{}.{}.fixture-signature",
        URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256"}"#),
        URL_SAFE_NO_PAD.encode(claims.to_string())
    )
}

struct Fixture {
    home: tempfile::TempDir,
    tenant: TenantScope,
}

impl Fixture {
    fn new(database: GlobalDatabase) -> anyhow::Result<Self> {
        let fixture = Self { home: tempfile::tempdir()?, tenant: TenantScope::new("p", "c")? };
        let home = fixture.home.path();
        let root = fixture.root();
        fs::create_dir_all(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let profile = codex_catalog::import(
            Some(b"model_provider='openai'\nmodel='fixture-model'\n"),
            Some(b"{}"),
        )
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        run_sync_database(|| {
            let providers = AppProviderStore::new(database).map_err(anyhow::Error::msg)?;
            providers
                .codex_save(&fixture.tenant, PROVIDER, &profile, None, None)
                .map_err(anyhow::Error::msg)?;
            Ok(())
        })?;
        Ok(fixture)
    }

    fn root(&self) -> std::path::PathBuf {
        let scope = serde_json::to_vec(&(
            self.tenant.platform_id().as_ref(),
            self.tenant.company_id().as_ref(),
        ))
        .unwrap();
        agent_paths::paths().data.join("apps").join(hex::encode(Sha256::digest(scope)))
    }

    async fn serve(&self, database: GlobalDatabase, oauth: SocketAddr) -> anyhow::Result<Server> {
        let mut runtime = Runtime::default();
        runtime.codex_device_oauth = Some(oauth);
        let router = super::build_router_with_auth(
            vec![],
            agent_apps_auth_read::router_with_runtime(
                Some(OWNER.password),
                OWNER.username,
                None,
                Arc::new(runtime),
            ),
        )
        .layer(Extension(database));
        Server::start(router).await
    }

    fn material(&self, id: &str) -> anyhow::Result<Option<AccountCredential>> {
        CredentialFileStore::new(&self.root())
            .map_err(anyhow::Error::msg)?
            .read_managed_account(
                AuthAccountScope {
                    tenant: &self.tenant,
                    principal: "desktop-local-user",
                    host: HOST,
                    service: SERVICE,
                },
                id,
            )
            .map_err(|code| anyhow::anyhow!("{code:?}"))
    }
}

async fn start(sdk: &GatewayClient) -> anyhow::Result<AuthFlowId> {
    let started = sdk
        .agent_apps_auth_start(HOST, SERVICE, &OWNER, &INPUT)
        .await
        .expect("Codex device start must return the public challenge");
    assert!(!serde_json::to_string(&started)?.contains(DEVICE_ID));
    let AuthStartResponse::Pending { flow_id, verification_url, user_code, .. } = started;
    assert_eq!(verification_url, "https://auth.openai.com/codex/device");
    assert_eq!(user_code.as_deref(), Some(USER_CODE));
    Ok(flow_id)
}

fn assert_http(error: AgentAppsAuthError, expected_status: u16, expected_code: AuthErrorCode) {
    assert!(
        matches!(
            error,
            AgentAppsAuthError::Http { status, code: Some(code), .. }
                if status == expected_status && code == expected_code
        ),
        "expected HTTP {expected_status} {expected_code:?}, got {error:?}"
    );
}

async fn commit(
    database: GlobalDatabase,
    reopen: impl FnOnce() -> anyhow::Result<GlobalDatabase> + Send,
) -> anyhow::Result<()> {
    let fixture = Fixture::new(database.clone())?;
    let oauth = OAuth::default();
    let upstream = oauth.serve().await?;
    let gateway = fixture.serve(database, upstream.address).await?;
    let sdk = gateway.sdk(&fixture.tenant)?;
    let flow = start(&sdk).await?;
    let approved = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let snapshot = sdk.agent_apps_auth_poll(HOST, SERVICE, &OWNER, &flow).await?;
            if !matches!(snapshot, AuthFlow::Pending { .. }) {
                return Ok::<_, AgentAppsAuthError>(snapshot);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await??;
    for secret in [DEVICE_ID, CODE, VERIFIER, REFRESH, &OAuth::access_token()] {
        assert!(!serde_json::to_string(&approved)?.contains(secret));
    }
    let AuthFlow::Approved { account, .. } = approved else {
        anyhow::bail!("Codex approval did not commit: {approved:?}");
    };
    let id = &account.account_id;
    assert_eq!(account.credential_state, AuthCredentialState::Ready);
    let Some(AccountCredential::CodexAuth(material)) = fixture.material(id)? else {
        anyhow::bail!("approved account has no managed Codex auth file");
    };
    assert!(material["tokens"]["refresh_token"] == REFRESH);
    assert!(material["tokens"]["access_token"] == OAuth::access_token());
    sdk.agent_apps_auth_bind(HOST, SERVICE, &OWNER, PROVIDER, Some(id)).await?;
    sdk.agent_apps_auth_set_default(HOST, SERVICE, &OWNER, id).await?;
    oauth.assert_requests(&["start", "poll", "exchange"]);
    gateway.stop().await;

    let gateway = fixture.serve(run_sync_database(reopen)?, upstream.address).await?;
    let sdk = gateway.sdk(&fixture.tenant)?;
    let accounts = sdk.agent_apps_auth_accounts(HOST, SERVICE, &OWNER).await?;
    let mut expected = account.clone();
    expected.is_default = true;
    assert_eq!(accounts, [expected]);
    assert_eq!(
        sdk.agent_apps_auth_status(HOST, SERVICE, &OWNER).await?.default_account_id.as_ref(),
        Some(id)
    );
    let bindings = sdk.agent_apps_auth_bindings(HOST, SERVICE, Some(id), &OWNER).await?;
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].provider_id, PROVIDER);
    assert_eq!(&bindings[0].account_id, id);
    assert_http(
        sdk.agent_apps_auth_poll(HOST, SERVICE, &OWNER, &flow).await.unwrap_err(),
        404,
        AuthErrorCode::NotFound,
    );
    assert_http(
        sdk.agent_apps_auth_delete(HOST, SERVICE, &OWNER, id).await.unwrap_err(),
        409,
        AuthErrorCode::AccountInUse,
    );
    assert!(fixture.material(id)?.is_some(), "bound deletion removed credentials");
    sdk.agent_apps_auth_logout(HOST, SERVICE, &OWNER, AgentAppsAuthLogout::Default).await?;
    let accounts = sdk.agent_apps_auth_accounts(HOST, SERVICE, &OWNER).await?;
    assert_eq!(accounts.len(), 1);
    assert_eq!(&accounts[0].account_id, id);
    assert_eq!(accounts[0].credential_state, AuthCredentialState::SignedOut);
    assert!(!accounts[0].is_default);
    assert!(fixture.material(id)?.is_none(), "logout left the managed credential file");
    assert_eq!(sdk.agent_apps_auth_bindings(HOST, SERVICE, Some(id), &OWNER).await?, bindings);
    sdk.agent_apps_auth_bind(HOST, SERVICE, &OWNER, PROVIDER, None).await?;
    sdk.agent_apps_auth_delete(HOST, SERVICE, &OWNER, id).await?;
    assert!(sdk.agent_apps_auth_accounts(HOST, SERVICE, &OWNER).await?.is_empty());
    assert!(sdk.agent_apps_auth_bindings(HOST, SERVICE, None, &OWNER).await?.is_empty());
    assert!(fixture.material(id)?.is_none());
    oauth.assert_requests(&["start", "poll", "exchange"]);
    gateway.stop().await;
    upstream.stop().await;
    Ok(())
}

async fn reject(database: GlobalDatabase) -> anyhow::Result<()> {
    let fixture = Fixture::new(database.clone())?;
    let oauth = OAuth::default();
    let upstream = oauth.serve().await?;
    let gateway = fixture.serve(database, upstream.address).await?;
    let sdk = gateway.sdk(&fixture.tenant)?;
    assert_http(
        sdk.agent_apps_auth_start("", SERVICE, &OWNER, &INPUT).await.unwrap_err(),
        400,
        AuthErrorCode::InvalidInput,
    );
    oauth.assert_requests(&[]);
    let flow = start(&sdk).await?;
    let sibling = gateway.sdk(&TenantScope::new("p", "other")?)?;
    assert_http(
        sibling.agent_apps_auth_cancel(HOST, SERVICE, &OWNER, &flow).await.unwrap_err(),
        404,
        AuthErrorCode::NotFound,
    );
    sdk.agent_apps_auth_cancel(HOST, SERVICE, &OWNER, &flow).await?;
    // Let the host's device worker cross the upstream polling interval.
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(matches!(
        sdk.agent_apps_auth_poll(HOST, SERVICE, &OWNER, &flow).await?,
        AuthFlow::Cancelled { .. }
    ));
    oauth.assert_requests(&["start"]);
    assert!(sdk.agent_apps_auth_accounts(HOST, SERVICE, &OWNER).await?.is_empty());
    assert!(sdk.agent_apps_auth_bindings(HOST, SERVICE, None, &OWNER).await?.is_empty());
    assert!(!fixture.root().join(".managed-v1").exists());
    assert!(!fixture.home.path().join("remote").exists());
    gateway.stop().await;
    upstream.stop().await;
    Ok(())
}

// Runtime paths are process-global: initialize them in a child before any route runs.
fn isolated(postgres: bool, rejects: bool) -> anyhow::Result<()> {
    const CHILD: &str = "SHIJU_AUTH_INTEGRATION_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let home = tempfile::tempdir()?;
        let fixture = postgres.then(PostgresTestDatabase::from_environment).transpose()?;
        let database = match &fixture {
            Some(fixture) => fixture.database().clone(),
            None => GlobalDatabase::for_home(home.path())?,
        };
        return tokio::runtime::Runtime::new()?.block_on(async {
            if rejects {
                reject(database).await
            } else {
                commit(database, || match &fixture {
                    Some(fixture) => fixture.reopen(),
                    None => GlobalDatabase::for_home(home.path()),
                })
                .await
            }
        });
    }
    let root = tempfile::tempdir()?;
    let name = format!(
        "host_migration_3_tests::agent_apps_auth_integration_tests::auth6_integration_{}{}",
        if rejects { "rejects_invalid_or_stale_input" } else { "commits_expected_scoped_result" },
        if postgres { "_postgres" } else { "" }
    );
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", &name, "--nocapture"])
        .env(CHILD, "1")
        .env("VIBEWINDOW_TEST_ROOT", root.path())
        .status()?;
    anyhow::ensure!(status.success(), "isolated integration test failed: {status}");
    Ok(())
}

#[test]
fn auth6_integration_commits_expected_scoped_result() -> anyhow::Result<()> {
    isolated(false, false)
}
#[test]
fn auth6_integration_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    isolated(true, false)
}
#[test]
fn auth6_integration_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    isolated(false, true)
}
#[test]
fn auth6_integration_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    isolated(true, true)
}
