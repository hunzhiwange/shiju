use crate::host_migration_3_tests::auth_router as agent_apps_auth_read;
use axum::{Extension, http::StatusCode};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Method, Response};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
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

const OWNER: &str = "fixture-remove-owner-token-aaaaaaaa";
const READER: &str = "fixture-remove-reader-token-bbbbbbb";
const ROOT: &str = "/v1/agent-apps/auth";
const SCOPE: &str = "host_id=desktop-local&service=codex_oauth";

fn isolated_test() -> anyhow::Result<bool> {
    const CHILD: &str = "SHIJU_AUTH_REMOVE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return Ok(false);
    }
    let home = tempfile::tempdir()?;
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", std::thread::current().name().unwrap(), "--nocapture"])
        .env(CHILD, "1")
        .env("VIBEWINDOW_TEST_ROOT", home.path().canonicalize()?)
        .status()?;
    anyhow::ensure!(status.success(), "isolated auth removal child failed: {status}");
    Ok(true)
}

fn credential_root(company: &str) -> anyhow::Result<PathBuf> {
    let root = local_app_auth::auth_policy_path(&agent_paths::paths().home, "p", company)
        .with_file_name("auth-credentials");
    fs::create_dir_all(&root)?;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    Ok(root)
}

struct Fixture {
    db: GlobalDatabase,
    root: PathBuf,
}

