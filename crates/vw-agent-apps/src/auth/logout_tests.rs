use super::{
    copilot_tests::oauth_server,
    dsh_account_tests::{
        Fixture, GRANT, ISSUER, OLD_TOKEN, document, grant, init_server, route, start_request,
    },
    flow::{AuthAccountScope, AuthFlowTiming, AuthFlows},
    logout::LogoutSelection,
};
use serde_json::json;
use std::time::{Duration, Instant};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthFlow, AuthStartResponse},
};

const INIT: &str = "/auth-api/v0/dsh/auth_init";
const LOGOUT: &str = "/auth-api/v0/users/logout";

#[tokio::test]
async fn auth6_logout_commits_expected_scoped_result() {
    let fixture = Fixture::new();
    let account = fixture.account();
    let flows = AuthFlows::default();
    let scope = fixture.scope();
    let initial = document();
    fixture.write(&initial);

    let (status, captured_status) =
        oauth_server("200 OK", "", json!({"code": 0, "data": {"biz_data": {}}})).await;
    let id = account
        .status_with("en_US", 0, route("/auth-api/v0/users/current", status))
        .await
        .unwrap()
        .unwrap()
        .account_id;
    captured_status.await.unwrap();

    let (server, request) = init_server().await;
    let (cancel_server, cancel_request) = oauth_server("200 OK", "", json!({"code": 0})).await;
    let init = route(INIT, server);
    let cancel = route("/auth-api/v0/dsh/auth_cancel", cancel_server);
    let login = account
        .start_login_with(&flows, start_request(), true, "en_US", 0, move |request| {
            if request.url().path() == INIT {
                init(request);
            } else {
                cancel(request);
            }
        })
        .await
        .unwrap();
    let wire = request.await.unwrap();
    let init_body: serde_json::Value =
        serde_json::from_str(wire.split_once("\r\n\r\n").unwrap().1).unwrap();
    let callback = init_body["redirect_uri"].as_str().unwrap();
    let AuthStartResponse::Pending { flow_id, .. } = login.response();

    assert_eq!(account.logout(&flows, scope, LogoutSelection::Account(&id)), Ok(()));
    assert_eq!(account.logout(&flows, scope, LogoutSelection::All), Ok(()));
    assert!(matches!(flows.snapshot(scope, flow_id, Instant::now), Ok(AuthFlow::Cancelled { .. })));
    let mut expected = initial.clone();
    expected["records"].as_object_mut().unwrap().remove(GRANT);
    assert_eq!(fixture.read(), expected);
    let cancel_wire = cancel_request.await.unwrap();
    assert!(cancel_wire.starts_with("POST /auth-api/v0/dsh/auth_cancel "));
    assert!(cancel_wire.contains("fixture-attempt"));
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    assert!(client.get(callback).send().await.unwrap_err().is_connect());

    fixture.write(&initial);
    let (server, captured) = oauth_server("200 OK", "", json!({"code": 0})).await;
    assert_eq!(
        account
            .revoke_with(&flows, scope, &id, "en_US", 28800, |request| {
                assert_eq!(fixture.read(), expected);
                assert!(request.headers()["x-dsh-auth-token"].is_sensitive());
                route(LOGOUT, server)(request);
            })
            .await,
        Ok(())
    );
    let request = captured.await.unwrap().to_ascii_lowercase();
    assert!(request.starts_with("post /auth-api/v0/users/logout http/1.1\r\n"));
    for header in [
        format!("x-dsh-auth-token: {OLD_TOKEN}"),
        "x-client-platform: web".into(),
        "x-client-bundle-id:".into(),
        "x-client-locale: en_us".into(),
        "x-client-timezone-offset: 28800".into(),
    ] {
        assert!(request.contains(&header));
    }
}

