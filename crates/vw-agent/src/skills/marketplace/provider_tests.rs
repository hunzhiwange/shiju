use std::sync::Arc;

use async_trait::async_trait;
use vw_gateway_client::{
    SkillMarketCategoryDto, SkillMarketCoordinateDto, SkillMarketSkillDetailDto,
    SkillMarketSkillFileDto, SkillMarketSkillLocatorDto, SkillMarketSkillPageDto,
    SkillMarketSkillQuery, SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto,
    SkillMarketSkillSetPageDto, SkillMarketSourceDto,
};

use super::{
    SkillMarketArtifact, SkillMarketArtifactProof, SkillMarketProvider, SkillMarketProviderError,
    SkillMarketProviderRegistry, SkillMarketRegistryError,
};

struct MockProvider {
    source: SkillMarketSourceDto,
}

impl MockProvider {
    fn new(source: &str) -> Self {
        Self {
            source: SkillMarketSourceDto {
                source: source.to_string(),
                display_name: "SkillHub".to_string(),
            },
        }
    }
}

#[async_trait]
impl SkillMarketProvider for MockProvider {
    fn source(&self) -> &SkillMarketSourceDto {
        &self.source
    }

    async fn recommended(
        &self,
    ) -> Result<Vec<vw_gateway_client::SkillMarketSkillDto>, SkillMarketProviderError> {
        Ok(Vec::new())
    }

    async fn skills(
        &self,
        _query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillPageDto, SkillMarketProviderError> {
        Ok(SkillMarketSkillPageDto::default())
    }

    async fn categories(&self) -> Result<Vec<SkillMarketCategoryDto>, SkillMarketProviderError> {
        Ok(Vec::new())
    }

    async fn skill_detail(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillDetailDto, SkillMarketProviderError> {
        Ok(SkillMarketSkillDetailDto {
            skill: skill(coordinate(locator)),
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
        Ok(SkillMarketSkillFileDto {
            coordinate: coordinate(locator),
            content: "# Mock".to_string(),
        })
    }

    async fn skill_artifact(
        &self,
        _coordinate: &SkillMarketCoordinateDto,
    ) -> Result<SkillMarketArtifact, SkillMarketProviderError> {
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
        _query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillSetPageDto, SkillMarketProviderError> {
        Ok(SkillMarketSkillSetPageDto::default())
    }

    async fn skill_set_detail(
        &self,
        source: &str,
        slug: &str,
    ) -> Result<SkillMarketSkillSetDetailDto, SkillMarketProviderError> {
        Ok(SkillMarketSkillSetDetailDto {
            skill_set: SkillMarketSkillSetDto {
                source: source.to_string(),
                slug: slug.to_string(),
                name: "Mock suite".to_string(),
                description: "Mock suite".to_string(),
                author: "Mock".to_string(),
                icon: None,
                member_count: 1,
                updated_at: "2026-07-30T00:00:00Z".to_string(),
            },
            members: vec![coordinate(&locator(source))],
            content: "# Mock suite".to_string(),
        })
    }
}

#[test]
fn provider_registry_resolves_stable_source_key() {
    let provider: Arc<dyn SkillMarketProvider> = Arc::new(MockProvider::new("skillhub"));
    let mut registry = SkillMarketProviderRegistry::new();
    registry.register(Arc::clone(&provider)).expect("stable source should register");

    let resolved = registry.resolve("skillhub").expect("registered source should resolve");

    assert!(Arc::ptr_eq(&provider, &resolved));
    assert_eq!(registry.sources(), vec![provider.source().clone()]);
}

#[test]
fn provider_registry_rejects_unknown_source() {
    let registry = SkillMarketProviderRegistry::new();

    let error = match registry.resolve("missing") {
        Ok(_) => panic!("unknown source should fail"),
        Err(error) => error,
    };

    assert_eq!(error, SkillMarketRegistryError::UnknownSource("missing".to_string()));
}

#[test]
fn provider_registry_registers_skillhub_as_default_source() {
    let registry =
        SkillMarketProviderRegistry::with_default_sources().expect("default sources should build");

    let provider = registry.resolve("skillhub").expect("skillhub should be registered");

    assert_eq!(provider.source().source, "skillhub");
    assert_eq!(provider.source().display_name, "SkillHub");
}

#[tokio::test]
async fn mock_provider_covers_all_current_read_capabilities() {
    let provider = MockProvider::new("skillhub");
    let locator = locator("skillhub");
    let query = SkillMarketSkillQuery::default();

    provider.recommended().await.expect("recommended");
    provider.skills(&query).await.expect("skills");
    provider.categories().await.expect("categories");
    provider.skill_detail(&locator).await.expect("skill detail");
    provider.skill_file(&locator).await.expect("skill file");
    provider.skill_artifact(&coordinate(&locator)).await.expect("skill artifact");
    provider.skill_sets(&query).await.expect("skill sets");
    provider.skill_set_detail("skillhub", "mock-suite").await.expect("skill set detail");
}

fn locator(source: &str) -> SkillMarketSkillLocatorDto {
    SkillMarketSkillLocatorDto {
        source: source.to_string(),
        namespace: "public".to_string(),
        slug: "mock-skill".to_string(),
    }
}

fn coordinate(locator: &SkillMarketSkillLocatorDto) -> SkillMarketCoordinateDto {
    SkillMarketCoordinateDto { locator: locator.clone(), version: "1.0.0".to_string() }
}

fn skill(coordinate: SkillMarketCoordinateDto) -> vw_gateway_client::SkillMarketSkillDto {
    vw_gateway_client::SkillMarketSkillDto {
        coordinate,
        name: "Mock skill".to_string(),
        description: "Mock skill".to_string(),
        author: "Mock".to_string(),
        category: "development".to_string(),
        tags: Vec::new(),
        icon: None,
        downloads: 0,
        installs: 0,
        rating: 0.0,
        updated_at: "2026-07-30T00:00:00Z".to_string(),
    }
}
