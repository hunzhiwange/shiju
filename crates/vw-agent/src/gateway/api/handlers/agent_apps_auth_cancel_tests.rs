use super::{
    agent_apps_auth_start::Runtime,
    agent_apps_auth_start_tests::{
        BASIC, CLAUDE, CODEX, FLOWS, Gateway, OTHER, OWNER, PASTE, READER,
    },
};
use crate::{
    agent_apps::auth_accounts::AuthAccountScope,
    global_database::{GlobalDatabase, test_support::PostgresTestDatabase},
};
use axum::http::{Method, StatusCode, header};
use serde_json::{Value, json};
use std::{fs, path::Path, sync::Arc, time::Instant};
use vw_agent_apps::auth::claude_keychain::{ClaudeKeychain, MacOsKeychain};
use vw_api_types::{
    agent_apps_auth::{AuthAccount, AuthCredentialState, AuthErrorCode, AuthFlowId, AuthService},
    id::TenantScope,
};

async fn request(
    gateway: &Gateway,
    method: Method,
    path: &str,
    query: &str,
    token: &str,
    company: &str,
    basic: bool,
) -> anyhow::Result<(StatusCode, Value)> {
    let mut request = gateway
        .client
        .request(method, format!("http://{}{path}?{query}", gateway.addr))
        .header("x-platform-id", "p")
        .header("x-company-id", company)
        .header("x-agent-apps-token", token);
    if basic {
        request = request.header(header::AUTHORIZATION, BASIC);
    }
    let response = request.send().await?;
    let status = response.status();
    if path.starts_with(FLOWS) {
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    let raw = response.text().await?;
    if status == StatusCode::NO_CONTENT {
        assert!(raw.is_empty());
    }
    if status.is_client_error() {
        assert!(!raw.contains(path.rsplit('/').next().unwrap()));
    }
    for secret in [PASTE, "fixture-private", "verification_url", "user_code", "access_token"] {
        assert!(!raw.contains(secret));
    }
    Ok((status, serde_json::from_str(&raw).unwrap_or(Value::Null)))
}

#[test]
fn auth6_api_cancel_commits_expected_scoped_result() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?.block_on(check_cancel(home.path(), database))
}

#[test]
fn auth6_api_cancel_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(check_cancel(home.path(), database.database().clone()))
}

async fn check_cancel(home: &Path, database: GlobalDatabase) -> anyhow::Result<()> {
    let runtime = Arc::new(Runtime::default());
    let gateway = Gateway::start_with_runtime(home, database, Arc::clone(&runtime)).await?;
    fs::write(
        home.join("claude"),
        format!(
            r#"#!/bin/sh
set -eu
printf '%s\n%s\n' "$$" "$CLAUDE_CONFIG_DIR" > '{}'
umask 077
printf '%s' '{{"fixture":"fixture-private"}}' > .credentials.json
printf '%s' '{{"oauthAccount":{{"accountUuid":"fixture-user","organizationUuid":"fixture-org"}}}}' > .claude.json
"$BROWSER" 'https://claude.com/cai/oauth/authorize?code=true&state=fixture'
IFS= read -r code
printf delivered > '{}'
"#,
            home.join("login-process").display(),
            home.join("delivered").display(),
        ),
    )?;
    let (status, started) =
        gateway.post(FLOWS, CLAUDE, OWNER, json!({"method":"copy_link"})).await?;
    assert_eq!(status, StatusCode::CREATED);
    let flow = started["flow_id"].as_str().unwrap();
    let path = format!("{FLOWS}/{flow}");
    let process = fs::read_to_string(home.join("login-process"))?;
    let mut lines = process.lines();
    let pid: libc::pid_t = lines.next().unwrap().parse()?;
    let directory = Path::new(lines.next().unwrap());
    assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
    assert!(directory.join(".credentials.json").is_file());
    assert!(MacOsKeychain.read(Some(directory)).unwrap().is_none());
    struct KeychainFixture<'a>(&'a Path);
    impl Drop for KeychainFixture<'_> {
        fn drop(&mut self) {
            let _ = MacOsKeychain.write(Some(self.0), None);
        }
    }
    let _keychain = KeychainFixture(directory);
    let payload = json!({"claudeAiOauth":{"refreshToken":"fixture-private"}});
    MacOsKeychain.write(Some(directory), Some(&payload)).unwrap();
    assert_eq!(MacOsKeychain.read(Some(directory)).unwrap(), Some(payload));

    for _ in 0..2 {
        let (status, body) =
            request(&gateway, Method::DELETE, &path, CLAUDE, OWNER, "c", true).await?;
        assert_eq!(status, StatusCode::NO_CONTENT, "owner confirms local cancellation: {body}");
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
        assert!(!directory.exists());
        assert!(MacOsKeychain.read(Some(directory)).unwrap().is_none());
    }
    let (status, snapshot) =
        request(&gateway, Method::GET, &path, CLAUDE, OWNER, "c", true).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        snapshot,
        json!({"state":"cancelled", "flow_id":flow, "expires_at":started["expires_at"]})
    );

    let tenant = TenantScope::new("p", "c")?;
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "host",
        service: AuthService::ClaudeOfficial,
    };
    let late_account = AuthAccount {
        account_id: "late-account".into(),
        service: scope.service,
        label: "Late account".into(),
        credential_state: AuthCredentialState::Ready,
        is_default: false,
        authenticated_at: None,
        login_expires_at: None,
        github_domain: None,
        region: None,
    };
    assert_eq!(
        runtime.flows.complete(
            scope,
            &AuthFlowId(flow.into()),
            Instant::now,
            Ok(late_account),
            |_, _, _| panic!("late completion must not reach persistence")
        ),
        Err(AuthErrorCode::Conflict)
    );
    assert_eq!(
        gateway.post(&format!("{path}/paste"), CLAUDE, OWNER, json!({"pasted":PASTE})).await?.0,
        StatusCode::CONFLICT
    );
    assert!(!home.join("delivered").exists());
    let (status, accounts) =
        request(&gateway, Method::GET, "/v1/agent-apps/auth/accounts", CLAUDE, OWNER, "c", true)
            .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(accounts["accounts"], json!([]));
    Ok(())
}

