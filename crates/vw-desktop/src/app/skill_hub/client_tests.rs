//! 技能中心客户端测试，验证生产网关路径前缀及响应夹具的协议映射。
//! 客户端构造保持在测试端点内，避免测试依赖真实服务或账户环境。

use super::*;
use crate::app::account::SecretSkey;
use crate::app::skill_hub::client::SKILL_HUB_BASE_URL;
use crate::app::skill_hub::models::ShijuPackageId;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn skill_hub_production_gateway_uses_sj_prefix() {
    assert_eq!(SKILL_HUB_BASE_URL, "https://gateway.shiju.cloud/sj/skill-hub/api/v1/");
}

fn fixture(status: &str, body: serde_json::Value) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind skill hub fixture");
    let address = listener.local_addr().expect("fixture address");
    let status = status.to_string();
    let body = body.to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept skill hub request");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = stream.read(&mut buffer).expect("read skill hub request");
            request.extend_from_slice(&buffer[..read]);
            if read == 0 || request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).expect("write skill hub response");
        String::from_utf8(request).expect("request UTF-8")
    });
    (format!("http://{address}"), handle)
}

fn client(base_url: &str) -> SkillHubClient {
    SkillHubClient::with_base_url(
        SecretSkey::new("fixture-package-secret").expect("credential"),
        base_url,
    )
    .expect("client")
}

#[tokio::test(flavor = "multi_thread")]
async fn latest_shiju_package_remains_direct_for_existing_installer_flow() {
    let (base_url, request) = fixture(
        "200 OK",
        serde_json::json!({
            "code": 200,
            "message": "ok",
            "data": {
                "download_url":
                    "https://skill.shiju.cloud/hub/releases/3.0.3/shiju.zip?signature=release-secret"
            }
        }),
    );

    let release = client(&base_url)
        .latest_shiju_package(ShijuPackageId::DarwinArm64)
        .await
        .expect("latest package");

    assert!(release.download_url.as_str().contains("/shiju.zip"));
    assert_eq!(format!("{:?}", release.download_url), "SkillDownloadUrl(<redacted>)");
    let request = request.join().expect("join fixture");
    assert!(
        request.starts_with("GET /skills/hub/latest?package_id=shiju-darwin-arm64 HTTP/1.1\r\n")
    );
    assert!(request.to_ascii_lowercase().contains("x-api-key: fixture-package-secret\r\n"));
}

#[tokio::test(flavor = "multi_thread")]
async fn latest_shiju_package_errors_remain_redacted() {
    let (base_url, request) = fixture(
        "403 Forbidden",
        serde_json::json!({"code": 403, "message": "forbidden", "data": null}),
    );

    let error = client(&base_url)
        .latest_shiju_package(ShijuPackageId::DarwinArm64)
        .await
        .expect_err("HTTP error");

    assert!(error.to_string().contains("403"));
    assert!(!format!("{error:?}").contains("fixture-package-secret"));
    request.join().expect("join fixture");
}
