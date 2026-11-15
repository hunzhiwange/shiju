use serde::{Deserialize, Serialize};

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
pub(super) struct SkillMarketInstallRequest {
    pub(super) project_path: String,
    pub(super) source: String,
    pub(super) namespace: String,
    pub(super) slug: String,
    pub(super) version: String,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SkillMarketMultiInstallRequest {
    pub(super) project_path: String,
    pub(super) source: String,
    pub(super) namespace: String,
    pub(super) slug: String,
    pub(super) version: String,
    pub(super) installations: Vec<SkillMarketInstallSelectionDto>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SkillMarketSkillSetInstallRequest {
    pub(super) project_path: String,
    pub(super) source: String,
    pub(super) slug: String,
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
