use super::agent_apps_auth_read;
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    app_providers::AppProviderStore,
    global_database::{GlobalDatabase, test_support::PostgresTestDatabase},
};
use axum::{Extension, http::StatusCode};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use tokio::{net::TcpListener, task::JoinHandle};
use vw_agent_apps::{
    auth::credential_file::{AccountCredential, CredentialFileStore, KimiCredential},
    providers::kimi,
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{AuthRegion, AuthService},
    id::TenantScope,
};

const OWNER: &str = "fixture-binding-owner-token-aaaaaaaa";
const OTHER: &str = "fixture-binding-other-token-bbbbbbbb";
const EDITOR: &str = "fixture-binding-editor-token-cccccccc";
const READER: &str = "fixture-binding-reader-token-dddddddd";
const BASIC: &str = "Basic dGVzdDpzZWNyZXQ=";
const ROOT: &str = "/v1/agent-apps/auth/bindings";
const PATH: &str = "/v1/agent-apps/auth/bindings/official";
const CODEX: &str = "host_id=host&service=codex_oauth";
const KIMI: &str = "host_id=host&service=kimi_oauth";
const KIMI_PROVIDER: &str = "managed:kimi-code";
const KIMI_PATH: &str = "/v1/agent-apps/auth/bindings/managed:kimi-code";
const EXPIRES: i64 = 4_102_444_800;
const KIMI_CONFIG: &str = r#"
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
max_context_size = 32000
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
        let grant = |company, host, write, bind| {
            json!({"platform_id":"p", "company_id":company, "host_id":host,
                "provider_config_read":true, "provider_config_write":write,
                "auth_account_read":["codex_oauth", "kimi_oauth"],
                "auth_binding_write":if bind { json!(["codex_oauth","kimi_oauth","dsh_account","agy_oauth","github_copilot","xai_oauth","claude_official"]) } else { json!([]) }})
        };
        let principal = |id, token: &str, grants| json!({"id":id, "token_sha256":hex::encode(Sha256::digest(token)), "grants":grants});
        let host = |id, online, binding, root: &Path| {
            json!({"id":id, "online":online, "binding":binding, "home":home.path(),
                "search_paths":[], "applications":home.path(), "credential_root":root})
        };
        let mut local = host("host", true, "local", &root);
        local["native_login_owners"] = json!([
            {"service":"kimi_oauth", "platform_id":"p", "company_id":"c", "principal":"owner"}
        ]);
        let remote_root = home.path().join("remote");
        let offline_root = home.path().join("offline");
        let policy = home.path().join("access.json");
        fs::write(
            &policy,
            serde_json::to_vec(&json!({
                "principals":[
                    principal("owner", OWNER, vec![grant("c","host",true,true), grant("neighbor","host",true,true),
                        grant("c","remote",true,true), grant("c","offline",true,true)]),
                    principal("other", OTHER, vec![grant("c","host",true,true)]),
                    principal("editor", EDITOR, vec![grant("c","host",true,false)]),
                    principal("reader", READER, vec![grant("c","host",false,true)])
                ],
                "hosts":[local, host("remote",true,"remote",&remote_root),
                    host("offline",false,"local",&offline_root)]
            }))?,
        )?;
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600))?;
        let store = AuthAccountStore::new(db.clone(), &root).map_err(anyhow::Error::msg)?;
        let tenant = TenantScope::new("p", "c")?;
        let neighbor = TenantScope::new("p", "neighbor")?;
        let scope = AuthAccountScope {
            tenant: &tenant,
            principal: "owner",
            host: "host",
            service: AuthService::CodexOauth,
        };
        let credential = AccountCredential::CodexAuth(json!({
            "auth_mode":"chatgpt",
            "tokens":{"access_token":"fixture-private-access","refresh_token":"fixture-private-refresh","account_id":"workspace",
                "id_token":format!("{}.{}.signature", URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#), URL_SAFE_NO_PAD.encode(br#"{"sub":"subject","chatgpt_account_id":"workspace"}"#))}
        }));
        for (scope, id) in [
            (scope, "first"),
            (scope, "second"),
            (AuthAccountScope { principal: "other", ..scope }, "foreign-owner"),
            (AuthAccountScope { tenant: &neighbor, ..scope }, "foreign-company"),
            (AuthAccountScope { host: "other-host", ..scope }, "foreign-host"),
        ] {
            CredentialFileStore::new(&root)
                .and_then(|store| store.for_owner(scope.principal))
                .and_then(|owner| owner.lease(scope.tenant, scope.host, AppId::Codex, id, true))
                .and_then(|lease| lease.save(&credential))
                .map_err(anyhow::Error::msg)?;
            store
                .insert_metadata(
                    &scope,
                    id,
                    &AuthAccountMetadata::new(r#"["workspace","subject"]"#, TimestampMs(1))
                        .unwrap(),
                )
                .unwrap();
        }
        store
            .insert_metadata(
                &scope,
                "stale",
                &AuthAccountMetadata::new("stale", TimestampMs(1)).unwrap(),
            )
            .unwrap();
        CredentialFileStore::new(&root)
            .and_then(|store| store.for_owner(scope.principal))
            .and_then(|owner| owner.lease(scope.tenant, scope.host, AppId::Codex, "stale", true))
            .map_err(anyhow::Error::msg)?;
        let providers = AppProviderStore::new(db.clone()).map_err(anyhow::Error::msg)?;
        for (id, config) in [
            ("official", "model_provider='openai'\nmodel='fixture-model'\n"),
            (
                "custom",
                "model_provider='custom'\n[model_providers.custom]\nname='custom'\nbase_url='https://example.test'\nwire_api='responses'\n",
            ),
        ] {
            let profile = vw_agent_apps::providers::codex_catalog::import(
                Some(config.as_bytes()),
                Some(b"{}"),
            )
            .unwrap();
            providers.codex_save(&tenant, id, &profile, None, None).map_err(anyhow::Error::msg)?;
        }
        let native = home.path().join(".kimi-code");
        fs::create_dir_all(native.join("credentials"))?;
        fs::create_dir(native.join("oauth"))?;
        fs::write(native.join("config.toml"), KIMI_CONFIG)?;
        let grant = native.join("credentials/kimi-code-fixture.json");
        fs::write(
            &grant,
            json!({
                "access_token":"kimi-private-access", "refresh_token":"kimi-private-refresh",
                "expires_at":EXPIRES,
            })
            .to_string(),
        )?;
        fs::set_permissions(grant, fs::Permissions::from_mode(0o600))?;
        let profile = kimi::declarations(Some(KIMI_CONFIG.as_bytes()))
            .unwrap()
            .remove(KIMI_PROVIDER)
            .unwrap();
        providers
            .kimi_save(&tenant, KIMI_PROVIDER, &profile, None, None)
            .map_err(anyhow::Error::msg)?;
        let kimi_scope = AuthAccountScope { service: AuthService::KimiOauth, ..scope };
        let mut metadata = AuthAccountMetadata::new("kimi-identity", TimestampMs(1)).unwrap();
        metadata.region = Some(AuthRegion::Global);
        store.insert_metadata(&kimi_scope, "kimi-account", &metadata).unwrap();
        CredentialFileStore::new(&root)
            .and_then(|store| store.lease_kimi(kimi_scope, "kimi-account", true))
            .and_then(|lease| {
                lease.save(&AccountCredential::KimiOauth(KimiCredential {
                    access_token: "kimi-private-access".into(),
                    refresh_token: "kimi-private-refresh".into(),
                    expires_at: TimestampMs(EXPIRES * 1_000),
                    device_id: "d8c8a37b-175c-4e59-9501-07351b0da278".into(),
                    region: AuthRegion::Global,
                }))
            })
            .map_err(anyhow::Error::msg)?;
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
    async fn put(
        &self,
        path: &str,
        query: &str,
        company: &str,
        basic: bool,
        token: Option<&str>,
        body: Value,
    ) -> anyhow::Result<(StatusCode, Value)> {
        let mut request = self
            .client
            .put(format!("{}{path}?{query}", self.url))
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            .json(&body);
        if basic {
            request = request.header("authorization", BASIC);
        }
        if let Some(token) = token {
            request = request.header("x-agent-apps-token", token);
        }
        let response = request.send().await?;
        let status = response.status();
        if path.starts_with(ROOT) {
            assert_eq!(response.headers()["cache-control"], "no-store");
        }
        let bytes = response.bytes().await?;
        if status == StatusCode::NO_CONTENT {
            assert!(bytes.is_empty());
            return Ok((status, Value::Null));
        }
        Ok((status, serde_json::from_slice(&bytes).unwrap_or(Value::Null)))
    }

    async fn assert_binding(&self, query: &str, expected: Value) -> anyhow::Result<()> {
        let response = self
            .client
            .get(format!("{}{ROOT}?{query}", self.url))
            .header("x-platform-id", "p")
            .header("x-company-id", "c")
            .header("authorization", BASIC)
            .header("x-agent-apps-token", OWNER)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json::<Value>().await?, json!({"bindings":expected}));
        Ok(())
    }
}

fn sqlite() -> anyhow::Result<(tempfile::TempDir, GlobalDatabase)> {
    let home = tempfile::tempdir()?;
    let db = GlobalDatabase::for_home(home.path())?;
    Ok((home, db))
}

#[test]
fn auth6_api_binding_commits_expected_scoped_result() -> anyhow::Result<()> {
    let (_home, db) = sqlite()?;
    tokio::runtime::Runtime::new()?.block_on(check_success(Fixture::new(db)?))
}

#[test]
fn auth6_api_binding_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    let (_home, db) = sqlite()?;
    tokio::runtime::Runtime::new()?.block_on(check_denials(Fixture::new(db)?))
}

