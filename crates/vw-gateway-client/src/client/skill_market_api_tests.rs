use super::{
    SkillMarketCoordinateDto, SkillMarketEnvelope, SkillMarketInstallScopeDto,
    SkillMarketInstallSelectionDto, SkillMarketInstallTargetDto, SkillMarketSkillLocatorDto,
    SkillMarketSkillPageDto, SkillMarketSkillQuery, SkillMarketSort,
};
use crate::client::test_support;
use serde_json::json;

#[test]
fn skill_market_contract_matches_skillhub_shaped_fixture() {
    let fixture = serde_json::json!({
        "code": 0,
        "data": {
            "skills": [{
                "source": "skillhub",
                "namespace": "public",
                "slug": "rust-reviewer",
                "version": "1.2.3",
                "name": "Rust Reviewer",
                "description": "Review Rust changes",
                "author": "VibeWindow",
                "category": "development",
                "tags": ["rust", "review"],
                "icon": null,
                "downloads": 1200,
                "installs": 300,
                "rating": 4.8,
                "updated_at": "2026-07-30T00:00:00Z"
            }],
            "total": 1,
            "page": 1,
            "page_size": 24
        },
        "message": "ok"
    });

    let envelope: SkillMarketEnvelope<SkillMarketSkillPageDto> =
        serde_json::from_value(fixture.clone()).expect("fixture should match the contract");

    assert_eq!(envelope.data.total, 1);
    assert_eq!(envelope.data.skills[0].coordinate.locator.source, "skillhub");
    assert_eq!(envelope.data.skills[0].coordinate.locator.namespace, "public");
    assert_eq!(envelope.data.skills[0].coordinate.locator.slug, "rust-reviewer");
    assert_eq!(envelope.data.skills[0].coordinate.version, "1.2.3");
    assert_eq!(serde_json::to_value(envelope).expect("contract should serialize"), fixture);
}

#[test]
fn skill_market_query_uses_stable_snake_case_sort_values() {
    let query = SkillMarketSkillQuery {
        keyword: Some("rust review".to_string()),
        category: Some("development".to_string()),
        namespace: Some("public".to_string()),
        sort: SkillMarketSort::UpdatedAt,
        page: 2,
        page_size: 24,
    };

    let value = serde_json::to_value(query).expect("query should serialize");
    assert_eq!(value["sort"], "updated_at");
    assert_eq!(value["page_size"], 24);
}

