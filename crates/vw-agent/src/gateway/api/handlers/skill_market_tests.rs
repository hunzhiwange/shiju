use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
use vw_gateway_client::{
    SkillMarketCategoryDto, SkillMarketCoordinateDto, SkillMarketInstallScopeDto,
    SkillMarketInstallSelectionDto, SkillMarketInstallTargetDto,
    SkillMarketSkillDetailDto, SkillMarketSkillDto, SkillMarketSkillFileDto,
    SkillMarketSkillLocatorDto, SkillMarketSkillPageDto, SkillMarketSkillQuery,
    SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto,
    SkillMarketSkillSetPageDto, SkillMarketSourceDto,
};

use crate::app::agent::skills::marketplace::provider::{
    SkillMarketArtifact, SkillMarketArtifactProof, SkillMarketProvider,
    SkillMarketProviderError, SkillMarketProviderRegistry,
    SkillMarketSkillSetArtifact, SkillMarketSkillSetVerificationPolicy,
};

struct RecordingProvider {
    source: SkillMarketSourceDto,
    calls: Arc<Mutex<Vec<String>>>,
    error: Option<SkillMarketProviderError>,
}

impl RecordingProvider {
    fn new(source: &str, calls: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            source: SkillMarketSourceDto {
                source: source.to_string(),
                display_name: "Fixture Market".to_string(),
            },
            calls,
            error: None,
        }
    }

    fn failing(source: &str, error: SkillMarketProviderError) -> Self {
        Self {
            source: SkillMarketSourceDto {
                source: source.to_string(),
                display_name: "Fixture Market".to_string(),
            },
            calls: Arc::new(Mutex::new(Vec::new())),
            error: Some(error),
        }
    }

    fn record(&self, call: String) -> Result<(), SkillMarketProviderError> {
        self.calls.lock().expect("calls lock").push(call);
        match &self.error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl SkillMarketProvider for RecordingProvider {
    fn source(&self) -> &SkillMarketSourceDto {
        &self.source
    }

    async fn recommended(&self) -> Result<Vec<SkillMarketSkillDto>, SkillMarketProviderError> {
        self.record("recommended".to_string())?;
        Ok(vec![skill(locator("fixture"))])
    }

    async fn skills(
        &self,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillPageDto, SkillMarketProviderError> {
        self.record(format!("skills:{query:?}"))?;
        Ok(SkillMarketSkillPageDto {
            skills: vec![skill(locator("fixture"))],
            total: 1,
            page: query.page,
            page_size: query.page_size,
        })
    }

    async fn categories(
        &self,
    ) -> Result<Vec<SkillMarketCategoryDto>, SkillMarketProviderError> {
        self.record("categories".to_string())?;
        Ok(vec![SkillMarketCategoryDto {
            slug: "development".to_string(),
            name: "Development".to_string(),
            skill_count: 1,
        }])
    }

    async fn skill_detail(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillDetailDto, SkillMarketProviderError> {
        self.record(format!(
            "skill_detail:{}:{}:{}",
            locator.source, locator.namespace, locator.slug
        ))?;
        Ok(SkillMarketSkillDetailDto {
            skill: skill(locator.clone()),
            license: None,
            homepage: None,
            repository: None,
            requires_api_key: false,
            security: None,
        })
    }

    async fn skill_file(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillFileDto, SkillMarketProviderError> {
        self.record(format!(
            "skill_file:{}:{}:{}",
            locator.source, locator.namespace, locator.slug
        ))?;
        Ok(SkillMarketSkillFileDto {
            coordinate: coordinate(locator.clone()),
            content: "# Fixture".to_string(),
        })
    }

    async fn skill_artifact(
        &self,
        coordinate: &SkillMarketCoordinateDto,
    ) -> Result<SkillMarketArtifact, SkillMarketProviderError> {
        self.record(format!(
            "skill_artifact:{}:{}:{}:{}",
            coordinate.locator.source,
            coordinate.locator.namespace,
            coordinate.locator.slug,
            coordinate.version
        ))?;
        Ok(SkillMarketArtifact {
            archive: Vec::new(),
            proof: SkillMarketArtifactProof {
                signed: false,
                payload: String::new(),
                signature_b64: String::new(),
                content_hash: String::new(),
                key_id: String::new(),
                public_key_b64: String::new(),
            },
        })
    }

    async fn skill_sets(
        &self,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillSetPageDto, SkillMarketProviderError> {
        self.record(format!("skill_sets:{query:?}"))?;
        Ok(SkillMarketSkillSetPageDto {
            skillsets: vec![skill_set("fixture")],
            total: 1,
            page: query.page,
            page_size: query.page_size,
        })
    }

    async fn skill_set_detail(
        &self,
        source: &str,
        slug: &str,
    ) -> Result<SkillMarketSkillSetDetailDto, SkillMarketProviderError> {
        self.record(format!("skill_set_detail:{source}:{slug}"))?;
        Ok(SkillMarketSkillSetDetailDto {
            skill_set: skill_set(source),
            members: vec![coordinate(locator(source))],
            content: "# Fixture suite".to_string(),
        })
    }

    async fn skill_set_artifact(
        &self,
        source: &str,
        slug: &str,
    ) -> Result<SkillMarketSkillSetArtifact, SkillMarketProviderError> {
        self.record(format!("skill_set_artifact:{source}:{slug}"))?;
        Ok(SkillMarketSkillSetArtifact {
            source: source.to_string(),
            slug: slug.to_string(),
            verification: SkillMarketSkillSetVerificationPolicy::TransportAndAudit,
            archive: Vec::new(),
            members: Vec::new(),
        })
    }
}

fn locator(source: &str) -> SkillMarketSkillLocatorDto {
    SkillMarketSkillLocatorDto {
        source: source.to_string(),
        namespace: "public".to_string(),
        slug: "fixture-skill".to_string(),
    }
}

fn coordinate(locator: SkillMarketSkillLocatorDto) -> SkillMarketCoordinateDto {
    SkillMarketCoordinateDto {
        locator,
        version: "1.0.0".to_string(),
    }
}

fn skill(locator: SkillMarketSkillLocatorDto) -> SkillMarketSkillDto {
    SkillMarketSkillDto {
        coordinate: coordinate(locator),
        name: "Fixture Skill".to_string(),
        description: "Fixture skill".to_string(),
        author: "Fixture".to_string(),
        category: "development".to_string(),
        tags: vec!["fixture".to_string()],
        icon: None,
        downloads: 1,
        installs: 1,
        rating: 5.0,
        updated_at: "2026-07-30T00:00:00Z".to_string(),
    }
}

fn skill_set(source: &str) -> SkillMarketSkillSetDto {
    SkillMarketSkillSetDto {
        source: source.to_string(),
        slug: "fixture-suite".to_string(),
        name: "Fixture Suite".to_string(),
        description: "Fixture suite".to_string(),
        author: "Fixture".to_string(),
        icon: None,
        member_count: 1,
        updated_at: "2026-07-30T00:00:00Z".to_string(),
    }
}

fn registry(provider: RecordingProvider) -> Arc<SkillMarketProviderRegistry> {
    let mut registry = SkillMarketProviderRegistry::new();
    registry.register(Arc::new(provider)).expect("register fixture provider");
    Arc::new(registry)
}

async fn response(path: &str, registry: Arc<SkillMarketProviderRegistry>) -> (StatusCode, Value) {
    let response = super::router_with_registry::<()>(registry)
        .oneshot(Request::get(path).body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("response body");
    let body = serde_json::from_slice(&body).expect("JSON response");
    (status, body)
}

async fn post_response(
    path: &str,
    body: Value,
    registry: Arc<SkillMarketProviderRegistry>,
) -> (StatusCode, Value) {
    let response = super::router_with_registry::<()>(registry)
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("response body");
    let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, body)
}

#[tokio::test]
async fn skill_market_routes_delegate_to_selected_provider() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let registry = registry(RecordingProvider::new("fixture", Arc::clone(&calls)));
    let paths = [
        "/skill-market/sources",
        "/skill-market/showcase/recommended?source=fixture",
        "/skill-market/skills?source=fixture&keyword=rust&category=development&namespace=public&sort=updated_at&page=2&page_size=12",
        "/skill-market/categories?source=fixture",
        "/skill-market/skills/fixture-skill?source=fixture&namespace=public",
        "/skill-market/skills/fixture-skill/file?source=fixture&namespace=public",
        "/skill-market/skillsets?source=fixture&keyword=suite&page=3&page_size=8",
        "/skill-market/skillsets/fixture-suite?source=fixture",
    ];

    for path in paths {
        let (status, body) = response(path, Arc::clone(&registry)).await;
        assert_eq!(status, StatusCode::OK, "{path}: {body}");
        assert_eq!(body["code"], 0, "{path}: {body}");
        assert_eq!(body["message"], "ok", "{path}: {body}");
        assert!(!body["data"].is_null(), "{path}: {body}");
    }

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        [
            "recommended",
            "skills:SkillMarketSkillQuery { keyword: Some(\"rust\"), category: Some(\"development\"), namespace: Some(\"public\"), sort: UpdatedAt, page: 2, page_size: 12 }",
            "categories",
            "skill_detail:fixture:public:fixture-skill",
            "skill_file:fixture:public:fixture-skill",
            "skill_sets:SkillMarketSkillQuery { keyword: Some(\"suite\"), category: None, namespace: None, sort: Score, page: 3, page_size: 8 }",
            "skill_set_detail:fixture:fixture-suite",
        ]
    );
}

#[tokio::test]
async fn skill_market_routes_reject_unknown_source() {
    let registry = Arc::new(SkillMarketProviderRegistry::new());

    let (status, body) =
        response("/skill-market/skills?source=missing", Arc::clone(&registry)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        json!({
            "code": 400,
            "data": null,
            "message": "unsupported skill market source: missing"
        })
    );
}

#[tokio::test]
async fn skill_market_install_delegates_only_canonical_coordinate() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let registry = registry(RecordingProvider::new("fixture", Arc::clone(&calls)));
    let project = tempfile::tempdir().expect("fixture project");
    let request = json!({
        "project_path": project.path(),
        "source": "fixture",
        "namespace": "public",
        "slug": "fixture-skill",
        "version": "1.0.0"
    });

    let (status, body) =
        post_response("/skill-market/skills/install", request, registry).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["skill_artifact:fixture:public:fixture-skill:1.0.0"]
    );
}

#[tokio::test]
async fn skill_market_install_rejects_unknown_source_and_download_url() {
    let registry = Arc::new(SkillMarketProviderRegistry::new());
    let project = tempfile::tempdir().expect("fixture project");

    let (unknown_status, unknown_body) = post_response(
        "/skill-market/skills/install",
        json!({
            "project_path": project.path(),
            "source": "missing",
            "namespace": "public",
            "slug": "fixture-skill",
            "version": "1.0.0"
        }),
        Arc::clone(&registry),
    )
    .await;
    assert_eq!(unknown_status, StatusCode::BAD_REQUEST, "{unknown_body}");

    let (url_status, _) = post_response(
        "/skill-market/skills/install",
        json!({
            "project_path": project.path(),
            "source": "missing",
            "namespace": "public",
            "slug": "fixture-skill",
            "version": "1.0.0",
            "download_url": "https://attacker.invalid/payload.zip"
        }),
        registry,
    )
    .await;
    assert!(url_status.is_client_error());
}

#[test]
fn explicit_install_targets_resolve_codex_and_claude_roots() {
    let temp = tempfile::tempdir().expect("fixture roots");
    let home = temp.path().join("home");
    let project = temp.path().join("project");
    std::fs::create_dir(&home).expect("create home");
    std::fs::create_dir(&project).expect("create project");
    let vibe_project_root = project.join(".vibewindow/skills");
    let installations = [
        SkillMarketInstallSelectionDto {
            target: SkillMarketInstallTargetDto::Codex,
            scope: SkillMarketInstallScopeDto::Global,
        },
        SkillMarketInstallSelectionDto {
            target: SkillMarketInstallTargetDto::ClaudeCode,
            scope: SkillMarketInstallScopeDto::Project,
        },
    ];

    let roots = super::resolve_explicit_install_roots(
        &home,
        Some(&project),
        Some(&vibe_project_root),
        &installations,
    )
    .expect("resolve explicit roots");

    assert_eq!(
        roots,
        [
            home.join(".codex/skills"),
            project.join(".claude/skills")
        ]
    );
}

#[tokio::test]
async fn explicit_global_install_does_not_require_project_path() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let registry = registry(RecordingProvider::new("fixture", Arc::clone(&calls)));

    let (status, body) = post_response(
        "/skill-market/skills/install-targets",
        json!({
            "project_path": "",
            "source": "fixture",
            "namespace": "public",
            "slug": "fixture-skill",
            "version": "1.0.0",
            "installations": [{"target": "codex", "scope": "global"}]
        }),
        registry,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["skill_artifact:fixture:public:fixture-skill:1.0.0"]
    );
}

