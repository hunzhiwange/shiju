use super::{
    copilot_tests::oauth_server,
    credential_file::KimiCredential,
    flow::{AuthAccountScope, AuthFlows},
    kimi::{KimiDeviceAuth, KimiDeviceStart, KimiLoginTarget},
};
use reqwest::{Request, Url};
use serde_json::json;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
use vw_api_types::{
    agent_apps_auth::{
        AuthErrorCode, AuthFlow, AuthLoginMethod, AuthRegion, AuthService, AuthStartRequest,
        AuthStartResponse,
    },
    id::TenantScope,
};

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "oidc:owner",
        host: "fixture-host",
        service: AuthService::KimiOauth,
    }
}

fn request(region: Option<AuthRegion>, account: Option<&str>) -> AuthStartRequest {
    AuthStartRequest {
        method: AuthLoginMethod::DeviceCode,
        github_domain: None,
        region,
        target_account_id: account.map(str::to_owned),
    }
}

fn route_to(outbound: &mut Request, expected: &str, mut route: Url) {
    assert_eq!(outbound.url().as_str(), expected);
    route.set_path(outbound.url().path());
    *outbound.url_mut() = route;
}

fn device_header(sent: &str) -> &str {
    sent.split("\r\n")
        .find_map(|line| line.strip_prefix("x-msh-device-id: "))
        .expect("device ID header")
}

async fn start(
    auth: &KimiDeviceAuth,
    flows: &AuthFlows,
    scope: AuthAccountScope<'_>,
    request: AuthStartRequest,
    target: Option<KimiLoginTarget>,
    domain: &str,
) -> (KimiDeviceStart, String) {
    let (url, sent) = oauth_server(
        "200 OK",
        "",
        json!({
            "device_code":"private-kimi-grant",
            "user_code":"KIMI-TEST",
            "verification_uri":format!("https://auth.{domain}/device"),
            "expires_in":"120",
            "interval":2,
        }),
    )
    .await;
    let result = auth
        .start_with(
            flows,
            scope,
            request,
            |_| Ok(target.unwrap()),
            |outbound| {
                route_to(
                    outbound,
                    &format!("https://auth.{domain}/api/oauth/device_authorization"),
                    url.clone(),
                );
            },
        )
        .await;
    assert_eq!(result.as_ref().err(), None, "valid regional Kimi device login must start");
    let started = result.unwrap();
    let AuthStartResponse::Pending { verification_url, user_code, retry_after_seconds, .. } =
        &started.response;
    assert_eq!(verification_url, &format!("https://auth.{domain}/device"));
    assert_eq!(user_code.as_deref(), Some("KIMI-TEST"));
    assert_eq!(*retry_after_seconds, 2);
    let sent = sent.await.unwrap();
    assert!(sent.starts_with("POST /api/oauth/device_authorization HTTP/1.1\r\n"));
    assert!(sent.contains("\r\nx-msh-platform: kimi_code_cli\r\n"));
    assert!(sent.ends_with("client_id=17e5f671-d194-4dfb-9706-5516cb48c098"));
    let device = device_header(&sent).to_owned();
    assert!(uuid::Uuid::parse_str(&device).is_ok());
    let public = serde_json::to_string(&started.response).unwrap();
    assert!(!public.contains(&device) && !public.contains("private-kimi-grant"));
    (started, device)
}

