use super::{
    agent_apps_auth_start::Runtime,
    agent_apps_auth_start_tests::{
        BASIC, CLAUDE, CODEX, FLOWS, Gateway, OTHER, OWNER, PASTE, READER,
    },
};
use agent_app_services::auth_accounts::AuthAccountScope;
use agent_database::{GlobalDatabase, test_support::PostgresTestDatabase};
use axum::http::{StatusCode, header};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use vw_agent_apps::auth::flow::AuthFlowTiming;
use {
    vw_api_app_auth::AuthErrorCode, vw_api_app_auth::AuthFlow, vw_api_app_auth::AuthFlowId,
    vw_api_app_auth::AuthService, vw_api_app_auth::AuthStartRequest, vw_api_transport::TimestampMs,
    vw_api_transport::id::TenantScope,
};

async fn get(
    gateway: &Gateway,
    path: &str,
    query: &str,
    token: &str,
    company: &str,
    basic: bool,
) -> anyhow::Result<(StatusCode, Value)> {
    let mut request = gateway
        .client
        .get(format!("http://{}{path}?{query}", gateway.addr))
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
    for secret in
        ["fixture-private", PASTE, "verification_url", "user_code", "access_token", "refresh_token"]
    {
        assert!(!raw.contains(secret));
    }
    Ok((status, serde_json::from_str(&raw).unwrap_or(Value::Null)))
}

fn check_isolated_poll(postgres: bool) -> anyhow::Result<()> {
    const CHILD_ROOT: &str = "SHIJU_AUTH_POLL_TEST_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let home = std::path::PathBuf::from(root);
        let postgres = postgres.then(PostgresTestDatabase::from_environment).transpose()?;
        let database = match &postgres {
            Some(fixture) => fixture.database().clone(),
            None => GlobalDatabase::for_home(&home)?,
        };
        return tokio::runtime::Runtime::new()?.block_on(check_poll(&home, database));
    }
    // Gateway execution uses runtime paths and PATH, rather than legacy policy hosts.
    // Set these before process initialization without mutating parallel tests' environment.
    let home = tempfile::tempdir()?;
    let path = std::env::join_paths(
        std::iter::once(home.path().to_path_buf())
            .chain([std::path::PathBuf::from("/usr/bin"), std::path::PathBuf::from("/bin")]),
    )?;
    let name = if postgres {
        "auth6_api_poll_commits_expected_scoped_result_postgres"
    } else {
        "auth6_api_poll_commits_expected_scoped_result"
    };
    let status = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            &format!("host_migration_3_tests::agent_apps_auth_poll_tests::{name}"),
            "--nocapture",
        ])
        .env(CHILD_ROOT, home.path())
        .env("VIBEWINDOW_TEST_ROOT", home.path())
        .env("HOME", home.path())
        .env("PATH", path)
        .env_remove("CLAUDE_CONFIG_DIR")
        .status()?;
    anyhow::ensure!(status.success(), "isolated poll test failed: {status}");
    Ok(())
}

#[test]
fn auth6_api_poll_commits_expected_scoped_result() -> anyhow::Result<()> {
    check_isolated_poll(false)
}

#[test]
fn auth6_api_poll_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    check_isolated_poll(true)
}

async fn check_poll(home: &Path, database: GlobalDatabase) -> anyhow::Result<()> {
    let gateway = Gateway::start_with_runtime(home, database, Arc::new(Runtime::default())).await?;
    let (status, started) =
        gateway.post(FLOWS, CLAUDE, OWNER, json!({"method":"copy_link"})).await?;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        started["verification_url"],
        "https://claude.com/cai/oauth/authorize?code=true&state=fixture"
    );
    let flow = started["flow_id"].as_str().unwrap();
    let path = format!("{FLOWS}/{flow}");
    for _ in 0..2 {
        let (status, pending) = get(&gateway, &path, CLAUDE, OWNER, "c", true).await?;
        assert_eq!(status, StatusCode::OK, "owner polls a pending flow");
        assert_eq!(
            pending,
            json!({
                "state":"pending", "flow_id":flow, "expires_at":started["expires_at"],
                "retry_after_seconds":started["retry_after_seconds"]
            })
        );
    }
    assert!(!home.join("delivered").exists());
    let (status, body) =
        gateway.post(&format!("{path}/paste"), CLAUDE, OWNER, json!({"pasted":PASTE})).await?;
    assert_eq!(status, StatusCode::ACCEPTED, "paste response: {body}");
    let approved = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let (status, body) = get(&gateway, &path, CLAUDE, OWNER, "c", true).await?;
            assert_eq!(status, StatusCode::OK);
            if body["state"] == "approved" {
                break Ok::<_, anyhow::Error>(body);
            }
            assert_eq!(body["state"], "pending");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await??;
    assert_eq!(approved["flow_id"], flow);
    assert_eq!(approved["expires_at"], started["expires_at"]);
    assert_eq!(approved["account"]["credential_state"], "ready");
    assert_eq!(approved["account"]["service"], "claude_official");
    assert_ne!(approved["account"]["account_id"], flow);
    assert!(matches!(
        serde_json::from_value::<AuthFlow>(approved.clone())?,
        AuthFlow::Approved { .. }
    ));
    assert_eq!(std::fs::read_to_string(home.join("delivered"))?, "delivered");
    let (status, accounts) =
        get(&gateway, "/v1/agent-apps/auth/accounts", CLAUDE, OWNER, "c", true).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(accounts["accounts"][0], approved["account"]);
    assert_eq!(get(&gateway, &path, CLAUDE, OWNER, "c", true).await?, (StatusCode::OK, approved));
    Ok(())
}

