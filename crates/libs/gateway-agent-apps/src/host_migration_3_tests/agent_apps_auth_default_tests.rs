use super::auth_router as agent_apps_auth_read;
use axum::{Extension, http::StatusCode};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use tokio::{net::TcpListener, task::JoinHandle};
use vw_agent_apps::auth::credential_file::{
    AccountCredential, CredentialFileStore, KimiCredential,
};
use {
    agent_app_services::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    agent_database::{GlobalDatabase, test_support::PostgresTestDatabase},
};
use {
    vw_api_app_auth::AuthRegion, vw_api_app_auth::AuthService, vw_api_transport::TimestampMs,
    vw_api_transport::id::TenantScope,
};

const SKEY: &str = "fixture-default-paired-skey-aaaaaaaa";
const ROOT: &str = "/v1/agent-apps/auth";
const COPILOT: &str = "host_id=desktop-local&service=github_copilot";
const KIMI: &str = "host_id=desktop-local&service=kimi_oauth";
const AGY: &str = "host_id=desktop-local&service=agy_oauth";
const CONFIG: &str = r#"
default_model = "original"
[providers.other]
type = "kimi"
[providers."managed:kimi-code"]
type = "kimi"
base_url = "https://api.kimi.ai/coding/v1"
[providers."managed:kimi-code".oauth]
storage = "file"
key = "oauth/kimi-code-fixture"
oauth_host = "https://auth.kimi.ai"
[models.original]
provider = "other"
model = "other-model"
[models.chosen]
provider = "managed:kimi-code"
model = "kimi-for-coding"
"#;

struct Fixture {
    home: PathBuf,
    db: GlobalDatabase,
}

impl Fixture {
    fn new(db: GlobalDatabase) -> anyhow::Result<Self> {
        let home = agent_paths::paths().home.clone();
        let credential_root = |tenant: &TenantScope| -> anyhow::Result<PathBuf> {
            let root = local_app_auth::auth_policy_path(
                &home,
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
            )
            .with_file_name("auth-credentials");
            fs::create_dir_all(&root)?;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
            Ok(root)
        };
        let tenant = TenantScope::new("p", "c")?;
        let neighbor = TenantScope::new("p", "neighbor")?;
        let root = credential_root(&tenant)?;
        let store = AuthAccountStore::new(db.clone(), &root).map_err(anyhow::Error::msg)?;
        let scope = AuthAccountScope {
            tenant: &tenant,
            principal: "desktop-local-user",
            host: local_app_auth::HOST_ID,
            service: AuthService::GithubCopilot,
        };
        for (scope, id) in [
            (scope, "first"),
            (scope, "second"),
            (AuthAccountScope { tenant: &neighbor, ..scope }, "foreign-company"),
            (AuthAccountScope { principal: "reader", ..scope }, "foreign-owner"),
            (AuthAccountScope { host: "foreign-host", ..scope }, "foreign-host"),
        ] {
            CredentialFileStore::new(&credential_root(scope.tenant)?)
                .map_err(anyhow::Error::msg)?
                .lease_copilot(scope, id, true)
                .and_then(|lease| {
                    lease.save(&AccountCredential::CopilotToken("fixture-private".into()))
                })
                .map_err(anyhow::Error::msg)?;
            store
                .insert_metadata(&scope, id, &AuthAccountMetadata::new(id, TimestampMs(1)).unwrap())
                .unwrap();
        }

        let kimi = AuthAccountScope { service: AuthService::KimiOauth, ..scope };
        let mut metadata = AuthAccountMetadata::new("kimi-fixture", TimestampMs(1)).unwrap();
        metadata.region = Some(AuthRegion::Global);
        store.insert_metadata(&kimi, "kimi-account", &metadata).unwrap();
        CredentialFileStore::new(&root)
            .map_err(anyhow::Error::msg)?
            .lease_kimi(kimi, "kimi-account", true)
            .and_then(|lease| {
                lease.save(&AccountCredential::KimiOauth(KimiCredential {
                    access_token: "fixture-private-access".into(),
                    refresh_token: "fixture-private-refresh".into(),
                    expires_at: TimestampMs(4_102_444_800_000),
                    device_id: "d8c8a37b-175c-4e59-9501-07351b0da278".into(),
                    region: AuthRegion::Global,
                }))
            })
            .map_err(anyhow::Error::msg)?;
        let native = home.join(".kimi-code");
        fs::create_dir_all(native.join("credentials"))?;
        fs::create_dir_all(native.join("oauth"))?;
        fs::write(native.join("config.toml"), CONFIG)?;
        Ok(Self { home, db })
    }