#[tokio::test]
async fn auth6_logout_rejects_invalid_or_stale_input() {
    let fixture = Fixture::new();
    let scope = fixture.scope();
    let foreign = AuthAccountScope { principal: "oidc:other", ..scope };
    let account = fixture.account();
    let flows = AuthFlows::default();
    let mut initial = document();
    initial["records"][GRANT] = grant(OLD_TOKEN, "https://example.invalid");
    fixture.write(&initial);
    assert_eq!(
        account.logout(&flows, foreign, LogoutSelection::All),
        Err(AuthErrorCode::Forbidden)
    );
    assert_eq!(
        account.logout(&flows, scope, LogoutSelection::Default),
        Err(AuthErrorCode::Unsupported)
    );
    assert_eq!(
        account.logout(&flows, scope, LogoutSelection::Account("../escape")),
        Err(AuthErrorCode::InvalidInput)
    );
    assert_eq!(fixture.read(), initial);

    for service in [
        vw_api_types::agent_apps_auth::AuthService::AgyOauth,
        vw_api_types::agent_apps_auth::AuthService::ClaudeOfficial,
        vw_api_types::agent_apps_auth::AuthService::CodexOauth,
        vw_api_types::agent_apps_auth::AuthService::GithubCopilot,
        vw_api_types::agent_apps_auth::AuthService::KimiOauth,
        vw_api_types::agent_apps_auth::AuthService::XaiOauth,
    ] {
        assert_eq!(
            super::logout::check_revocation_support(AuthAccountScope { service, ..scope }),
            Err(AuthErrorCode::Unsupported)
        );
    }
    let id = super::dsh_login::account(
        scope,
        vw_api_types::agent_apps_auth::AuthCredentialState::Ready,
        None,
    )
    .unwrap()
    .account_id;
    assert_eq!(
        account
            .revoke_with(&flows, scope, &id, "en_US", 0, |_| panic!(
                "foreign issuer must not be contacted"
            ))
            .await,
        Err(AuthErrorCode::ReauthRequired)
    );
    assert_eq!(fixture.read(), initial);
    initial["records"][GRANT] = grant(OLD_TOKEN, ISSUER);
    fixture.write(&initial);

    let lock = fixture.credentials.with_file_name(".credentials.yaml.lock");
    std::fs::write(&lock, b"fixture-lock").unwrap();
    assert_eq!(
        account
            .revoke_with(&flows, scope, &id, "en_US", 0, |_| panic!(
                "lock conflict must stop revocation"
            ))
            .await,
        Err(AuthErrorCode::Conflict)
    );
    assert_eq!(fixture.read(), initial);
    std::fs::remove_file(&lock).unwrap();

    let mut rotated = initial.clone();
    rotated["records"][GRANT] = grant("fixture-rotated-grant", ISSUER);
    let path = fixture.credentials.clone();
    let yaml = serde_yaml::to_string(&rotated).unwrap();
    let attempt = flows
        .start_with_cleanup(
            scope,
            start_request(),
            AuthFlowTiming {
                expires_at: TimestampMs(1000),
                deadline: Instant::now() + Duration::from_secs(30),
                retry_after_seconds: 1,
            },
            move || std::fs::write(&path, &yaml).map_err(|_| AuthErrorCode::StorageUnavailable),
        )
        .unwrap();
    let AuthFlow::Pending { flow_id, .. } = attempt else {
        panic!("expected pending flow");
    };
    assert_eq!(
        account
            .revoke_with(&flows, scope, &id, "en_US", 0, |_| panic!(
                "grant changed before local invalidation"
            ))
            .await,
        Err(AuthErrorCode::Conflict)
    );
    assert!(matches!(
        flows.snapshot(scope, &flow_id, Instant::now),
        Ok(AuthFlow::Cancelled { .. })
    ));
    assert_eq!(fixture.read(), rotated);

    let (server, captured) = oauth_server("200 OK", "", json!({"code": 40003})).await;
    assert_eq!(
        account.revoke_with(&flows, scope, &id, "en_US", 0, route(LOGOUT, server)).await,
        Err(AuthErrorCode::RemoteRevocationUnconfirmed)
    );
    captured.await.unwrap();
    assert!(fixture.read()["records"].get(GRANT).is_none());
    assert_eq!(
        account
            .revoke_with(&flows, scope, &id, "en_US", 0, |_| panic!(
                "missing grant must not send a request"
            ))
            .await,
        Err(AuthErrorCode::Conflict)
    );

    fixture.write(&initial);
    let (redirect_target, mut redirected) = oauth_server("200 OK", "", json!({"code": 0})).await;
    let headers = format!("Location: {redirect_target}\r\n");
    let (server, captured) =
        oauth_server("307 Temporary Redirect", &headers, json!({"code": 0})).await;
    assert_eq!(
        account.revoke_with(&flows, scope, &id, "en_US", 0, route(LOGOUT, server)).await,
        Err(AuthErrorCode::RemoteRevocationUnconfirmed)
    );
    captured.await.unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(100), &mut redirected).await.is_err());
    redirected.abort();
    assert!(fixture.read()["records"].get(GRANT).is_none());
}

