use super::{
    copilot_tests::oauth_server,
    dsh_login::{DshDevice, DshLogin},
    flow::{AuthAccountScope, AuthFlows},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Request, StatusCode, Url};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{io::Write, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    time::timeout,
};
use uuid::Uuid;
use vw_api_types::{
    agent_apps_auth::{
        AuthErrorCode, AuthFlow, AuthFlowId, AuthLoginMethod, AuthService, AuthStartRequest,
        AuthStartResponse,
    },
    id::TenantScope,
};

const ISSUER: &str = "https://platform.deepseek.com";
const AUTHORIZE: &str = "https://platform.deepseek.com/dsh/authorize?attempt=fixture";
const DEVICE_ID: Uuid = Uuid::from_u128(0x4826593b_e124_4af4_8c75_eb740b7318b7);

fn request() -> AuthStartRequest {
    AuthStartRequest {
        method: AuthLoginMethod::Browser,
        github_domain: None,
        region: None,
        target_account_id: None,
    }
}

fn device() -> DshDevice {
    DshDevice { id: DEVICE_ID, locale: "zh-CN".into(), timezone_offset_seconds: 28800 }
}

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "oidc:owner",
        host: "test-host",
        service: AuthService::DshAccount,
    }
}

fn envelope(data: Value) -> Value {
    json!({"code": 0, "data": {"biz_code": 0, "biz_data": data}})
}

fn init(url: &str) -> Value {
    envelope(json!({"authorize_url": url, "authorize_id": "fixture-attempt", "expires_in": 90}))
}

