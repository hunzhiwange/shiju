use crate::{
    SkillMarketCategoryDto,
    SkillMarketCoordinateDto,
    SkillMarketSkillDetailDto,
    SkillMarketSkillDto,
    SkillMarketSkillFileDto,
    SkillMarketSkillLocatorDto,
    SkillMarketSkillPageDto,
    SkillMarketSkillQuery,
    SkillMarketSkillSetDetailDto,
    SkillMarketSkillSetDto,
    SkillMarketSkillSetPageDto,
    SkillMarketSourceDto,
};
// 覆盖技能市场提供商路由、查询校验及安装目标解析。

use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use agent_skills::marketplace::provider::{
    SkillMarketArtifact, SkillMarketArtifactProof, SkillMarketProvider, SkillMarketProviderError,
    SkillMarketProviderRegistry, SkillMarketSkillSetArtifact, SkillMarketSkillSetMemberArtifact,
    SkillMarketSkillSetVerificationPolicy,
};


struct RecordingProvider {
    source: SkillMarketSourceDto,
    calls: Arc<Mutex<Vec<String>>>,
    error: Option<SkillMarketProviderError>,
    artifact: Option<SkillMarketSkillSetArtifact>,
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
            artifact: None,
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

    async fn categories(&self) -> Result<Vec<SkillMarketCategoryDto>, SkillMarketProviderError> {
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
        if let Some(artifact) = &self.artifact {
            return Ok(artifact.clone());
        }
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
    SkillMarketCoordinateDto { locator, version: "1.0.0".to_string() }
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

async fn post_response(
    path: &str,
    body: Value,
    registry: Arc<SkillMarketProviderRegistry>,
    config: Arc<dyn crate::routes::SkillMarketConfig>,
) -> (StatusCode, Value) {
    let response = crate::routes::router_with_registry::<()>(registry, config)
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

fn expert_fixture_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in files {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .unix_permissions(0o100644),
            )
            .expect("fixture ZIP entry");
        writer.write_all(content).expect("fixture ZIP content");
    }
    writer.finish().expect("fixture ZIP").into_inner()
}

fn valid_expert_fixture() -> SkillMarketSkillSetArtifact {
    let identify = "---\nname: fixture-suite\ndescription: Fixture suite\nversion: 3.0.0\nmetadata:\n  package_type: meta-skill\norchestration:\n  children: [first-skill]\n---\n# Workflow\n";
    let coordinate = SkillMarketCoordinateDto {
        locator: SkillMarketSkillLocatorDto {
            source: "fixture".to_string(),
            namespace: "public".to_string(),
            slug: "first-skill".to_string(),
        },
        version: "1.0.0".to_string(),
    };
    let member_skill = "---\nname: first-skill\ndescription: First\n---\n# First\n";
    let file_hash = hex::encode(Sha256::digest(member_skill.as_bytes()));
    let content_hash = hex::encode(Sha256::digest(format!("SKILL.md:{file_hash}\n")));
    let payload = format!(
        r#"{{"content_hash":"{content_hash}","skill_slug":"first-skill","skill_version":"1.0.0","v":1}}"#
    );
    let key_pair = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).expect("fixture signing key");
    let manifest = json!({
        "manifestVersion": "1",
        "type": "skillset",
        "slug": "fixture-suite",
        "skillSlugs": ["first-skill"],
        "skills": [{"slug": "first-skill"}],
        "files": [{
            "path": "identify.md",
            "sha256": hex::encode(Sha256::digest(identify.as_bytes())),
            "size": identify.len()
        }]
    });
    let manifest_bytes = serde_json::to_vec(&manifest).expect("suite manifest");
    SkillMarketSkillSetArtifact {
        source: "fixture".to_string(),
        slug: "fixture-suite".to_string(),
        verification: SkillMarketSkillSetVerificationPolicy::TransportAndAudit,
        archive: expert_fixture_zip(&[
            ("manifest.json", &manifest_bytes),
            ("identify.md", identify.as_bytes()),
        ]),
        members: vec![SkillMarketSkillSetMemberArtifact {
            coordinate,
            artifact: SkillMarketArtifact {
                archive: expert_fixture_zip(&[("SKILL.md", member_skill.as_bytes())]),
                proof: SkillMarketArtifactProof {
                    signed: true,
                    payload: payload.clone(),
                    signature_b64: STANDARD.encode(key_pair.sign(payload.as_bytes()).as_ref()),
                    content_hash,
                    key_id: "fixture-key-v1".to_string(),
                    public_key_b64: STANDARD.encode(key_pair.public_key().as_ref()),
                },
            },
        }],
    }
}


pub struct FixtureConfig;
impl crate::routes::SkillMarketConfig for FixtureConfig {
 fn skills(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = config_skills::SkillsConfig> + Send + '_>> {
  Box::pin(async { config_skills::SkillsConfig::default() })
 }
}
/// Signed fixture installation shared by route tests and the host tool contract.
pub async fn install_expert_fixture(project: &std::path::Path, config: Arc<dyn crate::routes::SkillMarketConfig>) -> (StatusCode, Value, Vec<String>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut provider = RecordingProvider::new("fixture", Arc::clone(&calls));
    provider.artifact = Some(valid_expert_fixture());
    let (status, body) = post_response("/skill-market/skillsets/install",
        json!({"project_path":project,"source":"fixture","slug":"fixture-suite"}), registry(provider), config).await;
    let calls = calls.lock().expect("calls lock").clone();
    (status, body, calls)
}
