use super::agent_apps_auth_read;
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    global_database::{GlobalDatabase, test_support::PostgresTestDatabase},
};
use axum::{Extension, http::StatusCode};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use tokio::{net::TcpListener, task::JoinHandle};
use vw_agent_apps::auth::credential_file::{
    AccountCredential, CredentialFileStore, KimiCredential,
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthRegion, AuthService},
    id::TenantScope,
};

const OWNER: &str = "fixture-default-owner-token-aaaaaaaa";
const READER: &str = "fixture-default-reader-token-bbbbbbb";
const ROOT: &str = "/v1/agent-apps/auth";
const COPILOT: &str = "host_id=host&service=github_copilot";
const KIMI: &str = "host_id=host&service=kimi_oauth";
const AGY: &str = "host_id=agy&service=agy_oauth";
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
    home: tempfile::TempDir,
    db: GlobalDatabase,
}

impl Fixture {
    fn new(db: GlobalDatabase) -> anyhow::Result<Self> {
        let home = tempfile::tempdir()?;
        let root = home.path().join("private");
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let services =
            json!(["github_copilot", "claude_official", "dsh_account", "kimi_oauth", "agy_oauth"]);
        let grant = |company, host, write| {
            json!({
                "platform_id": "p", "company_id": company, "host_id": host,
                "auth_account_read": services,
                "auth_default_write": if write { services.clone() } else { json!([]) }
            })
        };
        let principal = |id, token: &str, grants| {
            json!({
                "id": id, "token_sha256": hex::encode(Sha256::digest(token)), "grants": grants
            })
        };
        let host = |id, online, binding| {
            json!({
                "id": id, "online": online, "binding": binding, "home": home.path(),
                "search_paths": [], "applications": home.path(), "credential_root": root
            })
        };
        let mut local = host("host", true, "local");
        local["native_login_owners"] = json!([{"service":"kimi_oauth","platform_id":"p","company_id":"c","principal":"owner"}]);
        let mut agy = host("agy", true, "local");
        agy["home"] = json!(fs::canonicalize(std::env::var_os("HOME").unwrap())?);
        agy["native_login_owners"] =
            json!([{"service":"agy_oauth","platform_id":"p","company_id":"c","principal":"owner"}]);
        let policy = home.path().join("access.json");
        fs::write(
            &policy,
            serde_json::to_vec(&json!({
                "principals": [
                    principal("owner", OWNER, vec![grant("c","host",true), grant("neighbor","host",true),
                        grant("c","remote",true), grant("c","offline",true), grant("c","agy",true)]),
                    principal("reader", READER, vec![grant("c","host",false)])
                ],
                "hosts": [local, agy, host("remote",true,"remote"), host("offline",false,"local")]
            }))?,
        )?;
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600))?;

        let store = AuthAccountStore::new(db.clone(), &root).map_err(anyhow::Error::msg)?;
        let credentials = CredentialFileStore::new(&root).map_err(anyhow::Error::msg)?;
        let tenant = TenantScope::new("p", "c")?;
        let neighbor = TenantScope::new("p", "neighbor")?;
        let scope = AuthAccountScope {
            tenant: &tenant,
            principal: "owner",
            host: "host",
            service: AuthService::GithubCopilot,
        };
        for (scope, id) in [
            (scope, "first"),
            (scope, "second"),
            (AuthAccountScope { tenant: &neighbor, ..scope }, "foreign-company"),
            (AuthAccountScope { principal: "reader", ..scope }, "foreign-owner"),
            (AuthAccountScope { host: "foreign-host", ..scope }, "foreign-host"),
        ] {
            credentials
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
        credentials
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
        let native = home.path().join(".kimi-code");
        fs::create_dir_all(native.join("credentials"))?;
        fs::create_dir_all(native.join("oauth"))?;
        fs::write(native.join("config.toml"), CONFIG)?;
        Ok(Self { home, db })
    }

    async fn serve(&self) -> anyhow::Result<Gateway> {
        let router = crate::gateway::router::build_router_with_auth(
            vec![],
            agent_apps_auth_read::router(
                Some("secret"),
                "test",
                Some(self.home.path().join("access.json")),
            ),
        )
        .layer(Extension(self.db.clone()));
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
            .basic_auth("test", Some("secret"))
            .header("x-agent-apps-token", token)
            .send()
            .await?;
        assert_eq!(response.headers()["cache-control"], "no-store");
        Ok(response)
    }

    async fn status(&self, company: &str, token: &str) -> anyhow::Result<Value> {
        Ok(self
            .request(reqwest::Method::GET, &format!("{ROOT}/status?{COPILOT}"), company)
            .basic_auth("test", Some("secret"))
            .header("x-agent-apps-token", token)
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
        let response = gateway.put(id, COPILOT, company, OWNER).await?;
        assert_eq!(response.status(), StatusCode::NO_CONTENT, "default switch for {company}/{id}");
        assert!(response.bytes().await?.is_empty());
        let status = gateway.status(company, OWNER).await?;
        assert_eq!(status["default_account_id"], id);
        assert_eq!(status["capabilities"]["can_set_default"], true);
    }
    assert_eq!(gateway.status("neighbor", OWNER).await?["default_account_id"], "foreign-company");
    let reader = gateway.status("c", READER).await?;
    assert!(reader["default_account_id"].is_null());
    assert_eq!(reader["capabilities"]["can_set_default"], false);
    Ok(())
}

