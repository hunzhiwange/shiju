//! 验证 SkillHub 市场接口的响应解析、版本与下载契约。
//! 伪服务按请求序列返回目录、详情、技能集和制品，重定向仅信任固定 HTTPS 对象路径。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use serde_json::json;
use vw_gateway_client::{
    SkillMarketCoordinateDto, SkillMarketSkillLocatorDto, SkillMarketSkillQuery, SkillMarketSort,
};

use super::provider::{SkillMarketProvider, SkillMarketProviderError};
use super::skillhub::{SkillHubProvider, skill_artifact_redirect_url, skill_file_redirect_url};

// 响应序列夹具保留收到的请求，既验证 DTO 映射也验证查询参数编码。
struct FixtureResponse {
    status: &'static str,
    content_type: &'static str,
    body: String,
    location: Option<&'static str>,
}

impl FixtureResponse {
    fn json(body: serde_json::Value) -> Self {
        Self {
            status: "200 OK",
            content_type: "application/json",
            body: body.to_string(),
            location: None,
        }
    }

    fn raw(status: &'static str, content_type: &'static str, body: &str) -> Self {
        Self { status, content_type, body: body.to_string(), location: None }
    }

    fn redirect(location: &'static str) -> Self {
        Self {
            status: "302 Found",
            content_type: "text/plain",
            body: String::new(),
            location: Some(location),
        }
    }
}

fn fixture_sequence(responses: Vec<FixtureResponse>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind SkillHub fixture");
    let address = listener.local_addr().expect("read SkillHub fixture address");
    let handle = thread::spawn(move || {
        responses.into_iter().map(|response| respond(&listener, response)).collect()
    });
    (format!("http://{address}"), handle)
}

fn respond(listener: &TcpListener, response: FixtureResponse) -> String {
    let (mut stream, _) = listener.accept().expect("accept SkillHub request");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).expect("read SkillHub request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    let location =
        response.location.map(|location| format!("location: {location}\r\n")).unwrap_or_default();
    let reply = format!(
        "HTTP/1.1 {}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n{}\r\n{}",
        response.status,
        response.content_type,
        response.body.len(),
        location,
        response.body
    );
    stream.write_all(reply.as_bytes()).expect("write SkillHub response");
    String::from_utf8(request).expect("SkillHub request is UTF-8")
}

fn provider(base_url: &str) -> SkillHubProvider {
    SkillHubProvider::with_base_url(base_url).expect("build fixture provider")
}

fn locator(namespace: &str, slug: &str) -> SkillMarketSkillLocatorDto {
    SkillMarketSkillLocatorDto {
        source: "skillhub".to_string(),
        namespace: namespace.to_string(),
        slug: slug.to_string(),
    }
}

#[test]
fn skillhub_file_redirect_accepts_only_the_trusted_https_object_path() {
    for trusted in [
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com/skills/demo/1.2.3/files/SKILL.md",
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com/orgs/org-bv6b8qcb/tencent-docs/1.0.41/files/SKILL.md",
    ] {
        assert_eq!(skill_file_redirect_url(trusted).expect("trusted redirect").as_str(), trusted);
    }

    for untrusted in [
        "http://skillhub-1388575217.cos.accelerate.myqcloud.com/skills/demo/1.2.3/files/SKILL.md",
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com.evil.test/skills/demo/1.2.3/files/SKILL.md",
        "https://user@skillhub-1388575217.cos.accelerate.myqcloud.com/skills/demo/1.2.3/files/SKILL.md",
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com/other/demo/1.2.3/files/SKILL.md",
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com/orgs//demo/1.2.3/files/SKILL.md",
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com/orgs/team/demo/1.2.3/files/SKILL.md?token=secret",
    ] {
        assert!(skill_file_redirect_url(untrusted).is_err(), "{untrusted}");
    }
}

#[test]
fn skillhub_artifact_redirect_accepts_only_the_trusted_https_zip_path() {
    for trusted in [
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com/skills/demo/1.2.3.zip",
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com/skills/11288/tencent-docs/1.0.41.zip",
    ] {
        assert_eq!(
            skill_artifact_redirect_url(trusted).expect("trusted redirect").as_str(),
            trusted
        );
    }

    for untrusted in [
        "http://skillhub-1388575217.cos.accelerate.myqcloud.com/skills/demo/1.2.3.zip",
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com.evil.test/skills/demo/1.2.3.zip",
        "https://user@skillhub-1388575217.cos.accelerate.myqcloud.com/skills/demo/1.2.3.zip",
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com/other/demo/1.2.3.zip",
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com/skills/not-numeric/demo/1.2.3.zip",
        "https://skillhub-1388575217.cos.accelerate.myqcloud.com/skills/demo/1.2.3.zip?token=secret",
    ] {
        assert!(skill_artifact_redirect_url(untrusted).is_err(), "{untrusted}");
    }
}

