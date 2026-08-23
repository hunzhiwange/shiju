//! 统一技能市场网关，聚合多来源查询并校验安装目标与制品坐标。
//!
//! 查询接口保持只读；安装入口只接受提供者验证过的坐标，并限制写入选定技能根目录。
//!
//! Unified read-only skill-market gateway API.

use std::collections::BTreeSet;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use axum::extract::{Extension, Path, RawQuery};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use directories::BaseDirs;
use serde::Deserialize;
use serde_json::Value;
use vw_config_types::paths::HOME_CONFIG_DIR_NAME;
use vw_gateway_client::{
    DesktopSkillPathDto, SkillMarketCoordinateDto, SkillMarketEnvelope, SkillMarketInstallScopeDto,
    SkillMarketInstallSelectionDto, SkillMarketInstallTargetDto, SkillMarketSkillLocatorDto,
    SkillMarketSkillQuery, SkillMarketSort,
};

use crate::app::agent::skills::marketplace::install::{
    install_verified_market_artifact, install_verified_market_artifact_for_roots,
    install_verified_market_skill_set, validate_market_coordinate,
};
use crate::app::agent::skills::marketplace::provider::{
    SkillMarketProvider, SkillMarketProviderError, SkillMarketProviderRegistry,
    SkillMarketRegistryError,
};
use crate::app::agent::skills::workspace_skills_dir;

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 24;

/// 使用全部已启用来源构建生产技能市场路由。
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let registry = SkillMarketProviderRegistry::with_default_sources()
        .unwrap_or_else(|error| panic!("failed to initialize skill market providers: {error}"));
    router_with_registry(Arc::new(registry))
}

/// 使用显式提供者注册表构建路由，供确定性测试和受控来源复用。
pub(crate) fn router_with_registry<S>(registry: Arc<SkillMarketProviderRegistry>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/skill-market/sources", get(sources_get))
        .route("/skill-market/showcase/recommended", get(recommended_get))
        .route("/skill-market/skills", get(skills_get))
        .route("/skill-market/categories", get(categories_get))
        .route("/skill-market/skills/install", post(skill_install_post))
        .route("/skill-market/skills/install-targets", post(skill_install_targets_post))
        .route("/skill-market/skills/{slug}", get(skill_detail_get))
        .route("/skill-market/skills/{slug}/file", get(skill_file_get))
        .route("/skill-market/skillsets", get(skill_sets_get))
        .route("/skill-market/skillsets/install", post(skill_set_install_post))
        .route("/skill-market/skillsets/{slug}", get(skill_set_detail_get))
        .layer(Extension(registry))
}

async fn sources_get(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
) -> Json<SkillMarketEnvelope<Vec<vw_gateway_client::SkillMarketSourceDto>>> {
    success(registry.sources())
}

