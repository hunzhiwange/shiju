//! Unified skill-market transport contract.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::{DesktopSkillPathDto, GatewayClient};

/// Skill-market response envelope shared by every source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillMarketEnvelope<T> {
    pub code: i32,
    pub data: T,
    pub message: String,
}

/// A registered skill-market source exposed to clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMarketSourceDto {
    pub source: String,
    pub display_name: String,
}

/// Source-aware locator used to request the latest skill metadata or file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillMarketSkillLocatorDto {
    pub source: String,
    pub namespace: String,
    pub slug: String,
}

/// Canonical identity of one immutable skill version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillMarketCoordinateDto {
    #[serde(flatten)]
    pub locator: SkillMarketSkillLocatorDto,
    pub version: String,
}

/// Supported Agent skill directories for explicit market installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillMarketInstallTargetDto {
    VibeWindow,
    AgentSkills,
    Codex,
    WorkBuddy,
    ClaudeCode,
    OpenCode,
    GeminiCli,
    GitHubCopilot,
    Cursor,
    Windsurf,
}

/// Filesystem scope for one explicit market installation target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillMarketInstallScopeDto {
    Global,
    Project,
}

/// One explicit Agent target selected by a market user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillMarketInstallSelectionDto {
    pub target: SkillMarketInstallTargetDto,
    pub scope: SkillMarketInstallScopeDto,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct SkillMarketInstallRequest {
    project_path: String,
    source: String,
    namespace: String,
    slug: String,
    version: String,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct SkillMarketMultiInstallRequest {
    project_path: String,
    source: String,
    namespace: String,
    slug: String,
    version: String,
    installations: Vec<SkillMarketInstallSelectionDto>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct SkillMarketSkillSetInstallRequest {
    project_path: String,
    source: String,
    slug: String,
}

/// Sort orders supported by the unified skill catalog.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillMarketSort {
    #[default]
    Score,
    Downloads,
    UpdatedAt,
    Installs,
}

/// Source-independent catalog query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMarketSkillQuery {
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub namespace: Option<String>,
    pub sort: SkillMarketSort,
    pub page: u32,
    pub page_size: u32,
}

impl Default for SkillMarketSkillQuery {
    fn default() -> Self {
        Self {
            keyword: None,
            category: None,
            namespace: None,
            sort: SkillMarketSort::default(),
            page: 1,
            page_size: 24,
        }
    }
}

/// Skill summary used by recommended and catalog pages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillMarketSkillDto {
    #[serde(flatten)]
    pub coordinate: SkillMarketCoordinateDto,
    pub name: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub icon: Option<String>,
    pub downloads: u64,
    pub installs: u64,
    pub rating: f64,
    pub updated_at: String,
}

/// Paginated skill catalog.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillMarketSkillPageDto {
    pub skills: Vec<SkillMarketSkillDto>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

/// One source-independent skill category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMarketCategoryDto {
    pub slug: String,
    pub name: String,
    pub skill_count: u64,
}

/// Security assessment fields displayed on a skill detail page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMarketSecurityDto {
    pub status: String,
    pub summary: Option<String>,
}

/// Extended metadata for one skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillMarketSkillDetailDto {
    #[serde(flatten)]
    pub skill: SkillMarketSkillDto,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub requires_api_key: bool,
    pub security: Option<SkillMarketSecurityDto>,
}

/// Markdown file belonging to one canonical skill version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMarketSkillFileDto {
    #[serde(flatten)]
    pub coordinate: SkillMarketCoordinateDto,
    pub content: String,
}

/// Skill-set summary used by suite catalog pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMarketSkillSetDto {
    pub source: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub icon: Option<String>,
    pub member_count: u64,
    pub updated_at: String,
}

/// Paginated skill-set catalog.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMarketSkillSetPageDto {
    pub skillsets: Vec<SkillMarketSkillSetDto>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

/// Skill-set metadata, member coordinates, and meta-skill content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMarketSkillSetDetailDto {
    #[serde(flatten)]
    pub skill_set: SkillMarketSkillSetDto,
    pub members: Vec<SkillMarketCoordinateDto>,
    pub content: String,
}

impl GatewayClient {
    /// Lists the skill-market sources registered by the gateway.
    pub async fn skill_market_sources(&self) -> Result<Vec<SkillMarketSourceDto>, String> {
        self.skill_market_get("/v1/skill-market/sources", &[]).await
    }

    /// Lists the recommended skills from one registered source.
    pub async fn skill_market_recommended(
        &self,
        source: &str,
    ) -> Result<Vec<SkillMarketSkillDto>, String> {
        self.skill_market_get(
            "/v1/skill-market/showcase/recommended",
            &[("source".to_string(), source.to_string())],
        )
        .await
    }

