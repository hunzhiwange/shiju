use super::agent_apps_auth_read;
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    app_providers::AppProviderStore,
    global_database::{GlobalDatabase, test_support::PostgresTestDatabase},
};
use axum::{Extension, http::StatusCode};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Method, Response};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    net::SocketAddr,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{net::TcpListener, task::JoinHandle};
use vw_agent_apps::auth::{
    claude_keychain::{ClaudeKeychain, MacOsKeychain},
    credential_file::{AccountCredential, CredentialFileStore, KimiCredential},
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{AuthRegion, AuthService},
    id::TenantScope,
};

const OWNER: &str = "fixture-logout-owner-token-aaaaaaaa";
const READER: &str = "fixture-logout-reader-token-bbbbbbb";
const ROOT: &str = "/v1/agent-apps/auth";
const SCOPE: &str = "host_id=host&service=codex_oauth";

struct Fixture {
    home: tempfile::TempDir,
    db: GlobalDatabase,
    root: PathBuf,
}

impl Fixture {
    fn new(db: GlobalDatabase) -> anyhow::Result<Self> {
        let home = tempfile::tempdir()?;
        let root = home.path().join("private");
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let grant = |company, host, write| {
            json!({
                "platform_id":"p", "company_id":company, "host_id":host,
                "auth_account_read":["codex_oauth","dsh_account"],
                "auth_flow_start":if write { json!(["claude_official"]) } else { json!([]) },
                "auth_browser_local":write,
                "provider_config_read":true, "provider_config_write":write,
                "auth_binding_write":if write { json!(["codex_oauth"]) } else { json!([]) },
                "auth_default_write":if write { json!(["codex_oauth"]) } else { json!([]) },
                "auth_logout":if write { json!(["codex_oauth","claude_official","dsh_account","kimi_oauth"]) } else { json!([]) },
                "auth_account_revoke":if write { json!(["codex_oauth","dsh_account"]) } else { json!([]) }
            })
        };
        let principal = |id, token: &str, grants| json!({"id":id, "token_sha256":hex::encode(Sha256::digest(token)), "grants":grants});
        let host = |id, online, binding| {
            let execution_home = if id == "cli" {
                PathBuf::from(std::env::var_os("HOME").expect("test execution HOME"))
            } else {
                home.path().to_path_buf()
            };
            json!({"id":id, "online":online, "binding":binding, "home":execution_home,
                "search_paths":[home.path()], "applications":home.path(), "credential_root":root,
                "native_login_owners": if id == "host" {
                    json!([
                        {"service":"dsh_account","platform_id":"p","company_id":"c","principal":"owner"},
                        {"service":"kimi_oauth","platform_id":"p","company_id":"c","principal":"owner"},
                    ])
                } else { json!([]) }})
        };
        let policy = home.path().join("access.json");
        fs::write(
            &policy,
            serde_json::to_vec(&json!({
                "principals":[
                    principal("owner", OWNER, vec![grant("c","host",true), grant("neighbor","host",true),
                        grant("c","remote",true), grant("c","offline",true), grant("c","cli",true)]),
                    principal("reader", READER, vec![grant("c","host",false)])
                ],
                "hosts":[host("host",true,"local"), host("remote",true,"remote"), host("offline",false,"local"), host("cli",true,"local")]
            }))?,
        )?;
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600))?;
        let tenant = TenantScope::new("p", "c")?;
        let neighbor = TenantScope::new("p", "neighbor")?;
        let scope = AuthAccountScope {
            tenant: &tenant,
            principal: "owner",
            host: "host",
            service: AuthService::CodexOauth,
        };
        let accounts = AuthAccountStore::new(db.clone(), &root).map_err(anyhow::Error::msg)?;
        let credentials = CredentialFileStore::new(&root).map_err(anyhow::Error::msg)?;
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
        Ok(Self { home, db, root })
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
        let server = tokio::spawn(async move {
            axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        });
        let client =
            reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(15)).build()?;
        Ok(Gateway { url, client, server })
    }

    fn stored_default(&self, company: &str) -> anyhow::Result<Option<String>> {
        crate::global_database::run_sync_database(|| {
            let tenant = TenantScope::new("p", company)?;
            let store =
                AuthAccountStore::new(self.db.clone(), &self.root).map_err(anyhow::Error::msg)?;
            store
                .managed_default_account(AuthAccountScope {
                    tenant: &tenant,
                    principal: "owner",
                    host: "host",
                    service: AuthService::CodexOauth,
                })
                .map_err(|error| anyhow::anyhow!("{error:?}"))
        })
    }

    fn has_credential(&self, company: &str) -> anyhow::Result<bool> {
        let tenant = TenantScope::new("p", company)?;
        let lease = CredentialFileStore::new(&self.root)
            .and_then(|store| store.for_owner("owner"))
            .and_then(|owner| owner.lease(&tenant, "host", AppId::Codex, "managed", false))
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

    async fn revoke(
        &self,
        id: &str,
        scope: &str,
        company: &str,
        token: &str,
    ) -> anyhow::Result<Response> {
        let response = self
            .request(Method::POST, &format!("{ROOT}/accounts/{id}/revoke?{scope}"), company)
            .basic_auth("test", Some("secret"))
            .header("x-agent-apps-token", token)
            .send()
            .await?;
        assert_eq!(response.headers()["cache-control"], "no-store");
        Ok(response)
    }

    async fn logout(
        &self,
        scope: &str,
        company: &str,
        token: &str,
        body: Value,
    ) -> anyhow::Result<Response> {
        let response = self
            .request(Method::POST, &format!("{ROOT}/logout?{scope}"), company)
            .basic_auth("test", Some("secret"))
            .header("x-agent-apps-token", token)
            .json(&body)
            .send()
            .await?;
        assert_eq!(response.headers()["cache-control"], "no-store");
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

const DSH_SCOPE: &str = "host_id=host&service=dsh_account";
const DSH_GRANT: &str = "deepseek-account-platform/default";

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
    let response = gateway.logout(SCOPE, "c", OWNER, json!({"selection":"default"})).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT, "local default logout");
    assert!(response.bytes().await?.is_empty());
    assert_eq!(fixture.stored_default("c")?, None);
    assert_eq!(fixture.stored_default("neighbor")?.as_deref(), Some("managed"));
    let accounts = gateway.read("accounts", "c").await?;
    assert_eq!(accounts["accounts"][0]["account_id"], "managed");
    assert_eq!(accounts["accounts"][0]["credential_state"], "signed_out");
    assert_eq!(gateway.read("bindings", "c").await?["bindings"][0]["account_id"], "managed");
    assert!(!fixture.has_credential("c")?);
    assert!(fixture.has_credential("neighbor")?);

    // A foreign issuer can be cleared locally, but must never receive revocation traffic.
    let tenant = TenantScope::new("p", "c")?;
    let id = URL_SAFE_NO_PAD.encode(Sha256::digest(serde_json::to_vec(&(
        &tenant,
        "owner",
        "host",
        AuthService::DshAccount,
        DSH_GRANT,
    ))?));
    let native = fixture.home.path().join(".dsh");
    fs::create_dir(&native)?;
    fs::set_permissions(&native, fs::Permissions::from_mode(0o700))?;
    let credentials = native.join(".credentials.yaml");
    let initial = json!({"version":1,"records":{
        (DSH_GRANT):{"kind":"grant","payload":{"version":1,
            "token":"fixture-private-dsh","issuer":"https://example.invalid"}},
        "unrelated":{"preserve":true}
    }});
    fs::write(&credentials, serde_yaml::to_string(&initial)?)?;
    fs::set_permissions(&credentials, fs::Permissions::from_mode(0o600))?;
    assert_error(gateway.revoke(&id, DSH_SCOPE, "c", OWNER).await?, 409, "reauth_required").await?;
    assert_eq!(serde_yaml::from_slice::<Value>(&fs::read(&credentials)?)?, initial);
    assert_eq!(
        gateway.logout(DSH_SCOPE, "c", OWNER, json!({"selection":"all"})).await?.status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        serde_yaml::from_slice::<Value>(&fs::read(&credentials)?)?,
        json!({"version":1,"records":{"unrelated":{"preserve":true}}})
    );
    assert_error(gateway.revoke(&id, DSH_SCOPE, "c", OWNER).await?, 409, "conflict").await?;
    let status: Value = gateway
        .request(Method::GET, &format!("{ROOT}/status?{DSH_SCOPE}"), "c")
        .basic_auth("test", Some("secret"))
        .header("x-agent-apps-token", OWNER)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(status["capabilities"]["can_revoke"], true);
    assert_eq!(gateway.read("status", "c").await?["capabilities"]["can_revoke"], false);
    assert_kimi_logout(&fixture, &gateway).await?;
    assert_claude_cleanup(&fixture, &gateway).await
}

