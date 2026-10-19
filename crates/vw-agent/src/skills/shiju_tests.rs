//! 验证始居技能来源的查询、安装与错误映射行为。
//! 模拟始居目录覆盖私有列表、README、远端安装和显式版本，同时防止上游错误泄露凭据。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use super::*;
use vw_gateway_client::{
    ShijuCredential, ShijuMaintainedSkillQuery, ShijuSkillSearchQuery, ShijuSystemSkillQuery,
};

#[test]
fn shiju_skills_production_gateway_uses_sj_prefix() {
    assert_eq!(SHIJU_SKILL_BASE_URL, "https://gateway.shiju.cloud/sj/skill-hub/api/v1/");
}

fn fixture(responses: Vec<(String, String)>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind SHIJU skill fixture");
    let address = listener.local_addr().expect("fixture address");
    let handle = thread::spawn(move || {
        responses
            .into_iter()
            .map(|(status, body)| {
                let (mut stream, _) = listener.accept().expect("accept SHIJU skill request");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 2048];
                loop {
                    let read = stream.read(&mut buffer).expect("read SHIJU skill request");
                    request.extend_from_slice(&buffer[..read]);
                    if read == 0 || request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write SHIJU skill response");
                String::from_utf8(request).expect("request UTF-8")
            })
            .collect()
    });
    (format!("http://{address}"), handle)
}

fn envelope(data: serde_json::Value) -> String {
    serde_json::json!({"code": 200, "message": "", "data": data}).to_string()
}