#[tokio::test]
async fn auth6_logout_agy_keychain_clears_owned_credentials_and_default() {
    use super::{
        agy_account::logout_accounts_in,
        agy_account_tests::{Fixture, credential, scope},
    };

    let mut fixture = Fixture::new();
    let first = fixture.managed("first", &credential("first", 1), "ready");
    let spare = fixture.managed("spare", &credential("spare", 1), "ready");
    let mut other = Fixture::new();
    let foreign = other.managed("spare", &credential("spare", 1), "ready");
    fixture.switch(&first).unwrap();
    let scope = scope(&fixture.tenant);
    let flows = AuthFlows::default();
    let entry = fixture.entry();
    let default = LogoutSelection::Default;
    assert_eq!(
        logout_accounts_in(&flows, scope, default, fixture.target(None), &entry),
        Err(AuthErrorCode::Forbidden)
    );
    assert!(entry.read().unwrap().is_some());
    let target = fixture.target(Some(scope));
    let (mut server, request) =
        oauth_server("200 OK", "", json!({"access_token": "late-agy-access", "expires_in": 3600}))
            .await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let refresh = super::agy_refresh::refresh_with(scope, &first, &client, |outbound| {
        assert_eq!(outbound.url().as_str(), "https://oauth2.googleapis.com/token");
        // The adapter has captured its keychain generation and dropped the lock.
        assert_eq!(logout_accounts_in(&flows, scope, default, target, &entry), Ok(()));
        server.set_path(outbound.url().path());
        *outbound.url_mut() = server;
    })
    .await;
    assert_eq!(refresh.as_ref().err(), Some(&AuthErrorCode::Conflict));
    assert!(request.await.unwrap().starts_with("POST /token HTTP/1.1"));
    assert!(entry.read().unwrap().is_none());
    let record = fixture.record(&first);
    assert_eq!(record["account"]["credential_state"], "signed_out");
    assert_eq!(record["account"]["account_id"], first);
    assert!(record["credential"].is_null());
    assert_eq!(fixture.record(&spare)["account"]["credential_state"], "ready");
    assert_eq!(
        logout_accounts_in(&flows, scope, default, target, &entry),
        Err(AuthErrorCode::NotFound)
    );
    assert_eq!(logout_accounts_in(&flows, scope, LogoutSelection::All, target, &entry), Ok(()));
    assert_eq!(fixture.record(&spare)["account"]["credential_state"], "signed_out");
    assert_eq!(other.record(&foreign)["account"]["credential_state"], "ready");
}

