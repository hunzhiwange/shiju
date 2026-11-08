use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

fn ready_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture
        .credentials
        .lease_kimi(fixture.target(AuthRegion::Global).scope, ACCOUNT, false)
        .unwrap()
        .save(&AccountCredential::KimiOauth(KimiCredential {
            access_token: "fixture-current-access".into(),
            refresh_token: "fixture-current-refresh".into(),
            expires_at: TimestampMs(FUTURE * 1_000),
            device_id: DEVICE.into(),
            region: AuthRegion::Global,
        }))
        .unwrap();
    fixture.write_native(&native_tokens("fixture-current", FUTURE));
    fixture
}

fn connection() -> Value {
    crate::providers::kimi::declarations(Some(CONFIG.as_bytes())).unwrap()["subscription"]
        .declaration["provider"]
        .clone()
}

#[tokio::test]
async fn auth6_binding_commits_expected_scoped_result() {
    let fixture = ready_fixture();
    let connection = connection();
    let target = fixture.target(AuthRegion::Global);
    let scope = target.scope;
    fixture
        .native()
        .bind_account(&fixture.credentials, target, None, &connection, || true)
        .await
        .unwrap();
    assert_eq!(fixture.native().bound_account(scope, &connection), Ok(Some(ACCOUNT.into())));

    let auth = KimiDeviceAuth::new().unwrap();
    let (url, sent) = oauth_server("200 OK", "", json!({"user_id":41})).await;
    // The connection has changed after the caller obtained the committed account ID.
    let changed = CONFIG.replace("/coding/v1", "/coding/v1/");
    fs::write(fixture.config(), &changed).unwrap();
    assert_eq!(
        auth.select_account(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            Some(&connection),
            || true,
            |request| *request.url_mut() = url.clone(),
        )
        .await,
        Err(AuthErrorCode::Conflict),
        "a resolved binding cannot select a model for a different connection",
    );
    assert_eq!(fs::read_to_string(fixture.config()).unwrap(), changed);

    fs::write(fixture.config(), CONFIG).unwrap();
    let current = Arc::new(AtomicBool::new(true));
    let catalog = Arc::clone(&current);
    assert_eq!(
        auth.select_account(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            Some(&connection),
            move || catalog.load(Ordering::Relaxed),
            |request| {
                current.store(false, Ordering::Relaxed);
                *request.url_mut() = url.clone();
            },
        )
        .await,
        Err(AuthErrorCode::Conflict)
    );
    assert!(sent.await.unwrap().contains("Bearer fixture-current-access"));
    assert_eq!(fs::read_to_string(fixture.config()).unwrap(), CONFIG);

    let (url, sent) = oauth_server("200 OK", "", json!({"user_id":41})).await;
    auth.select_account(
        &fixture.credentials,
        fixture.target(AuthRegion::Global),
        fixture.native(),
        Some(&connection),
        || true,
        |request| {
            assert_eq!(request.url().as_str(), "https://api.kimi.ai/coding/v1/me");
            *request.url_mut() = url.clone();
        },
    )
    .await
    .unwrap();
    assert!(sent.await.unwrap().contains("Bearer fixture-current-access"));
    assert_eq!(
        fs::read_to_string(fixture.config()).unwrap(),
        CONFIG.replacen("\"original\" #", "\"chosen\" #", 1)
    );
}

#[tokio::test]
async fn auth6_binding_rejects_invalid_or_stale_input() {
    let fixture = ready_fixture();
    let auth = KimiDeviceAuth::new().unwrap();
    let connection = connection();
    let mut target = fixture.target(AuthRegion::Global);
    for service in [
        AuthService::GithubCopilot,
        AuthService::CodexOauth,
        AuthService::XaiOauth,
        AuthService::ClaudeOfficial,
        AuthService::AgyOauth,
        AuthService::DshAccount,
    ] {
        target.scope.service = service;
        assert_eq!(
            fixture.native().bound_account(target.scope, &connection),
            Err(AuthErrorCode::Unsupported)
        );
    }
    // A usable account alone cannot stand in for an explicit binding.
    let (url, _sent) = oauth_server("200 OK", "", json!({"user_id":41})).await;
    assert_eq!(
        auth.select_account(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            Some(&connection),
            || true,
            |request| *request.url_mut() = url.clone(),
        )
        .await,
        Err(AuthErrorCode::NotFound),
    );
    assert_eq!(fs::read_to_string(fixture.config()).unwrap(), CONFIG);
}