fn register(
    runtime: &Runtime,
    scope: AuthAccountScope<'_>,
    deadline: Instant,
    expires_at: TimestampMs,
) -> anyhow::Result<AuthFlowId> {
    let request = serde_json::from_value::<AuthStartRequest>(json!({"method":"copy_link"}))?;
    let timing = AuthFlowTiming { deadline, expires_at, retry_after_seconds: 1 };
    let pending = runtime.flows.start(scope, request, timing).expect("register test flow");
    let AuthFlow::Pending { flow_id, .. } = pending else {
        unreachable!("new flow must be pending")
    };
    Ok(flow_id)
}

#[test]
fn auth6_api_poll_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?.block_on(async {
        let runtime = Arc::new(Runtime::default());
        let tenant = TenantScope::new("p", "c")?;
        let scope = AuthAccountScope {
            tenant: &tenant,
            principal: "owner",
            host: "host",
            service: AuthService::ClaudeOfficial,
        };
        let deadline = Instant::now() + Duration::from_secs(60);
        let expires_at = TimestampMs(1000);
        let expired = register(&runtime, scope, deadline, expires_at)?;
        runtime.flows.reap_expired(deadline).expect("expire test flow");
        let failed = register(&runtime, scope, deadline, expires_at)?;
        runtime
            .flows
            .complete(
                scope,
                &failed,
                Instant::now,
                Err(AuthErrorCode::AuthorizationDenied),
                |_, _, _| panic!("denied authorization must not persist credentials"),
            )
            .expect("record provider denial");
        let cancelled = register(&runtime, scope, deadline, expires_at)?;
        runtime.flows.cancel(scope, &cancelled, Instant::now).expect("cancel test flow");
        let gateway = Gateway::start_with_runtime(home.path(), database, runtime).await?;
        let path = format!("{FLOWS}/{}", expired.0);
        let (status, denied) = get(&gateway, &path, CLAUDE, OTHER, "c", true).await?;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "another authorized principal cannot poll the flow"
        );
        assert_eq!(denied["code"], "not_found");
        let missing = format!("{FLOWS}/{}", uuid::Uuid::new_v4());
        let malformed = format!("{FLOWS}/invalid-flow");
        for (path, query, token, company, basic, expected, code) in [
            (&missing, CLAUDE, OWNER, "c", true, StatusCode::NOT_FOUND, "not_found"),
            (&malformed, CLAUDE, OWNER, "c", true, StatusCode::NOT_FOUND, "not_found"),
            (&path, CODEX, OWNER, "c", true, StatusCode::NOT_FOUND, "not_found"),
            (&path, CLAUDE, OWNER, "other", true, StatusCode::FORBIDDEN, "forbidden"),
            (&path, CLAUDE, READER, "c", true, StatusCode::FORBIDDEN, "forbidden"),
            (&path, CLAUDE, OWNER, "c", false, StatusCode::UNAUTHORIZED, "unauthenticated"),
            (&path, CLAUDE, "", "c", true, StatusCode::UNAUTHORIZED, "unauthenticated"),
            (&path, "host_id=host", OWNER, "c", true, StatusCode::BAD_REQUEST, "invalid_input"),
        ] {
            let (status, body) = get(&gateway, path, query, token, company, basic).await?;
            assert_eq!(status, expected);
            assert_eq!(body["code"], code);
            assert_ne!(body["correlation_id"], Value::Null);
            assert_eq!(body.as_object().unwrap().len(), 3);
        }
        for (id, state) in [(expired, "expired"), (failed, "failed"), (cancelled, "cancelled")] {
            let path = format!("{FLOWS}/{}", id.0);
            let (status, body) = get(&gateway, &path, CLAUDE, OWNER, "c", true).await?;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["state"], state);
            assert_eq!(body["flow_id"], id.0);
            assert_eq!(body["expires_at"], expires_at.0);
            if state == "failed" {
                assert_eq!(body["error"]["code"], "authorization_denied");
                assert!(body["error"]["correlation_id"].is_string());
            }
            serde_json::from_value::<AuthFlow>(body.clone())?;
            assert_eq!(get(&gateway, &path, CLAUDE, OWNER, "c", true).await?, (status, body));
        }
        let path = path.trim_start_matches("/v1");
        for path in [path.to_string(), format!("/api{path}")] {
            assert_eq!(
                get(&gateway, &path, CLAUDE, OWNER, "c", true).await?.0,
                StatusCode::NOT_FOUND
            );
        }
        Ok(())
    })
}