fn route(method: &'static str, url: Url) -> impl Fn(&mut Request) + Clone + Send + Sync + 'static {
    move |request| {
        assert_eq!(request.url().as_str(), format!("{ISSUER}/auth-api/v0/dsh/{method}"));
        assert_eq!(request.method(), reqwest::Method::POST);
        assert!(!request.headers().contains_key("authorization"));
        assert!(!request.headers().contains_key("x-dsh-auth-token"));
        assert_eq!(request.headers()["x-client-platform"], "web");
        assert_eq!(request.headers()["x-client-bundle-id"], "");
        assert_eq!(request.headers()["x-client-locale"], "zh_CN");
        assert_eq!(request.headers()["x-client-timezone-offset"], "28800");
        let mut target = url.clone();
        target.set_path(request.url().path());
        *request.url_mut() = target;
    }
}

fn request_body(request: &str, method: &str) -> Value {
    assert!(request.starts_with(&format!("POST /auth-api/v0/dsh/{method} HTTP/1.1\r\n")));
    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
}

fn params(login: &DshLogin) -> &AuthFlowId {
    let AuthStartResponse::Pending { flow_id, verification_url, user_code, can_paste, .. } =
        &login.response;
    assert_eq!(verification_url, AUTHORIZE);
    assert!(user_code.is_none() && can_paste.is_none());
    flow_id
}

fn browser() -> Client {
    Client::builder().no_proxy().timeout(Duration::from_secs(3)).build().unwrap()
}

fn callback(init: &Value, state: &str) -> Url {
    let mut uri = Url::parse(init["redirect_uri"].as_str().unwrap()).unwrap();
    assert_eq!(uri.scheme(), "http");
    assert_eq!(uri.host_str(), Some("127.0.0.1"));
    assert_eq!(uri.path(), "/oauth/callback");
    uri.query_pairs_mut().extend_pairs([("state", state), ("code", "fixture-code")]);
    uri
}

#[tokio::test]
async fn auth6_dsh_login_commits_expected_scoped_result() {
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let owner = scope(&tenant);
    let (init_url, init_received) = oauth_server("200 OK", "", init(AUTHORIZE)).await;
    let started = DshLogin::start_with(
        &flows,
        owner,
        request(),
        true,
        device(),
        route("auth_init", init_url),
    )
    .await;
    assert_eq!(started.as_ref().err(), None, "local DSH login must start");
    let login = started.unwrap();
    let flow_id = params(&login);
    let begin = request_body(&init_received.await.unwrap(), "auth_init");
    let state = begin["state"].as_str().unwrap();
    assert_ne!(flow_id.0, state);
    assert_eq!(begin["code_challenge_method"], "S256");
    assert_eq!(begin["locale"], "zh_CN");
    assert_eq!(begin["login_source"], "web");
    assert!(begin.get("code_verifier").is_none());

    let browser = browser();
    assert_eq!(browser.get(callback(&begin, state)).send().await.unwrap().status(), StatusCode::OK);
    let (exchange_url, exchange_received) =
        oauth_server("200 OK", "", envelope(json!({"token": "fixture-grant"}))).await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scoped-grant");
    let done = login
        .poll_with(&flows, owner, route("auth_exchange", exchange_url), |bound, account, grant| {
            assert!(bound == owner);
            assert_eq!(account.service, owner.service);
            assert!(!account.is_default);
            let mut output = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
            writeln!(output, "{}\n{}", grant.issuer(), grant.token()).unwrap();
            output.persist(&path).unwrap();
            Ok(())
        })
        .await
        .unwrap();
    assert!(matches!(&done, AuthFlow::Approved { flow_id: id, .. } if id == flow_id));
    assert_eq!(std::fs::read_to_string(path).unwrap(), format!("{ISSUER}\nfixture-grant\n"));
    let public = serde_json::to_string(&done).unwrap();
    assert!(!public.contains("fixture-grant") && !public.contains("fixture-code"));

    let exchange = request_body(&exchange_received.await.unwrap(), "auth_exchange");
    assert_eq!(exchange["code"], "fixture-code");
    assert_eq!(exchange["redirect_uri"], begin["redirect_uri"]);
    assert_eq!(exchange["device_id"], DEVICE_ID.to_string());
    assert!(exchange["device_model"].as_str().unwrap().starts_with("darwin-"));
    assert!(exchange["os_version"].as_str().unwrap().starts_with("darwin "));
    let verifier = exchange["code_verifier"].as_str().unwrap();
    assert!((43..=128).contains(&verifier.len()));
    assert_ne!(verifier, state);
    assert_eq!(begin["code_challenge"], URL_SAFE_NO_PAD.encode(Sha256::digest(verifier)));
    assert!(!public.contains(verifier));
    assert_eq!(
        login
            .poll_with(
                &flows,
                owner,
                |_| panic!("duplicate exchange"),
                |_, _, _| panic!("duplicate commit")
            )
            .await
            .unwrap(),
        done
    );
}

#[tokio::test]
async fn auth6_dsh_login_rejects_invalid_or_stale_input() {
    let flows = AuthFlows::default();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let owner = scope(&tenant);
    let stranger = AuthAccountScope { principal: "oidc:stranger", ..owner };
    let denied = DshLogin::start_with(&flows, owner, request(), false, device(), |_| {
        panic!("unreachable browser")
    });
    assert_eq!(denied.await.err(), Some(AuthErrorCode::CallbackUnreachable));

    for url in [
        "http://platform.deepseek.com/dsh/authorize",
        "https://platform.deepseek.com.attacker.test/dsh/authorize",
        "https://x@platform.deepseek.com/dsh/authorize",
        "https://platform.deepseek.com/dsh/other",
        "https://platform.deepseek.com/dsh/authorize#bad",
    ] {
        let (server, request_log) = oauth_server("200 OK", "", init(url)).await;
        let result = DshLogin::start_with(
            &flows,
            owner,
            request(),
            true,
            device(),
            route("auth_init", server),
        )
        .await;
        assert_eq!(result.err(), Some(AuthErrorCode::UpstreamFailure));
        request_log.await.unwrap();
    }

    let (server, init_log) = oauth_server("200 OK", "", init(AUTHORIZE)).await;
    let (cancel_server, cancel_log) = oauth_server("200 OK", "", envelope(json!({}))).await;
    let login = DshLogin::start_with(&flows, owner, request(), true, device(), move |request| {
        if request.url().path().ends_with("/auth_cancel") {
            route("auth_cancel", cancel_server.clone())(request);
        } else {
            route("auth_init", server.clone())(request);
        }
    })
    .await
    .unwrap();
    let begin = request_body(&init_log.await.unwrap(), "auth_init");
    let state = begin["state"].as_str().unwrap();
    let client = browser();
    assert_eq!(
        client.get(callback(&begin, "bad")).send().await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    let mut duplicate = callback(&begin, state);
    duplicate.query_pairs_mut().append_pair("code", "duplicate");
    assert_eq!(client.get(duplicate).send().await.unwrap().status(), StatusCode::BAD_REQUEST);
    let callback_url = callback(&begin, state);
    let mut socket = TcpStream::connect(("127.0.0.1", callback_url.port().unwrap())).await.unwrap();
    let mut incomplete_header = format!(
        "GET {}?{} HTTP/1.1\r\nHost: localhost\r\nX-Pad: ",
        callback_url.path(),
        callback_url.query().unwrap()
    )
    .into_bytes();
    // Exhaust the callback's buffer without leaving unread bytes in the socket.
    incomplete_header.resize(8 * 1024, b'a');
    socket.write_all(&incomplete_header).await.unwrap();
    let mut response = String::new();
    timeout(Duration::from_secs(3), socket.read_to_string(&mut response)).await.unwrap().unwrap();
    assert!(response.starts_with("HTTP/1.1 431 "));
    assert_eq!(client.get(callback_url).send().await.unwrap().status(), StatusCode::OK);
    assert_eq!(login.cancel(&flows, stranger), Err(AuthErrorCode::NotFound));

    let (server, exchange_log) =
        oauth_server("200 OK", "", envelope(json!({"token": "late-grant"}))).await;
    let (entered, exchanging) = oneshot::channel();
    let poll = login.poll_with(
        &flows,
        owner,
        |outbound| {
            route("auth_exchange", server)(outbound);
            entered.send(()).unwrap();
        },
        |_, _, _| panic!("cancelled exchange committed"),
    );
    let cancel = async {
        exchanging.await.unwrap();
        assert_eq!(login.cancel(&flows, owner), Ok(()));
    };
    let (result, ()) = tokio::join!(poll, cancel);
    assert!(matches!(result.unwrap(), AuthFlow::Cancelled { .. }));
    let exchange = request_body(&exchange_log.await.unwrap(), "auth_exchange");
    let cancelled = timeout(Duration::from_secs(3), cancel_log).await.unwrap().unwrap();
    let cancelled = request_body(&cancelled, "auth_cancel");
    assert_eq!(cancelled["authorize_id"], "fixture-attempt");
    assert_eq!(cancelled["code_verifier"], exchange["code_verifier"]);
    assert_eq!(
        begin["code_challenge"],
        URL_SAFE_NO_PAD.encode(Sha256::digest(cancelled["code_verifier"].as_str().unwrap()))
    );
    assert!(client.get(callback(&begin, state)).send().await.is_err());
    assert!(matches!(
        login
            .poll_with(
                &flows,
                owner,
                |_| panic!("cancelled exchange"),
                |_, _, _| panic!("cancelled commit")
            )
            .await
            .unwrap(),
        AuthFlow::Cancelled { .. }
    ));

    let sink = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let location = format!("Location: http://{}/capture\r\n", sink.local_addr().unwrap());
    for response in [
        ("307 Temporary Redirect", location.as_str(), json!({})),
        (
            "200 OK",
            "",
            envelope(json!({"token": "fixture-grant", "issuer": "https://attacker.test"})),
        ),
    ] {
        let (server, init_log) = oauth_server("200 OK", "", init(AUTHORIZE)).await;
        let login = DshLogin::start_with(
            &flows,
            owner,
            request(),
            true,
            device(),
            route("auth_init", server),
        )
        .await
        .unwrap();
        let begin = request_body(&init_log.await.unwrap(), "auth_init");
        assert_eq!(
            client
                .get(callback(&begin, begin["state"].as_str().unwrap()))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let (server, exchange_log) = oauth_server(response.0, response.1, response.2).await;
        let result = login
            .poll_with(&flows, owner, route("auth_exchange", server), |_, _, _| {
                panic!("invalid exchange committed")
            })
            .await
            .unwrap();
        assert!(
            matches!(result, AuthFlow::Failed { error, .. } if error.code == AuthErrorCode::UpstreamFailure)
        );
        exchange_log.await.unwrap();
    }
    assert!(timeout(Duration::from_millis(100), sink.accept()).await.is_err());
}