fn skill_summary() -> serde_json::Value {
    json!({
        "category": "development",
        "description": "Builds reliable tools",
        "downloads": 42,
        "iconUrl": "https://cdn.example/icon.png",
        "installs": 7,
        "labels": {"requires_api_key": "false"},
        "name": "Fixture Skill",
        "namespace": {
            "canonicalName": "@fixture/fixture-skill",
            "displayName": "Fixture",
            "handle": "fixture",
            "publicSlug": "fixture-skill"
        },
        "ownerName": "Fixture Author",
        "score": 4.75,
        "slug": "fixture-skill",
        "subCategories": [{"key": "rust", "name": "Rust"}],
        "tags": null,
        "updated_at": 1785369600000_u64,
        "version": "1.2.3"
    })
}

fn skill_detail() -> serde_json::Value {
    json!({
        "latestVersion": {
            "changelog": "Stable",
            "createdAt": 1785369600000_u64,
            "version": "1.2.3"
        },
        "namespace": {
            "canonicalName": "@fixture/fixture-skill",
            "displayName": "Fixture",
            "handle": "fixture",
            "publicSlug": "fixture-skill"
        },
        "owner": {
            "displayName": "Fixture Author",
            "handle": "fixture",
            "image": null
        },
        "securityReports": {
            "keen": {
                "reportUrl": "https://security.example/report",
                "status": "benign",
                "statusText": "No risk found"
            }
        },
        "skill": {
            "category": "development",
            "displayName": "Fixture Skill",
            "iconUrl": "https://cdn.example/icon.png",
            "labels": {"requires_api_key": "false"},
            "slug": "fixture-skill",
            "sourceUrl": "https://github.com/example/fixture-skill",
            "stats": {
                "comments": 0,
                "downloads": 42,
                "installs": 7,
                "stars": 5,
                "versions": 2
            },
            "subCategories": [{"key": "rust", "name": "Rust"}],
            "summary": "Builds reliable tools",
            "updatedAt": 1785369600000_u64
        },
        "slug": "fixture-skill"
    })
}

fn skill_set() -> serde_json::Value {
    json!({
        "slug": "fixture-suite",
        "displayName": "Fixture Suite",
        "summary": "A complete fixture workflow",
        "iconUrl": null,
        "content": "# Fixture Suite",
        "skills": [{"slug": "fixture-skill", "namespace": "fixture"}],
        "skillCount": 1,
        "updatedAt": 1785369600000_u64
    })
}

