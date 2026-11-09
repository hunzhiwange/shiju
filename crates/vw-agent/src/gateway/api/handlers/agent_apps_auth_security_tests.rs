use super::{
    agent_apps_auth_start::Runtime,
    agent_apps_auth_start_tests::{BASIC, FLOWS, Gateway, OWNER},
};
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    app_providers::AppProviderStore,
    global_database::{GlobalDatabase, run_sync_database, test_support::PostgresTestDatabase},
};
use axum::http::{Method, StatusCode, header};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::{fs, path::Path, sync::Arc};
use vw_agent_apps::auth::{
    claude_keychain::{ClaudeKeychain, MacOsKeychain},
    codex_device::managed_identity,
    codex_refresh::CodexRefresh,
    credential_file::{AccountCredential, CredentialFileStore, XaiCredential},
    xai_refresh::XaiRefresh,
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

const ROOT: &str = "/v1/agent-apps/auth";
const BINDING: &str = "/v1/agent-apps/auth/bindings/official";
const DEFAULT: &str = "/v1/agent-apps/auth/accounts/primary/default";
const IDENTITY: &str = r#"["workspace","subject"]"#;
const PRIVATE: &str = "fixture-private";

#[derive(Clone, Copy)]
struct Target {
    company: &'static str,
    host: &'static str,
    service: AuthService,
}

const LOCAL: Target = Target { company: "c", host: "host", service: AuthService::CodexOauth };
const COMPANY: Target = Target { company: "neighbor", ..LOCAL };
const HOST: Target = Target { host: "second-host", ..LOCAL };
const SERVICE: Target = Target { service: AuthService::XaiOauth, ..LOCAL };
const CLAUDE: Target = Target { service: AuthService::ClaudeOfficial, ..LOCAL };
const REMOTE: Target = Target { host: "remote", ..CLAUDE };
const OFFLINE: Target = Target { host: "offline", ..CLAUDE };

impl Target {
    fn scope(self, tenant: &TenantScope) -> AuthAccountScope<'_> {
        AuthAccountScope { tenant, principal: "owner", host: self.host, service: self.service }
    }

    fn request(self, gateway: &Gateway, method: Method, path: &str) -> reqwest::RequestBuilder {
        gateway
            .client
            .request(method, format!("http://{}{path}", gateway.addr))
            .query(&[("host_id", self.host)])
            .query(&[("service", self.service)])
            .header("x-platform-id", "p")
            .header("x-company-id", self.company)
    }
}

async fn response(request: reqwest::RequestBuilder) -> anyhow::Result<(StatusCode, Value)> {
    let response = request.send().await?;
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let status = response.status();
    let body = response.text().await?;
    assert!(!body.contains(PRIVATE));
    Ok((status, if body.is_empty() { Value::Null } else { serde_json::from_str(&body)? }))
}

async fn send(request: reqwest::RequestBuilder) -> anyhow::Result<(StatusCode, Value)> {
    response(request.header(header::AUTHORIZATION, BASIC).header("x-agent-apps-token", OWNER)).await
}

async fn reject(
    request: reqwest::RequestBuilder,
    expected: StatusCode,
    code: &str,
) -> anyhow::Result<()> {
    let (status, body) = send(request).await?;
    assert_eq!(status, expected, "expected {code}");
    assert_eq!(body["code"], code);
    Ok(())
}

fn codex_token(id: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
    let claims = URL_SAFE_NO_PAD.encode(
        json!({"sub":"subject", "chatgpt_account_id":"workspace", "exp":4_102_444_800_i64,
               "jti":id})
        .to_string(),
    );
    format!("{header}.{claims}.signature")
}