async fn assert_kimi_logout(fixture: &Fixture, gateway: &Gateway) -> anyhow::Result<()> {
    let native = fixture.home.path().join(".kimi-code");
    fs::create_dir_all(native.join("credentials"))?;
    fs::create_dir(native.join("oauth"))?;
    let config = r#"default_model = "global"

[providers.global]
type = "kimi"
base_url = "https://api.kimi.ai/coding/v1"
[providers.global.oauth]
storage = "file"
key = "oauth/fixture-global"
oauth_host = "https://auth.kimi.ai"

[providers.mainland]
type = "kimi"
base_url = "https://api.kimi.com/coding/v1"
[providers.mainland.oauth]
storage = "file"
key = "oauth/fixture-mainland"
oauth_host = "https://auth.kimi.com"

[models.global]
provider = "global"
model = "kimi-for-coding"

[models.mainland]
provider = "mainland"
model = "kimi-for-coding"
"#;
    fs::write(native.join("config.toml"), config)?;
    let tenant = TenantScope::new("p", "c")?;
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "host",
        service: AuthService::KimiOauth,
    };
    let credentials = CredentialFileStore::new(&fixture.root).map_err(anyhow::Error::msg)?;
    let accounts =
        AuthAccountStore::new(fixture.db.clone(), &fixture.root).map_err(anyhow::Error::msg)?;
    let native_file = |id| native.join("credentials").join(format!("fixture-{id}.json"));
    for (id, region) in [("global", AuthRegion::Global), ("mainland", AuthRegion::MainlandCn)] {
        let access_token = format!("fixture-private-{id}-access");
        let refresh_token = format!("fixture-private-{id}-refresh");
        fs::write(
            native_file(id),
            json!({
                "access_token":access_token, "refresh_token":refresh_token, "expires_at":1,
                "expires_in":900, "token_type":"Bearer", "scope":""
            })
            .to_string(),
        )?;
        fs::set_permissions(native_file(id), fs::Permissions::from_mode(0o600))?;
        credentials
            .lease_kimi(scope, id, true)
            .and_then(|lease| {
                lease.save(&AccountCredential::KimiOauth(KimiCredential {
                    access_token,
                    refresh_token,
                    expires_at: TimestampMs(1_000),
                    region,
                    device_id: "d8c8a37b-175c-4e59-9501-07351b0da278".into(),
                }))
            })
            .map_err(anyhow::Error::msg)?;
        let mut metadata = AuthAccountMetadata::new(id, TimestampMs(1)).unwrap();
        metadata.region = Some(region);
        crate::global_database::run_sync_database(|| {
            accounts
                .insert_metadata(&scope, id, &metadata)
                .map_err(|error| anyhow::anyhow!("{error:?}"))
        })?;
    }
    let query = "host_id=host&service=kimi_oauth";
    let response = gateway.logout(query, "c", OWNER, json!({"selection":"default"})).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT, "default Kimi logout");
    assert!(!native_file("global").exists());
    assert!(native_file("mainland").exists());
    assert!(credentials.lease_kimi(scope, "global", false).and_then(|lease| lease.read()).is_err());

    let response = gateway.logout(query, "c", OWNER, json!({"selection":"all"})).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT, "remaining Kimi logout");
    assert!(!native_file("mainland").exists());
    assert!(
        credentials.lease_kimi(scope, "mainland", false).and_then(|lease| lease.read()).is_err()
    );
    assert_eq!(fs::read_to_string(native.join("config.toml"))?, config);
    Ok(())
}

