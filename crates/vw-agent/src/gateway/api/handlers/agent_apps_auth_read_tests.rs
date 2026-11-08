use super::agent_apps_auth_read;
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    app_providers::AppProviderStore,
    global_database::{GlobalDatabase, test_support::PostgresTestDatabase},
};
use axum::{
    Extension, Router,
    http::{StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use tokio::{net::TcpListener, task::JoinHandle};
use vw_agent_apps::auth::credential_file::{AccountCredential, CredentialFileStore};
use vw_api_types::{TimestampMs, agent_apps::AppId, agent_apps_auth::AuthService, id::TenantScope};

const OWNER: &str = "fixture-owner-token-aaaaaaaaaaaa";
const OTHER: &str = "fixture-other-token-bbbbbbbbbbbb";
const READ_ONLY: &str = "fixture-account-reader-dddddddddddd";
const NO_READ: &str = "fixture-legacy-token-cccccccccccc";
const BASIC: &str = "Basic dGVzdDpzZWNyZXQ=";
const QUERY: &str = "host_id=host&service=codex_oauth";

struct Fixture {
    _home: tempfile::TempDir,
    database: GlobalDatabase,
    policy: std::path::PathBuf,
}

impl Fixture {
    fn new(database: GlobalDatabase) -> anyhow::Result<Self> {
        let home = tempfile::tempdir()?;
        let root = home.path().join("private");
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let policy = home.path().join("access.json");
        let principal = |id, token: &str, read: bool| {
            let mut grant = json!({"platform_id":"p", "company_id":"c", "host_id":"host", "provider_config_read":id != "account-reader"});
            if read {
                grant["auth_account_read"] = json!(["codex_oauth"]);
            }
            json!({"id":id, "token_sha256":hex::encode(Sha256::digest(token)), "grants":[grant]})
        };
        fs::write(
            &policy,
            serde_json::to_vec(&json!({
                "principals":[principal("owner", OWNER, true), principal("other", OTHER, true), principal("legacy", NO_READ, false), principal("account-reader", READ_ONLY, true)],
                "hosts":[{"id":"host", "online":true, "binding":"local", "home":home.path(),
                          "search_paths":[], "applications":home.path(), "credential_root":root}]
            }))?,
        )?;
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600))?;

        let accounts =
            AuthAccountStore::new(database.clone(), &root).map_err(anyhow::Error::msg)?;
        let tenant = TenantScope::new("p", "c")?;
        let neighbor = TenantScope::new("p", "different")?;
        let scope = AuthAccountScope {
            tenant: &tenant,
            principal: "owner",
            host: "host",
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
        Ok(Self { _home: home, database, policy })
    }

    fn router(&self, password: Option<&'static str>) -> Router {
        crate::gateway::router::build_router_with_auth(
            vec![],
            agent_apps_auth_read::router(password, "test", Some(self.policy.clone())),
        )
        .layer(Extension(self.database.clone()))
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
    if let Some(token) = token {
        request = request.header("x-agent-apps-token", token);
    }
    let response = request.send().await?;
    let status = response.status();
    if path.starts_with("/v1/agent-apps/auth/") {
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
    let (_home, db) = sqlite()?;
    tokio::runtime::Runtime::new()?.block_on(check_success(Fixture::new(db)?))
}

#[test]
fn auth6_api_read_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    let (_home, db) = sqlite()?;
    tokio::runtime::Runtime::new()?.block_on(check_denials(Fixture::new(db)?))
}

#[test]
fn auth6_api_read_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    let db = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(check_success(Fixture::new(db.database().clone())?))
}

#[test]
fn auth6_api_read_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
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
        json!({"service":"codex_oauth","default_account_id":"managed","capabilities":{"login_methods":[],"can_set_default":false,"can_bind":false,"can_revoke":false}})
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
    assert_eq!(body, json!({"bindings":[]}));
    Ok(())
}

async fn check_denials(fixture: Fixture) -> anyhow::Result<()> {
    let app = HttpGateway::start(fixture.router(Some("secret"))).await?;
    let accounts = "/v1/agent-apps/auth/accounts";
    assert_eq!(
        query(&app, accounts, QUERY, Some(BASIC), Some(NO_READ), "c").await?.0,
        StatusCode::FORBIDDEN,
        "provider-config permission cannot authorize account reads"
    );
    for (auth, token) in
        [(None, Some(OWNER)), (Some("Bearer skey"), Some(OWNER)), (Some(BASIC), None)]
    {
        assert_eq!(
            query(&app, accounts, QUERY, auth, token, "c").await?.0,
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
        StatusCode::UNAUTHORIZED
    );
    for (query_text, company) in [
        (QUERY, "different"),
        ("host_id=foreign-host&service=codex_oauth", "c"),
        ("host_id=host&service=xai_oauth", "c"),
    ] {
        assert_eq!(
            query(&app, accounts, query_text, Some(BASIC), Some(OWNER), company).await?.0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        query(&app, accounts, QUERY, Some(BASIC), Some(READ_ONLY), "c").await?,
        (StatusCode::OK, json!({"accounts":[]}))
    );
    assert_eq!(
        query(&app, "/v1/agent-apps/auth/bindings", QUERY, Some(BASIC), Some(READ_ONLY), "c")
            .await?
            .0,
        StatusCode::FORBIDDEN
    );
    for id in ["other-account", "missing"] {
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
    let aliases = HttpGateway::start(crate::gateway::router::handler_router()).await?;
    for alias in ["/agent-apps/auth/accounts", "/api/agent-apps/auth/accounts"] {
        assert_eq!(
            query(&aliases, alias, QUERY, Some(BASIC), Some(OWNER), "c").await?.0,
            StatusCode::NOT_FOUND
        );
    }
    Ok(())
}
