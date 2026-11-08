use super::{
    agy_account_tests::{Fixture, credential, encode, scope},
    agy_refresh::{reconcile_in, refresh_with},
    copilot_tests::oauth_server,
    keychain::KeychainEntry,
};
use reqwest::{Client, Url};
use serde_json::json;
use std::{collections::HashMap, io::Read as _, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::timeout,
};
use vw_api_types::agent_apps_auth::AuthErrorCode;

fn client() -> Client {
    Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap()
}

#[tokio::test]
async fn auth6_agy_refresh_commits_expected_scoped_result() {
    let mut fixture = Fixture::new();
    let mut first = credential("first", 1);
    first["token"]["expiry"] = json!("2020-01-01T00:00:00Z");
    let second = credential("second", 1);
    let a = fixture.managed("first", &first, "ready");
    let b = fixture.managed("second", &second, "ready");
    let owner = scope(&fixture.tenant);
    fixture.switch(&a).unwrap();

    let mut rotated = credential("first", 2);
    rotated["token"]["expiry"] = json!("2020-01-02T00:00:00Z");
    rotated["token"]["refresh_token"] = first["token"]["refresh_token"].clone();
    fixture.entry().replace_base64(Some(encode(&first).as_bytes()), &encode(&rotated)).unwrap();
    assert_eq!(
        reconcile_in(owner, &a, fixture.target(Some(owner)), &fixture.entry()),
        Ok(()),
        "the bound account can absorb a newer native credential",
    );
    assert_eq!(fixture.record(&a)["credential"], rotated);

    let (mut route, received) = oauth_server(
        "200 OK",
        "",
        json!({"access_token":"new-oauth-access","expires_in":7200,"token_type":"Bearer"}),
    )
    .await;
    let result = refresh_with(owner, &a, &client(), |request| {
        assert_eq!(request.url().as_str(), "https://oauth2.googleapis.com/token");
        route.set_path(request.url().path());
        *request.url_mut() = route;
    })
    .await;
    assert_eq!(result, Ok("new-oauth-access".into()), "refresh commits the scoped token");
    let request = received.await.unwrap();
    let (headers, body) = request.split_once("\r\n\r\n").unwrap();
    assert!(headers.starts_with("POST /token HTTP/1.1"));
    assert!(headers.contains("\r\ncontent-type: application/x-www-form-urlencoded\r\n"));
    let form: HashMap<_, _> = Url::parse(&format!("https://fixture.invalid/?{body}"))
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect();
    assert_eq!(form.len(), 4);
    assert_eq!(form["grant_type"], "refresh_token");
    assert_eq!(form["refresh_token"], "synthetic-refresh-1");
    assert_eq!(
        form["client_id"],
        "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com"
    );
    assert!(form["client_secret"].starts_with("GOCSPX-"));

    let saved = &fixture.record(&a)["credential"];
    assert_eq!(saved["token"]["access_token"], "new-oauth-access");
    assert_eq!(saved["token"]["refresh_token"], rotated["token"]["refresh_token"]);
    assert_eq!(saved["token"]["expires_in"], 7200);
    assert_eq!(saved["id_token"], rotated["id_token"]);
    assert_eq!(saved["unknown"], rotated["unknown"]);
    assert_eq!(fixture.record(&b)["credential"], second);
    assert_eq!(fixture.entry().read().unwrap(), Some(encode(&rotated).into_bytes()));
    assert_eq!(reconcile_in(owner, &a, fixture.target(Some(owner)), &fixture.entry()), Ok(()));
    assert_eq!(
        &fixture.record(&a)["credential"],
        saved,
        "older native tokens cannot roll back OAuth"
    );
    fixture.remove_entries();
}

