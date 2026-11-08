use super::{
    copilot::CopilotAuth,
    flow::{AuthAccountScope, AuthFlows},
};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
use vw_api_types::{
    agent_apps_auth::{
        AuthErrorCode, AuthFlow, AuthLoginMethod, AuthService, AuthStartRequest, AuthStartResponse,
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
            .unwrap()
            .1
            .trim()
            .parse()
            .unwrap();
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