async fn setup(
    home: &Path,
    database: GlobalDatabase,
) -> anyhow::Result<(Gateway, AuthAccountStore)> {
    let gateway =
        Gateway::start_with_runtime(home, database.clone(), Arc::new(Runtime::default())).await?;
    let file = home.join("policy.json");
    let mut policy: Value = serde_json::from_slice(&fs::read(&file)?)?;
    let mut second_host = policy["hosts"][0].clone();
    second_host["id"] = json!(HOST.host);
    policy["hosts"].as_array_mut().unwrap().push(second_host);
    // Give the caller real grants in each neighboring scope to exercise resource checks.
    let services = json!(["codex_oauth", "xai_oauth", "claude_official", "agy_oauth"]);
    policy["principals"][0]["grants"] =
        json!([LOCAL, COMPANY, HOST, REMOTE, OFFLINE].map(|target| json!({
            "platform_id":"p", "company_id":target.company, "host_id":target.host,
            "provider_config_read":true, "provider_config_write":true,
            "auth_account_read":services, "auth_default_write":services,
            "auth_binding_write":services, "auth_flow_start":services
        })));
    // A native-slot grant on a remote host still needs a real remote execution channel.
    policy["hosts"][1]["native_login_owners"] = json!([
        {"service":"agy_oauth", "platform_id":"p", "company_id":"c", "principal":"owner"}
    ]);
    fs::write(file, serde_json::to_vec(&policy)?)?;

    let root = home.join("credentials");
    let store = AuthAccountStore::new(database.clone(), &root).map_err(anyhow::Error::msg)?;
    let credentials = CredentialFileStore::new(&root).map_err(anyhow::Error::msg)?;
    run_sync_database(|| {
        let providers = AppProviderStore::new(database).map_err(anyhow::Error::msg)?;
        let profile = vw_agent_apps::providers::codex_catalog::import(
            Some(b"model_provider='openai'\nmodel='fixture-model'\n"),
            Some(b"{}"),
        )
        .unwrap();
        for company in [LOCAL.company, COMPANY.company] {
            providers
                .codex_save(&TenantScope::new("p", company)?, "official", &profile, None, None)
                .map_err(anyhow::Error::msg)?;
        }
        for (target, id) in
            [(LOCAL, "primary"), (COMPANY, "company"), (HOST, "host"), (SERVICE, "service")]
        {
            let tenant = TenantScope::new("p", target.company)?;
            let scope = target.scope(&tenant);
            if target.service == AuthService::CodexOauth {
                let auth = json!({"auth_mode":"chatgpt", "tokens":{
                    "account_id":"workspace", "access_token":codex_token(id), "id_token":codex_token(id),
                    "refresh_token":PRIVATE
                }});
                assert_eq!(managed_identity(&auth), Ok(IDENTITY.to_owned()));
                credentials
                    .for_owner(scope.principal)
                    .and_then(|owner| owner.lease(scope.tenant, scope.host, AppId::Codex, id, true))
                    .and_then(|lease| lease.save(&AccountCredential::CodexAuth(auth)))
                    .map_err(anyhow::Error::msg)?;
            } else {
                credentials
                    .lease_xai(scope, id, true)
                    .and_then(|lease| {
                        lease.save(&AccountCredential::XaiOauth(XaiCredential {
                            access_token: PRIVATE.into(),
                            refresh_token: PRIVATE.into(),
                            expires_at: TimestampMs(4_102_444_800_000),
                            requires_reauth: false,
                        }))
                    })
                    .map_err(anyhow::Error::msg)?;
            }
            store
                .insert_metadata(
                    &scope,
                    id,
                    &AuthAccountMetadata::new(IDENTITY, TimestampMs(1)).unwrap(),
                )
                .unwrap();
        }
        Ok(())
    })?;
    Ok((gateway, store))
}

async fn select_default(gateway: &Gateway, target: Target, id: &str) -> anyhow::Result<()> {
    let path = format!("{ROOT}/accounts/{id}/default");
    assert_eq!(send(target.request(gateway, Method::PUT, &path)).await?.0, StatusCode::NO_CONTENT);
    Ok(())
}

async fn bind(gateway: &Gateway, target: Target, id: &str) -> anyhow::Result<()> {
    let request = target.request(gateway, Method::PUT, BINDING).json(&json!({"account_id":id}));
    assert_eq!(send(request).await?.0, StatusCode::NO_CONTENT);
    Ok(())
}

async fn assert_account(
    gateway: &Gateway,
    target: Target,
    id: &str,
    default: bool,
) -> anyhow::Result<()> {
    let (status, body) =
        send(target.request(gateway, Method::GET, &format!("{ROOT}/accounts"))).await?;
    assert_eq!(status, StatusCode::OK);
    let accounts = body["accounts"].as_array().unwrap();
    assert_eq!(accounts.len(), 1, "account list is scoped to {}", target.host);
    assert_eq!(accounts[0]["account_id"], id);
    assert_eq!(accounts[0]["service"], json!(target.service));
    assert_eq!(accounts[0]["is_default"], default);
    assert_eq!(accounts[0]["credential_state"], "ready");
    Ok(())
}

