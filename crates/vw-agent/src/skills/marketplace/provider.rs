//! Skill marketplace provider boundary and registry.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use vw_gateway_client::{
    SkillMarketCategoryDto, SkillMarketCoordinateDto, SkillMarketSkillDetailDto,
    SkillMarketSkillDto, SkillMarketSkillFileDto, SkillMarketSkillLocatorDto,
    SkillMarketSkillPageDto, SkillMarketSkillQuery, SkillMarketSkillSetDetailDto,
    SkillMarketSkillSetPageDto, SkillMarketSourceDto,
};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) trait SkillMarketProviderBounds: Send + Sync {}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> SkillMarketProviderBounds for T {}

#[cfg(target_arch = "wasm32")]
pub(crate) trait SkillMarketProviderBounds {}

#[cfg(target_arch = "wasm32")]
impl<T> SkillMarketProviderBounds for T {}

/// Failures returned by a concrete source adapter.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum SkillMarketProviderError {
    #[error("skill market provider timed out")]
    Timeout,
    #[error("skill market upstream request failed: {0}")]
    Upstream(String),
    #[error("skill market response is invalid: {0}")]
    InvalidResponse(String),
}

/// Immutable archive bytes and the source proof required by the shared installer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillMarketArtifact {
    pub(crate) archive: Vec<u8>,
    pub(crate) proof: SkillMarketArtifactProof,
}

/// Raw proof fields verified by the shared installer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillMarketArtifactProof {
    pub(crate) signed: bool,
    pub(crate) payload: String,
    pub(crate) signature_b64: String,
    pub(crate) content_hash: String,
    pub(crate) key_id: String,
    pub(crate) public_key_b64: String,
}

/// Verification policy explicitly selected for an unsigned skill-set transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillMarketSkillSetVerificationPolicy {
    TransportAndAudit,
    Unsupported(String),
}

/// One canonical member and its independently verified artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillMarketSkillSetMemberArtifact {
    pub(crate) coordinate: SkillMarketCoordinateDto,
    pub(crate) artifact: SkillMarketArtifact,
}

/// Canonical skill-set archive plus the members declared by its detail endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillMarketSkillSetArtifact {
    pub(crate) source: String,
    pub(crate) slug: String,
    pub(crate) verification: SkillMarketSkillSetVerificationPolicy,
    pub(crate) archive: Vec<u8>,
    pub(crate) members: Vec<SkillMarketSkillSetMemberArtifact>,
}

/// Capabilities required by the current skill-market pages and single-skill install flow.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub(crate) trait SkillMarketProvider: SkillMarketProviderBounds {
    fn source(&self) -> &SkillMarketSourceDto;

    async fn recommended(&self) -> Result<Vec<SkillMarketSkillDto>, SkillMarketProviderError>;

    async fn skills(
        &self,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillPageDto, SkillMarketProviderError>;

    async fn categories(&self) -> Result<Vec<SkillMarketCategoryDto>, SkillMarketProviderError>;

    async fn skill_detail(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillDetailDto, SkillMarketProviderError>;

    async fn skill_file(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillFileDto, SkillMarketProviderError>;

    async fn skill_artifact(
        &self,
        coordinate: &SkillMarketCoordinateDto,
    ) -> Result<SkillMarketArtifact, SkillMarketProviderError>;

    async fn skill_sets(
        &self,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillSetPageDto, SkillMarketProviderError>;

    async fn skill_set_detail(
        &self,
        source: &str,
        slug: &str,
    ) -> Result<SkillMarketSkillSetDetailDto, SkillMarketProviderError>;

    async fn skill_set_artifact(
        &self,
        source: &str,
        slug: &str,
    ) -> Result<SkillMarketSkillSetArtifact, SkillMarketProviderError> {
        let _ = (source, slug);
        Err(SkillMarketProviderError::InvalidResponse(
            "skill-set installation is not supported by this source".to_string(),
        ))
    }
}

/// Registration and lookup failures for source keys.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum SkillMarketRegistryError {
    #[error("invalid skill market source key: {0}")]
    InvalidSourceKey(String),
    #[error("skill market source is already registered: {0}")]
    DuplicateSource(String),
    #[error("unsupported skill market source: {0}")]
    UnknownSource(String),
}

/// Explicit registry of the concrete skill-market sources enabled by the gateway.
#[derive(Default)]
pub(crate) struct SkillMarketProviderRegistry {
    providers: BTreeMap<String, Arc<dyn SkillMarketProvider>>,
}

impl SkillMarketProviderRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with_default_sources() -> Result<Self, SkillMarketProviderError> {
        let mut registry = Self::new();
        let provider = Arc::new(super::skillhub::SkillHubProvider::new()?);
        registry.register(provider).map_err(|_| {
            SkillMarketProviderError::InvalidResponse(
                "failed to register the default SkillHub source".to_string(),
            )
        })?;
        Ok(registry)
    }

    pub(crate) fn register(
        &mut self,
        provider: Arc<dyn SkillMarketProvider>,
    ) -> Result<(), SkillMarketRegistryError> {
        let source = provider.source().source.as_str();
        if !is_stable_source_key(source) {
            return Err(SkillMarketRegistryError::InvalidSourceKey(source.to_string()));
        }
        if self.providers.contains_key(source) {
            return Err(SkillMarketRegistryError::DuplicateSource(source.to_string()));
        }
        self.providers.insert(source.to_string(), provider);
        Ok(())
    }

    pub(crate) fn resolve(
        &self,
        source: &str,
    ) -> Result<Arc<dyn SkillMarketProvider>, SkillMarketRegistryError> {
        self.providers
            .get(source)
            .cloned()
            .ok_or_else(|| SkillMarketRegistryError::UnknownSource(source.to_string()))
    }

    pub(crate) fn sources(&self) -> Vec<SkillMarketSourceDto> {
        self.providers.values().map(|provider| provider.source().clone()).collect()
    }
}

fn is_stable_source_key(source: &str) -> bool {
    !source.is_empty()
        && source
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && source.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && source.as_bytes().last().is_some_and(u8::is_ascii_alphanumeric)
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
