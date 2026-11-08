#[cfg(unix)]
use super::credential_file::{AccountCredential, CredentialFileStore};
use super::{
    copilot::{CopilotAuth, CopilotDeviceStart},
    flow::{AuthAccountScope, AuthFlows},
};
use serde_json::json;
use std::{
    cell::Cell,
    collections::VecDeque,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
use vw_api_types::{
    agent_apps_auth::{
        AuthAccount, AuthErrorCode, AuthFlow, AuthLoginMethod, AuthService, AuthStartRequest,
        AuthStartResponse,
    },
    id::TenantScope,
};

fn request(domain: Option<&str>) -> AuthStartRequest {
    AuthStartRequest {
        method: AuthLoginMethod::DeviceCode,
        github_domain: domain.map(str::to_owned),
        region: None,
        target_account_id: None,
    }
}

async fn oauth_server(
    status: &str,
    headers: &str,
    body: serde_json::Value,
) -> (reqwest::Url, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = reqwest::Url::parse(&format!(
        "http://{}/login/device/code",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n{body}",
        body.to_string().len(),
    );
    let received = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(stream.read_u8().await.unwrap());
        }
        let headers = String::from_utf8(headers).unwrap();
        let size = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .map_or(0, |(_, value)| value.trim().parse().unwrap());
        let mut body = vec![0; size];
        stream.read_exact(&mut body).await.unwrap();
        stream.write_all(reply.as_bytes()).await.unwrap();
        format!("{headers}{}", String::from_utf8(body).unwrap())
    });
    (url, received)
}

fn challenge(verification_uri: &str) -> serde_json::Value {
    json!({
        "device_code": "fixture-private-grant",
        "user_code": "ABCD-EFGH",
        "verification_uri": verification_uri,
        "expires_in": 900,
        "interval": 5,
    })
}

#[tokio::test]
async fn auth6_copilot_start_commits_expected_scoped_result() {
    let auth = CopilotAuth::new().unwrap();
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "oidc:owner",
        host: "host",
        service: AuthService::GithubCopilot,
    };

    for (input, authority, expected_client) in [
        (None, "github.com", "Iv1.b507a08c87ecfe98"),
        (Some(" Company.GHE.Com "), "company.ghe.com", "Ov23li8tweQw6odWQebz"),
        (Some("company.ghe.com:8443"), "company.ghe.com:8443", "Ov23li8tweQw6odWQebz"),
    ] {
        let verification = format!("https://{authority}/login/device");
        let (route, received) = oauth_server("200 OK", "", challenge(&verification)).await;
        let started = auth
            .start_with(
                &flows,
                scope,
                request(input),
                &["company.ghe.com", "company.ghe.com:8443"],
                |outbound| {
                    assert_eq!(
                        outbound.url().as_str(),
                        format!("https://{authority}/login/device/code")
                    );
                    *outbound.url_mut() = route;
                },
            )
            .await;
        assert!(started.is_ok(), "valid GitHub authority rejected: {:?}", started.err());
        let started = started.unwrap();
        let sent = received.await.unwrap();
        let (headers, body) = sent.split_once("\r\n\r\n").unwrap();
        let headers = headers.to_ascii_lowercase();
        assert!(headers.starts_with("post /login/device/code http/1.1\r\n"));
        assert!(headers.contains("\r\naccept: application/json\r\n"));
        assert!(headers.contains("\r\nuser-agent: githubcopilotchat/0.38.2\r\n"));
        assert!(headers.contains("\r\ncontent-type: application/x-www-form-urlencoded"));
        assert_eq!(body, format!("client_id={expected_client}&scope=read%3Auser"));

        let serialized = serde_json::to_string(&started.response).unwrap();
        assert!(!serialized.contains("fixture-private-grant"));
        let AuthStartResponse::Pending {
            flow_id,
            expires_at,
            retry_after_seconds,
            verification_url,
            user_code,
            can_paste,
        } = started.response;
        assert_eq!(verification_url, verification);
        assert_eq!(user_code.as_deref(), Some("ABCD-EFGH"));
        assert_eq!(retry_after_seconds, 5);
        assert_eq!(can_paste, None);
        assert!(expires_at.0 > 0);
        assert!(matches!(flows.snapshot(scope, &flow_id, Instant::now), Ok(AuthFlow::Pending {
            expires_at: bound_expiry, retry_after_seconds: 5, ..
        }) if bound_expiry == expires_at));
        let foreign = AuthAccountScope { principal: "oidc:other", ..scope };
        assert_eq!(flows.snapshot(foreign, &flow_id, Instant::now), Err(AuthErrorCode::NotFound));
        {
            let grant = started.grant.lock().unwrap();
            let grant = grant.as_ref().unwrap();
            assert_eq!(grant.authority, authority);
            assert!(grant.device_code == "fixture-private-grant");
        }
        if input.is_none() {
            flows.cancel(scope, &flow_id, Instant::now).unwrap();
        } else {
            flows.reap_expired(Instant::now() + Duration::from_secs(901)).unwrap();
            assert!(matches!(
                flows.snapshot(scope, &flow_id, Instant::now),
                Ok(AuthFlow::Expired { .. })
            ));
        }
        assert!(started.grant.lock().unwrap().is_none());
    }
}