async fn assert_binding(gateway: &Gateway, target: Target, id: Option<&str>) -> anyhow::Result<()> {
    let (status, body) =
        send(target.request(gateway, Method::GET, &format!("{ROOT}/bindings"))).await?;
    assert_eq!(status, StatusCode::OK);
    let bindings = id.map_or_else(
        || json!([]),
        |id| json!([{"provider_id":"official", "service":"codex_oauth", "account_id":id}]),
    );
    assert_eq!(body["bindings"], bindings);
    Ok(())
}

async fn commits(home: &Path, database: GlobalDatabase) -> anyhow::Result<()> {
    let (gateway, accounts) = setup(home, database).await?;
    let selections =
        [(LOCAL, "primary"), (COMPANY, "company"), (HOST, "host"), (SERVICE, "service")];
    for (target, id) in selections {
        select_default(&gateway, target, id).await?;
    }
    bind(&gateway, LOCAL, "primary").await?;
    bind(&gateway, COMPANY, "company").await?;
    for (target, id) in selections {
        assert_account(&gateway, target, id, true).await?;
    }
    for (target, id) in
        [(LOCAL, Some("primary")), (COMPANY, Some("company")), (HOST, None), (SERVICE, None)]
    {
        assert_binding(&gateway, target, id).await?;
    }
    let refresh = CodexRefresh::new().unwrap();
    for (target, id) in [(LOCAL, "primary"), (COMPANY, "company"), (HOST, "host")] {
        let tenant = TenantScope::new("p", target.company)?;
        let result = accounts.codex_access_token(&refresh, target.scope(&tenant), id).await;
        assert!(result.is_ok_and(|token| token == codex_token(id)), "token belongs to {id}");
    }
    let tenant = TenantScope::new("p", SERVICE.company)?;
    let xai = XaiRefresh::new().unwrap();
    let result = accounts.xai_access_token(&xai, SERVICE.scope(&tenant), "service").await;
    assert!(result.is_ok_and(|token| token == PRIVATE), "xAI resolves its own token");
    Ok(())
}

async fn rejects(home: &Path, database: GlobalDatabase) -> anyhow::Result<()> {
    let (gateway, accounts) = setup(home, database).await?;
    select_default(&gateway, LOCAL, "primary").await?;
    bind(&gateway, LOCAL, "primary").await?;

    let accounts_path = format!("{ROOT}/accounts");
    let forged = Target { company: "no-membership", ..LOCAL };
    reject(
        forged.request(&gateway, Method::GET, &accounts_path),
        StatusCode::FORBIDDEN,
        "forbidden",
    )
    .await?;
    for request in [
        LOCAL.request(&gateway, Method::GET, &accounts_path).header("x-agent-apps-token", OWNER),
        LOCAL.request(&gateway, Method::GET, &accounts_path).header(header::AUTHORIZATION, BASIC),
    ] {
        let (status, body) = response(request).await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "unauthenticated");
    }
    reject(
        LOCAL
            .request(&gateway, Method::GET, &accounts_path)
            .header("x-company-id", COMPANY.company),
        StatusCode::BAD_REQUEST,
        "invalid_input",
    )
    .await?;

    let refresh = CodexRefresh::new().unwrap();
    for target in [COMPANY, HOST, SERVICE] {
        reject(target.request(&gateway, Method::PUT, DEFAULT), StatusCode::NOT_FOUND, "not_found")
            .await?;
        let expected = if target.service == AuthService::CodexOauth {
            (StatusCode::NOT_FOUND, "not_found", AuthErrorCode::NotFound)
        } else {
            (StatusCode::NOT_IMPLEMENTED, "unsupported", AuthErrorCode::Unsupported)
        };
        reject(
            target.request(&gateway, Method::PUT, BINDING).json(&json!({"account_id":"primary"})),
            expected.0,
            expected.1,
        )
        .await?;
        let tenant = TenantScope::new("p", target.company)?;
        let result = accounts.codex_access_token(&refresh, target.scope(&tenant), "primary").await;
        assert!(
            result.is_err_and(|error| error == expected.2),
            "foreign scope must stop before OAuth"
        );
        assert_binding(&gateway, target, None).await?;
    }
    let tenant = TenantScope::new("p", LOCAL.company)?;
    let xai = XaiRefresh::new().unwrap();
    let result = accounts.xai_access_token(&xai, SERVICE.scope(&tenant), "primary").await;
    assert!(result.is_err_and(|error| error == AuthErrorCode::NotFound));
    let result = accounts.codex_access_token(&refresh, LOCAL.scope(&tenant), "primary").await;
    assert!(result.is_ok_and(|token| token == codex_token("primary")));
    assert_account(&gateway, LOCAL, "primary", true).await?;
    assert_binding(&gateway, LOCAL, Some("primary")).await?;
    check_flow_and_keychain(home, &gateway).await
}