async fn poll(
    auth: &KimiDeviceAuth,
    flows: &AuthFlows,
    scope: AuthAccountScope<'_>,
    started: &KimiDeviceStart,
    domain: &str,
    device: &str,
    identity: u64,
) -> (AuthFlow, Option<KimiCredential>) {
    let (token, token_sent) = oauth_server(
        "200 OK",
        "",
        json!({
            "access_token":"private-kimi-access",
            "refresh_token":"private-kimi-refresh",
            "expires_in":3600
        }),
    )
    .await;
    let (me, me_sent) =
        oauth_server("200 OK", "", json!({"user_id":identity, "nickname":"Kimi User"})).await;
    let mut routes = VecDeque::from([
        (format!("https://auth.{domain}/api/oauth/token"), token),
        (format!("https://api.{domain}/coding/v1/me"), me),
    ]);
    let mut saved = None;
    let due = Instant::now() + Duration::from_secs(3);
    let result = auth
        .poll_with(
            flows,
            scope,
            started,
            || due,
            |outbound| {
                let (expected, url) = routes.pop_front().unwrap();
                route_to(outbound, &expected, url);
            },
            |bound, _, account, external_identity, credential| {
                assert!(bound == scope);
                assert_eq!(account.region, Some(credential.region));
                assert!(external_identity.ends_with(&format!(":{identity}")));
                assert_eq!(credential.device_id, device);
                saved = Some(credential);
                Ok(())
            },
        )
        .await
        .unwrap();
    assert!(routes.is_empty());
    let token_sent = token_sent.await.unwrap();
    let me_sent = me_sent.await.unwrap();
    assert!(token_sent.starts_with("POST /api/oauth/token HTTP/1.1\r\n"));
    assert!(token_sent.ends_with("client_id=17e5f671-d194-4dfb-9706-5516cb48c098&device_code=private-kimi-grant&grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"));
    assert!(me_sent.starts_with("GET /coding/v1/me HTTP/1.1\r\n"));
    assert!(me_sent.contains("\r\nauthorization: Bearer private-kimi-access\r\n"));
    assert_eq!(device_header(&token_sent), device);
    assert_eq!(device_header(&me_sent), device);
    (result, saved)
}

#[tokio::test]
async fn auth6_kimi_device_commits_expected_scoped_result() {
    let auth = KimiDeviceAuth::new().unwrap();
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let mut results = Vec::new();
    for (region, domain) in [(AuthRegion::MainlandCn, "kimi.com"), (AuthRegion::Global, "kimi.ai")]
    {
        let (started, device) =
            start(&auth, &flows, scope, request(Some(region), None), None, domain).await;
        let (result, saved) = poll(&auth, &flows, scope, &started, domain, &device, 42).await;
        let AuthFlow::Approved { account, .. } = result else {
            panic!("expected Kimi approval: {result:?}")
        };
        assert_eq!(account.region, Some(region));
        assert_eq!(account.label, "Kimi User");
        let credential = saved.unwrap();
        assert_eq!(credential.region, region);
        assert_eq!(credential.expires_at.0 - account.authenticated_at.unwrap().0, 3_600_000);
        results.push((account.account_id, credential));
    }
    assert_ne!(results[0].0, results[1].0);
    assert_ne!(results[0].1.device_id, results[1].1.device_id);

    let (id, credential) = &results[1];
    let target =
        KimiLoginTarget { region: credential.region, device_id: credential.device_id.clone() };
    let (started, device) =
        start(&auth, &flows, scope, request(None, Some(id)), Some(target), "kimi.ai").await;
    assert_eq!(device, credential.device_id);
    let (result, _) = poll(&auth, &flows, scope, &started, "kimi.ai", &device, 42).await;
    assert!(matches!(result, AuthFlow::Approved { account, .. } if account.account_id == *id));
}