fn credential() -> ShijuCredential {
    ShijuCredential::new("fixture-private-skill-secret").expect("credential")
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_skill_adapter_reads_markdown_readme_envelope() {
    let content = "# 使用说明\n支持 UTF-8 正文。";
    let (base, requests) = fixture(vec![(
        "200 OK".to_string(),
        envelope(serde_json::json!({
            "skill_id": 31,
            "version": "0.1.3",
            "markdown": content
        })),
    )]);
    let client = ShijuSkillClient::with_base_url(&base).expect("client");

    let readme = client.readme(&credential(), 31).await.expect("README");

    assert_eq!(readme.markdown.as_deref(), Some(content));
    let requests = requests.join().expect("join fixture");
    assert!(requests[0].starts_with("GET /skills/31/readme "));
    assert!(
        requests[0].to_ascii_lowercase().contains("x-api-key: fixture-private-skill-secret\r\n")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_skill_adapter_accepts_missing_version_readme() {
    let (base, requests) = fixture(vec![(
        "200 OK".to_string(),
        envelope(serde_json::json!({
            "skill_id": 31,
            "version": "0.1.3",
            "markdown": null
        })),
    )]);
    let client = ShijuSkillClient::with_base_url(&base).expect("client");

    let readme = client.readme(&credential(), 31).await.expect("README");

    assert_eq!(readme.markdown, None);
    requests.join().expect("join fixture");
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_skill_adapter_maps_all_private_catalog_paths() {
    let detail = serde_json::json!({
        "id": 31,
        "slug": "visual-skill",
        "name": "视觉技能",
        "description": "分析图片",
        "level": "company",
        "modality": "multimodal",
        "format": "skill_md",
        "company_id": "company-1",
        "created_uid": "user-1",
        "maintainer_uid": "user-2",
        "status": "published",
        "is_all_company_accessible": 1,
        "current_version_id": 88,
        "current_version": null,
        "created_at": "2026-07-01T00:00:00Z",
        "updated_at": "2026-07-02T00:00:00Z",
        "tags": []
    });
    let (base, requests) = fixture(vec![
        (
            "200 OK".to_string(),
            envelope(serde_json::json!({
                "items": [],
                "next_cursor": "cursor-2",
                "has_more": true
            })),
        ),
        ("200 OK".to_string(), envelope(serde_json::json!([]))),
        (
            "200 OK".to_string(),
            envelope(serde_json::json!({"items": [], "page": 2, "page_size": 20, "total": 0})),
        ),
        (
            "200 OK".to_string(),
            envelope(serde_json::json!({"items": [], "page": 4, "page_size": 20, "total": 0})),
        ),
        ("200 OK".to_string(), envelope(detail)),
        ("200 OK".to_string(), envelope(serde_json::json!([]))),
    ]);
    let client = ShijuSkillClient::with_base_url(&base).expect("client");
    let credential = credential();

    let search = client
        .search(
            &credential,
            &ShijuSkillSearchQuery {
                query: Some("视觉 & 分析".to_string()),
                cursor: Some("cursor/next".to_string()),
                page_size: 20,
            },
        )
        .await
        .expect("search");
    client.installed(&credential).await.expect("installed");
    client
        .system(
            &credential,
            &ShijuSystemSkillQuery {
                page: 2,
                page_size: 20,
                name: Some("视觉".to_string()),
                tag_id: Some(2),
            },
        )
        .await
        .expect("system");
    client
        .maintained(
            &credential,
            &ShijuMaintainedSkillQuery {
                page: 4,
                page_size: 20,
                keyword: Some("个人".to_string()),
                tag_ids: vec![2, 1],
                company_tag_ids: vec![5],
                statuses: vec![
                    "draft".to_string(),
                    "pending_review".to_string(),
                    "published".to_string(),
                    "unpublished".to_string(),
                ],
            },
        )
        .await
        .expect("maintained");
    let detail = client.detail(&credential, 31).await.expect("detail");
    client.versions(&credential, 31).await.expect("versions");

    assert_eq!(search.next_cursor.as_deref(), Some("cursor-2"));
    assert_eq!(detail.name, "视觉技能");
    let requests = requests.join().expect("join fixture");
    let expected = [
        "GET /marketplace/skills?page_size=20&q=%E8%A7%86%E8%A7%89+%26+%E5%88%86%E6%9E%90&cursor=cursor%2Fnext ",
        "GET /me/skills ",
        "GET /tenant/system-skills?page=2&page_size=20&name=%E8%A7%86%E8%A7%89&tag_id=2 ",
        "GET /me/maintained-skills?page=4&page_size=20&keyword=%E4%B8%AA%E4%BA%BA&tag_ids=2%2C1&company_tag_ids=5&statuses=draft%2Cpending_review%2Cpublished%2Cunpublished ",
        "GET /skills/31 ",
        "GET /skills/31/versions ",
    ];
    for (request, expected) in requests.iter().zip(expected) {
        assert!(request.starts_with(expected), "{request}");
        assert!(
            request.to_ascii_lowercase().contains("x-api-key: fixture-private-skill-secret\r\n")
        );
        assert!(!request.lines().next().unwrap_or_default().contains(credential.expose()));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_skill_adapter_uses_current_default_catalog_contracts() {
    let page = envelope(serde_json::json!({
        "items": [],
        "page": 1,
        "page_size": 20,
        "total": 0
    }));
    let (base, requests) =
        fixture(vec![("200 OK".to_string(), page.clone()), ("200 OK".to_string(), page)]);
    let client = ShijuSkillClient::with_base_url(&base).expect("client");
    let credential = credential();

    client
        .search(&credential, &ShijuSkillSearchQuery { query: None, cursor: None, page_size: 20 })
        .await
        .expect("marketplace");
    client
        .maintained(
            &credential,
            &ShijuMaintainedSkillQuery {
                page: 1,
                page_size: 20,
                keyword: None,
                tag_ids: Vec::new(),
                company_tag_ids: Vec::new(),
                statuses: Vec::new(),
            },
        )
        .await
        .expect("maintained");

    let requests = requests.join().expect("join fixture");
    let expected =
        ["GET /marketplace/skills?page_size=20 ", "GET /me/maintained-skills?page=1&page_size=20 "];
    for (request, expected) in requests.iter().zip(expected) {
        assert!(request.starts_with(expected), "{request}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_skill_adapter_maps_remote_install_relationship() {
    let (base, requests) = fixture(vec![
        ("204 No Content".to_string(), String::new()),
        ("204 No Content".to_string(), String::new()),
    ]);
    let client = ShijuSkillClient::with_base_url(&base).expect("client");
    let credential = credential();

    client.remote_install(&credential, 31).await.expect("install");
    client.remote_uninstall(&credential, 31).await.expect("uninstall");

    let requests = requests.join().expect("join fixture");
    assert!(requests[0].starts_with("POST /skills/31/install "));
    assert!(requests[1].starts_with("DELETE /skills/31/install "));
    for request in requests {
        assert!(
            request.to_ascii_lowercase().contains("x-api-key: fixture-private-skill-secret\r\n")
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_skill_adapter_forwards_explicit_version_id() {
    let (base, requests) = fixture(vec![("204 No Content".to_string(), String::new())]);
    let client = ShijuSkillClient::with_base_url(&base).expect("client");
    let credential = credential();

    client.remote_install_version(&credential, 31, 88).await.expect("install explicit version");

    let requests = requests.join().expect("join fixture");
    assert!(requests[0].starts_with("POST /skills/31/install "));
    assert!(requests[0].ends_with(r#"{"version_id":88}"#));
    assert!(
        requests[0].to_ascii_lowercase().contains("x-api-key: fixture-private-skill-secret\r\n")
    );
    assert!(!requests[0].lines().next().unwrap_or_default().contains(credential.expose()));
}

#[tokio::test(flavor = "multi_thread")]
// 分页参数先在本地拒绝，上游错误只保留稳定上下文且不得包含凭据。
async fn shiju_skill_adapter_rejects_invalid_paging_and_upstream_errors_without_secrets() {
    let client = ShijuSkillClient::with_base_url("http://127.0.0.1:9").expect("client");
    let credential = credential();
    let invalid = client
        .system(
            &credential,
            &ShijuSystemSkillQuery { page: 0, page_size: 20, name: None, tag_id: None },
        )
        .await
        .expect_err("zero page");
    assert!(invalid.to_string().contains("page"));

    let invalid = client.readme(&credential, 0).await.expect_err("zero skill ID");
    assert!(invalid.to_string().contains("skill id"));

    let invalid = client
        .maintained(
            &credential,
            &ShijuMaintainedSkillQuery {
                page: 1,
                page_size: 20,
                keyword: None,
                tag_ids: vec![2, 0, 1],
                company_tag_ids: Vec::new(),
                statuses: vec!["draft".to_string()],
            },
        )
        .await
        .expect_err("zero tag ID");
    assert!(invalid.to_string().contains("tag_ids"));

    let invalid = client
        .maintained(
            &credential,
            &ShijuMaintainedSkillQuery {
                page: 1,
                page_size: 20,
                keyword: None,
                tag_ids: vec![2, 1],
                company_tag_ids: vec![5, 0],
                statuses: vec!["draft".to_string()],
            },
        )
        .await
        .expect_err("zero company tag ID");
    assert!(invalid.to_string().contains("company_tag_ids"));

    let invalid = client
        .maintained(
            &credential,
            &ShijuMaintainedSkillQuery {
                page: 1,
                page_size: 20,
                keyword: None,
                tag_ids: vec![2, 1],
                company_tag_ids: Vec::new(),
                statuses: vec!["archived".to_string()],
            },
        )
        .await
        .expect_err("unknown status");
    assert!(invalid.to_string().contains("statuses"));

    let (base, requests) = fixture(vec![
        (
            "503 Service Unavailable".to_string(),
            r#"{"code":503,"message":"secret upstream detail","data":null}"#.to_string(),
        ),
        (
            "200 OK".to_string(),
            r#"{"code":403,"message":"secret business detail","data":null}"#.to_string(),
        ),
    ]);
    let client = ShijuSkillClient::with_base_url(&base).expect("client");
    let http_error = client.installed(&credential).await.expect_err("HTTP error");
    let business_error = client.installed(&credential).await.expect_err("business error");
    let errors = format!("{http_error:?}{business_error:?}");
    assert!(errors.contains("503"));
    assert!(errors.contains("403"));
    assert!(!errors.contains(credential.expose()));
    assert!(!errors.contains("secret upstream detail"));
    assert!(!errors.contains("secret business detail"));
    requests.join().expect("join error fixture");
}