struct KeychainItem(std::path::PathBuf);

impl Drop for KeychainItem {
    fn drop(&mut self) {
        let _ = MacOsKeychain.write(Some(&self.0), None);
    }
}

async fn check_flow_and_keychain(home: &Path, gateway: &Gateway) -> anyhow::Result<()> {
    let process = home.join("process");
    // Keep the actual child process blocked on stdin until the authorized cancel.
    fs::write(
        home.join("claude"),
        format!(
            r#"#!/bin/sh
set -eu
printf '%s\n%s\n' "$$" "$CLAUDE_CONFIG_DIR" > '{}'
umask 077
printf '%s' '{{"oauthAccount":{{"accountUuid":"fixture-user","organizationUuid":"fixture-org"}}}}' > .claude.json
"$BROWSER" 'https://claude.com/cai/oauth/authorize?code=true&state=fixture'
IFS= read -r code
"#,
            process.display()
        ),
    )?;
    for (target, status, code) in [
        (REMOTE, StatusCode::NOT_IMPLEMENTED, "execution_channel_unavailable"),
        (OFFLINE, StatusCode::SERVICE_UNAVAILABLE, "execution_host_unavailable"),
    ] {
        reject(
            target.request(gateway, Method::POST, FLOWS).json(&json!({"method":"copy_link"})),
            status,
            code,
        )
        .await?;
    }
    let remote_keychain = Target { service: AuthService::AgyOauth, ..REMOTE };
    reject(
        remote_keychain.request(gateway, Method::GET, &format!("{ROOT}/accounts")),
        StatusCode::NOT_IMPLEMENTED,
        "execution_channel_unavailable",
    )
    .await?;
    assert!(!process.exists(), "unavailable execution must not start the local CLI");

    let (status, flow) =
        send(CLAUDE.request(gateway, Method::POST, FLOWS).json(&json!({"method":"copy_link"})))
            .await?;
    assert_eq!(status, StatusCode::CREATED);
    let path = format!("{FLOWS}/{}", flow["flow_id"].as_str().unwrap());
    let contents = fs::read_to_string(&process)?;
    let (pid, directory) = contents.trim_end().split_once('\n').unwrap();
    let pid: libc::pid_t = pid.parse()?;
    assert!(MacOsKeychain.read(Some(Path::new(directory))).unwrap().is_none());
    let keychain = KeychainItem(directory.into());
    let payload = json!({"claudeAiOauth":{"refreshToken":PRIVATE}});
    MacOsKeychain.write(Some(&keychain.0), Some(&payload)).unwrap();

    for target in [
        Target { company: COMPANY.company, ..CLAUDE },
        Target { host: HOST.host, ..CLAUDE },
        SERVICE,
    ] {
        reject(target.request(gateway, Method::DELETE, &path), StatusCode::NOT_FOUND, "not_found")
            .await?;
    }
    assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
    assert!(MacOsKeychain.read(Some(&keychain.0)).unwrap().is_some_and(|stored| stored == payload));
    assert_eq!(send(CLAUDE.request(gateway, Method::GET, &path)).await?.1["state"], "pending");
    assert_eq!(
        send(CLAUDE.request(gateway, Method::DELETE, &path)).await?.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    assert!(!keychain.0.exists());
    assert!(MacOsKeychain.read(Some(&keychain.0)).unwrap().is_none());
    assert_eq!(send(CLAUDE.request(gateway, Method::GET, &path)).await?.1["state"], "cancelled");
    Ok(())
}

#[test]
fn auth6_security_commits_expected_scoped_result() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?.block_on(commits(home.path(), database))
}

#[test]
fn auth6_security_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?.block_on(rejects(home.path(), database))
}

#[test]
fn auth6_security_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(commits(home.path(), database.database().clone()))
}

#[test]
fn auth6_security_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(rejects(home.path(), database.database().clone()))
}