impl Fixture {
    fn new(db: GlobalDatabase) -> anyhow::Result<Self> {
        let root = credential_root("c")?;
        let tenant = TenantScope::new("p", "c")?;
        let neighbor = TenantScope::new("p", "neighbor")?;
        let scope = AuthAccountScope {
            tenant: &tenant,
            principal: "desktop-local-user",
            host: "desktop-local",
            service: AuthService::CodexOauth,
        };
        let accounts = AuthAccountStore::new(db.clone(), &root).map_err(anyhow::Error::msg)?;
        let credential = AccountCredential::CodexAuth(json!({
            "auth_mode":"chatgpt",
            "tokens":{
                "access_token":"fixture-private-access", "refresh_token":"fixture-private-refresh",
                "account_id":"workspace",
                "id_token":format!("{}.{}.signature",
                    URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#),
                    URL_SAFE_NO_PAD.encode(br#"{"sub":"subject","chatgpt_account_id":"workspace"}"#))
            }
        }));
        for (scope, id) in [
            (scope, "managed"),
            (AuthAccountScope { tenant: &neighbor, ..scope }, "managed"),
            (AuthAccountScope { tenant: &neighbor, ..scope }, "foreign-company"),
            (AuthAccountScope { principal: "reader", ..scope }, "foreign-owner"),
            (AuthAccountScope { host: "foreign-host", ..scope }, "foreign-host"),
        ] {
            let credentials =
                CredentialFileStore::new(&credential_root(scope.tenant.company_id().as_ref())?)
                    .map_err(anyhow::Error::msg)?;
            credentials
                .for_owner(scope.principal)
                .and_then(|owner| owner.lease(scope.tenant, scope.host, AppId::Codex, id, true))
                .and_then(|lease| lease.save(&credential))
                .map_err(anyhow::Error::msg)?;
            accounts
                .insert_metadata(
                    &scope,
                    id,
                    &AuthAccountMetadata::new(r#"["workspace","subject"]"#, TimestampMs(1))
                        .unwrap(),
                )
                .unwrap();
        }
        let providers = AppProviderStore::new(db.clone()).map_err(anyhow::Error::msg)?;
        let profile = vw_agent_apps::providers::codex_catalog::import(
            Some(b"model_provider='openai'\nmodel='fixture-model'\n"),
            Some(b"{}"),
        )
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        providers
            .codex_save(&tenant, "official", &profile, None, None)
            .map_err(anyhow::Error::msg)?;
        providers
            .bind_codex_account(scope, &root, "official", None, Some("managed"))
            .map_err(|code| anyhow::anyhow!("{code:?}"))?;
        Ok(Self { db, root })
    }

    async fn serve(&self) -> anyhow::Result<Gateway> {
        let router = crate::host_migration_3_tests::build_router_with_auth(
            vec![],
            agent_apps_auth_read::router(Some("secret"), "test", None),
        )
        .layer(Extension(self.db.clone()))
        .layer(axum::middleware::from_fn(|request, next| async move {
            agent_gateway::middleware::basic_auth_middleware_with_credentials(
                request,
                next,
                Some("secret"),
                "test",
            )
            .await
        }));
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let url = format!("http://{}", listener.local_addr()?);
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Ok(Gateway { url, client: reqwest::Client::builder().no_proxy().build()?, server })
    }

    fn stored_default(&self, company: &str) -> anyhow::Result<Option<String>> {
        agent_database::run_sync_database(|| {
            let tenant = TenantScope::new("p", company)?;
            let store =
                AuthAccountStore::new(self.db.clone(), &self.root).map_err(anyhow::Error::msg)?;
            store
                .managed_default_account(AuthAccountScope {
                    tenant: &tenant,
                    principal: "desktop-local-user",
                    host: "desktop-local",
                    service: AuthService::CodexOauth,
                })
                .map_err(|error| anyhow::anyhow!("{error:?}"))
        })
    }

    fn has_credential(&self, company: &str) -> anyhow::Result<bool> {
        let tenant = TenantScope::new("p", company)?;
        let lease = CredentialFileStore::new(&credential_root(company)?)
            .and_then(|store| store.for_owner("desktop-local-user"))
            .and_then(|owner| owner.lease(&tenant, "desktop-local", AppId::Codex, "managed", false))
            .map_err(anyhow::Error::msg)?;
        Ok(lease.read().is_ok())
    }
}

struct Gateway {
    url: String,
    client: reqwest::Client,
    server: JoinHandle<()>,
}

impl Drop for Gateway {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl Gateway {
    fn request(&self, method: Method, path: &str, company: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{path}", self.url))
            .header("x-platform-id", "p")
            .header("x-company-id", company)
    }

    async fn delete(
        &self,
        id: &str,
        scope: &str,
        company: &str,
        token: &str,
    ) -> anyhow::Result<Response> {
        let response = self
            .request(Method::DELETE, &format!("{ROOT}/accounts/{id}?{scope}"), company)
            .basic_auth("test", Some(if token == READER { "invalid" } else { "secret" }))
            .send()
            .await?;
        if response.status() != StatusCode::UNAUTHORIZED {
            assert_eq!(response.headers()["cache-control"], "no-store");
        }
        Ok(response)
    }

    async fn read(&self, resource: &str, company: &str) -> anyhow::Result<Value> {
        Ok(self
            .request(Method::GET, &format!("{ROOT}/{resource}?{SCOPE}"), company)
            .basic_auth("test", Some("secret"))
            .header("x-agent-apps-token", OWNER)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

async fn assert_error(response: Response, status: u16, code: &str) -> anyhow::Result<Value> {
    assert_eq!(response.status().as_u16(), status, "expected {code}");
    let mut body: Value = response.json().await?;
    let fields = body.as_object_mut().unwrap();
    assert!(fields.remove("correlation_id").unwrap().is_string());
    assert_eq!(fields.len(), 2);
    assert_eq!(body["code"], code);
    assert!(!body.to_string().contains("fixture-private"));
    Ok(body)
}

async fn commits(fixture: Fixture) -> anyhow::Result<()> {
    let gateway = fixture.serve().await?;
    for company in ["c", "neighbor"] {
        let response = gateway
            .request(Method::PUT, &format!("{ROOT}/accounts/managed/default?{SCOPE}"), company)
            .basic_auth("test", Some("secret"))
            .header("x-agent-apps-token", OWNER)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
    let response = gateway.delete("managed", SCOPE, "c", OWNER).await?;
    let body = assert_error(response, 409, "account_in_use").await?;
    assert!(!body.to_string().contains("official"));
    assert_eq!(gateway.read("status", "c").await?["default_account_id"], "managed");
    assert!(fixture.has_credential("c")?);

    let response = gateway
        .request(Method::PUT, &format!("{ROOT}/bindings/official?{SCOPE}"), "c")
        .basic_auth("test", Some("secret"))
        .header("x-agent-apps-token", OWNER)
        .json(&json!({"account_id":null}))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = gateway.delete("managed", SCOPE, "c", OWNER).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.bytes().await?.is_empty());
    assert!(gateway.read("status", "c").await?["default_account_id"].is_null());
    assert_eq!(fixture.stored_default("c")?, None);
    assert_eq!(fixture.stored_default("neighbor")?.as_deref(), Some("managed"));
    assert_eq!(gateway.read("accounts", "c").await?["accounts"], json!([]));
    assert!(!fixture.has_credential("c")?);
    assert_eq!(gateway.read("status", "neighbor").await?["default_account_id"], "managed");
    assert!(fixture.has_credential("neighbor")?);
    assert_error(gateway.delete("managed", SCOPE, "c", OWNER).await?, 404, "not_found").await?;
    Ok(())
}

async fn rejects(fixture: Fixture) -> anyhow::Result<()> {
    let gateway = fixture.serve().await?;
    assert_error(gateway.delete("%2E%2E%2Fbad", SCOPE, "c", OWNER).await?, 400, "invalid_input")
        .await?;
    let absent =
        assert_error(gateway.delete("missing", SCOPE, "c", OWNER).await?, 404, "not_found").await?;
    for id in ["foreign-company", "foreign-host", "foreign-owner"] {
        let response = gateway.delete(id, SCOPE, "c", OWNER).await?;
        assert_eq!(assert_error(response, 404, "not_found").await?, absent);
    }
    // Gateway credentials enforce access; removed application grants are not authority.
    assert_eq!(
        gateway.delete("managed", SCOPE, "c", READER).await?.status(),
        StatusCode::UNAUTHORIZED
    );
    // A caller-selected host must never expose a foreign host's account.
    for host in ["remote", "offline"] {
        assert_error(
            gateway
                .delete("foreign-host", &format!("host_id={host}&service=codex_oauth"), "c", OWNER)
                .await?,
            404,
            "not_found",
        )
        .await?;
    }
    let response = gateway
        .request(Method::DELETE, &format!("{ROOT}/accounts/managed?{SCOPE}"), "c")
        .header("x-agent-apps-token", OWNER)
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    for prefix in ["/agent-apps/auth", "/api/agent-apps/auth"] {
        let response = gateway
            .request(Method::DELETE, &format!("{prefix}/accounts/managed?{SCOPE}"), "c")
            .basic_auth("test", Some("secret"))
            .header("x-agent-apps-token", OWNER)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    assert!(fixture.has_credential("c")?);
    assert!(fixture.has_credential("neighbor")?);
    Ok(())
}

#[test]
fn auth6_api_remove_commits_expected_scoped_result() -> anyhow::Result<()> {
    if isolated_test()? {
        return Ok(());
    }
    let home = tempfile::tempdir()?;
    let fixture = Fixture::new(GlobalDatabase::for_home(home.path())?)?;
    tokio::runtime::Runtime::new()?.block_on(commits(fixture))
}

#[test]
fn auth6_api_remove_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    if isolated_test()? {
        return Ok(());
    }
    let home = tempfile::tempdir()?;
    let fixture = Fixture::new(GlobalDatabase::for_home(home.path())?)?;
    tokio::runtime::Runtime::new()?.block_on(rejects(fixture))
}

#[test]
fn auth6_api_remove_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    if isolated_test()? {
        return Ok(());
    }
    let db = PostgresTestDatabase::from_environment()?;
    let fixture = Fixture::new(db.database().clone())?;
    let result = tokio::runtime::Runtime::new()?.block_on(commits(fixture));
    db.cleanup()?;
    result
}

#[test]
fn auth6_api_remove_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    if isolated_test()? {
        return Ok(());
    }
    let db = PostgresTestDatabase::from_environment()?;
    let fixture = Fixture::new(db.database().clone())?;
    let result = tokio::runtime::Runtime::new()?.block_on(rejects(fixture));
    db.cleanup()?;
    result
}
