//! 验证始居账号状态的序列化、缓存与错误映射契约。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use super::*;

fn fixture(responses: Vec<String>) -> (String, thread::JoinHandle<Vec<String>>) {
    fixture_with_status(responses.into_iter().map(|body| ("200 OK".to_string(), body)).collect())
}

fn fixture_with_status(
    responses: Vec<(String, String)>,
) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind SHIJU fixture");
    let address = listener.local_addr().expect("fixture address");
    let handle = thread::spawn(move || {
        responses
            .into_iter()
            .map(|(status, body)| {
                let (mut stream, _) = listener.accept().expect("accept SHIJU request");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 2048];
                loop {
                    let read = stream.read(&mut buffer).expect("read SHIJU request");
                    request.extend_from_slice(&buffer[..read]);
                    if read == 0 {
                        break;
                    }
                    let Some(header_end) =
                        request.windows(4).position(|window| window == b"\r\n\r\n")
                    else {
                        continue;
                    };
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    if request.len() >= header_end + 4 + content_length {
                        break;
                    }
                }
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write SHIJU response");
                String::from_utf8(request).expect("request UTF-8")
            })
            .collect()
    });
    (format!("http://{address}"), handle)
}

fn envelope(data: serde_json::Value) -> String {
    serde_json::json!({"code": 200, "message": "", "data": data}).to_string()
}