#[tokio::test]
async fn auth6_copilot_start_rejects_invalid_or_stale_input() {
    let auth = CopilotAuth::new().unwrap();
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "oidc:owner",
        host: "host",
        service: AuthService::GithubCopilot,
    };
    let trusted = ["company.ghe.com", "127.0.0.1", "localhost"];
    for domain in [
        "",
        "https://company.ghe.com",
        "company.ghe.com/login/device",
        "company.ghe.com?query",
        "company.ghe.com#fragment",
        "company.ghe.com:444",
        "company.ghe.com:+443",
        "github.com:8443",
        "company.ghe.com\\evil.example",
        "company.ghe.com@evil.example",
        "company.ghe.com.evil.example",
        "unapproved.ghe.com",
        "127.0.0.1",
        "localhost",
    ] {
        let result = auth
            .start_with(&flows, scope, request(Some(domain)), &trusted, |_| {
                panic!("rejected authority reached outbound HTTP")
            })
            .await;
        assert_eq!(result.err(), Some(AuthErrorCode::InvalidInput), "domain: {domain}");
    }

    let foreign = AuthAccountScope { service: AuthService::XaiOauth, ..scope };
    assert_eq!(
        auth.start(&flows, foreign, request(None), &trusted).await.err(),
        Some(AuthErrorCode::Unsupported)
    );

    let mut expired = challenge("https://github.com/login/device");
    expired["expires_in"] = json!(0);
    let mut invalid_interval = challenge("https://github.com/login/device");
    invalid_interval["interval"] = json!(0);
    let (redirect_target, redirect_request) =
        oauth_server("200 OK", "", challenge("https://github.com/login/device")).await;
    let redirect_header = format!("Location: {redirect_target}\r\n");
    for (status, headers, body) in [
        ("200 OK", "", expired),
        ("200 OK", "", invalid_interval),
        ("200 OK", "", challenge("https://evil.example/login/device")),
        ("200 OK", "", challenge("https://github.com/login/device?private=fixture-private-grant")),
        ("307 Temporary Redirect", redirect_header.as_str(), json!({})),
    ] {
        let (route, received) = oauth_server(status, headers, body).await;
        let result = auth
            .start_with(&flows, scope, request(None), &trusted, |outbound| {
                *outbound.url_mut() = route;
            })
            .await;
        assert_eq!(result.err(), Some(AuthErrorCode::UpstreamFailure));
        received.await.unwrap();
    }
    assert!(!redirect_request.is_finished());
    redirect_request.abort();
}

async fn start_poll_fixture(
    auth: &CopilotAuth,
    flows: &AuthFlows,
    scope: AuthAccountScope<'_>,
    authority: &str,
) -> CopilotDeviceStart {
    let (route, received) =
        oauth_server("200 OK", "", challenge(&format!("https://{authority}/login/device"))).await;
    let started = auth
        .start_with(flows, scope, request(Some(authority)), &[authority], |outbound| {
            *outbound.url_mut() = route;
        })
        .await
        .unwrap();
    received.await.unwrap();
    started
}