    async fn serve(&self) -> anyhow::Result<Gateway> {
        let state = agent_gateway::gateway_test_fixture::state::<(), (), ()>(
            Default::default(),
            true,
            &[SKEY.to_owned()],
        );
        let router =
            super::build_router_with_auth(vec![], agent_apps_auth_read::router(None, "test", None))
                .layer(Extension(self.db.clone()))
                .layer(axum::middleware::from_fn_with_state(
                    state,
                    agent_gateway::request_policy::skey_auth_middleware::<(), (), ()>,
                ));
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let url = format!("http://{}", listener.local_addr()?);
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Ok(Gateway { url, client: reqwest::Client::builder().no_proxy().build()?, server })
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
    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        company: &str,
    ) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{path}", self.url))
            .header("x-platform-id", "p")
            .header("x-company-id", company)
    }

    async fn put(
        &self,
        id: &str,
        query: &str,
        company: &str,
        token: &str,
    ) -> anyhow::Result<reqwest::Response> {
        let response = self
            .request(
                reqwest::Method::PUT,
                &format!("{ROOT}/accounts/{id}/default?{query}"),
                company,
            )
            .bearer_auth(token)
            .send()
            .await?;
        if response.status() != StatusCode::UNAUTHORIZED {
            assert_eq!(response.headers()["cache-control"], "no-store");
        }
        Ok(response)
    }

    async fn status(&self, company: &str, token: &str) -> anyhow::Result<Value> {
        Ok(self
            .request(reqwest::Method::GET, &format!("{ROOT}/status?{COPILOT}"), company)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

async fn commits(fixture: Fixture) -> anyhow::Result<()> {
    let gateway = fixture.serve().await?;
    for (company, id) in
        [("neighbor", "foreign-company"), ("c", "first"), ("c", "second"), ("c", "second")]
    {
        let response = gateway.put(id, COPILOT, company, SKEY).await?;
        assert_eq!(response.status(), StatusCode::NO_CONTENT, "default switch for {company}/{id}");
        assert!(response.bytes().await?.is_empty());
        let status = gateway.status(company, SKEY).await?;
        assert_eq!(status["default_account_id"], id);
        assert_eq!(status["capabilities"]["can_set_default"], true);
    }
    assert_eq!(gateway.status("neighbor", SKEY).await?["default_account_id"], "foreign-company");
    let outsider = gateway.status("other", SKEY).await?;
    assert!(outsider["default_account_id"].is_null());
    Ok(())
}

async fn rejects(fixture: Fixture) -> anyhow::Result<()> {
    let gateway = fixture.serve().await?;
    for (id, query, company, token, status, code) in [
        ("%2E%2E%2Fbad", COPILOT, "c", SKEY, 400, "invalid_input"),
        ("foreign-company", COPILOT, "c", SKEY, 404, "not_found"),
        ("foreign-owner", COPILOT, "c", SKEY, 404, "not_found"),
        ("foreign-host", COPILOT, "c", SKEY, 404, "not_found"),
        ("random", COPILOT, "c", SKEY, 404, "not_found"),
        ("first", "host_id=desktop-local&service=claude_official", "c", SKEY, 501, "unsupported"),
        ("first", "host_id=desktop-local&service=dsh_account", "c", SKEY, 501, "unsupported"),
        ("first", "host_id=%2Fbad&service=github_copilot", "c", SKEY, 400, "invalid_input"),
        ("short", AGY, "c", SKEY, 400, "invalid_input"),
        ("kimi-account", KIMI, "neighbor", SKEY, 404, "not_found"),
        ("kimi-account", KIMI, "c", SKEY, 409, "reauth_required"),
    ] {
        let response = gateway.put(id, query, company, token).await?;
        assert_eq!(response.status().as_u16(), status, "{id} {query} should return {code}");
        let body: Value = response.json().await?;
        assert_eq!(body["code"], code);
        assert!(body["correlation_id"].is_string());
        assert!(!body.to_string().contains("fixture-private"));
    }
    assert_eq!(fs::read_to_string(fixture.home.join(".kimi-code/config.toml"))?, CONFIG);

    // The isolated execution home has no native keychain directory.
    let absent = format!("fixture-{}abc", uuid::Uuid::new_v4().simple());
    let response = gateway.put(&absent, AGY, "c", SKEY).await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.json::<Value>().await?["code"], "keychain_unavailable");

    let response = gateway.put("first", COPILOT, "c", SKEY).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = gateway.put("foreign-company", COPILOT, "c", SKEY).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(gateway.status("c", SKEY).await?["default_account_id"], "first");
    for path in
        ["/agent-apps/auth/accounts/first/default", "/api/agent-apps/auth/accounts/first/default"]
    {
        let response = gateway
            .request(reqwest::Method::PUT, &format!("{path}?{COPILOT}"), "c")
            .bearer_auth(SKEY)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let response = gateway.put("second", COPILOT, "c", "unpaired-key").await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = gateway
        .request(reqwest::Method::PUT, &format!("{ROOT}/accounts/second/default?{COPILOT}"), "c")
        .header("x-agent-apps-token", SKEY)
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(gateway.status("c", SKEY).await?["default_account_id"], "first");
    Ok(())
}

fn run_isolated(postgres: bool, commit: bool) -> anyhow::Result<()> {
    const CHILD_ROOT: &str = "SHIJU_AUTH_DEFAULT_CHILD_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let postgres = postgres.then(PostgresTestDatabase::from_environment).transpose()?;
        let database = match &postgres {
            Some(db) => db.database().clone(),
            None => GlobalDatabase::for_home(Path::new(&root))?,
        };
        let fixture = Fixture::new(database)?;
        tokio::runtime::Runtime::new()?.block_on(async {
            if commit { commits(fixture).await } else { rejects(fixture).await }
        })?;
        return postgres.map_or(Ok(()), PostgresTestDatabase::cleanup);
    }
    let home = tempfile::tempdir()?;
    let root = home.path().canonicalize()?;
    let thread = std::thread::current();
    let name = thread.name().expect("named test");
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", name, "--nocapture"])
        .env(CHILD_ROOT, &root)
        .env("VIBEWINDOW_TEST_ROOT", &root)
        .env("HOME", &root)
        .status()?;
    anyhow::ensure!(status.success(), "isolated default account test failed: {status}");
    Ok(())
}

#[test]
fn auth6_api_default_commits_expected_scoped_result() -> anyhow::Result<()> {
    run_isolated(false, true)
}

#[test]
fn auth6_api_default_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    run_isolated(false, false)
}

#[test]
fn auth6_api_default_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    run_isolated(true, true)
}

#[test]
fn auth6_api_default_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    run_isolated(true, false)
}