async fn assert_claude_cleanup(fixture: &Fixture, gateway: &Gateway) -> anyhow::Result<()> {
    let home = fixture.home.path();
    let program = home.join("claude");
    fs::write(
        &program,
        format!(
            r#"#!/bin/sh
set -eu
printf '%s\n%s\n' "$$" "$CLAUDE_CONFIG_DIR" > '{}'
umask 077
printf '%s' '{{"oauthAccount":{{"accountUuid":"fixture-user","organizationUuid":"fixture-org"}}}}' > .claude.json
"$BROWSER" 'https://claude.com/cai/oauth/authorize?code=true&state=fixture'
IFS= read -r code
"#,
            home.join("login-process").display(),
        ),
    )?;
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700))?;
    let scope = "host_id=cli&service=claude_official";
    let started: Value = gateway
        .request(Method::POST, &format!("{ROOT}/flows?{scope}"), "c")
        .basic_auth("test", Some("secret"))
        .header("x-agent-apps-token", OWNER)
        .json(&json!({"method":"copy_link"}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let flow = started["flow_id"].as_str().expect("Claude flow ID");
    let process = fs::read_to_string(home.join("login-process"))?;
    let mut lines = process.lines();
    let pid: libc::pid_t = lines.next().unwrap().parse()?;
    let directory = Path::new(lines.next().unwrap());
    assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
    struct KeychainFixture<'a>(&'a Path);
    impl Drop for KeychainFixture<'_> {
        fn drop(&mut self) {
            let _ = MacOsKeychain.write(Some(self.0), None);
        }
    }
    let _keychain = KeychainFixture(directory);
    let payload = json!({"claudeAiOauth":{"refreshToken":"fixture-private-claude"}});
    MacOsKeychain.write(Some(directory), Some(&payload)).unwrap();
    assert_eq!(MacOsKeychain.read(Some(directory)).unwrap(), Some(payload));

    let response = gateway.logout(scope, "c", OWNER, json!({"selection":"all"})).await?;
    let status = response.status();
    let body = response.text().await?;
    assert_eq!(status, StatusCode::NO_CONTENT, "Claude logout: {body}");
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    assert!(!directory.exists());
    assert!(MacOsKeychain.read(Some(directory)).unwrap().is_none());

    let snapshot: Value = gateway
        .request(Method::GET, &format!("{ROOT}/flows/{flow}?{scope}"), "c")
        .basic_auth("test", Some("secret"))
        .header("x-agent-apps-token", OWNER)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(snapshot["state"], "cancelled");
    Ok(())
}