#[tokio::test]
async fn skill_market_skillset_install_delegates_only_source_and_slug() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let registry = registry(RecordingProvider::new("fixture", Arc::clone(&calls)));
    let project = tempfile::tempdir().expect("fixture project");

    let (status, body) = post_response(
        "/skill-market/skillsets/install",
        json!({
            "project_path": project.path(),
            "source": "fixture",
            "slug": "fixture-suite"
        }),
        registry,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["skill_set_artifact:fixture:fixture-suite"]
    );
}

#[tokio::test]
async fn skill_market_routes_reject_invalid_queries_before_provider_call() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let registry = registry(RecordingProvider::new("fixture", Arc::clone(&calls)));
    let invalid_paths = [
        "/skill-market/skills?source=fixture&page=0",
        "/skill-market/skills?source=fixture&page_size=0",
        "/skill-market/skills?source=fixture&sort=unknown",
        "/skill-market/skills/%20?source=fixture&namespace=public",
        "/skill-market/skills/fixture?source=fixture&namespace=%20",
        "/skill-market/categories?source=%20",
    ];

    for path in invalid_paths {
        let (status, body) = response(path, Arc::clone(&registry)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {body}");
        assert_eq!(body["code"], 400, "{path}: {body}");
        assert!(body["message"].as_str().is_some_and(|message| !message.is_empty()));
    }

    assert!(calls.lock().expect("calls lock").is_empty());
}

#[tokio::test]
async fn skill_market_routes_map_provider_failures_without_leaking_details() {
    let cases = [
        (
            SkillMarketProviderError::Timeout,
            StatusCode::GATEWAY_TIMEOUT,
            "skill market provider timed out",
        ),
        (
            SkillMarketProviderError::Upstream("private upstream response".to_string()),
            StatusCode::BAD_GATEWAY,
            "skill market upstream request failed",
        ),
        (
            SkillMarketProviderError::InvalidResponse("private invalid payload".to_string()),
            StatusCode::BAD_GATEWAY,
            "skill market upstream response is invalid",
        ),
    ];

    for (error, expected_status, expected_message) in cases {
        let registry = registry(RecordingProvider::failing("fixture", error));
        let (status, body) =
            response("/skill-market/categories?source=fixture", registry).await;

        assert_eq!(status, expected_status);
        assert_eq!(body["code"], u16::from(expected_status));
        assert_eq!(body["message"], expected_message);
        assert!(!body.to_string().contains("private"));
    }
}
