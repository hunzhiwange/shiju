use super::auth_router as agent_apps_auth_read;
use axum::{Extension, http::StatusCode};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use tokio::{net::TcpListener, task::JoinHandle};
use vw_agent_apps::{
    auth::credential_file::{AccountCredential, CredentialFileStore, KimiCredential},
    providers::kimi,
};
use {
    agent_app_services::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    agent_database::{GlobalDatabase, test_support::PostgresTestDatabase},
    app_provider_services::AppProviderStore,
};
use {
    vw_api_app_auth::AuthRegion, vw_api_app_auth::AuthService, vw_api_app_identity::AppId,
    vw_api_transport::TimestampMs, vw_api_transport::id::TenantScope,
};

const SKEY: &str = "fixture-binding-paired-skey-aaaaaaaa";
const ROOT: &str = "/v1/agent-apps/auth/bindings";
const PATH: &str = "/v1/agent-apps/auth/bindings/official";
const CODEX: &str = "host_id=desktop-local&service=codex_oauth";
const KIMI: &str = "host_id=desktop-local&service=kimi_oauth";
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
            CredentialFileStore::new(&credential_root(scope.tenant)?)
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
            if id == "official" {
                providers
                    .codex_save(&neighbor, id, &profile, None, None)
                    .map_err(anyhow::Error::msg)?;
            }
        }
        let native = home.join(".kimi-code");
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
    async fn put(
        &self,
        path: &str,
        query: &str,
        company: &str,
        token: Option<&str>,
        body: Value,
    ) -> anyhow::Result<(StatusCode, Value)> {
        let mut request = self
            .client
            .put(format!("{}{path}?{query}", self.url))
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            .json(&body);
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await?;
        let status = response.status();
        if path.starts_with(ROOT) && status != StatusCode::UNAUTHORIZED {
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
            .bearer_auth(SKEY)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json::<Value>().await?, json!({"bindings":expected}));
        Ok(())
    }
}

fn run_isolated(postgres: bool, commit: bool) -> anyhow::Result<()> {
    const CHILD_ROOT: &str = "SHIJU_AUTH_BINDING_CHILD_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = Path::new(&root);
        anyhow::ensure!(
            root.is_absolute()
                && root.is_dir()
                && std::env::var_os("VIBEWINDOW_TEST_ROOT").as_deref() == Some(root.as_os_str())
                && std::env::var_os("HOME").as_deref() == Some(root.as_os_str())
                && std::env::var_os("CODEX_HOME").as_deref()
                    == Some(root.join(".codex").as_os_str())
                && std::env::var_os("KIMI_HOME").as_deref()
                    == Some(root.join(".kimi-code").as_os_str()),
            "binding test requires isolated runtime and Codex paths"
        );
        let paths = agent_paths::paths();
        anyhow::ensure!(
            paths.home == root
                && [&paths.data, &paths.config, &paths.state]
                    .into_iter()
                    .all(|path| path.starts_with(root)),
            "unexpected runtime paths"
        );
        let postgres = postgres.then(PostgresTestDatabase::from_environment).transpose()?;
        let database = match &postgres {
            Some(db) => db.database().clone(),
            None => GlobalDatabase::for_home(root)?,
        };
        let fixture = Fixture::new(database)?;
        tokio::runtime::Runtime::new()?.block_on(async {
            if commit { check_success(fixture).await } else { check_denials(fixture).await }
        })?;
        return postgres.map_or(Ok(()), PostgresTestDatabase::cleanup);
    }
    let home = tempfile::tempdir()?;
    let root = home.path().canonicalize()?;
    let thread = std::thread::current();
    let name = thread.name().expect("named binding test");
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", name, "--nocapture"])
        .env(CHILD_ROOT, &root)
        .env_remove("VW_MODEL_RECOVERY_TEST_ROOT")
        .env("VIBEWINDOW_TEST_ROOT", &root)
        .env("HOME", &root)
        .env("CODEX_HOME", root.join(".codex"))
        .env("KIMI_HOME", root.join(".kimi-code"))
        .status()?;
    anyhow::ensure!(status.success(), "isolated binding test failed: {status}");
    Ok(())
}

#[test]
fn auth6_api_binding_commits_expected_scoped_result() -> anyhow::Result<()> {
    run_isolated(false, true)
}

#[test]
fn auth6_api_binding_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    run_isolated(false, false)
}

#[test]
fn auth6_api_binding_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    run_isolated(true, true)
}