async fn recommended_get(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<SkillMarketEnvelope<Vec<vw_gateway_client::SkillMarketSkillDto>>>, MarketApiError>
{
    let query = MarketQuery::parse(raw_query.as_deref())?;
    let provider = resolve_provider(&registry, query.required_source()?)?;
    let skills = provider.recommended().await.map_err(MarketApiError::provider)?;
    Ok(success(skills))
}

async fn skills_get(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<SkillMarketEnvelope<vw_gateway_client::SkillMarketSkillPageDto>>, MarketApiError> {
    let query = MarketQuery::parse(raw_query.as_deref())?;
    let provider = resolve_provider(&registry, query.required_source()?)?;
    let catalog_query = query.catalog_query()?;
    let page = provider.skills(&catalog_query).await.map_err(MarketApiError::provider)?;
    Ok(success(page))
}

async fn categories_get(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<SkillMarketEnvelope<Vec<vw_gateway_client::SkillMarketCategoryDto>>>, MarketApiError>
{
    let query = MarketQuery::parse(raw_query.as_deref())?;
    let provider = resolve_provider(&registry, query.required_source()?)?;
    let categories = provider.categories().await.map_err(MarketApiError::provider)?;
    Ok(success(categories))
}

async fn skill_detail_get(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
    Path(slug): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<SkillMarketEnvelope<vw_gateway_client::SkillMarketSkillDetailDto>>, MarketApiError>
{
    let query = MarketQuery::parse(raw_query.as_deref())?;
    let locator = query.locator(slug)?;
    let provider = resolve_provider(&registry, &locator.source)?;
    let detail = provider.skill_detail(&locator).await.map_err(MarketApiError::provider)?;
    Ok(success(detail))
}

async fn skill_file_get(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
    Path(slug): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<SkillMarketEnvelope<vw_gateway_client::SkillMarketSkillFileDto>>, MarketApiError> {
    let query = MarketQuery::parse(raw_query.as_deref())?;
    let locator = query.locator(slug)?;
    let provider = resolve_provider(&registry, &locator.source)?;
    let file = provider.skill_file(&locator).await.map_err(MarketApiError::provider)?;
    Ok(success(file))
}

async fn skill_install_post(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
    Json(body): Json<SkillMarketInstallRequest>,
) -> Result<Json<SkillMarketEnvelope<DesktopSkillPathDto>>, MarketApiError> {
    let coordinate = body.coordinate();
    validate_market_coordinate(&coordinate)
        .map_err(|error| MarketApiError::bad_request(error.to_string()))?;
    let project_path = resolve_install_project_path(&body.project_path)?;
    let provider = resolve_provider(&registry, &coordinate.locator.source)?;
    let artifact = provider.skill_artifact(&coordinate).await.map_err(MarketApiError::provider)?;
    let skills_config = crate::app::agent::config::get().await.skills.clone();
    let skills_root = workspace_skills_dir(&project_path, skills_config.directory_provider);
    let security = skills_config.security;
    let installed = tokio::task::spawn_blocking(move || {
        install_verified_market_artifact(&skills_root, &coordinate, artifact, &security)
    })
    .await
    .map_err(|_| MarketApiError::internal("skill market install task failed"))?
    .map_err(|error| MarketApiError::bad_request(error.to_string()))?;
    Ok(success(DesktopSkillPathDto { path: installed.display().to_string() }))
}

async fn skill_install_targets_post(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
    Json(body): Json<SkillMarketMultiInstallRequest>,
) -> Result<Json<SkillMarketEnvelope<Vec<DesktopSkillPathDto>>>, MarketApiError> {
    let coordinate = body.coordinate();
    validate_market_coordinate(&coordinate)
        .map_err(|error| MarketApiError::bad_request(error.to_string()))?;
    if body.installations.is_empty() {
        return Err(MarketApiError::bad_request("at least one skill install target is required"));
    }
    let project_path = optional_value(Some(body.project_path))
        .map(|path| resolve_install_project_path(&path))
        .transpose()?;
    let home = BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .ok_or_else(|| MarketApiError::bad_request("user home directory is unavailable"))?
        .canonicalize()
        .map_err(|_| MarketApiError::bad_request("user home directory could not be resolved"))?;
    let skills_config = crate::app::agent::config::get().await.skills.clone();
    let vibe_project_root = project_path
        .as_ref()
        .map(|project| workspace_skills_dir(project, skills_config.directory_provider));
    let roots = resolve_explicit_install_roots(
        &home,
        project_path.as_deref(),
        vibe_project_root.as_deref(),
        &body.installations,
    )?;
    let provider = resolve_provider(&registry, &coordinate.locator.source)?;
    let artifact = provider.skill_artifact(&coordinate).await.map_err(MarketApiError::provider)?;
    let security = skills_config.security;
    let installed = tokio::task::spawn_blocking(move || {
        install_verified_market_artifact_for_roots(&roots, &coordinate, artifact, &security)
    })
    .await
    .map_err(|_| MarketApiError::internal("multi-target skill install task failed"))?
    .map_err(|error| MarketApiError::bad_request(error.to_string()))?;
    Ok(success(
        installed
            .into_iter()
            .map(|path| DesktopSkillPathDto { path: path.display().to_string() })
            .collect(),
    ))
}

async fn skill_sets_get(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<SkillMarketEnvelope<vw_gateway_client::SkillMarketSkillSetPageDto>>, MarketApiError>
{
    let query = MarketQuery::parse(raw_query.as_deref())?;
    let provider = resolve_provider(&registry, query.required_source()?)?;
    let catalog_query = query.catalog_query()?;
    let page = provider.skill_sets(&catalog_query).await.map_err(MarketApiError::provider)?;
    Ok(success(page))
}

async fn skill_set_detail_get(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
    Path(slug): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Result<
    Json<SkillMarketEnvelope<vw_gateway_client::SkillMarketSkillSetDetailDto>>,
    MarketApiError,
> {
    let query = MarketQuery::parse(raw_query.as_deref())?;
    let source = query.required_source()?;
    let slug = required_value("slug", Some(slug))?;
    let provider = resolve_provider(&registry, source)?;
    let detail =
        provider.skill_set_detail(source, &slug).await.map_err(MarketApiError::provider)?;
    Ok(success(detail))
}

async fn skill_set_install_post(
    Extension(registry): Extension<Arc<SkillMarketProviderRegistry>>,
    Json(body): Json<SkillMarketSkillSetInstallRequest>,
) -> Result<Json<SkillMarketEnvelope<Vec<DesktopSkillPathDto>>>, MarketApiError> {
    let project_path = resolve_install_project_path(&body.project_path)?;
    let source = required_value("source", Some(body.source))?;
    let slug = required_value("slug", Some(body.slug))?;
    let provider = resolve_provider(&registry, &source)?;
    let artifact =
        provider.skill_set_artifact(&source, &slug).await.map_err(MarketApiError::provider)?;
    let skills_config = crate::app::agent::config::get().await.skills.clone();
    let skills_root = workspace_skills_dir(&project_path, skills_config.directory_provider);
    let security = skills_config.security;
    let installed = tokio::task::spawn_blocking(move || {
        install_verified_market_skill_set(&skills_root, artifact, &security)
    })
    .await
    .map_err(|_| MarketApiError::internal("skill-set install task failed"))?
    .map_err(|error| MarketApiError::bad_request(error.to_string()))?;
    Ok(success(
        installed
            .into_iter()
            .map(|path| DesktopSkillPathDto { path: path.display().to_string() })
            .collect(),
    ))
}

fn success<T>(data: T) -> Json<SkillMarketEnvelope<T>> {
    Json(SkillMarketEnvelope { code: 0, data, message: "ok".to_string() })
}

fn resolve_provider(
    registry: &SkillMarketProviderRegistry,
    source: &str,
) -> Result<Arc<dyn SkillMarketProvider>, MarketApiError> {
    registry.resolve(source).map_err(MarketApiError::registry)
}

#[derive(Debug, Default)]
struct MarketQuery {
    source: Option<String>,
    keyword: Option<String>,
    category: Option<String>,
    namespace: Option<String>,
    sort: Option<String>,
    page: Option<String>,
    page_size: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillMarketInstallRequest {
    project_path: String,
    source: String,
    namespace: String,
    slug: String,
    version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillMarketMultiInstallRequest {
    project_path: String,
    source: String,
    namespace: String,
    slug: String,
    version: String,
    installations: Vec<SkillMarketInstallSelectionDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillMarketSkillSetInstallRequest {
    project_path: String,
    source: String,
    slug: String,
}

impl SkillMarketInstallRequest {
    fn coordinate(&self) -> SkillMarketCoordinateDto {
        SkillMarketCoordinateDto {
            locator: SkillMarketSkillLocatorDto {
                source: self.source.clone(),
                namespace: self.namespace.clone(),
                slug: self.slug.clone(),
            },
            version: self.version.clone(),
        }
    }
}

impl SkillMarketMultiInstallRequest {
    fn coordinate(&self) -> SkillMarketCoordinateDto {
        SkillMarketCoordinateDto {
            locator: SkillMarketSkillLocatorDto {
                source: self.source.clone(),
                namespace: self.namespace.clone(),
                slug: self.slug.clone(),
            },
            version: self.version.clone(),
        }
    }
}

impl MarketQuery {
    fn parse(raw_query: Option<&str>) -> Result<Self, MarketApiError> {
        let mut query = Self::default();
        for (key, value) in url::form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
            match key.as_ref() {
                "source" => query.source = Some(value.into_owned()),
                "keyword" => query.keyword = Some(value.into_owned()),
                "category" => query.category = Some(value.into_owned()),
                "namespace" => query.namespace = Some(value.into_owned()),
                "sort" => query.sort = Some(value.into_owned()),
                "page" => query.page = Some(value.into_owned()),
                "page_size" => query.page_size = Some(value.into_owned()),
                _ => {}
            }
        }
        Ok(query)
    }

    fn required_source(&self) -> Result<&str, MarketApiError> {
        self.source
            .as_deref()
            .map(str::trim)
            .filter(|source| !source.is_empty())
            .ok_or_else(|| MarketApiError::bad_request("source is required"))
    }

    fn locator(&self, slug: String) -> Result<SkillMarketSkillLocatorDto, MarketApiError> {
        Ok(SkillMarketSkillLocatorDto {
            source: self.required_source()?.to_string(),
            namespace: required_value("namespace", self.namespace.clone())?,
            slug: required_value("slug", Some(slug))?,
        })
    }

    fn catalog_query(&self) -> Result<SkillMarketSkillQuery, MarketApiError> {
        let page = parse_positive_u32("page", self.page.as_deref(), DEFAULT_PAGE)?;
        let page_size =
            parse_positive_u32("page_size", self.page_size.as_deref(), DEFAULT_PAGE_SIZE)?;
        let sort = parse_sort(self.sort.as_deref())?;
        Ok(SkillMarketSkillQuery {
            keyword: optional_value(self.keyword.clone()),
            category: optional_value(self.category.clone()),
            namespace: optional_value(self.namespace.clone()),
            sort,
            page,
            page_size,
        })
    }
}

fn optional_value(value: Option<String>) -> Option<String> {
    value.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn required_value(name: &str, value: Option<String>) -> Result<String, MarketApiError> {
    optional_value(value).ok_or_else(|| MarketApiError::bad_request(format!("{name} is required")))
}

fn resolve_install_project_path(project_path: &str) -> Result<PathBuf, MarketApiError> {
    let project_path = project_path.trim();
    if project_path.is_empty() {
        return Err(MarketApiError::bad_request("project_path is required"));
    }
    let path = PathBuf::from(project_path);
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|_| MarketApiError::bad_request("project_path must be an existing directory"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MarketApiError::bad_request("project_path must be a real directory"));
    }
    path.canonicalize()
        .map_err(|_| MarketApiError::bad_request("project_path could not be resolved"))
}

fn resolve_explicit_install_roots(
    home: &FsPath,
    project: Option<&FsPath>,
    vibe_project_root: Option<&FsPath>,
    installations: &[SkillMarketInstallSelectionDto],
) -> Result<Vec<PathBuf>, MarketApiError> {
    validate_real_base(home, "user home directory")?;
    if installations.iter().any(|selection| selection.scope == SkillMarketInstallScopeDto::Project)
    {
        validate_real_base(
            project.ok_or_else(|| {
                MarketApiError::bad_request("project_path is required for project targets")
            })?,
            "project_path",
        )?;
    }

    let mut seen = BTreeSet::new();
    let mut roots = Vec::with_capacity(installations.len());
    for selection in installations {
        let base = match selection.scope {
            SkillMarketInstallScopeDto::Global => home,
            SkillMarketInstallScopeDto::Project => project.expect("project scope validated"),
        };
        let root = match (selection.target, selection.scope) {
            (SkillMarketInstallTargetDto::VibeWindow, SkillMarketInstallScopeDto::Global) => {
                home.join(HOME_CONFIG_DIR_NAME).join("skills")
            }
            (SkillMarketInstallTargetDto::VibeWindow, SkillMarketInstallScopeDto::Project) => {
                vibe_project_root
                    .ok_or_else(|| {
                        MarketApiError::bad_request(
                            "VibeWindow project skill directory is unavailable",
                        )
                    })?
                    .to_path_buf()
            }
            (SkillMarketInstallTargetDto::AgentSkills, _) => base.join(".agents/skills"),
            (SkillMarketInstallTargetDto::Codex, _) => base.join(".codex/skills"),
            (SkillMarketInstallTargetDto::WorkBuddy, _) => base.join(".workbuddy/skills"),
            (SkillMarketInstallTargetDto::ClaudeCode, _) => base.join(".claude/skills"),
            (SkillMarketInstallTargetDto::OpenCode, SkillMarketInstallScopeDto::Global) => {
                base.join(".config/opencode/skills")
            }
            (SkillMarketInstallTargetDto::OpenCode, SkillMarketInstallScopeDto::Project) => {
                base.join(".opencode/skills")
            }
            (SkillMarketInstallTargetDto::GeminiCli, _) => base.join(".gemini/skills"),
            (SkillMarketInstallTargetDto::GitHubCopilot, SkillMarketInstallScopeDto::Global) => {
                base.join(".copilot/skills")
            }
            (SkillMarketInstallTargetDto::GitHubCopilot, SkillMarketInstallScopeDto::Project) => {
                base.join(".github/skills")
            }
            (SkillMarketInstallTargetDto::Cursor, _) => base.join(".cursor/skills"),
            (SkillMarketInstallTargetDto::Windsurf, SkillMarketInstallScopeDto::Global) => {
                base.join(".codeium/windsurf/skills")
            }
            (SkillMarketInstallTargetDto::Windsurf, SkillMarketInstallScopeDto::Project) => {
                base.join(".windsurf/skills")
            }
        };
        validate_target_root(base, &root)?;
        if seen.insert(root.clone()) {
            roots.push(root);
        }
    }
    Ok(roots)
}

fn validate_real_base(path: &FsPath, label: &str) -> Result<(), MarketApiError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| MarketApiError::bad_request(format!("{label} must exist")))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MarketApiError::bad_request(format!("{label} must be a real directory")));
    }
    Ok(())
}

fn validate_target_root(base: &FsPath, target: &FsPath) -> Result<(), MarketApiError> {
    let relative = target
        .strip_prefix(base)
        .map_err(|_| MarketApiError::bad_request("skill install target escapes its base"))?;
    let mut current = base.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(MarketApiError::bad_request(
                "skill install target contains an invalid component",
            ));
        };
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(MarketApiError::bad_request(
                    "skill install target cannot traverse a symbolic link",
                ));
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(MarketApiError::bad_request(
                    "skill install target ancestor must be a directory",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(_) => {
                return Err(MarketApiError::bad_request(
                    "skill install target could not be inspected",
                ));
            }
        }
    }
    Ok(())
}

fn parse_positive_u32(
    name: &str,
    value: Option<&str>,
    default: u32,
) -> Result<u32, MarketApiError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let parsed = value
        .parse::<u32>()
        .map_err(|_| MarketApiError::bad_request(format!("{name} must be a positive integer")))?;
    if parsed == 0 {
        return Err(MarketApiError::bad_request(format!("{name} must be a positive integer")));
    }
    Ok(parsed)
}

fn parse_sort(value: Option<&str>) -> Result<SkillMarketSort, MarketApiError> {
    match value.unwrap_or("score").trim() {
        "score" => Ok(SkillMarketSort::Score),
        "downloads" => Ok(SkillMarketSort::Downloads),
        "updated_at" => Ok(SkillMarketSort::UpdatedAt),
        "installs" => Ok(SkillMarketSort::Installs),
        _ => Err(MarketApiError::bad_request(
            "sort must be one of: score, downloads, updated_at, installs",
        )),
    }
}

#[derive(Debug)]
struct MarketApiError {
    status: StatusCode,
    message: String,
}

impl MarketApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: message.into() }
    }

    fn registry(error: SkillMarketRegistryError) -> Self {
        Self::bad_request(error.to_string())
    }

    fn internal(message: impl Into<String>) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: message.into() }
    }

    fn provider(error: SkillMarketProviderError) -> Self {
        match error {
            SkillMarketProviderError::Timeout => Self {
                status: StatusCode::GATEWAY_TIMEOUT,
                message: "skill market provider timed out".to_string(),
            },
            SkillMarketProviderError::Upstream(_) => Self {
                status: StatusCode::BAD_GATEWAY,
                message: "skill market upstream request failed".to_string(),
            },
            SkillMarketProviderError::InvalidResponse(_) => Self {
                status: StatusCode::BAD_GATEWAY,
                message: "skill market upstream response is invalid".to_string(),
            },
        }
    }
}

impl IntoResponse for MarketApiError {
    fn into_response(self) -> Response {
        let body = SkillMarketEnvelope {
            code: i32::from(self.status.as_u16()),
            data: Value::Null,
            message: self.message,
        };
        (self.status, Json(body)).into_response()
    }
}

#[cfg(test)]
#[path = "skill_market_tests.rs"]
mod skill_market_tests;