#[cfg(unix)]
#[tokio::test]
async fn auth6_copilot_poll_commits_expected_scoped_result() {
    let auth = CopilotAuth::new().unwrap();
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "oidc:owner",
        host: "host",
        service: AuthService::GithubCopilot,
    };
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let credentials = CredentialFileStore::new(root.path()).unwrap();
    let mut ids = Vec::new();
    for (owner, authority) in [
        (scope, "github.com"),
        (scope, "github.com"),
        (scope, "enterprise.ghe.com:8443"),
        (AuthAccountScope { principal: "oidc:other", ..scope }, "github.com"),
    ] {
        let started = start_poll_fixture(&auth, &flows, owner, authority).await;
        let now = Instant::now();
        let no_commit = |_: AuthAccountScope<'_>, _: &AuthAccount, _: &str, _: &str| {
            panic!("pending login attempted to persist")
        };
        let result = auth
            .poll_with(
                &flows,
                owner,
                &started,
                || now,
                |_| panic!("poll before the interval reached GitHub"),
                no_commit,
            )
            .await;
        assert!(
            matches!(result, Ok(AuthFlow::Pending { retry_after_seconds: 5, .. })),
            "{result:?}"
        );

        let public = authority == "github.com";
        let client_id = if public { "Iv1.b507a08c87ecfe98" } else { "Ov23li8tweQw6odWQebz" };
        for (offset, error, expected) in [(6, "authorization_pending", 5), (11, "slow_down", 10)] {
            let (route, received) = oauth_server("200 OK", "", json!({"error": error})).await;
            let result = auth
                .poll_with(
                    &flows,
                    owner,
                    &started,
                    || now + Duration::from_secs(offset),
                    |outbound| {
                        assert_eq!(
                            outbound.url().as_str(),
                            format!("https://{authority}/login/oauth/access_token")
                        );
                        let mut route = route.clone();
                        route.set_path(outbound.url().path());
                        *outbound.url_mut() = route;
                    },
                    no_commit,
                )
                .await;
            assert!(
                matches!(result, Ok(AuthFlow::Pending { retry_after_seconds, .. }) if retry_after_seconds == expected),
                "{result:?}"
            );
            let sent = received.await.unwrap();
            assert!(sent.starts_with("POST /login/oauth/access_token HTTP/1.1\r\n"));
            assert_eq!(
                sent.split_once("\r\n\r\n").unwrap().1,
                format!(
                    "client_id={client_id}&device_code=fixture-private-grant&grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"
                )
            );
        }
        let result = auth
            .poll_with(
                &flows,
                owner,
                &started,
                || now + Duration::from_secs(20),
                |_| panic!("slow_down did not extend the interval"),
                no_commit,
            )
            .await;
        assert!(matches!(result, Ok(AuthFlow::Pending { retry_after_seconds: 10, .. })));

        let mut routes = VecDeque::new();
        let mut requests = Vec::new();
        for reply in [
            json!({"access_token":"fixture-access-token", "token_type":"bearer", "scope":"read:user"}),
            json!({"id": 42, "login": "octocat", "avatar_url": "https://example.test/avatar"}),
            json!({"token":"fixture-session-token", "expires_at": 4_000_000_000_i64, "refresh_in": 1500}),
        ].into_iter().take(if public { 3 } else { 2 }) {
            let (route, received) = oauth_server("200 OK", "", reply).await;
            routes.push_back(route);
            requests.push(received);
        }
        let api = if public {
            "https://api.github.com".into()
        } else {
            format!("https://{authority}/api/v3")
        };
        let mut expected_urls = VecDeque::from([
            format!("https://{authority}/login/oauth/access_token"),
            format!("{api}/user"),
        ]);
        if public {
            expected_urls.push_back(format!("{api}/copilot_internal/v2/token"));
        }
        let approved = auth
            .poll_with(
                &flows,
                owner,
                &started,
                || now + Duration::from_secs(21),
                |outbound| {
                    assert_eq!(outbound.url().as_str(), expected_urls.pop_front().unwrap());
                    let mut route = routes.pop_front().unwrap();
                    route.set_path(outbound.url().path());
                    *outbound.url_mut() = route;
                },
                |bound, account, identity, token| {
                    assert!(bound == owner);
                    assert_eq!(account.github_domain.as_deref(), Some(authority));
                    assert!(!account.is_default);
                    assert_eq!(identity, format!("{authority}/42"));
                    assert!(token == "fixture-access-token");
                    credentials
                        .lease_copilot(bound, &account.account_id, true)
                        .unwrap()
                        .save(&AccountCredential::CopilotToken(token.into()))
                        .unwrap();
                    Ok(())
                },
            )
            .await
            .unwrap();
        let serialized = serde_json::to_string(&approved).unwrap();
        assert!(!serialized.contains("fixture-"));
        let AuthFlow::Approved { account, .. } = &approved else { panic!("{approved:?}") };
        assert_eq!(account.label, "octocat");
        ids.push(account.account_id.clone());
        assert!(matches!(
            credentials.lease_copilot(owner, &account.account_id, false).unwrap().read().unwrap(),
            AccountCredential::CopilotToken(token) if token == "fixture-access-token"
        ));
        let foreign = AuthAccountScope { host: "different-host", ..owner };
        assert!(credentials.lease_copilot(foreign, &account.account_id, false).is_err());
        for (index, received) in requests.into_iter().enumerate() {
            let sent = received.await.unwrap().to_ascii_lowercase();
            if index > 0 {
                assert!(sent.starts_with("get "));
                assert!(sent.contains("\r\nauthorization: token fixture-access-token\r\n"));
            }
        }
        assert!(started.grant.lock().unwrap().is_none());
        assert_eq!(
            auth.poll_with(
                &flows,
                owner,
                &started,
                || now + Duration::from_secs(30),
                |_| { panic!("terminal flow reached GitHub") },
                no_commit
            )
            .await
            .unwrap(),
            approved
        );
    }
    assert_eq!(ids[0], ids[1]);
    assert_ne!(ids[0], ids[2]);
    assert_ne!(ids[0], ids[3]);
}

