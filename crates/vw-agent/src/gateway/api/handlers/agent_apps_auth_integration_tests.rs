use super::{agent_apps_auth_read, agent_apps_auth_start::Runtime};
use crate::{
    agent_apps::auth_accounts::AuthAccountScope,
    app_providers::AppProviderStore,
    global_database::{GlobalDatabase, run_sync_database, test_support::PostgresTestDatabase},
};
use axum::{Extension, Form, Json, Router, extract::State, routing::post};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    net::SocketAddr,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{net::TcpListener, task::JoinHandle};
use vw_agent_apps::{
    auth::credential_file::{AccountCredential, CredentialFileStore},
    providers::codex_catalog,
};
use vw_api_types::{
    agent_apps_auth::{
        AuthCredentialState, AuthErrorCode, AuthFlow, AuthFlowId, AuthLoginMethod, AuthService,
        AuthStartRequest, AuthStartResponse,
    },
    id::TenantScope,
};
use vw_gateway_client::{
    AgentAppsAuthCredentials, AgentAppsAuthError, AgentAppsAuthLogout, GatewayClient,
    GatewayEndpoint,
};

const HOST: &str = "host";
const SERVICE: AuthService = AuthService::CodexOauth;
const PROVIDER: &str = "official";
const OWNER: AgentAppsAuthCredentials<'_> = AgentAppsAuthCredentials {
    username: "test",
    password: "secret",
    principal_token: "fixture-auth56-owner-token-aaaaaaaa",
};
const OTHER: AgentAppsAuthCredentials<'_> =
    AgentAppsAuthCredentials { principal_token: "fixture-auth56-other-token-bbbbbbbb", ..OWNER };
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
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let grant = |host| {
            json!({
                "platform_id":"p","company_id":"c","host_id":host,
                "provider_config_read":true,"provider_config_write":true,
                "auth_account_read":[SERVICE],"auth_flow_start":[SERVICE],
                "auth_binding_write":[SERVICE],"auth_default_write":[SERVICE],
                "auth_account_delete":[SERVICE],"auth_logout":[SERVICE]
            })
        };
        let principal = |id, token: &str, grants| json!({"id":id,"token_sha256":hex::encode(Sha256::digest(token)),"grants":grants});
        let host = |id, binding, root: &Path| {
            json!({"id":id,"binding":binding,"online":true,"home":home,"applications":home,
                "search_paths":[],"credential_root":root})
        };
        let policy = home.join("policy.json");
        fs::write(
            &policy,
            serde_json::to_vec(&json!({
                "principals":[
                    principal("owner", OWNER.principal_token, vec![grant(HOST), grant("remote")]),
                    principal("other", OTHER.principal_token, vec![grant(HOST)])
                ],
                "hosts":[host(HOST,"local",&root),host("remote","remote",&home.join("remote"))]
            }))?,
        )?;
        fs::set_permissions(policy, fs::Permissions::from_mode(0o600))?;
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
        self.home.path().join("private")
    }

    async fn serve(&self, database: GlobalDatabase, oauth: SocketAddr) -> anyhow::Result<Server> {
        let mut runtime = Runtime::default();
        runtime.codex_device_oauth = Some(oauth);
        let router = crate::gateway::router::build_router_with_auth(
            vec![],
            agent_apps_auth_read::router_with_runtime(
                Some(OWNER.password),
                OWNER.username,
                Some(self.home.path().join("policy.json")),
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
                    principal: "owner",
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
        sdk.agent_apps_auth_start("remote", SERVICE, &OWNER, &INPUT).await.unwrap_err(),
        501,
        AuthErrorCode::ExecutionChannelUnavailable,
    );
    oauth.assert_requests(&[]);
    let flow = start(&sdk).await?;
    assert_http(
        sdk.agent_apps_auth_cancel(HOST, SERVICE, &OTHER, &flow).await.unwrap_err(),
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
    assert!(fs::read_dir(fixture.root())?.next().is_none());
    assert!(!fixture.home.path().join("remote").exists());
    gateway.stop().await;
    upstream.stop().await;
    Ok(())
}

#[test]
fn auth6_integration_commits_expected_scoped_result() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?
        .block_on(commit(database, || GlobalDatabase::for_home(home.path())))
}

#[test]
fn auth6_integration_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    let database = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?
        .block_on(commit(database.database().clone(), || database.reopen()))
}

#[test]
fn auth6_integration_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?.block_on(reject(database))
}

#[test]
fn auth6_integration_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    let database = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(reject(database.database().clone()))
}