#[tokio::test]
async fn auth6_agy_refresh_rejects_invalid_or_stale_input() {
    let mut fixture = Fixture::new();
    let first = credential("first", 1);
    let second = credential("second", 1);
    let a = fixture.managed("first", &first, "ready");
    let b = fixture.managed("second", &second, "ready");
    let owner = scope(&fixture.tenant);
    fixture.switch(&a).unwrap();
    let mut old_process = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "auth::agy_refresh_tests::agy_writeback_fixture_process",
            "--nocapture",
        ])
        .env("SHIJU_AGY_TEST_SERVICE", &fixture.native_service)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut output = BufReader::new(old_process.stdout.take().unwrap()).lines();
    timeout(Duration::from_secs(10), async {
        while !output.next_line().await.unwrap().expect("writer exited").contains("writer ready") {}
    })
    .await
    .unwrap();
    fixture.switch(&b).unwrap();
    old_process.stdin.take().unwrap().write_all(b"1").await.unwrap();
    assert!(timeout(Duration::from_secs(10), old_process.wait()).await.unwrap().unwrap().success());

    let live = fixture.entry().read().unwrap();
    assert!(live.is_some() && live.as_deref() != Some(encode(&second).as_bytes()));
    let before_a = fixture.record(&a);
    let before_b = fixture.record(&b);
    assert_eq!(
        reconcile_in(owner, &b, fixture.target(None), &fixture.entry()),
        Err(AuthErrorCode::Forbidden),
        "the native slot requires an exclusive owner",
    );
    assert_eq!(
        reconcile_in(owner, &b, fixture.target(Some(owner)), &fixture.entry()),
        Err(AuthErrorCode::Conflict),
        "an old process must not rebind the current account",
    );
    assert_eq!(fixture.record(&a), before_a);
    assert_eq!(fixture.record(&b), before_b);
    assert_eq!(fixture.entry().read().unwrap(), live);

    let foreign = super::flow::AuthAccountScope { principal: "oidc:foreign", ..owner };
    assert_eq!(
        refresh_with(foreign, &b, &client(), |_| panic!("foreign grant was sent")).await,
        Err(AuthErrorCode::NotFound),
    );
    let other_token = first["id_token"].clone();
    let (route, received) =
        oauth_server("200 OK", "", json!({"access_token":"other-access","id_token":other_token}))
            .await;
    assert_eq!(
        refresh_with(owner, &b, &client(), |request| *request.url_mut() = route).await,
        Err(AuthErrorCode::IdentityMismatch),
    );
    received.await.unwrap();
    assert_eq!(fixture.record(&b), before_b);

    let (route, received) =
        oauth_server("503 Service Unavailable", "", json!({"error":"temporarily_unavailable"}))
            .await;
    assert_eq!(
        refresh_with(owner, &b, &client(), |request| *request.url_mut() = route).await,
        Err(AuthErrorCode::UpstreamFailure),
    );
    received.await.unwrap();
    assert_eq!(fixture.record(&b), before_b);

    // A same-grant rotation changes the stored generation while OAuth is in flight.
    let mut rotated = credential("second", 2);
    rotated["token"]["refresh_token"] = second["token"]["refresh_token"].clone();
    let (route, received) =
        oauth_server("400 Bad Request", "", json!({"error":"invalid_grant"})).await;
    assert_eq!(
        refresh_with(owner, &b, &client(), |request| {
            fixture.entry().replace_base64(live.as_deref(), &encode(&rotated)).unwrap();
            reconcile_in(owner, &b, fixture.target(Some(owner)), &fixture.entry()).unwrap();
            *request.url_mut() = route;
        })
        .await,
        Err(AuthErrorCode::Conflict),
    );
    received.await.unwrap();
    assert_eq!(fixture.record(&b)["account"]["credential_state"], "ready");
    assert_eq!(fixture.record(&b)["credential"], rotated);

    let (route, received) = oauth_server(
        "400 Bad Request",
        "",
        json!({"error":"invalid_grant", "error_description":"synthetic-private-detail"}),
    )
    .await;
    assert_eq!(
        refresh_with(owner, &b, &client(), |request| *request.url_mut() = route).await,
        Err(AuthErrorCode::ReauthRequired),
    );
    received.await.unwrap();
    assert_eq!(fixture.record(&b)["account"]["credential_state"], "reauth_required");
    assert_eq!(fixture.record(&b)["credential"], rotated);
    assert_eq!(fixture.record(&a), before_a);
    assert_eq!(
        refresh_with(owner, &b, &client(), |_| panic!("stale grant was sent again")).await,
        Err(AuthErrorCode::ReauthRequired),
    );
    fixture.remove_entries();
}

#[test]
#[ignore = "spawned by the AGY write-back test"]
fn agy_writeback_fixture_process() {
    let service = std::env::var("SHIJU_AGY_TEST_SERVICE").unwrap();
    assert!(service.starts_with("shiju.agy.switch.test."));
    let entry = KeychainEntry::new(&service, "antigravity").unwrap();
    let first = credential("first", 1);
    assert_eq!(entry.read().unwrap(), Some(encode(&first).into_bytes()));
    println!("writer ready");
    std::io::stdin().read_exact(&mut [0]).unwrap();
    let mut rotated = credential("first", 3);
    rotated["token"]["refresh_token"] = first["token"]["refresh_token"].clone();
    let live = entry.read().unwrap();
    assert_eq!(live, Some(encode(&credential("second", 1)).into_bytes()));
    entry.replace_base64(live.as_deref(), &encode(&rotated)).unwrap();
}