#[tokio::test]
async fn auth6_copilot_poll_rejects_invalid_or_stale_input() {
    let auth = CopilotAuth::new().unwrap();
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let owner = AuthAccountScope {
        tenant: &tenant,
        principal: "oidc:owner",
        host: "host",
        service: AuthService::GithubCopilot,
    };
    let started = start_poll_fixture(&auth, &flows, owner, "github.com").await;
    let due = Instant::now() + Duration::from_secs(6);
    let no_commit = |_: AuthAccountScope<'_>, _: &AuthAccount, _: &str, _: &str| {
        panic!("ineligible login attempted to persist")
    };
    assert_eq!(
        auth.poll_with(
            &flows,
            AuthAccountScope { host: "other", ..owner },
            &started,
            || due,
            |_| { panic!("foreign host reached GitHub") },
            no_commit
        )
        .await,
        Err(AuthErrorCode::NotFound)
    );

    for (status, reply, expected) in [
        (
            "200 OK",
            json!({"error":"access_denied", "error_description":"fixture-sensitive"}),
            Some(AuthErrorCode::AuthorizationDenied),
        ),
        ("200 OK", json!({"error":"expired_token"}), None),
        ("200 OK", json!({"access_token":""}), Some(AuthErrorCode::UpstreamFailure)),
        (
            "500 Internal Server Error",
            json!({"error":"authorization_pending"}),
            Some(AuthErrorCode::UpstreamFailure),
        ),
    ] {
        let started = start_poll_fixture(&auth, &flows, owner, "github.com").await;
        let (route, received) = oauth_server(status, "", reply).await;
        let result = auth
            .poll_with(
                &flows,
                owner,
                &started,
                || Instant::now() + Duration::from_secs(6),
                |outbound| {
                    *outbound.url_mut() = route.clone();
                },
                no_commit,
            )
            .await
            .unwrap();
        received.await.unwrap();
        assert!(!serde_json::to_string(&result).unwrap().contains("fixture-sensitive"));
        match expected {
            Some(code) => {
                assert!(matches!(result, AuthFlow::Failed { error, .. } if error.code == code))
            }
            None => assert!(matches!(result, AuthFlow::Expired { .. })),
        }
        assert!(started.grant.lock().unwrap().is_none());
    }

    let started = start_poll_fixture(&auth, &flows, owner, "enterprise.ghe.com").await;
    let AuthStartResponse::Pending { flow_id, .. } = &started.response;
    let now = Cell::new(Instant::now() + Duration::from_secs(6));
    let (exchange_route, exchange) =
        oauth_server("200 OK", "", json!({"access_token":"fixture-access-token"})).await;
    let (user_route, user) = oauth_server("200 OK", "", json!({"id":42,"login":"octocat"})).await;
    let mut routes = VecDeque::from([exchange_route, user_route]);
    let result = auth
        .poll_with(
            &flows,
            owner,
            &started,
            || now.get(),
            |outbound| {
                *outbound.url_mut() = routes.pop_front().unwrap();
                if routes.is_empty() {
                    flows.cancel(owner, flow_id, || now.get()).unwrap();
                }
            },
            no_commit,
        )
        .await
        .unwrap();
    exchange.await.unwrap();
    user.await.unwrap();
    assert!(matches!(result, AuthFlow::Cancelled { .. }));
    assert!(started.grant.lock().unwrap().is_none());
}
