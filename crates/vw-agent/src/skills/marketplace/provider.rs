//! 技能市场提供方的抽象边界与注册表。

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

/// 屏蔽原生与 WASM 平台线程安全差异的市场提供方边界。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) trait SkillMarketProviderBounds: Send + Sync {}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> SkillMarketProviderBounds for T {}

#[cfg(target_arch = "wasm32")]
pub(crate) trait SkillMarketProviderBounds {}

#[cfg(target_arch = "wasm32")]
impl<T> SkillMarketProviderBounds for T {}

/// 具体市场来源适配器返回的稳定错误分类。
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
/// 单个技能的归档内容与坐标信息。
pub(crate) struct SkillMarketArtifact {
    pub(crate) archive: Vec<u8>,
    pub(crate) proof: SkillMarketArtifactProof,
}

/// Raw proof fields verified by the shared installer.
#[derive(Debug, Clone, PartialEq, Eq)]
/// 用于验证市场制品来源与完整性的证明。
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
/// 技能集成员制品采用的验证策略。
pub(crate) enum SkillMarketSkillSetVerificationPolicy {
    TransportAndAudit,
    Unsupported(String),
}

/// One canonical member and its independently verified artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
/// 技能集中的一个具名成员制品。
pub(crate) struct SkillMarketSkillSetMemberArtifact {
    pub(crate) coordinate: SkillMarketCoordinateDto,
    pub(crate) artifact: SkillMarketArtifact,
}

/// Canonical skill-set archive plus the members declared by its detail endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
/// 包含清单、证明与全部成员归档的技能集制品。
pub(crate) struct SkillMarketSkillSetArtifact {
    pub(crate) source: String,
    pub(crate) slug: String,
    pub(crate) verification: SkillMarketSkillSetVerificationPolicy,
    pub(crate) archive: Vec<u8>,
    pub(crate) members: Vec<SkillMarketSkillSetMemberArtifact>,
}

/// Capabilities required by the current skill-market pages and single-skill install flow.
/// 技能市场来源必须实现的查询、详情与制品下载契约。
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
/// 市场注册表解析来源时返回的错误。
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
/// 按稳定来源键注册和解析市场提供方的注册表。
pub(crate) struct SkillMarketProviderRegistry {
    providers: BTreeMap<String, Arc<dyn SkillMarketProvider>>,
}

impl SkillMarketProviderRegistry {
    /// 创建空注册表。
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 创建包含内置公共来源的注册表。
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

    /// 注册来源；来源键必须稳定且不得重复。
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

    /// 按来源键解析提供方。
    pub(crate) fn resolve(
        &self,
        source: &str,
    ) -> Result<Arc<dyn SkillMarketProvider>, SkillMarketRegistryError> {
        self.providers
            .get(source)
            .cloned()
            .ok_or_else(|| SkillMarketRegistryError::UnknownSource(source.to_string()))
    }

    /// 按键排序返回全部已注册来源元数据。
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