// Public API contract rechecked 2026-10-25: skill-set members may omit version,
// and unresolved catalog entries may have a null namespace. API ZIP contains
// manifest.json and identify.md; these fixtures retain only relevant fields.
#[tokio::test(flavor = "multi_thread")]
async fn expert_package_rejects_unresolved_members_before_download() {
    for member in [
        json!({"slug": "unresolved-null", "namespace": null}),
        json!({"slug": "unresolved-empty", "namespace": ""}),
        json!({"slug": "", "namespace": "fixture"}),
    ] {
        let mut suite = skill_set();
        suite["skills"] =
            json!([{"slug": "valid-first", "namespace": "fixture", "version": "1.0.0"}, member]);
        let (base_url, requests) = fixture_sequence(vec![FixtureResponse::json(suite)]);
        let error = provider(&base_url)
            .skill_set_artifact("skillhub", "fixture-suite")
            .await
            .expect_err("all members must pass before any download");
        let message = match error {
            SkillMarketProviderError::InvalidResponse(message) => message,
            other => panic!("expected member error, got {other}"),
        };
        assert!(message.contains("member"), "{message}");
        assert!(
            message.contains(member["slug"].as_str().unwrap_or("")) || message.contains("#2"),
            "{message}"
        );
        assert_eq!(requests.join().expect("join fixture").len(), 1);
    }

    let mut suite = skill_set();
    suite["slug"] = json!("another-suite");
    let (base_url, requests) = fixture_sequence(vec![FixtureResponse::json(suite)]);
    let error = provider(&base_url)
        .skill_set_artifact("skillhub", "fixture-suite")
        .await
        .expect_err("detail slug must match");
    assert!(matches!(error, SkillMarketProviderError::InvalidResponse(message)
        if message.contains("slug")));
    assert_eq!(requests.join().expect("join fixture").len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn expert_package_resolves_and_pins_member_versions() {
    let mut suite = skill_set();
    suite["skills"] = json!([
        {"slug": "fixture-skill", "namespace": "fixture", "version": "1.1.0"},
        {"slug": "resolved-skill", "namespace": "fixture"}
    ]);
    let mut resolved = skill_detail();
    resolved["skill"]["slug"] = json!("resolved-skill");
    resolved["latestVersion"]["version"] = json!("2.0.0");
    let (base_url, requests) = fixture_sequence(vec![
        FixtureResponse::json(suite),
        FixtureResponse::json(resolved),
        FixtureResponse::raw("200 OK", "application/zip", "suite-zip"),
        FixtureResponse::raw("200 OK", "application/zip", "first-zip"),
        FixtureResponse::json(json!({"content_hash":"hash","signed":false})),
        FixtureResponse::raw("200 OK", "application/zip", "second-zip"),
        FixtureResponse::json(json!({"content_hash":"hash","signed":false})),
    ]);
    let artifact = provider(&base_url)
        .skill_set_artifact("skillhub", "fixture-suite")
        .await
        .expect("complete package");
    assert_eq!(
        artifact
            .members
            .iter()
            .map(|member| member.coordinate.version.as_str())
            .collect::<Vec<_>>(),
        ["1.1.0", "2.0.0"]
    );
    let requests = requests.join().expect("join fixture");
    assert_eq!(requests.len(), 7);
    assert!(requests[1].starts_with("GET /api/v1/skills/resolved-skill?namespace=fixture "));
    assert!(requests[2].starts_with("GET /api/v1/skillsets/fixture-suite/download "));
    assert!(
        requests[3].starts_with(
            "GET /api/v1/download?slug=fixture-skill&version=1.1.0&namespace=fixture "
        )
    );
    assert!(
        requests[5].starts_with(
            "GET /api/v1/download?slug=resolved-skill&version=2.0.0&namespace=fixture "
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn expert_package_rejects_mismatched_resolution_and_download_redirect() {
    let mut detail = skill_detail();
    detail["skill"]["slug"] = json!("another-skill");
    let (base_url, requests) =
        fixture_sequence(vec![FixtureResponse::json(skill_set()), FixtureResponse::json(detail)]);
    let error = provider(&base_url)
        .skill_set_artifact("skillhub", "fixture-suite")
        .await
        .expect_err("resolved slug must match member");
    assert!(matches!(error, SkillMarketProviderError::InvalidResponse(message)
        if message.contains("fixture-skill") && message.contains("slug")));
    assert_eq!(requests.join().expect("join fixture").len(), 2);

    let mut suite = skill_set();
    suite["skills"][0]["version"] = json!("1.2.3");
    let (base_url, requests) = fixture_sequence(vec![
        FixtureResponse::json(suite),
        FixtureResponse::redirect("http://untrusted.example/skills/fixture-skill/1.2.3.zip"),
    ]);
    let error = provider(&base_url)
        .skill_set_artifact("skillhub", "fixture-suite")
        .await
        .expect_err("untrusted download redirect must fail");
    assert!(matches!(error, SkillMarketProviderError::InvalidResponse(message)
        if message.contains("redirect target is not trusted")));
    assert_eq!(requests.join().expect("join fixture").len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn skillhub_provider_displays_unresolved_skillset_members_but_rejects_installation() {
    let mut suite = skill_set();
    suite["skills"] = json!([
        {"slug": "fixture-skill", "namespace": "fixture"},
        {"slug": "unresolved-skill", "namespace": null},
        {"slug": "missing-namespace"}
    ]);
    suite["skillCount"] = json!(3);
    let (base_url, requests) = fixture_sequence(vec![
        FixtureResponse::json(json!({"skillSets": [suite.clone()], "total": 1})),
        FixtureResponse::json(suite.clone()),
    ]);
    let provider = provider(&base_url);
    let page = provider.skill_sets(&SkillMarketSkillQuery::default()).await.expect("catalog");
    assert_eq!(page.total, 1);
    assert_eq!(page.skillsets[0].member_count, 3);
    let detail = provider.skill_set_detail("skillhub", "fixture-suite").await.expect("detail");
    assert_eq!(detail.members.len(), 3);
    assert_eq!(detail.members[0].locator.namespace, "fixture");
    assert!(detail.members[1].locator.namespace.is_empty());
    assert!(detail.members[2].locator.namespace.is_empty());
    requests.join().expect("join catalog fixture");

    suite["skills"] = json!([{"slug": "unresolved-skill", "namespace": null}]);
    let (base_url, requests) = fixture_sequence(vec![FixtureResponse::json(suite)]);
    let error = self::provider(&base_url)
        .skill_set_artifact("skillhub", "fixture-suite")
        .await
        .err()
        .expect("unresolved member must not install");
    assert!(matches!(error, SkillMarketProviderError::InvalidResponse(message)
        if message.contains("unresolved-skill") && message.contains("namespace")));
    assert_eq!(requests.join().expect("join artifact fixture").len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn skillhub_provider_maps_all_catalog_responses() {
    let (base_url, requests) = fixture_sequence(vec![
        FixtureResponse::json(json!({
            "section": "recommended",
            "skills": [skill_summary()],
            "total": 1
        })),
        FixtureResponse::json(json!({
            "code": 0,
            "data": {"skills": [skill_summary()], "total": 1},
            "message": "success"
        })),
        FixtureResponse::json(json!({
            "count": 1,
            "items": [{
                "key": "development",
                "name": "开发工具"
            }]
        })),
        FixtureResponse::json(json!({
            "code": 0,
            "data": {"skills": [], "total": 12},
            "message": "success"
        })),
        FixtureResponse::json(skill_detail()),
        FixtureResponse::json(skill_detail()),
        FixtureResponse::raw("200 OK", "text/markdown", "# Fixture Skill"),
        FixtureResponse::json(json!({
            "skillSets": [skill_set()],
            "total": 1
        })),
        FixtureResponse::json(skill_set()),
    ]);
    let provider = provider(&base_url);
    let query = SkillMarketSkillQuery::default();
    let locator = locator("fixture", "fixture-skill");

    let recommended = provider.recommended().await.expect("recommended");
    let skills = provider.skills(&query).await.expect("skills");
    let categories = provider.categories().await.expect("categories");
    let detail = provider.skill_detail(&locator).await.expect("detail");
    let file = provider.skill_file(&locator).await.expect("file");
    let skill_sets = provider.skill_sets(&query).await.expect("skill sets");
    let skill_set_detail =
        provider.skill_set_detail("skillhub", "fixture-suite").await.expect("skill set detail");

    assert_eq!(provider.source().source, "skillhub");
    assert_eq!(recommended[0].coordinate.locator.namespace, "fixture");
    assert_eq!(recommended[0].coordinate.version, "1.2.3");
    assert_eq!(recommended[0].tags, vec!["rust"]);
    assert_eq!(skills.total, 1);
    assert_eq!(skills.page, 1);
    assert_eq!(skills.page_size, 24);
    assert_eq!(categories[0].skill_count, 12);
    assert_eq!(detail.skill.author, "Fixture Author");
    assert_eq!(detail.repository.as_deref(), Some("https://github.com/example/fixture-skill"));
    assert!(!detail.requires_api_key);
    assert_eq!(detail.security.expect("security").status, "benign");
    assert_eq!(file.coordinate.version, "1.2.3");
    assert_eq!(file.content, "# Fixture Skill");
    assert_eq!(skill_sets.total, 1);
    assert_eq!(skill_sets.skillsets[0].member_count, 1);
    assert_eq!(skill_set_detail.members[0].locator.namespace, "fixture");
    assert_eq!(skill_set_detail.members[0].version, "");
    assert_eq!(skill_set_detail.content, "# Fixture Suite");

    let requests = requests.join().expect("join SkillHub fixture");
    assert_eq!(requests.len(), 9);
    assert!(requests[3].starts_with(
        "GET /api/skills?page=1&pageSize=1&sortBy=downloads&order=desc&category=development HTTP/1.1\r\n"
    ));
    for request in requests {
        let request = request.to_ascii_lowercase();
        assert!(!request.contains("authorization:"));
        assert!(!request.contains("cookie:"));
        assert!(!request.contains("x-api-key:"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn skillhub_provider_encodes_queries_and_namespace() {
    let (base_url, requests) = fixture_sequence(vec![
        FixtureResponse::json(json!({
            "code": 0,
            "data": {"skills": [], "total": 0},
            "message": "success"
        })),
        FixtureResponse::json(skill_detail()),
        FixtureResponse::json(skill_detail()),
        FixtureResponse::raw("200 OK", "text/markdown", "# Encoded"),
        FixtureResponse::json(skill_set()),
    ]);
    let provider = provider(&base_url);
    let query = SkillMarketSkillQuery {
        keyword: Some("Rust & 中文".to_string()),
        category: Some("dev/tools".to_string()),
        namespace: Some("team name".to_string()),
        sort: SkillMarketSort::UpdatedAt,
        page: 3,
        page_size: 17,
    };
    let locator = locator("team/name + 中文", "slug/with + 中文");

    provider.skills(&query).await.expect("encoded skills query");
    provider.skill_detail(&locator).await.expect("encoded detail");
    provider.skill_file(&locator).await.expect("encoded file");
    provider
        .skill_set_detail("skillhub", "suite/with + 中文")
        .await
        .expect("encoded skill set detail");

    let requests = requests.join().expect("join SkillHub fixture");
    assert!(requests[0].starts_with(
        "GET /api/skills?page=3&pageSize=17&sortBy=updated_at&order=desc&keyword=Rust+%26+%E4%B8%AD%E6%96%87&category=dev%2Ftools&namespace=team+name HTTP/1.1\r\n"
    ));
    assert!(requests[1].starts_with(
        "GET /api/v1/skills/slug%2Fwith%20+%20%E4%B8%AD%E6%96%87?namespace=team%2Fname+%2B+%E4%B8%AD%E6%96%87 HTTP/1.1\r\n"
    ));
    assert!(requests[2].starts_with(
        "GET /api/v1/skills/slug%2Fwith%20+%20%E4%B8%AD%E6%96%87?namespace=team%2Fname+%2B+%E4%B8%AD%E6%96%87 HTTP/1.1\r\n"
    ));
    assert!(requests[3].starts_with(
        "GET /api/v1/skills/slug%2Fwith%20+%20%E4%B8%AD%E6%96%87/file?path=SKILL.md&version=1.2.3&namespace=team%2Fname+%2B+%E4%B8%AD%E6%96%87 HTTP/1.1\r\n"
    ));
    assert!(
        requests[4].starts_with(
            "GET /api/v1/skillsets/suite%2Fwith%20+%20%E4%B8%AD%E6%96%87 HTTP/1.1\r\n"
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn skillhub_provider_routes_canonical_coordinate_to_artifact_contract() {
    let (base_url, requests) = fixture_sequence(vec![
        FixtureResponse::raw("200 OK", "application/zip", "fixture-zip"),
        FixtureResponse::json(json!({
            "content_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "hash_version": 1,
            "key_id": "fixture-key-v1",
            "payload": "{\"content_hash\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"skill_slug\":\"fixture-skill\",\"skill_version\":\"1.2.3\",\"v\":1}",
            "signature": "c2lnbmF0dXJl",
            "signed": true
        })),
        FixtureResponse::json(json!({
            "keys": [{
                "key_id": "fixture-key-v1",
                "algorithm": "Ed25519",
                "public_key_raw_b64": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
                "status": "active"
            }]
        })),
    ]);
    let provider = provider(&base_url);
    let coordinate = SkillMarketCoordinateDto {
        locator: locator("fixture", "fixture-skill"),
        version: "1.2.3".to_string(),
    };

    let artifact = provider.skill_artifact(&coordinate).await.expect("artifact");

    assert_eq!(artifact.archive, b"fixture-zip");
    assert!(artifact.proof.signed);
    assert_eq!(artifact.proof.key_id, "fixture-key-v1");
    let requests = requests.join().expect("join SkillHub fixture");
    assert!(requests[0].starts_with(
        "GET /api/v1/download?slug=fixture-skill&version=1.2.3&namespace=fixture HTTP/1.1\r\n"
    ));
    assert!(requests[1].starts_with(
        "GET /api/v1/open/skills/fixture-skill/versions/1.2.3/signature?namespace=fixture HTTP/1.1\r\n"
    ));
    assert!(requests[2].starts_with("GET /api/v1/open/platform/keys HTTP/1.1\r\n"));
}

#[tokio::test(flavor = "multi_thread")]
async fn skillhub_provider_preserves_unsigned_artifact_for_explicit_rejection() {
    let (base_url, requests) = fixture_sequence(vec![
        FixtureResponse::raw("200 OK", "application/zip", "unsigned-zip"),
        FixtureResponse::json(json!({
            "content_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "hash_version": 1,
            "signed": false
        })),
    ]);
    let provider = provider(&base_url);
    let coordinate = SkillMarketCoordinateDto {
        locator: locator("fixture", "unsigned-skill"),
        version: "1.2.3".to_string(),
    };

    let artifact = provider.skill_artifact(&coordinate).await.expect("artifact");

    assert_eq!(artifact.archive, b"unsigned-zip");
    assert!(!artifact.proof.signed);
    assert!(artifact.proof.key_id.is_empty());
    assert!(artifact.proof.payload.is_empty());
    assert!(artifact.proof.signature_b64.is_empty());
    assert!(artifact.proof.public_key_b64.is_empty());
    assert_eq!(requests.join().expect("join SkillHub fixture").len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn skillhub_provider_resolves_skillset_and_member_artifacts() {
    let (base_url, requests) = fixture_sequence(vec![
        FixtureResponse::json(skill_set()),
        FixtureResponse::json(skill_detail()),
        FixtureResponse::raw("200 OK", "application/zip", "fixture-suite-zip"),
        FixtureResponse::raw("200 OK", "application/zip", "fixture-member-zip"),
        FixtureResponse::json(json!({
            "content_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "hash_version": 1,
            "key_id": "fixture-key-v1",
            "payload": "{\"content_hash\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"skill_slug\":\"fixture-skill\",\"skill_version\":\"1.2.3\",\"v\":1}",
            "signature": "c2lnbmF0dXJl",
            "signed": true
        })),
        FixtureResponse::json(json!({
            "keys": [{
                "key_id": "fixture-key-v1",
                "algorithm": "Ed25519",
                "public_key_raw_b64": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
                "status": "active"
            }]
        })),
    ]);
    let provider = provider(&base_url);

    let artifact =
        provider.skill_set_artifact("skillhub", "fixture-suite").await.expect("skill-set artifact");

    assert_eq!(artifact.slug, "fixture-suite");
    assert_eq!(artifact.archive, b"fixture-suite-zip");
    assert_eq!(artifact.members.len(), 1);
    assert_eq!(artifact.members[0].coordinate.version, "1.2.3");
    assert_eq!(artifact.members[0].coordinate.locator.namespace, "fixture");
    assert_eq!(artifact.members[0].artifact.archive, b"fixture-member-zip");
    let requests = requests.join().expect("join SkillHub fixture");
    assert!(requests[0].starts_with("GET /api/v1/skillsets/fixture-suite HTTP/1.1\r\n"));
    assert!(
        requests[1].starts_with("GET /api/v1/skills/fixture-skill?namespace=fixture HTTP/1.1\r\n")
    );
    assert!(requests[2].starts_with("GET /api/v1/skillsets/fixture-suite/download HTTP/1.1\r\n"));
    assert!(requests[3].starts_with(
        "GET /api/v1/download?slug=fixture-skill&version=1.2.3&namespace=fixture HTTP/1.1\r\n"
    ));
}

#[tokio::test(flavor = "multi_thread")]
// 错误正文可能含上游内部信息，对外错误只保留状态与稳定上下文。
async fn skillhub_provider_rejects_upstream_errors_without_payload_leak() {
    let (base_url, requests) = fixture_sequence(vec![
        FixtureResponse::raw(
            "503 Service Unavailable",
            "application/json",
            r#"{"error":"private-upstream-payload"}"#,
        ),
        FixtureResponse::json(json!({
            "code": 40123,
            "data": {"skills": [], "total": 0},
            "message": "private-business-payload"
        })),
    ]);
    let provider = provider(&base_url);

    let status_error = provider.categories().await.expect_err("HTTP error should fail");
    let business_error = provider
        .skills(&SkillMarketSkillQuery {
            keyword: Some("private-query-payload".to_string()),
            ..SkillMarketSkillQuery::default()
        })
        .await
        .expect_err("business error should fail");

    assert!(matches!(status_error, SkillMarketProviderError::Upstream(_)));
    assert!(matches!(business_error, SkillMarketProviderError::Upstream(_)));
    let errors = format!("{status_error} {business_error}");
    assert!(!errors.contains("private-upstream-payload"));
    assert!(!errors.contains("private-business-payload"));
    assert!(!errors.contains("private-query-payload"));
    assert_eq!(requests.join().expect("join SkillHub fixture").len(), 2);
}