#[tokio::test]
async fn auth6_logout_kimi_only_clears_the_matching_native_grant() {
    use super::{
        credential_file::AccountCredential,
        kimi_account_tests::{ACCOUNT, CONFIG, Fixture, native_tokens},
    };
    use vw_api_types::agent_apps_auth::AuthRegion;

    let fixture = Fixture::new();
    let native = fixture.native();
    let target = fixture.target(AuthRegion::Global);
    let ids = [ACCOUNT.to_owned()];
    assert_eq!(
        native.default_managed_account(&fixture.credentials, target.scope, &ids),
        Err(AuthErrorCode::NotFound)
    );
    let config = CONFIG.replace("default_model = \"original\"", "default_model = \"chosen\"");
    std::fs::write(fixture.config(), &config).unwrap();
    assert_eq!(
        native.default_managed_account(&fixture.credentials, target.scope, &ids),
        Ok(ACCOUNT.to_owned())
    );

    let lease = fixture.credentials.lease_kimi(target.scope, ACCOUNT, false).unwrap();
    let independent = native_tokens("independent", 2);
    fixture.write_native(&independent);
    native.clear_matching_grant(target.scope, ACCOUNT, AuthRegion::Global, &lease).unwrap();
    let read_grant = || {
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(fixture.credential()).unwrap())
            .unwrap()
    };
    assert_eq!(read_grant(), independent);
    fixture.write_native(&native_tokens("fixture-expired", 1));
    native.clear_matching_grant(target.scope, ACCOUNT, AuthRegion::Global, &lease).unwrap();
    assert!(!fixture.credential().exists());
    assert_eq!(std::fs::read_to_string(fixture.config()).unwrap(), config);
    assert!(matches!(lease.read().unwrap(), AccountCredential::KimiOauth(_)));
}

#[tokio::test]
async fn auth6_revoke_race_commits_expected_scoped_result() {
    use super::codex_refresh_tests as codex;
    use vw_api_types::agent_apps::AppId;

    let tenant = vw_api_types::id::TenantScope::new("platform", "company").unwrap();
    let scope = codex::scope(&tenant);
    if let Some((root, action)) = revoke_race_child() {
        let store = super::credential_file::CredentialFileStore::new(&root).unwrap();
        if action == "invalidate" {
            super::logout::clear_managed_credentials(&store, scope, "alpha", [0; 32], None, || {
                Ok(())
            })
            .expect("logout should commit while the response is pending");
        } else {
            let server = reqwest::Url::parse(&action).unwrap();
            let refresh = super::codex_refresh::CodexRefresh::new().unwrap();
            assert_eq!(
                refresh
                    .access_token_with(&store, scope, "alpha", |request| {
                        codex::route(request, &server);
                    })
                    .await
                    .as_ref()
                    .err(),
                Some(&AuthErrorCode::Conflict),
                "a successful OAuth response for a signed-out account must be discarded"
            );
        }
        return;
    }

    let (root, store) = super::xai_refresh_tests::store();
    codex::seed(&store, scope, "alpha");
    let other = AuthAccountScope { principal: "oidc:other", ..scope };
    let untouched = codex::seed(&store, other, "alpha");
    let (server, seen, release, server_task) = codex::gated_refresh(json!({
        "access_token": "late-access", "refresh_token": "late-refresh", "expires_in": 3600
    }))
    .await;
    let mut refresh = spawn_revoke_race_child(
        "auth6_revoke_race_commits_expected_scoped_result",
        root.path(),
        server.as_str(),
    );
    tokio::time::timeout(Duration::from_secs(10), seen).await.unwrap().unwrap();
    let mut logout = spawn_revoke_race_child(
        "auth6_revoke_race_commits_expected_scoped_result",
        root.path(),
        "invalidate",
    );
    let status = wait_revoke_race_child(&mut logout).await;
    release.send(()).unwrap();
    let refreshed = wait_revoke_race_child(&mut refresh).await;
    server_task.abort();
    assert!(status.success(), "logout subprocess must commit");
    assert!(refreshed.success(), "refresh subprocess must reject its late response");
    assert_eq!(
        store
            .for_owner(scope.principal)
            .unwrap()
            .read(scope.tenant, scope.host, AppId::Codex, "alpha")
            .err()
            .as_deref(),
        Some("official account not found"),
    );
    assert_eq!(codex::saved(&store, other, "alpha"), untouched);
}