#[tokio::test]
async fn skill_market_client_uses_gateway_paths_and_query_fields() {
    let empty = |data| json!({"code": 0, "data": data, "message": "ok"});
    let server = test_support::server(vec![
        (200, empty(json!([{"source": "skillhub", "display_name": "SkillHub"}]))),
        (200, empty(json!([]))),
        (200, empty(json!({"skills": [], "total": 0, "page": 2, "page_size": 12}))),
        (200, empty(json!([]))),
        (
            200,
            empty(json!({
                "source": "skillhub",
                "namespace": "public team",
                "slug": "rust/review",
                "version": "1.0.0",
                "name": "Rust Review",
                "description": "Review Rust",
                "author": "Fixture",
                "category": "development",
                "tags": [],
                "icon": null,
                "downloads": 0,
                "installs": 0,
                "rating": 0.0,
                "updated_at": "2026-07-30T00:00:00Z",
                "license": null,
                "homepage": null,
                "repository": null,
                "requires_api_key": false,
                "security": null
            })),
        ),
        (
            200,
            empty(json!({
                "source": "skillhub",
                "namespace": "public team",
                "slug": "rust/review",
                "version": "1.0.0",
                "content": "# Rust Review"
            })),
        ),
        (200, empty(json!({"skillsets": [], "total": 0, "page": 2, "page_size": 12}))),
        (
            200,
            empty(json!({
                "source": "skillhub",
                "slug": "review/suite",
                "name": "Review Suite",
                "description": "Review suite",
                "author": "Fixture",
                "icon": null,
                "member_count": 0,
                "updated_at": "2026-07-30T00:00:00Z",
                "members": [],
                "content": "# Review Suite"
            })),
        ),
        (
            200,
            empty(json!({
                "path": "/repo/.vibewindow/skills/skillhub--public-team--rust-review"
            })),
        ),
        (
            200,
            empty(json!([
                {"path": "/repo/.vibewindow/skills/skillhub--skillset--review-suite"},
                {"path": "/repo/.vibewindow/skills/skillhub--public-team--rust-review"}
            ])),
        ),
    ]);
    let query = SkillMarketSkillQuery {
        keyword: Some("rust review".to_string()),
        category: Some("development".to_string()),
        namespace: Some("public team".to_string()),
        sort: SkillMarketSort::UpdatedAt,
        page: 2,
        page_size: 12,
    };
    let locator = SkillMarketSkillLocatorDto {
        source: "skillhub".to_string(),
        namespace: "public team".to_string(),
        slug: "rust/review".to_string(),
    };

    assert_eq!(server.client().skill_market_sources().await.unwrap().len(), 1);
    server.client().skill_market_recommended("skillhub").await.unwrap();
    server.client().skill_market_skills("skillhub", &query).await.unwrap();
    server.client().skill_market_categories("skillhub").await.unwrap();
    server.client().skill_market_skill_detail(&locator).await.unwrap();
    server.client().skill_market_skill_file(&locator).await.unwrap();
    server.client().skill_market_skill_sets("skillhub", &query).await.unwrap();
    server.client().skill_market_skill_set_detail("skillhub", "review/suite").await.unwrap();
    let installed = server
        .client()
        .skill_market_install(
            "/repo",
            &SkillMarketCoordinateDto { locator: locator.clone(), version: "1.0.0".to_string() },
        )
        .await
        .unwrap();
    let installed_set = server
        .client()
        .skill_market_skill_set_install("/repo", "skillhub", "review/suite")
        .await
        .unwrap();

    let expected_paths = [
        "/v1/skill-market/sources",
        "/v1/skill-market/showcase/recommended?source=skillhub",
        "/v1/skill-market/skills?source=skillhub&keyword=rust+review&category=development&namespace=public+team&sort=updated_at&page=2&page_size=12",
        "/v1/skill-market/categories?source=skillhub",
        "/v1/skill-market/skills/rust%2Freview?source=skillhub&namespace=public+team",
        "/v1/skill-market/skills/rust%2Freview/file?source=skillhub&namespace=public+team",
        "/v1/skill-market/skillsets?source=skillhub&keyword=rust+review&category=development&namespace=public+team&sort=updated_at&page=2&page_size=12",
        "/v1/skill-market/skillsets/review%2Fsuite?source=skillhub",
    ];
    for expected_path in expected_paths {
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, expected_path);
        assert!(!request.path.contains("api.skillhub.cn"));
    }
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/skill-market/skills/install");
    assert_eq!(
        request.body,
        json!({
            "project_path": "/repo",
            "source": "skillhub",
            "namespace": "public team",
            "slug": "rust/review",
            "version": "1.0.0"
        })
    );
    assert_eq!(installed, "/repo/.vibewindow/skills/skillhub--public-team--rust-review");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/skill-market/skillsets/install");
    assert_eq!(
        request.body,
        json!({
            "project_path": "/repo",
            "source": "skillhub",
            "slug": "review/suite"
        })
    );
    assert_eq!(
        installed_set,
        [
            "/repo/.vibewindow/skills/skillhub--skillset--review-suite",
            "/repo/.vibewindow/skills/skillhub--public-team--rust-review"
        ]
    );
    server.join();
}

#[tokio::test]
async fn skill_market_client_rejects_nonzero_success_envelope() {
    let server = test_support::server(vec![(
        200,
        json!({"code": 42, "data": [], "message": "catalog unavailable"}),
    )]);

    let error = server
        .client()
        .skill_market_sources()
        .await
        .expect_err("nonzero business code should fail");

    assert_eq!(error, "skill market request failed (code 42): catalog unavailable");
    server.join();
}

#[tokio::test]
async fn skill_market_client_installs_to_explicit_agent_targets() {
    let server = test_support::server(vec![(
        200,
        json!({
            "code": 0,
            "data": [
                {"path": "/home/user/.codex/skills/skillhub--public--review"},
                {"path": "/repo/.claude/skills/skillhub--public--review"}
            ],
            "message": "ok"
        }),
    )]);
    let coordinate = SkillMarketCoordinateDto {
        locator: SkillMarketSkillLocatorDto {
            source: "skillhub".to_string(),
            namespace: "public".to_string(),
            slug: "review".to_string(),
        },
        version: "1.0.0".to_string(),
    };
    let selections = [
        SkillMarketInstallSelectionDto {
            target: SkillMarketInstallTargetDto::Codex,
            scope: SkillMarketInstallScopeDto::Global,
        },
        SkillMarketInstallSelectionDto {
            target: SkillMarketInstallTargetDto::ClaudeCode,
            scope: SkillMarketInstallScopeDto::Project,
        },
    ];

    let paths = server
        .client()
        .skill_market_install_to_targets(Some("/repo"), &coordinate, &selections)
        .await
        .expect("multi-target install response");

    assert_eq!(paths.len(), 2);
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/skill-market/skills/install-targets");
    assert_eq!(
        request.body,
        json!({
            "project_path": "/repo",
            "source": "skillhub",
            "namespace": "public",
            "slug": "review",
            "version": "1.0.0",
            "installations": [
                {"target": "codex", "scope": "global"},
                {"target": "claude_code", "scope": "project"}
            ]
        })
    );
    server.join();
}
