use super::{copilot_tests::oauth_server, credential_file::{AccountCredential, KimiCredential}, kimi::KimiDeviceAuth};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, process::Command, sync::Arc};
use vw_api_transport::TimestampMs;
use vw_api_app_auth::{AuthErrorCode, AuthRegion, AuthService};

pub(crate) use super::kimi_test_support::*;

#[tokio::test]
async fn auth6_kimi_account_commits_expected_scoped_result() {
    let fixture = Fixture::new();
    let auth = KimiDeviceAuth::new().unwrap();
    let (refresh_url, refresh_sent) = oauth_server(
        "200 OK",
        "",
        json!({"access_token":"fixture-new-access","refresh_token":"fixture-new-refresh","expires_in":900}),
    )
    .await;
    let (me_url, me_sent) = oauth_server("200 OK", "", json!({"user_id":41})).await;
    let result = auth
        .use_default_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.home.path(),
            &fixture.transactions,
            |request| {
                assert!(fixture.lock().is_dir());
                match request.url().as_str() {
                    "https://auth.kimi.ai/api/oauth/token" => {
                        *request.url_mut() = refresh_url.clone()
                    }
                    "https://api.kimi.ai/coding/v1/me" => *request.url_mut() = me_url.clone(),
                    _ => panic!("unexpected Kimi endpoint"),
                }
            },
        )
        .await;
    assert_eq!(result, Ok(()), "verified regional account must become the default");
    let sent = refresh_sent.await.unwrap();
    assert!(sent.ends_with("client_id=17e5f671-d194-4dfb-9706-5516cb48c098&grant_type=refresh_token&refresh_token=fixture-expired-refresh"));
    assert!(sent.contains(&format!("\r\nx-msh-device-id: {DEVICE}\r\n")));
    assert!(me_sent.await.unwrap().contains("\r\nauthorization: Bearer fixture-new-access\r\n"));
    assert_eq!(
        fs::read_to_string(fixture.config()).unwrap(),
        CONFIG.replacen("\"original\" #", "\"chosen\" #", 1)
    );
    let saved: Value = serde_json::from_slice(&fs::read(fixture.credential()).unwrap()).unwrap();
    assert_eq!(saved["refresh_token"], "fixture-new-refresh");
    assert!(!fixture.lock().exists());
    let lease = fixture
        .credentials
        .lease_kimi(fixture.target(AuthRegion::Global).scope, ACCOUNT, false)
        .unwrap();
    let AccountCredential::KimiOauth(saved) = lease.read().unwrap() else {
        panic!("expected a Kimi credential");
    };
    assert_eq!(saved.refresh_token, "fixture-new-refresh");
    drop(lease);

    let fixture = Fixture::new();
    let (refresh_url, refresh_sent) =
        oauth_server("400 Bad Request", "", json!({"error":"invalid_grant"})).await;
    let (me_url, me_sent) = oauth_server("200 OK", "", json!({"user_id":41})).await;
    let rotated = native_tokens("fixture-external", FUTURE);
    let staged = fixture.credential().with_extension("rotating");
    fs::write(&staged, rotated.to_string()).unwrap();
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).unwrap();
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            |request| match request.url().path() {
                "/api/oauth/token" => {
                    assert!(fixture.lock().is_dir());
                    // A second process has already prepared the next generation.
                    assert!(
                        Command::new("/bin/mv")
                            .arg(&staged)
                            .arg(fixture.credential())
                            .status()
                            .unwrap()
                            .success()
                    );
                    *request.url_mut() = refresh_url.clone();
                }
                "/coding/v1/me" => *request.url_mut() = me_url.clone(),
                _ => panic!("unexpected Kimi endpoint"),
            },
        )
        .await;
    assert_eq!(result, Ok(()));
    assert!(refresh_sent.await.unwrap().ends_with("refresh_token=fixture-expired-refresh"));
    assert!(
        me_sent.await.unwrap().contains("\r\nauthorization: Bearer fixture-external-access\r\n")
    );
    assert_eq!(fs::read_to_string(fixture.credential()).unwrap(), rotated.to_string());
    assert_eq!(
        fs::read_to_string(fixture.config()).unwrap(),
        CONFIG.replacen("\"original\" #", "\"chosen\" #", 1)
    );
    assert!(!fixture.lock().exists());
}

#[tokio::test]
async fn auth6_kimi_account_rejects_invalid_or_stale_input() {
    let fixture = Fixture::new();
    let auth = KimiDeviceAuth::new().unwrap();
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::MainlandCn),
            fixture.native(),
            |_| panic!("region mismatch must fail before contacting Kimi"),
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::IdentityMismatch));

    assert!(Command::new("/bin/mkdir").arg(fixture.lock()).status().unwrap().success());
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            |_| panic!("a contended slot must fail before contacting Kimi"),
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::AccountInUse));
    assert!(fixture.lock().exists());
    fs::remove_dir(fixture.lock()).unwrap();

    fixture.write_native(&native_tokens("fixture-foreign", FUTURE));
    let (url, sent) = oauth_server("200 OK", "", json!({"user_id":42})).await;
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            |request| {
                assert_eq!(request.url().as_str(), "https://api.kimi.ai/coding/v1/me");
                *request.url_mut() = url.clone();
            },
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::IdentityMismatch));
    sent.await.unwrap();
    assert_eq!(fs::read_to_string(fixture.config()).unwrap(), CONFIG);
    assert_eq!(
        fs::read_to_string(fixture.credential()).unwrap(),
        native_tokens("fixture-foreign", FUTURE).to_string()
    );
    assert!(!fixture.lock().exists());

    fixture.write_native(&native_tokens("fixture-expired-other", 2));
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            |_| panic!("an expired independent grant cannot prove its identity"),
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::ReauthRequired));
    assert_eq!(
        fs::read_to_string(fixture.credential()).unwrap(),
        native_tokens("fixture-expired-other", 2).to_string()
    );

    fixture.write_native(&native_tokens("fixture-expired", 1));
    let (url, sent) = oauth_server("400 Bad Request", "", json!({"error":"invalid_grant"})).await;
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            |request| {
                assert_eq!(request.url().as_str(), "https://auth.kimi.ai/api/oauth/token");
                *request.url_mut() = url.clone();
            },
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::ReauthRequired));
    assert!(sent.await.unwrap().ends_with("refresh_token=fixture-expired-refresh"));
    let revoked: Value = serde_json::from_slice(&fs::read(fixture.credential()).unwrap()).unwrap();
    assert_eq!(revoked["refresh_token"], "");
    assert_eq!(revoked["access_token"], "");
    assert_eq!(revoked["expires_at"], 0);
    assert_eq!(fs::read_to_string(fixture.config()).unwrap(), CONFIG);
    assert!(!fixture.lock().exists());
}

#[path = "kimi_account/binding_tests.rs"]
mod binding_tests;

#[path = "binding_tests.rs"]
mod scoped_binding_tests;