#[test]
fn auth6_api_cancel_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?.block_on(async {
        let gateway =
            Gateway::start_with_runtime(home.path(), database, Arc::new(Runtime::default()))
                .await?;
        let policy = home.path().join("policy.json");
        let mut fixture: Value = serde_json::from_slice(&fs::read(&policy)?)?;
        let grants = fixture["principals"][0]["grants"].as_array_mut().unwrap();
        let mut second_tenant = grants[0].clone();
        second_tenant["company_id"] = "other-member".into();
        grants.push(second_tenant);
        fs::write(&policy, serde_json::to_vec(&fixture)?)?;

        let (status, started) =
            gateway.post(FLOWS, CLAUDE, OWNER, json!({"method":"copy_link"})).await?;
        assert_eq!(status, StatusCode::CREATED);
        let path = format!("{FLOWS}/{}", started["flow_id"].as_str().unwrap());
        let invalid = format!("{FLOWS}/invalid-flow");
        for (path, query, token, company, basic, expected, code) in [
            (&path, CLAUDE, OWNER, "other-member", true, StatusCode::NOT_FOUND, "not_found"),
            (&path, CLAUDE, OTHER, "c", true, StatusCode::NOT_FOUND, "not_found"),
            (&path, CODEX, OWNER, "c", true, StatusCode::NOT_FOUND, "not_found"),
            (&invalid, CLAUDE, OWNER, "c", true, StatusCode::NOT_FOUND, "not_found"),
            (&path, CLAUDE, OWNER, "other", true, StatusCode::FORBIDDEN, "forbidden"),
            (&path, CLAUDE, READER, "c", true, StatusCode::FORBIDDEN, "forbidden"),
            (&path, CLAUDE, OWNER, "c", false, StatusCode::UNAUTHORIZED, "unauthenticated"),
            (&path, "host_id=host", OWNER, "c", true, StatusCode::BAD_REQUEST, "invalid_input"),
        ] {
            let (status, body) =
                request(&gateway, Method::DELETE, path, query, token, company, basic).await?;
            assert_eq!(status, expected, "scoped DELETE rejection: {code}");
            assert_eq!(body["code"], code);
            assert!(body["correlation_id"].is_string());
            assert_eq!(body.as_object().unwrap().len(), 3);
        }
        let unversioned = path.trim_start_matches("/v1");
        for alias in [unversioned.to_string(), format!("/api{unversioned}")] {
            assert_eq!(
                request(&gateway, Method::DELETE, &alias, CLAUDE, OWNER, "c", true).await?.0,
                StatusCode::NOT_FOUND
            );
        }
        assert_eq!(
            gateway.post(&format!("{path}/paste"), CLAUDE, OWNER, json!({"pasted":PASTE})).await?.0,
            StatusCode::ACCEPTED
        );
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let (status, body) =
                    request(&gateway, Method::GET, &path, CLAUDE, OWNER, "c", true).await?;
                assert_eq!(status, StatusCode::OK);
                if body["state"] == "approved" {
                    break Ok::<_, anyhow::Error>(());
                }
                assert_eq!(body["state"], "pending");
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await??;
        let (status, body) =
            request(&gateway, Method::DELETE, &path, CLAUDE, OWNER, "c", true).await?;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], "conflict");
        let (status, body) =
            request(&gateway, Method::GET, &path, CLAUDE, OWNER, "c", true).await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["state"], "approved");
        Ok(())
    })
}