#[test]
fn shiju_account_production_routes_login_and_iam_separately() {
    assert_eq!(PASSPORT_BASE_URL, "https://passport.shiju.cloud");
    assert_eq!(GATEWAY_BASE_URL, "https://gateway.shiju.cloud/sj/");
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_account_adapter_maps_login_poll_and_profile() {
    let credential = "fixture-credential-secret";
    let (base, requests) = fixture(vec![
        envelope(serde_json::json!({
            "state": "fixture-state",
            "authUrl": "https://auth.shiju.cloud:443/plugin/login?state=fixture-state",
            "poll_interval": 2
        })),
        envelope(serde_json::json!({"status": "approved", "api_key": credential})),
        envelope(serde_json::json!({
            "company": {"company_name": "示例公司", "status": "enabled"},
            "current_user": {
                "name": "测试用户",
                "account_name": "tester",
                "status": "enabled",
                "departments": ["AI 创新组"],
                "roles": ["管理员"]
            }
        })),
    ]);
    let client = ShijuAccountClient::with_base_urls(&base, &base).expect("client");

    let challenge = client.start_login().await.expect("login");
    let poll = client.poll_login(challenge.state()).await.expect("poll");
    let profile = client.profile(credential).await.expect("profile");

    assert_eq!(challenge.state(), "fixture-state");
    assert_eq!(challenge.poll_interval_seconds(), 2);
    assert!(matches!(poll, ShijuAccountPoll::Approved(ref value) if value == credential));
    assert_eq!(profile.company_name, "示例公司");
    assert_eq!(profile.departments, ["AI 创新组"]);

    let requests = requests.join().expect("join fixture");
    assert!(requests[0].starts_with("POST /api/v1/plugin/login HTTP/1.1\r\n"));
    assert!(requests[0].contains(r#"{"source":"shiju-cli"}"#));
    assert!(requests[1].starts_with("GET /api/v1/plugin/auth/token?state=fixture-state "));
    assert!(requests[2].starts_with("GET /iam/api/v1/dashboard/basic-info "));
    assert!(
        requests[2]
            .to_ascii_lowercase()
            .contains(&format!("x-api-key: {credential}").to_ascii_lowercase())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_account_adapter_rejects_untrusted_authorization_url() {
    let state = "fixture-state-secret";
    let credential = "fixture-credential-secret";
    let (base, requests) = fixture(vec![envelope(serde_json::json!({
        "state": state,
        "authUrl": format!("https://example.test/login?state={state}"),
        "poll_interval": 2
    }))]);
    let client = ShijuAccountClient::with_base_urls(&base, &base).expect("client");

    let error = client.start_login().await.expect_err("untrusted URL");

    assert_eq!(error.to_string(), "SHIJU authorization URL is not trusted");
    assert!(!error.to_string().contains(state));
    assert!(!error.to_string().contains(credential));
    requests.join().expect("join fixture");
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_account_me_uses_x_api_key_and_maps_profile_update() {
    let credential = "fixture-account-me-secret";
    let (base, requests) = fixture(vec![
        envelope(serde_json::json!({
            "user_id": "101026",
            "company_id": "10011",
            "account_id": "101026",
            "account_name": "liuxiangmin",
            "name": "刘祥敏2",
            "source_type": "manual",
            "external_user_id": "",
            "allow_console_login": true,
            "status": "enabled",
            "departments": [{
                "department_id": "101014",
                "department_name": "AI创新组",
                "parent_department_id": "101005"
            }],
            "roles": [{
                "role_id": "102",
                "role_name": "默认角色",
                "role_code": "default"
            }]
        })),
        envelope(serde_json::json!({
            "user_id": "101026",
            "company_id": "10011",
            "name": "刘祥敏3",
            "status": "enabled"
        })),
    ]);
    let client = ShijuAccountClient::with_base_urls(&base, &base).expect("client");

    let me = client.current_user(credential).await.expect("current user");
    let update = vw_gateway_client::ShijuAccountUpdateRequest {
        name: "刘祥敏3".to_string(),
        username: "liuxiangmin".to_string(),
    };
    let updated =
        client.update_current_user(credential, &update).await.expect("updated current user");

    assert_eq!(me.account_name, "liuxiangmin");
    assert_eq!(me.departments[0].department_name, "AI创新组");
    assert_eq!(me.roles[0].role_name, "默认角色");
    assert_eq!(updated.name, "刘祥敏3");

    let requests = requests.join().expect("join fixture");
    assert!(requests[0].starts_with("GET /iam/api/v1/me HTTP/1.1\r\n"));
    assert!(requests[1].starts_with("PUT /iam/api/v1/me HTTP/1.1\r\n"));
    for request in &requests {
        let lower = request.to_ascii_lowercase();
        assert!(lower.contains(&format!("x-api-key: {credential}").to_ascii_lowercase()));
        assert!(!lower.contains("\r\ncookie:"));
    }
    assert!(requests[1].ends_with(r#"{"name":"刘祥敏3","username":"liuxiangmin"}"#));
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_account_password_uses_x_api_key_without_cookie() {
    let credential = "fixture-password-secret";
    let old_password = "old-password-123";
    let new_password = "new-password-456";
    let (base, requests) = fixture(vec![envelope(serde_json::Value::Null)]);
    let client = ShijuAccountClient::with_base_urls(&base, &base).expect("client");
    let request = vw_gateway_client::ShijuAccountPasswordUpdateRequest {
        old_password: old_password.to_string(),
        new_password: new_password.to_string(),
    };

    client.update_password(credential, &request).await.expect("password update");

    let requests = requests.join().expect("join fixture");
    let request = &requests[0];
    assert!(request.starts_with("PUT /iam/api/v1/me/password HTTP/1.1\r\n"));
    let lower = request.to_ascii_lowercase();
    assert!(lower.contains(&format!("x-api-key: {credential}").to_ascii_lowercase()));
    assert!(!lower.contains("\r\ncookie:"));
    assert!(
        request
            .ends_with(r#"{"old_password":"old-password-123","new_password":"new-password-456"}"#)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_account_password_preserves_client_business_error() {
    let message = "用户名或密码错误，请重新输入";
    let (base, requests) = fixture_with_status(vec![(
        "500 Internal Server Error".to_string(),
        serde_json::json!({
            "code": 401501011,
            "message": message,
            "data": null
        })
        .to_string(),
    )]);
    let client = ShijuAccountClient::with_base_urls(&base, &base).expect("client");
    let request = vw_gateway_client::ShijuAccountPasswordUpdateRequest {
        old_password: "wrong-password-123".to_string(),
        new_password: "new-password-456".to_string(),
    };

    let error =
        client.update_password("fixture-password-secret", &request).await.expect_err("password");

    assert_eq!(error.status(), Some(StatusCode::UNAUTHORIZED));
    assert_eq!(error.to_string(), message);
    requests.join().expect("join fixture");
}
