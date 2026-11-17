use super::{
    agent_apps_auth_start::Runtime,
    agent_apps_auth_start_tests::{BASIC, FLOWS, Gateway, OWNER},
};
use {
    agent_app_services::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    app_provider_services::AppProviderStore,
    agent_database::{GlobalDatabase, run_sync_database, test_support::PostgresTestDatabase},
};
use axum::http::{Method, StatusCode, header};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, sync::Arc};
use vw_agent_apps::auth::{
    claude_keychain::{ClaudeKeychain, MacOsKeychain},
    codex_device::managed_identity,
    codex_refresh::CodexRefresh,
    credential_file::{AccountCredential, CredentialFileStore, XaiCredential},
    xai_refresh::XaiRefresh,
};
use {
    vw_api_transport::TimestampMs,
    vw_api_app_identity::AppId,
    vw_api_app_auth::AuthErrorCode,
    vw_api_app_auth::AuthService,
    vw_api_transport::id::TenantScope,
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

const LOCAL: Target = Target { company: "c", host: "desktop-local", service: AuthService::CodexOauth };
const COMPANY: Target = Target { company: "neighbor", ..LOCAL };
const HOST: Target = Target { host: "second-host", ..LOCAL };
const SERVICE: Target = Target { service: AuthService::XaiOauth, ..LOCAL };
const CLAUDE: Target = Target { service: AuthService::ClaudeOfficial, ..LOCAL };

impl Target {
    fn scope(self, tenant: &TenantScope) -> AuthAccountScope<'_> {
        AuthAccountScope { tenant, principal: "desktop-local-user", host: self.host, service: self.service }
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
    // Resolve the same tenant directory as the local Gateway, inside the child test root.
    let root = local_app_auth::auth_policy_path(&agent_paths::paths().home, "p", "c")
        .with_file_name("auth-credentials");
    fs::create_dir_all(&root)?;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    let store = AuthAccountStore::new(database.clone(), &root).map_err(anyhow::Error::msg)?;
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
            let tenant_root = local_app_auth::auth_policy_path(&agent_paths::paths().home, "p", target.company)
                .with_file_name("auth-credentials");
            fs::create_dir_all(&tenant_root)?;
            fs::set_permissions(&tenant_root, fs::Permissions::from_mode(0o700))?;
            let credentials = CredentialFileStore::new(&tenant_root).map_err(anyhow::Error::msg)?;
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
    let (gateway, accounts) = setup(home, database.clone()).await?;
    let selections =
        [(LOCAL, "primary"), (COMPANY, "company"), (SERVICE, "service")];
    for (target, id) in selections {
        select_default(&gateway, target, id).await?;
    }
    // host_id is legacy input: the Gateway always executes as its local host.
    select_default(&gateway, HOST, "primary").await?;
    bind(&gateway, LOCAL, "primary").await?;
    bind(&gateway, COMPANY, "company").await?;
    for (target, id) in selections {
        assert_account(&gateway, target, id, true).await?;
    }
    for (target, id) in
        [(LOCAL, Some("primary")), (COMPANY, Some("company")), (SERVICE, None)]
    {
        assert_binding(&gateway, target, id).await?;
    }
    let refresh = CodexRefresh::new().unwrap();
    for (target, id) in [(LOCAL, "primary"), (COMPANY, "company"), (HOST, "host")] {
        let tenant = TenantScope::new("p", target.company)?;
        let root = local_app_auth::auth_policy_path(&agent_paths::paths().home, "p", target.company)
            .with_file_name("auth-credentials");
        let scoped_accounts = AuthAccountStore::new(database.clone(), &root).map_err(anyhow::Error::msg)?;
        let result = scoped_accounts.codex_access_token(&refresh, target.scope(&tenant), id).await;
        assert!(result.is_ok_and(|token| token == codex_token(id)), "token belongs to {id}");
    }
    let tenant = TenantScope::new("p", SERVICE.company)?;
    let xai = XaiRefresh::new().unwrap();
    let result = accounts.xai_access_token(&xai, SERVICE.scope(&tenant), "service").await;
    assert!(result.is_ok_and(|token| token == PRIVATE), "xAI resolves its own token");
    Ok(())
}

async fn rejects(home: &Path, database: GlobalDatabase) -> anyhow::Result<()> {
    let (gateway, accounts) = setup(home, database.clone()).await?;
    select_default(&gateway, LOCAL, "primary").await?;
    // host_id is legacy input: the Gateway always executes as its local host.
    select_default(&gateway, HOST, "primary").await?;
    bind(&gateway, LOCAL, "primary").await?;

    // Membership and pairing are enforced by the enclosing Gateway middleware;
    // application routes still reject ambiguous tenant headers.
    reject(
        LOCAL
            .request(&gateway, Method::GET, &format!("{ROOT}/accounts"))
            .header("x-company-id", COMPANY.company),
        StatusCode::BAD_REQUEST,
        "invalid_input",
    )
    .await?;

    let refresh = CodexRefresh::new().unwrap();
    for target in [COMPANY, SERVICE] {
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
    assert!(accounts.codex_access_token(&refresh, HOST.scope(&tenant), "primary").await
        .is_err_and(|error| error == AuthErrorCode::NotFound));
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
    let bin = agent_paths::paths().bin.clone();
    fs::create_dir_all(&bin)?;
    fs::copy(home.join("claude"), bin.join("claude"))?;
    fs::set_permissions(bin.join("claude"), fs::Permissions::from_mode(0o700))?;
    assert!(!process.exists(), "the CLI starts only when requested");

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

fn isolated_test() -> anyhow::Result<bool> {
    const CHILD: &str = "SHIJU_AUTH_SECURITY_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return Ok(false);
    }
    let root = tempfile::tempdir()?;
    let home = root.path().canonicalize()?;
    fs::create_dir(home.join("bin"))?;
    // Preserve the real OS keychain boundary while isolating application files.
    let keychain = std::process::Command::new("/usr/bin/security")
        .arg("default-keychain")
        .output()?;
    anyhow::ensure!(keychain.status.success(), "resolve default test keychain");
    let keychain = String::from_utf8(keychain.stdout)?;
    let keychain = Path::new(keychain.trim().trim_matches('"'));
    fs::create_dir_all(home.join("Library/Keychains"))?;
    std::os::unix::fs::symlink(keychain, home.join("Library/Keychains/login.keychain-db"))?;
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", std::thread::current().name().unwrap(), "--nocapture"])
        .env(CHILD, "1")
        .env("HOME", &home)
        .env("VIBEWINDOW_TEST_ROOT", &home)
        .env("PATH", std::env::join_paths([home.join("bin"), "/usr/bin".into(), "/bin".into()])?)
        .env_remove("CLAUDE_CONFIG_DIR")
        .status()?;
    anyhow::ensure!(status.success(), "isolated security child failed: {status}");
    Ok(true)
}

#[test]
fn auth6_security_commits_expected_scoped_result() -> anyhow::Result<()> {
    if isolated_test()? { return Ok(()); }
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?.block_on(commits(home.path(), database))
}

#[test]
fn auth6_security_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    if isolated_test()? { return Ok(()); }
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?.block_on(rejects(home.path(), database))
}

#[test]
fn auth6_security_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    if isolated_test()? { return Ok(()); }
    let home = tempfile::tempdir()?;
    let database = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(commits(home.path(), database.database().clone()))
}

#[test]
fn auth6_security_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    if isolated_test()? { return Ok(()); }
    let home = tempfile::tempdir()?;
    let database = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(rejects(home.path(), database.database().clone()))
}
