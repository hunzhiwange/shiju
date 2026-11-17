use super::auth_router as agent_apps_auth_read;
use axum::{
    Extension, Router,
    http::{StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt};
use tokio::{net::TcpListener, task::JoinHandle};
use vw_agent_apps::auth::credential_file::{AccountCredential, CredentialFileStore};
use {
    agent_app_services::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    agent_database::{GlobalDatabase, test_support::PostgresTestDatabase},
    app_provider_services::AppProviderStore,
};
use {
    vw_api_app_auth::AuthService, vw_api_app_identity::AppId, vw_api_transport::TimestampMs,
    vw_api_transport::id::TenantScope,
};

const OWNER: &str = "fixture-owner-token-aaaaaaaaaaaa";
const OTHER: &str = "fixture-other-token-bbbbbbbbbbbb";
const READ_ONLY: &str = "fixture-account-reader-dddddddddddd";
const NO_READ: &str = "fixture-legacy-token-cccccccccccc";
const BASIC: &str = "Basic dGVzdDpzZWNyZXQ=";
const QUERY: &str = "host_id=desktop-local&service=codex_oauth";

fn isolated_test() -> anyhow::Result<bool> {
    const CHILD: &str = "SHIJU_AUTH_READ_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return Ok(false);
    }
    let home = tempfile::tempdir()?;
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", std::thread::current().name().unwrap(), "--nocapture"])
        .env(CHILD, "1")
        .env("VIBEWINDOW_TEST_ROOT", home.path().canonicalize()?)
        .status()?;
    anyhow::ensure!(status.success(), "isolated auth read child failed: {status}");
    Ok(true)
}

struct Fixture {
    database: GlobalDatabase,
}

impl Fixture {
    fn new(database: GlobalDatabase) -> anyhow::Result<Self> {
        let root = local_app_auth::auth_policy_path(&agent_paths::paths().home, "p", "c")
            .with_file_name("auth-credentials");
        fs::create_dir_all(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let accounts =
            AuthAccountStore::new(database.clone(), &root).map_err(anyhow::Error::msg)?;
        let tenant = TenantScope::new("p", "c")?;
        let neighbor = TenantScope::new("p", "different")?;
        let scope = AuthAccountScope {
            tenant: &tenant,
            principal: "desktop-local-user",
            host: "desktop-local",
            service: AuthService::CodexOauth,
        };
        let credential = AccountCredential::CodexAuth(json!({
            "auth_mode":"chatgpt",
            "tokens":{"access_token":"fixture-private-access","refresh_token":"fixture-private-refresh","account_id":"private-workspace",
                "id_token":format!("{}.{}.signature", URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#), URL_SAFE_NO_PAD.encode(br#"{"sub":"subject","chatgpt_account_id":"private-workspace"}"#))}
        }));
        for (scope, id) in [
            (scope, "managed"),
            (AuthAccountScope { principal: "other", ..scope }, "other-account"),
            (AuthAccountScope { tenant: &neighbor, ..scope }, "foreign-company"),
            (AuthAccountScope { host: "foreign-host", ..scope }, "foreign-host-account"),
        ] {
            let owner = CredentialFileStore::new(&root)
                .and_then(|s| s.for_owner(scope.principal))
                .map_err(anyhow::Error::msg)?;
            owner
                .lease(scope.tenant, scope.host, AppId::Codex, id, true)
                .and_then(|lease| lease.save(&credential))
                .map_err(anyhow::Error::msg)?;
            let metadata =
                AuthAccountMetadata::new(r#"["private-workspace","subject"]"#, TimestampMs(1))
                    .unwrap();
            accounts.insert_metadata(&scope, id, &metadata).unwrap();
        }

        accounts
            .insert_metadata(
                &scope,
                "signed-out",
                &AuthAccountMetadata::new("other-identity", TimestampMs(2)).unwrap(),
            )
            .unwrap();
        tokio::runtime::Runtime::new()?
            .block_on(accounts.set_default_account(scope, "managed", None))
            .unwrap();

        let store = AppProviderStore::new(database.clone()).map_err(anyhow::Error::msg)?;
        let profile = vw_agent_apps::providers::codex_catalog::import(
            Some(b"model_provider='openai'\nmodel='fixture-model'\n"),
            Some(b"{}"),
        )
        .unwrap();
        store.codex_save(&tenant, "official", &profile, None, None).unwrap();
        store.bind_codex_account(scope, &root, "official", None, Some("managed")).unwrap();
        Ok(Self { database })
    }

    fn router(&self, password: Option<&'static str>) -> Router {
        super::build_router_with_auth(vec![], agent_apps_auth_read::router(password, "test", None))
            .layer(Extension(self.database.clone()))
            .layer(axum::middleware::from_fn(move |request, next| async move {
                agent_gateway::middleware::basic_auth_middleware_with_credentials(
                    request, next, password, "test",
                )
                .await
            }))
    }
}

struct HttpGateway {
    address: std::net::SocketAddr,
    client: reqwest::Client,
    server: JoinHandle<()>,
}

impl HttpGateway {
    async fn start(router: Router) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = reqwest::Client::builder().no_proxy().build()?;
        Ok(Self { address, client, server })
    }
}

impl Drop for HttpGateway {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn query(
    app: &HttpGateway,
    path: &str,
    query: &str,
    auth: Option<&str>,
    token: Option<&str>,
    company: &str,
) -> anyhow::Result<(StatusCode, Value)> {
    let mut request = app
        .client
        .get(format!("http://{}{path}?{query}", app.address))
        .header("x-platform-id", "p")
        .header("x-company-id", company);
    if let Some(auth) = auth {
        request = request.header(header::AUTHORIZATION, auth);
    }
    let response = request.send().await?;
    let status = response.status();
    if path.starts_with("/v1/agent-apps/auth/") && status != StatusCode::UNAUTHORIZED {
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    let body = response.bytes().await?;
    Ok((status, serde_json::from_slice(&body).unwrap_or(Value::Null)))
}

fn sqlite() -> anyhow::Result<(tempfile::TempDir, GlobalDatabase)> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    Ok((home, database))
}

#[test]
fn auth6_api_read_commits_expected_scoped_result() -> anyhow::Result<()> {
    if isolated_test()? {
        return Ok(());
    }
    let (_home, db) = sqlite()?;
    tokio::runtime::Runtime::new()?.block_on(check_success(Fixture::new(db)?))
}

#[test]
fn auth6_api_read_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    if isolated_test()? {
        return Ok(());
    }
    let (_home, db) = sqlite()?;
    tokio::runtime::Runtime::new()?.block_on(check_denials(Fixture::new(db)?))
}

#[test]
fn auth6_api_read_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    if isolated_test()? {
        return Ok(());
    }
    let db = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(check_success(Fixture::new(db.database().clone())?))
}

#[test]
fn auth6_api_read_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    if isolated_test()? {
        return Ok(());
    }
    let db = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(check_denials(Fixture::new(db.database().clone())?))
}

async fn check_success(fixture: Fixture) -> anyhow::Result<()> {
    let app = HttpGateway::start(fixture.router(Some("secret"))).await?;
    let (status, body) =
        query(&app, "/v1/agent-apps/auth/accounts", QUERY, Some(BASIC), Some(OWNER), "c").await?;
    assert_eq!(status, StatusCode::OK, "scoped accounts route");
    assert_eq!(
        body,
        json!({"accounts":[{"account_id":"managed","service":"codex_oauth","label":"ChatGPT","credential_state":"ready","is_default":true,"authenticated_at":1},
                   {"account_id":"signed-out","service":"codex_oauth","label":"ChatGPT","credential_state":"signed_out","is_default":false,"authenticated_at":2}]})
    );
    let (status, body) =
        query(&app, "/v1/agent-apps/auth/status", QUERY, Some(BASIC), Some(OWNER), "c").await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"service":"codex_oauth","default_account_id":"managed","capabilities":{"login_methods":[],"can_set_default":true,"can_bind":false,"can_revoke":false}})
    );
    let (status, body) = query(
        &app,
        "/v1/agent-apps/auth/bindings",
        &format!("{QUERY}&account_id=managed"),
        Some(BASIC),
        Some(OWNER),
        "c",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"bindings":[{"provider_id":"official","service":"codex_oauth","account_id":"managed"}]})
    );
    let (status, body) =
        query(&app, "/v1/agent-apps/auth/bindings", QUERY, Some(BASIC), Some(OTHER), "c").await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"bindings":[{"provider_id":"official","service":"codex_oauth","account_id":"managed"}]}),
        "legacy tokens cannot switch the gateway identity"
    );
    Ok(())
}