async fn rejects(fixture: Fixture) -> anyhow::Result<()> {
    let gateway = fixture.serve().await?;
    let selected = |id| json!({"selection":"account","account_id":id});
    for body in [
        json!({"selection":"all","account_id":"managed"}),
        json!({"selection":"account"}),
        selected("../bad"),
    ] {
        assert_error(gateway.logout(SCOPE, "c", OWNER, body).await?, 400, "invalid_input").await?;
    }
    let without_principal = gateway
        .request(Method::POST, &format!("{ROOT}/logout?{SCOPE}"), "c")
        .basic_auth("test", Some("secret"))
        .json(&json!({"selection":"unknown"}))
        .send()
        .await?;
    assert_error(without_principal, 401, "unauthenticated").await?;
    let absent = assert_error(
        gateway.logout(SCOPE, "c", OWNER, selected("missing")).await?,
        404,
        "not_found",
    )
    .await?;
    for id in ["foreign-company", "foreign-host", "foreign-owner"] {
        let response = gateway.logout(SCOPE, "c", OWNER, selected(id)).await?;
        assert_eq!(assert_error(response, 404, "not_found").await?, absent);
    }
    assert_error(gateway.logout(SCOPE, "c", READER, selected("managed")).await?, 403, "forbidden")
        .await?;
    assert_error(gateway.revoke("managed", SCOPE, "c", READER).await?, 403, "forbidden").await?;
    assert_error(gateway.revoke("managed", SCOPE, "c", OWNER).await?, 501, "unsupported").await?;
    assert_error(
        gateway.revoke("managed", "host_id=host&service=claude_official", "c", OWNER).await?,
        403,
        "forbidden",
    )
    .await?;
    assert_error(gateway.revoke("missing", DSH_SCOPE, "c", OWNER).await?, 404, "not_found").await?;
    assert_error(gateway.revoke("missing", DSH_SCOPE, "neighbor", OWNER).await?, 403, "forbidden")
        .await?;
    assert_error(
        gateway.logout(DSH_SCOPE, "c", OWNER, json!({"selection":"default"})).await?,
        501,
        "unsupported",
    )
    .await?;
    for prefix in ["/agent-apps/auth", "/api/agent-apps/auth"] {
        for resource in ["logout", "accounts/managed/revoke"] {
            let response = gateway
                .request(Method::POST, &format!("{prefix}/{resource}?{SCOPE}"), "c")
                .basic_auth("test", Some("secret"))
                .header("x-agent-apps-token", OWNER)
                .json(&selected("managed"))
                .send()
                .await?;
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }
    assert!(fixture.has_credential("c")?);
    assert!(fixture.has_credential("neighbor")?);
    Ok(())
}

#[test]
fn auth6_api_logout_commits_expected_scoped_result() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let fixture = Fixture::new(GlobalDatabase::for_home(home.path())?)?;
    tokio::runtime::Runtime::new()?.block_on(commits(fixture))
}

#[test]
fn auth6_api_logout_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let fixture = Fixture::new(GlobalDatabase::for_home(home.path())?)?;
    tokio::runtime::Runtime::new()?.block_on(rejects(fixture))
}

#[test]
fn auth6_api_logout_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    let db = PostgresTestDatabase::from_environment()?;
    let fixture = Fixture::new(db.database().clone())?;
    let result = tokio::runtime::Runtime::new()?.block_on(commits(fixture));
    db.cleanup()?;
    result
}

#[test]
fn auth6_api_logout_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    let db = PostgresTestDatabase::from_environment()?;
    let fixture = Fixture::new(db.database().clone())?;
    let result = tokio::runtime::Runtime::new()?.block_on(rejects(fixture));
    db.cleanup()?;
    result
}