    /// Queries the unified skill catalog for one registered source.
    pub async fn skill_market_skills(
        &self,
        source: &str,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillPageDto, String> {
        self.skill_market_get("/v1/skill-market/skills", &skill_market_catalog_query(source, query))
            .await
    }

    /// Lists the unified categories exposed by one registered source.
    pub async fn skill_market_categories(
        &self,
        source: &str,
    ) -> Result<Vec<SkillMarketCategoryDto>, String> {
        self.skill_market_get(
            "/v1/skill-market/categories",
            &[("source".to_string(), source.to_string())],
        )
        .await
    }

    /// Reads one source-aware skill detail.
    pub async fn skill_market_skill_detail(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillDetailDto, String> {
        self.skill_market_get(
            &format!("/v1/skill-market/skills/{}", encode_skill_market_path_segment(&locator.slug)),
            &skill_market_locator_query(locator),
        )
        .await
    }

    /// Reads the canonical Markdown file for one source-aware skill.
    pub async fn skill_market_skill_file(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillFileDto, String> {
        self.skill_market_get(
            &format!(
                "/v1/skill-market/skills/{}/file",
                encode_skill_market_path_segment(&locator.slug)
            ),
            &skill_market_locator_query(locator),
        )
        .await
    }

    /// Installs one canonical source coordinate into the current project.
    pub async fn skill_market_install(
        &self,
        project_path: &str,
        coordinate: &SkillMarketCoordinateDto,
    ) -> Result<String, String> {
        let request = SkillMarketInstallRequest {
            project_path: project_path.to_string(),
            source: coordinate.locator.source.clone(),
            namespace: coordinate.locator.namespace.clone(),
            slug: coordinate.locator.slug.clone(),
            version: coordinate.version.clone(),
        };
        let envelope: SkillMarketEnvelope<DesktopSkillPathDto> =
            self.post_json("/v1/skill-market/skills/install", &[], &request).await?;
        if envelope.code != 0 {
            return Err(format!(
                "skill market request failed (code {}): {}",
                envelope.code, envelope.message
            ));
        }
        Ok(envelope.data.path)
    }

    pub async fn skill_market_install_to_targets(
        &self,
        project_path: Option<&str>,
        coordinate: &SkillMarketCoordinateDto,
        installations: &[SkillMarketInstallSelectionDto],
    ) -> Result<Vec<String>, String> {
        let request = SkillMarketMultiInstallRequest {
            project_path: project_path.unwrap_or_default().to_string(),
            source: coordinate.locator.source.clone(),
            namespace: coordinate.locator.namespace.clone(),
            slug: coordinate.locator.slug.clone(),
            version: coordinate.version.clone(),
            installations: installations.to_vec(),
        };
        let envelope: SkillMarketEnvelope<Vec<DesktopSkillPathDto>> =
            self.post_json("/v1/skill-market/skills/install-targets", &[], &request).await?;
        if envelope.code != 0 {
            return Err(format!(
                "skill market request failed (code {}): {}",
                envelope.code, envelope.message
            ));
        }
        Ok(envelope.data.into_iter().map(|item| item.path).collect())
    }

    /// Queries skill sets from one registered source.
    pub async fn skill_market_skill_sets(
        &self,
        source: &str,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillSetPageDto, String> {
        self.skill_market_get(
            "/v1/skill-market/skillsets",
            &skill_market_catalog_query(source, query),
        )
        .await
    }

    /// Reads one skill-set detail from a registered source.
    pub async fn skill_market_skill_set_detail(
        &self,
        source: &str,
        slug: &str,
    ) -> Result<SkillMarketSkillSetDetailDto, String> {
        self.skill_market_get(
            &format!("/v1/skill-market/skillsets/{}", encode_skill_market_path_segment(slug)),
            &[("source".to_string(), source.to_string())],
        )
        .await
    }

    /// Installs one source-aware skill set as a single atomic transaction.
    pub async fn skill_market_skill_set_install(
        &self,
        project_path: &str,
        source: &str,
        slug: &str,
    ) -> Result<Vec<String>, String> {
        let request = SkillMarketSkillSetInstallRequest {
            project_path: project_path.to_string(),
            source: source.to_string(),
            slug: slug.to_string(),
        };
        let envelope: SkillMarketEnvelope<Vec<DesktopSkillPathDto>> =
            self.post_json("/v1/skill-market/skillsets/install", &[], &request).await?;
        if envelope.code != 0 {
            return Err(format!(
                "skill market request failed (code {}): {}",
                envelope.code, envelope.message
            ));
        }
        Ok(envelope.data.into_iter().map(|item| item.path).collect())
    }

    async fn skill_market_get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<T, String> {
        let envelope: SkillMarketEnvelope<T> = self.get_json(path, query).await?;
        if envelope.code != 0 {
            return Err(format!(
                "skill market request failed (code {}): {}",
                envelope.code, envelope.message
            ));
        }
        Ok(envelope.data)
    }
}

fn skill_market_catalog_query(
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Vec<(String, String)> {
    let mut pairs = vec![("source".to_string(), source.to_string())];
    if let Some(keyword) = &query.keyword {
        pairs.push(("keyword".to_string(), keyword.clone()));
    }
    if let Some(category) = &query.category {
        pairs.push(("category".to_string(), category.clone()));
    }
    if let Some(namespace) = &query.namespace {
        pairs.push(("namespace".to_string(), namespace.clone()));
    }
    pairs.extend([
        ("sort".to_string(), skill_market_sort_value(query.sort).to_string()),
        ("page".to_string(), query.page.to_string()),
        ("page_size".to_string(), query.page_size.to_string()),
    ]);
    pairs
}

fn skill_market_locator_query(locator: &SkillMarketSkillLocatorDto) -> Vec<(String, String)> {
    vec![
        ("source".to_string(), locator.source.clone()),
        ("namespace".to_string(), locator.namespace.clone()),
    ]
}

const fn skill_market_sort_value(sort: SkillMarketSort) -> &'static str {
    match sort {
        SkillMarketSort::Score => "score",
        SkillMarketSort::Downloads => "downloads",
        SkillMarketSort::UpdatedAt => "updated_at",
        SkillMarketSort::Installs => "installs",
    }
}

fn encode_skill_market_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "skill_market_api_tests.rs"]
mod skill_market_api_tests;