#[tokio::test]
async fn auth6_revoke_race_rejects_invalid_or_stale_input() {
    use super::xai_refresh_tests as xai;
    use tokio::io::AsyncWriteExt;

    let tenant = vw_api_types::id::TenantScope::new("platform", "company").unwrap();
    let scope = xai::scope(&tenant);
    if let Some((root, action)) = revoke_race_child() {
        let store = super::credential_file::CredentialFileStore::new(&root).unwrap();
        if action == "invalidate" {
            let lease = store
                .lease_xai(scope, xai::ACCOUNT, false)
                .expect("deletion should acquire the account while OAuth is pending");
            super::accounts::remove_managed_oauth_file(&lease).unwrap();
            assert_eq!(lease.read().err().as_deref(), Some("official account not found"));
            drop(lease);
            // Recreating the same account with identical bytes must not validate the old response.
            xai::seed(&store, scope, xai::ACCOUNT, 1);
        } else {
            let token = reqwest::Url::parse(&action).unwrap();
            let (discover, discovery) = xai::discovery().await;
            let result = super::xai_refresh::XaiRefresh::new()
                .unwrap()
                .access_token_with(&store, scope, xai::ACCOUNT, |request| {
                    xai::route(request, &discover, &token);
                })
                .await;
            assert!(discovery.await.unwrap().starts_with("GET /.well-known/openid-configuration "));
            assert_eq!(result.as_ref().err(), Some(&AuthErrorCode::Conflict));
        }
        return;
    }

    let (root, store) = xai::store();
    xai::seed(&store, scope, xai::ACCOUNT, 1);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let test = "auth6_revoke_race_rejects_invalid_or_stale_input";
    let mut refresh = spawn_revoke_race_child(test, root.path(), &url);
    let (mut stream, _) =
        tokio::time::timeout(Duration::from_secs(10), listener.accept()).await.unwrap().unwrap();
    let request = tokio::time::timeout(
        Duration::from_secs(5),
        super::copilot_tests::read_request(&mut stream),
    )
    .await
    .unwrap();
    assert!(request.starts_with("POST /oauth2/token HTTP/1.1\r\n"));
    assert!(request.ends_with("grant_type=refresh_token&client_id=b1a00492-073a-47ea-816f-4c329264a828&refresh_token=refresh-fixture-account&scope=openid+profile+email+offline_access+grok-cli%3Aaccess+api%3Aaccess"));
    let mut delete = spawn_revoke_race_child(test, root.path(), "invalidate");
    let deleted = wait_revoke_race_child(&mut delete).await;
    let reply = r#"{"error":"invalid_grant"}"#;
    stream.write_all(format!(
        "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len(),
    ).as_bytes()).await.unwrap();
    let refreshed = wait_revoke_race_child(&mut refresh).await;
    assert!(deleted.success(), "deletion subprocess must commit");
    assert!(refreshed.success(), "refresh subprocess must reject its late response");
    let replacement = xai::saved(&store, scope, xai::ACCOUNT);
    assert_eq!(replacement.expires_at.0, 1);
    assert!(!replacement.requires_reauth);
}

fn revoke_race_child() -> Option<(std::path::PathBuf, String)> {
    let root = std::env::var_os("SHIJU_REVOKE_RACE_ROOT")?;
    Some((root.into(), std::env::var("SHIJU_REVOKE_RACE_ACTION").unwrap()))
}

fn spawn_revoke_race_child(
    test: &str,
    root: &std::path::Path,
    action: &str,
) -> tokio::process::Child {
    tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("auth::logout_tests::{test}"), "--nocapture"])
        .env("SHIJU_REVOKE_RACE_ROOT", root)
        .env("SHIJU_REVOKE_RACE_ACTION", action)
        .kill_on_drop(true)
        .spawn()
        .unwrap()
}

async fn wait_revoke_race_child(child: &mut tokio::process::Child) -> std::process::ExitStatus {
    tokio::time::timeout(Duration::from_secs(10), child.wait()).await.unwrap().unwrap()
}