#[tokio::test]
async fn auth6_kimi_device_rejects_invalid_or_stale_input() {
    let auth = KimiDeviceAuth::new().unwrap();
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let missing_region = auth
        .start_with(
            &flows,
            scope,
            request(None, None),
            |_| panic!("no target"),
            |_| panic!("invalid request reached HTTP"),
        )
        .await;
    assert_eq!(missing_region.err(), Some(AuthErrorCode::InvalidInput));

    let (old, _) =
        start(&auth, &flows, scope, request(Some(AuthRegion::MainlandCn), None), None, "kimi.com")
            .await;
    let foreign = AuthAccountScope { principal: "oidc:other", ..scope };
    let (other, _) =
        start(&auth, &flows, foreign, request(Some(AuthRegion::Global), None), None, "kimi.ai")
            .await;
    // Hold the old exchange at the identity response until the new region
    // has started. A late successful response must never publish credentials.
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::oneshot,
    };
    let (token, token_sent) = oauth_server(
        "200 OK",
        "",
        json!({"access_token":"late-access","refresh_token":"late-refresh","expires_in":3600}),
    )
    .await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let me_url = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let (received, seen) = oneshot::channel();
    let (release, ready) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut connection, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(connection.read_u8().await.unwrap());
        }
        assert!(request.starts_with(b"GET /coding/v1/me HTTP/1.1\r\n"));
        received.send(()).unwrap();
        ready.await.unwrap();
        let payload = r#"{"user_id":42}"#;
        connection
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });
    let due = Instant::now() + Duration::from_secs(3);
    let mut routes = VecDeque::from([
        ("https://auth.kimi.com/api/oauth/token", token),
        ("https://api.kimi.com/coding/v1/me", me_url),
    ]);
    let old_poll = auth.poll_with(
        &flows,
        scope,
        &old,
        || due,
        |outbound| {
            let (expected, url) = routes.pop_front().unwrap();
            route_to(outbound, expected, url);
        },
        |_, _, _, _, _| panic!("late token persisted"),
    );
    let restart = async {
        seen.await.unwrap();
        let next =
            start(&auth, &flows, scope, request(Some(AuthRegion::Global), None), None, "kimi.ai")
                .await;
        release.send(()).unwrap();
        next
    };
    let (late, (replacement, device)) = tokio::join!(old_poll, restart);
    assert!(matches!(late.unwrap(), AuthFlow::Cancelled { .. }));
    token_sent.await.unwrap();
    server.await.unwrap();
    let expired = auth
        .poll_with(
            &flows,
            scope,
            &old,
            Instant::now,
            |_| panic!("superseded flow reached HTTP"),
            |_, _, _, _, _| panic!("superseded flow persisted"),
        )
        .await
        .unwrap();
    assert!(matches!(expired, AuthFlow::Cancelled { .. }));
    let AuthStartResponse::Pending { flow_id, .. } = &other.response;
    assert!(matches!(
        flows.snapshot(foreign, flow_id, Instant::now).unwrap(),
        AuthFlow::Pending { .. }
    ));
    let (result, _) = poll(&auth, &flows, scope, &replacement, "kimi.ai", &device, 42).await;
    let AuthFlow::Approved { account, .. } = result else { panic!("expected approval") };

    let target = || KimiLoginTarget { region: AuthRegion::Global, device_id: device.clone() };
    let conflict = auth
        .start_with(
            &flows,
            scope,
            request(Some(AuthRegion::MainlandCn), Some(&account.account_id)),
            |_| Ok(target()),
            |_| panic!("mismatched region reached HTTP"),
        )
        .await;
    assert_eq!(conflict.err(), Some(AuthErrorCode::IdentityMismatch));

    let (reauth, _) = start(
        &auth,
        &flows,
        scope,
        request(None, Some(&account.account_id)),
        Some(target()),
        "kimi.ai",
    )
    .await;
    let (failed, saved) = poll(&auth, &flows, scope, &reauth, "kimi.ai", &device, 43).await;
    assert!(
        matches!(failed, AuthFlow::Failed { error, .. } if error.code == AuthErrorCode::IdentityMismatch)
    );
    assert!(saved.is_none());

    let (old, _) =
        start(&auth, &flows, scope, request(Some(AuthRegion::Global), None), None, "kimi.ai").await;
    let (url, sent) = oauth_server(
        "200 OK",
        "",
        json!({
            "device_code":"private-kimi-grant",
            "user_code":"KIMI-TEST",
            "verification_uri":"https://auth.kimi.com/device",
        }),
    )
    .await;
    let failed = auth
        .start_with(
            &flows,
            scope,
            request(Some(AuthRegion::Global), None),
            |_| panic!("no target"),
            |outbound| {
                route_to(
                    outbound,
                    "https://auth.kimi.ai/api/oauth/device_authorization",
                    url.clone(),
                )
            },
        )
        .await;
    assert_eq!(failed.err(), Some(AuthErrorCode::UpstreamFailure));
    sent.await.unwrap();
    let AuthStartResponse::Pending { flow_id, .. } = &old.response;
    assert!(matches!(
        flows.snapshot(scope, flow_id, Instant::now).unwrap(),
        AuthFlow::Cancelled { .. }
    ));
}