async fn check_denials(fixture: Fixture) -> anyhow::Result<()> {
    let app = HttpGateway::start(fixture.router(Some("secret"))).await?;
    let accounts = "/v1/agent-apps/auth/accounts";
    // Gateway credentials authorize the request; legacy app tokens do not.
    for token in [None, Some(NO_READ), Some(READ_ONLY), Some(OTHER)] {
        let (status, body) = query(&app, accounts, QUERY, Some(BASIC), token, "c").await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["accounts"].as_array().unwrap().len(), 2);
    }
    for auth in [None, Some("Bearer skey"), Some("Basic dGVzdDp3cm9uZw==")] {
        assert_eq!(
            query(&app, accounts, QUERY, auth, Some(OWNER), "c").await?.0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        query(
            &HttpGateway::start(fixture.router(None)).await?,
            accounts,
            QUERY,
            Some(BASIC),
            Some(OWNER),
            "c"
        )
        .await?
        .0,
        StatusCode::OK,
        "Gateway without a configured password uses its development mode"
    );
    let (status, body) =
        query(&app, accounts, QUERY, Some(BASIC), Some(OWNER), "different").await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"accounts":[{"account_id":"foreign-company","service":"codex_oauth","label":"ChatGPT","credential_state":"signed_out","is_default":false,"authenticated_at":1}]}),
        "only the requested tenant metadata is visible; its separate credential root has no credential"
    );
    for query_text in ["host_id=..&service=codex_oauth", "host_id=desktop-local&service=unknown"] {
        assert_eq!(
            query(&app, accounts, query_text, Some(BASIC), Some(OWNER), "c").await?.0,
            StatusCode::BAD_REQUEST
        );
    }
    for id in ["other-account", "foreign-company", "foreign-host-account", "missing"] {
        let (status, body) = query(
            &app,
            "/v1/agent-apps/auth/bindings",
            &format!("{QUERY}&account_id={id}"),
            Some(BASIC),
            Some(OWNER),
            "c",
        )
        .await?;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }
    let (status, body) = query(
        &app,
        accounts,
        &format!("{QUERY}&path=fixture-private-path"),
        Some(BASIC),
        Some(OWNER),
        "c",
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "invalid_input");
    Ok(())
}