#[test]
fn auth6_api_binding_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    let db = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(check_success(Fixture::new(db.database().clone())?))
}

#[test]
fn auth6_api_binding_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    let db = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(check_denials(Fixture::new(db.database().clone())?))
}

async fn check_success(fixture: Fixture) -> anyhow::Result<()> {
    let app = fixture.serve().await?;
    for account in [Some("first"), Some("first"), Some("second"), None, None] {
        let (status, _) =
            app.put(PATH, CODEX, "c", true, Some(OWNER), json!({"account_id":account})).await?;
        assert_eq!(status, StatusCode::NO_CONTENT, "authorized scoped binding update");
        let expected = account.map_or(
            json!([]),
            |id| json!([{"provider_id":"official","service":"codex_oauth","account_id":id}]),
        );
        app.assert_binding(CODEX, expected).await?;
    }
    let native = fixture.home.path().join(".kimi-code");
    let config = native.join("config.toml");
    let grant = native.join("credentials/kimi-code-fixture.json");
    let before_grant = fs::read(&grant)?;
    let (status, body) = app
        .put(KIMI_PATH, KIMI, "c", true, Some(OWNER), json!({"account_id":"kimi-account"}))
        .await?;
    assert_eq!(status, StatusCode::NO_CONTENT, "Kimi binding returned {}", body["code"]);
    app.assert_binding(
        KIMI,
        json!([{"provider_id":KIMI_PROVIDER,"service":"kimi_oauth","account_id":"kimi-account"}]),
    )
    .await?;
    let selected = fs::read_to_string(&config)?;
    assert!(selected.contains("default_model = \"chosen\""));
    for _ in 0..2 {
        assert_eq!(
            app.put(KIMI_PATH, KIMI, "c", true, Some(OWNER), json!({"account_id":null})).await?.0,
            204
        );
        app.assert_binding(KIMI, json!([])).await?;
    }
    assert_eq!(fs::read_to_string(config)?, selected);
    assert_eq!(fs::read(grant)?, before_grant);
    Ok(())
}