#[test]
fn auth6_api_binding_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    run_isolated(true, false)
}

async fn check_success(fixture: Fixture) -> anyhow::Result<()> {
    let app = fixture.serve().await?;
    for account in [Some("first"), Some("first"), Some("second"), None, None] {
        let (status, _) =
            app.put(PATH, CODEX, "c", Some(SKEY), json!({"account_id":account})).await?;
        assert_eq!(status, StatusCode::NO_CONTENT, "authorized scoped binding update");
        let expected = account.map_or(
            json!([]),
            |id| json!([{"provider_id":"official","service":"codex_oauth","account_id":id}]),
        );
        app.assert_binding(CODEX, expected).await?;
    }
    let native = fixture.home.join(".kimi-code");
    let config = native.join("config.toml");
    let grant = native.join("credentials/kimi-code-fixture.json");
    let before_grant = fs::read(&grant)?;
    let (status, body) =
        app.put(KIMI_PATH, KIMI, "c", Some(SKEY), json!({"account_id":"kimi-account"})).await?;
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
            app.put(KIMI_PATH, KIMI, "c", Some(SKEY), json!({"account_id":null})).await?.0,
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
        (PATH, CODEX, "c", Some(SKEY), json!({"account_id":"foreign-owner"}), 404, "not_found"),
        (PATH, CODEX, "c", Some(SKEY), json!({"account_id":"foreign-company"}), 404, "not_found"),
        (PATH, CODEX, "c", Some(SKEY), json!({"account_id":"foreign-host"}), 404, "not_found"),
        (PATH, CODEX, "neighbor", Some(SKEY), json!({"account_id":"first"}), 404, "not_found"),
        (PATH, CODEX, "c", Some(SKEY), json!({"account_id":"stale"}), 409, "reauth_required"),
        (PATH, CODEX, "c", Some(SKEY), json!({}), 400, "invalid_input"),
        (PATH, CODEX, "c", Some(SKEY), json!({"account_id":".."}), 400, "invalid_input"),
        (
            PATH,
            CODEX,
            "c",
            Some(SKEY),
            json!({"account_id":null,"service":"kimi_oauth"}),
            400,
            "invalid_input",
        ),
        (
            "/v1/agent-apps/auth/bindings/custom",
            CODEX,
            "c",
            Some(SKEY),
            json!({"account_id":null}),
            501,
            "unsupported",
        ),
        (
            PATH,
            "host_id=desktop-local&service=xai_oauth",
            "c",
            Some(SKEY),
            json!({"account_id":null}),
            501,
            "unsupported",
        ),
    ] {
        let (status, response) = app.put(path, query, company, token, body).await?;
        assert_eq!(status.as_u16(), code, "scoped binding rejection {expected}");
        assert_eq!(response["code"], expected);
        assert!(response["correlation_id"].is_string());
        assert!(!response.to_string().contains("fixture-private"));
    }
    for service in ["dsh_account", "github_copilot"] {
        let query = format!("host_id=desktop-local&service={service}");
        let (status, body) =
            app.put(PATH, &query, "c", Some(SKEY), json!({"account_id":null})).await?;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(body["code"], "unsupported");
    }
    assert_eq!(
        app.put(PATH, CODEX, "c", Some(SKEY), json!({"account_id":"first"})).await?.0,
        StatusCode::NO_CONTENT
    );
    for token in [None, Some("unpaired-binding-skey")] {
        assert_eq!(
            app.put(PATH, CODEX, "c", token, json!({"account_id":null})).await?.0,
            StatusCode::UNAUTHORIZED
        );
    }
    let response = app
        .client
        .put(format!("{}{PATH}?{CODEX}", app.url))
        .header("x-platform-id", "p")
        .header("x-company-id", "c")
        .header("x-agent-apps-token", SKEY)
        .json(&json!({"account_id":null}))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let (status, body) =
        app.put(PATH, CODEX, "c", Some(SKEY), json!({"account_id":"stale"})).await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "reauth_required");
    app.assert_binding(
        CODEX,
        json!([{"provider_id":"official","service":"codex_oauth","account_id":"first"}]),
    )
    .await?;
    for path in ["/agent-apps/auth/bindings/official", "/api/agent-apps/auth/bindings/official"] {
        let (status, _) = app.put(path, CODEX, "c", Some(SKEY), json!({"account_id":null})).await?;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    Ok(())
}