async fn rejects(fixture: Fixture) -> anyhow::Result<()> {
    let gateway = fixture.serve().await?;
    for (id, query, company, token, status, code) in [
        ("%2E%2E%2Fbad", COPILOT, "c", OWNER, 400, "invalid_input"),
        ("foreign-company", COPILOT, "c", OWNER, 404, "not_found"),
        ("foreign-owner", COPILOT, "c", OWNER, 404, "not_found"),
        ("foreign-host", COPILOT, "c", OWNER, 404, "not_found"),
        ("random", COPILOT, "c", OWNER, 404, "not_found"),
        ("first", COPILOT, "c", READER, 403, "forbidden"),
        ("first", "host_id=host&service=claude_official", "c", OWNER, 501, "unsupported"),
        ("first", "host_id=host&service=dsh_account", "c", OWNER, 501, "unsupported"),
        (
            "first",
            "host_id=remote&service=github_copilot",
            "c",
            OWNER,
            501,
            "execution_channel_unavailable",
        ),
        (
            "first",
            "host_id=offline&service=github_copilot",
            "c",
            OWNER,
            503,
            "execution_host_unavailable",
        ),
        ("kimi-account", KIMI, "neighbor", OWNER, 403, "forbidden"),
        ("kimi-account", KIMI, "c", OWNER, 409, "reauth_required"),
    ] {
        let response = gateway.put(id, query, company, token).await?;
        assert_eq!(response.status().as_u16(), status, "{id} {query} should return {code}");
        let body: Value = response.json().await?;
        assert_eq!(body["code"], code);
        assert!(body["correlation_id"].is_string());
        assert!(!body.to_string().contains("fixture-private"));
    }
    assert_eq!(fs::read_to_string(fixture.home.path().join(".kimi-code/config.toml"))?, CONFIG);

    let absent = format!("fixture-{}abc", uuid::Uuid::new_v4().simple());
    let response = gateway.put(&absent, AGY, "c", OWNER).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.json::<Value>().await?["code"], "not_found");

    let response = gateway.put("first", COPILOT, "c", OWNER).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = gateway.put("foreign-company", COPILOT, "c", OWNER).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(gateway.status("c", OWNER).await?["default_account_id"], "first");
    for path in
        ["/agent-apps/auth/accounts/first/default", "/api/agent-apps/auth/accounts/first/default"]
    {
        let response = gateway
            .request(reqwest::Method::PUT, &format!("{path}?{COPILOT}"), "c")
            .basic_auth("test", Some("secret"))
            .header("x-agent-apps-token", OWNER)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let response = gateway
        .request(reqwest::Method::PUT, &format!("{ROOT}/accounts/first/default?{COPILOT}"), "c")
        .header("x-agent-apps-token", OWNER)
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[test]
fn auth6_api_default_commits_expected_scoped_result() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let fixture = Fixture::new(GlobalDatabase::for_home(home.path())?)?;
    tokio::runtime::Runtime::new()?.block_on(commits(fixture))
}

#[test]
fn auth6_api_default_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let fixture = Fixture::new(GlobalDatabase::for_home(home.path())?)?;
    tokio::runtime::Runtime::new()?.block_on(rejects(fixture))
}

#[test]
fn auth6_api_default_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    let db = PostgresTestDatabase::from_environment()?;
    let fixture = Fixture::new(db.database().clone())?;
    tokio::runtime::Runtime::new()?.block_on(commits(fixture))?;
    db.cleanup()
}

#[test]
fn auth6_api_default_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    let db = PostgresTestDatabase::from_environment()?;
    let fixture = Fixture::new(db.database().clone())?;
    tokio::runtime::Runtime::new()?.block_on(rejects(fixture))?;
    db.cleanup()
}