async fn check_denials(fixture: Fixture) -> anyhow::Result<()> {
    let app = fixture.serve().await?;
    for (path, query, company, token, body, code, expected) in [
        (PATH, CODEX, "c", Some(OWNER), json!({"account_id":"foreign-owner"}), 404, "not_found"),
        (PATH, CODEX, "c", Some(OWNER), json!({"account_id":"foreign-company"}), 404, "not_found"),
        (PATH, CODEX, "c", Some(OWNER), json!({"account_id":"foreign-host"}), 404, "not_found"),
        (PATH, CODEX, "neighbor", Some(OWNER), json!({"account_id":"first"}), 404, "not_found"),
        (PATH, CODEX, "c", Some(OWNER), json!({"account_id":"stale"}), 409, "reauth_required"),
        (PATH, CODEX, "c", Some(OWNER), json!({}), 400, "invalid_input"),
        (PATH, CODEX, "c", Some(OWNER), json!({"account_id":".."}), 400, "invalid_input"),
        (
            PATH,
            CODEX,
            "c",
            Some(OWNER),
            json!({"account_id":null,"service":"kimi_oauth"}),
            400,
            "invalid_input",
        ),
        (
            "/v1/agent-apps/auth/bindings/custom",
            CODEX,
            "c",
            Some(OWNER),
            json!({"account_id":null}),
            501,
            "unsupported",
        ),
        (
            PATH,
            "host_id=remote&service=codex_oauth",
            "c",
            Some(OWNER),
            json!({"account_id":null}),
            501,
            "execution_channel_unavailable",
        ),
        (
            PATH,
            "host_id=offline&service=codex_oauth",
            "c",
            Some(OWNER),
            json!({"account_id":null}),
            503,
            "execution_host_unavailable",
        ),
        (
            PATH,
            "host_id=host&service=agy_oauth",
            "c",
            Some(OWNER),
            json!({"account_id":null}),
            403,
            "forbidden",
        ),
        (
            PATH,
            "host_id=host&service=xai_oauth",
            "c",
            Some(OWNER),
            json!({"account_id":null}),
            501,
            "unsupported",
        ),
        (PATH, CODEX, "c", Some(EDITOR), json!({"account_id":null}), 403, "forbidden"),
        (PATH, CODEX, "c", Some(READER), json!({"account_id":null}), 403, "forbidden"),
        (PATH, CODEX, "c", None, json!({"account_id":null}), 401, "unauthenticated"),
    ] {
        let (status, response) = app.put(path, query, company, true, token, body).await?;
        assert_eq!(status.as_u16(), code, "scoped binding rejection {expected}");
        assert_eq!(response["code"], expected);
        assert!(response["correlation_id"].is_string());
        assert!(!response.to_string().contains("fixture-private"));
    }
    for service in ["dsh_account", "github_copilot"] {
        let query = format!("host_id=host&service={service}");
        let (status, body) =
            app.put(PATH, &query, "c", true, Some(OWNER), json!({"account_id":null})).await?;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(body["code"], "unsupported");
    }
    let (status, body) =
        app.put(KIMI_PATH, KIMI, "c", true, Some(OTHER), json!({"account_id":null})).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "forbidden");
    assert_eq!(
        app.put(PATH, CODEX, "c", false, Some(OWNER), json!({"account_id":null})).await?.0,
        401
    );
    assert_eq!(
        app.put(PATH, CODEX, "c", true, Some(OWNER), json!({"account_id":"first"})).await?.0,
        204
    );
    assert_eq!(
        app.put(PATH, CODEX, "c", true, Some(OTHER), json!({"account_id":null})).await?.0,
        404
    );
    let (status, body) =
        app.put(PATH, CODEX, "c", true, Some(OWNER), json!({"account_id":"stale"})).await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "reauth_required");
    app.assert_binding(
        CODEX,
        json!([{"provider_id":"official","service":"codex_oauth","account_id":"first"}]),
    )
    .await?;
    for path in ["/agent-apps/auth/bindings/official", "/api/agent-apps/auth/bindings/official"] {
        let (status, _) =
            app.put(path, CODEX, "c", true, Some(OWNER), json!({"account_id":null})).await?;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    Ok(())
}
