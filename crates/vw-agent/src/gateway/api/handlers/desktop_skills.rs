//! 桌面端技能目录与本地技能管理接口。
//!
//! 本模块把随程序打包的内置技能、本地 workspace/ancestor/global 技能目录汇聚成
//! 桌面 UI 可展示的目录，并提供创建、安装、启用/禁用和删除本地技能的入口。
//! 文件系统操作集中放在阻塞任务中执行，避免阻塞异步网关运行时。

use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::routing::{get, post};
use include_dir::{Dir, include_dir};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use vw_gateway_client::{DesktopSkillCatalogEntryDto, DesktopSkillDetailDto, DesktopSkillPathDto};

use crate::app::agent::gateway::ApiError;
use crate::app::agent::skills::{
    LocalSkillSourceKind, discover_local_skill_source_dirs_with_provider, is_local_skill_disabled,
    local_skill_disabled_marker_path, read_markdown_skill_metadata, workspace_skills_dir,
};
use vw_config_types::skills::SkillsDirectoryProvider;

static BUILT_IN_SKILLS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/skills");
static RECOMMENDED_SKILL_IDS: &[&str] =
    &["create-task-plans", "do-task-plans", "rustcodegraph"];
static BUILT_IN_SKILLS: Lazy<Vec<BuiltInSkillMeta>> = Lazy::new(discover_built_in_skills);

/// 新建本地技能时写入的最小 `SKILL.md` 模板。
const NEW_SKILL_TEMPLATE: &str = r#"---
name: {name}
description: 当用户需要 {name} 相关帮助时使用该技能。
---

# {title}

## 目的

说明该技能要解决的问题。

## 使用方式

1. 明确输入内容。
2. 根据目标执行步骤。
3. 返回结果并说明限制。
"#;

/// 构建前端中立的 skills API 路由表。
///
/// 这些接口由 gateway 统一聚合技能目录，桌面、Web 或其他前端都只消费这里的
/// DTO，不直接扫描本地技能目录。
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/skills", get(catalog_get))
        .route("/skills/detail", get(detail_get))
        .route("/skills/create", post(create_post))
        .route("/skills/install-built-in", post(install_builtin_post))
        .route("/skills/set-enabled", post(set_enabled_post))
        .route("/skills/delete", post(delete_post))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BuiltInSkillGroup {
    Recommended,
    System,
}

#[derive(Debug, Clone)]
struct BuiltInSkillMeta {
    id: String,
    title: String,
    description: String,
    group: BuiltInSkillGroup,
    resource_count: usize,
}

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone)]
struct LocalCatalogSkillMeta {
    id: String,
    title: String,
    description: String,
    resource_count: usize,
    enabled: bool,
    source: String,
    source_path: Option<String>,
    origin: Option<String>,
    namespace: Option<String>,
    package_type: Option<String>,
    source_pack: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct LocalMarketMetadata {
    origin: Option<String>,
    source: Option<String>,
    namespace: Option<String>,
    package_type: Option<String>,
    source_pack: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SkillTomlCatalogManifest {
    skill: SkillTomlCatalogMeta,
}

#[derive(Debug, Deserialize)]
struct SkillTomlCatalogMeta {
    name: String,
    description: String,
}

/// 桌面技能目录查询参数。
#[derive(Debug, Deserialize)]
pub(crate) struct SkillsCatalogQuery {
    /// 可选项目路径；存在时会优先发现该项目及其祖先目录中的本地技能。
    #[serde(default)]
    pub(crate) project_path: Option<String>,
}

/// 桌面技能详情查询参数。
#[derive(Debug, Deserialize)]
pub(crate) struct SkillsDetailQuery {
    /// 可选项目路径；用于解析可覆盖内置技能的本地技能。
    #[serde(default)]
    pub(crate) project_path: Option<String>,
    /// 技能 id，来自目录项或内置技能目录名。
    pub(crate) skill_id: String,
}

/// 需要项目上下文的技能操作请求。
#[derive(Debug, Deserialize)]
pub(crate) struct SkillsProjectRequest {
    /// 项目根目录，作为 `skills/` 目录的创建位置。
    pub(crate) project_path: String,
}

/// 安装内置技能到项目目录的请求。
#[derive(Debug, Deserialize)]
pub(crate) struct SkillsInstallBuiltInRequest {
    /// 目标项目根目录。
    pub(crate) project_path: String,
    /// 待复制的内置技能 id。
    pub(crate) skill_id: String,
}

/// 对已存在本地技能执行变更的请求。
#[derive(Debug, Deserialize)]
pub(crate) struct SkillsMutationRequest {
    /// 可选项目路径；为空时按本地技能发现规则搜索可管理技能。
    #[serde(default)]
    pub(crate) project_path: Option<String>,
    /// 目标技能 id。
    pub(crate) skill_id: String,
}

/// 启用或禁用本地技能的请求。
#[derive(Debug, Deserialize)]
pub(crate) struct SkillsSetEnabledRequest {
    /// 可选项目路径；为空时按本地技能发现规则搜索可管理技能。
    #[serde(default)]
    pub(crate) project_path: Option<String>,
    /// 目标技能 id。
    pub(crate) skill_id: String,
    /// `true` 表示移除禁用标记，`false` 表示写入禁用标记。
    pub(crate) enabled: bool,
}

/// 获取桌面技能目录。
///
/// # 参数
///
/// * `query` - 可选项目路径，用于合并项目本地技能与内置技能。
///
/// # 返回值
///
/// 返回排序后的技能目录 DTO 列表，包含安装状态、启用状态和资源数量。
///
/// # 错误处理
///
/// 本地目录读取、项目路径解析或阻塞任务 Join 失败时返回 [`ApiError`]。
pub(crate) async fn catalog_get(
    Query(query): Query<SkillsCatalogQuery>,
) -> Result<Json<Vec<DesktopSkillCatalogEntryDto>>, ApiError> {
    let project_path = normalize_optional_project_path(query.project_path);
    let directory_provider = crate::app::agent::config::get().await.skills.directory_provider;
    let items = tokio::task::spawn_blocking(move || {
        collect_catalog_skills(project_path.as_deref(), directory_provider)
    })
    .await
    .map_err(|err| ApiError::internal(format!("desktop skills catalog task failed: {err}")))?
    .map_err(ApiError::bad_request)?;

    Ok(Json(items))
}

/// 获取单个技能的详情与文档内容。
///
/// # 参数
///
/// * `query` - 技能 id 与可选项目路径。
///
/// # 返回值
///
/// 返回技能元数据、可执行操作标记以及 `SKILL.md`/`SKILL.toml` 内容。
///
/// # 错误处理
///
/// 技能 id 为空、技能不存在、文档读取失败或阻塞任务失败时返回 [`ApiError`]。
pub(crate) async fn detail_get(
    Query(query): Query<SkillsDetailQuery>,
) -> Result<Json<DesktopSkillDetailDto>, ApiError> {
    let project_path = normalize_optional_project_path(query.project_path);
    let skill_id = query.skill_id.trim().to_string();
    if skill_id.is_empty() {
        return Err(ApiError::bad_request("skill_id is required"));
    }
    let directory_provider = crate::app::agent::config::get().await.skills.directory_provider;

    let detail = tokio::task::spawn_blocking(move || {
        resolve_skill_detail(project_path.as_deref(), &skill_id, directory_provider)
    })
    .await
    .map_err(|err| ApiError::internal(format!("desktop skill detail task failed: {err}")))?
    .map_err(ApiError::bad_request)?;

    Ok(Json(detail))
}

/// 在项目中创建一个新的本地技能骨架。
///
/// # 参数
///
/// * `body` - 包含项目根目录。
///
/// # 返回值
///
/// 返回新建技能目录路径。
///
/// # 错误处理
///
/// 项目路径无效、目录创建失败或模板写入失败时返回 [`ApiError`]。
pub(crate) async fn create_post(
    Json(body): Json<SkillsProjectRequest>,
) -> Result<Json<DesktopSkillPathDto>, ApiError> {
    let project_path = resolve_project_path(&body.project_path).map_err(ApiError::bad_request)?;
    let directory_provider = crate::app::agent::config::get().await.skills.directory_provider;
    let path = tokio::task::spawn_blocking(move || {
        create_new_skill_scaffold(&project_path, directory_provider)
    })
    .await
    .map_err(|err| ApiError::internal(format!("desktop skills create task failed: {err}")))?
    .map_err(ApiError::bad_request)?;

    Ok(Json(DesktopSkillPathDto { path }))
}

/// 将内置技能安装到项目 `skills/` 目录。
///
/// # 参数
///
/// * `body` - 目标项目路径与内置技能 id。
///
/// # 返回值
///
/// 返回安装后的本地技能目录路径；已存在时保持幂等并返回现有路径。
///
/// # 错误处理
///
/// 技能 id 为空、内置技能不存在、项目目录无效或复制失败时返回 [`ApiError`]。
pub(crate) async fn install_builtin_post(
    Json(body): Json<SkillsInstallBuiltInRequest>,
) -> Result<Json<DesktopSkillPathDto>, ApiError> {
    let project_path = resolve_project_path(&body.project_path).map_err(ApiError::bad_request)?;
    let skill_id = body.skill_id.trim().to_string();
    if skill_id.is_empty() {
        return Err(ApiError::bad_request("skill_id is required"));
    }
    let directory_provider = crate::app::agent::config::get().await.skills.directory_provider;

    let path = tokio::task::spawn_blocking(move || {
        install_built_in_skill(&project_path, &skill_id, directory_provider)
    })
    .await
    .map_err(|err| ApiError::internal(format!("desktop skills install task failed: {err}")))?
    .map_err(ApiError::bad_request)?;

    Ok(Json(DesktopSkillPathDto { path }))
}

/// 设置本地技能启用状态。
///
/// # 参数
///
/// * `body` - 目标技能 id、可选项目路径和启用标记。
///
/// # 返回值
///
/// 返回被修改的本地技能目录路径。
///
/// # 错误处理
///
/// 技能 id 为空、技能不可管理或禁用标记写入/删除失败时返回 [`ApiError`]。
pub(crate) async fn set_enabled_post(
    Json(body): Json<SkillsSetEnabledRequest>,
) -> Result<Json<DesktopSkillPathDto>, ApiError> {
    let project_path = normalize_optional_project_path(body.project_path);
    let skill_id = body.skill_id.trim().to_string();
    if skill_id.is_empty() {
        return Err(ApiError::bad_request("skill_id is required"));
    }
    let directory_provider = crate::app::agent::config::get().await.skills.directory_provider;

    let enabled = body.enabled;
    let path = tokio::task::spawn_blocking(move || {
        set_local_skill_enabled(project_path.as_deref(), &skill_id, enabled, directory_provider)
    })
    .await
    .map_err(|err| ApiError::internal(format!("desktop skill toggle task failed: {err}")))?
    .map_err(ApiError::bad_request)?;

    Ok(Json(DesktopSkillPathDto { path }))
}

/// 删除一个可管理的本地技能目录。
///
/// # 参数
///
/// * `body` - 目标技能 id 与可选项目路径。
///
/// # 返回值
///
/// 返回被删除的目录路径。
///
/// # 错误处理
///
/// 技能 id 为空、技能不存在或目录删除失败时返回 [`ApiError`]。
pub(crate) async fn delete_post(
    Json(body): Json<SkillsMutationRequest>,
) -> Result<Json<DesktopSkillPathDto>, ApiError> {
    let project_path = normalize_optional_project_path(body.project_path);
    let skill_id = body.skill_id.trim().to_string();
    if skill_id.is_empty() {
        return Err(ApiError::bad_request("skill_id is required"));
    }
    let directory_provider = crate::app::agent::config::get().await.skills.directory_provider;

    let path = tokio::task::spawn_blocking(move || {
        delete_local_skill(project_path.as_deref(), &skill_id, directory_provider)
    })
    .await
    .map_err(|err| ApiError::internal(format!("desktop skill delete task failed: {err}")))?
    .map_err(ApiError::bad_request)?;

    Ok(Json(DesktopSkillPathDto { path }))
}

fn collect_catalog_skills(
    project_path: Option<&str>,
    directory_provider: SkillsDirectoryProvider,
) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
    let local_skills = discover_local_skills(project_path, directory_provider)?;
    let local_skill_ids = local_skills.iter().map(|skill| skill.id.clone()).collect::<HashSet<_>>();
    let local_by_id = local_skills
        .iter()
        .cloned()
        .map(|skill| (skill.id.clone(), skill))
        .collect::<HashMap<_, _>>();
    let built_in_ids = BUILT_IN_SKILLS.iter().map(|skill| skill.id.clone()).collect::<HashSet<_>>();

    // 内置技能先入表，再用同 id 的本地技能覆盖展示信息，表达“安装后可本地修改”。
    let mut items = BUILT_IN_SKILLS
        .iter()
        .map(|skill| {
            let local = local_by_id.get(&skill.id);

            DesktopSkillCatalogEntryDto {
                id: skill.id.clone(),
                title: local.map(|item| item.title.clone()).unwrap_or_else(|| skill.title.clone()),
                description: local
                    .map(|item| item.description.clone())
                    .unwrap_or_else(|| skill.description.clone()),
                kind: match skill.group {
                    BuiltInSkillGroup::Recommended => "recommended".to_string(),
                    BuiltInSkillGroup::System => "system".to_string(),
                },
                resource_count: local
                    .map(|item| item.resource_count)
                    .unwrap_or(skill.resource_count),
                installed: local_skill_ids.contains(&skill.id),
                enabled: local.map(|item| item.enabled).unwrap_or(false),
                source: local
                    .map(|item| item.source.clone())
                    .unwrap_or_else(|| "bundled".to_string()),
                source_path: local.and_then(|item| item.source_path.clone()),
                origin: local.and_then(|item| item.origin.clone()),
                namespace: local.and_then(|item| item.namespace.clone()),
                package_type: local.and_then(|item| item.package_type.clone()),
                source_pack: local.and_then(|item| item.source_pack.clone()),
            }
        })
        .collect::<Vec<_>>();

    items.extend(local_skills.into_iter().filter(|skill| !built_in_ids.contains(&skill.id)).map(
        |skill| DesktopSkillCatalogEntryDto {
            id: skill.id,
            title: skill.title,
            description: skill.description,
            kind: "personal".to_string(),
            resource_count: skill.resource_count,
            installed: true,
            enabled: skill.enabled,
            source: skill.source,
            source_path: skill.source_path,
            origin: skill.origin,
            namespace: skill.namespace,
            package_type: skill.package_type,
            source_pack: skill.source_pack,
        },
    ));

    items.sort_by(|left, right| {
        skill_kind_sort_key(&left.kind)
            .cmp(&skill_kind_sort_key(&right.kind))
            .then_with(|| left.title.to_ascii_lowercase().cmp(&right.title.to_ascii_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(items)
}

fn skill_kind_sort_key(kind: &str) -> u8 {
    match kind {
        "recommended" => 0,
        "system" => 1,
        "personal" => 2,
        _ => 3,
    }
}

fn discover_built_in_skills() -> Vec<BuiltInSkillMeta> {
    let mut skills = Vec::new();

    for dir in BUILT_IN_SKILLS_DIR.dirs() {
        let Some(skill_id) = dir.path().file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(skill_file) = find_bundled_skill_file(dir, "SKILL.md") else {
            continue;
        };
        let Ok(skill_markdown) = std::str::from_utf8(skill_file.contents()) else {
            continue;
        };

        let frontmatter = parse_skill_frontmatter(skill_markdown);
        let title = frontmatter
            .as_ref()
            .and_then(|meta| meta.name.as_ref())
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| humanize_skill_id(skill_id));
        let description = frontmatter
            .as_ref()
            .and_then(|meta| meta.description.as_ref())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "内置技能，由 gateway service 提供目录视图。".to_string());

        let group = if RECOMMENDED_SKILL_IDS.contains(&skill_id) {
            BuiltInSkillGroup::Recommended
        } else {
            BuiltInSkillGroup::System
        };
        let resource_count = dir.files().count().saturating_sub(1) + dir.dirs().count();

        skills.push(BuiltInSkillMeta {
            id: skill_id.to_string(),
            title,
            description,
            group,
            resource_count,
        });
    }

    skills.sort_by(|left, right| {
        left.group
            .cmp(&right.group)
            .then_with(|| left.title.to_ascii_lowercase().cmp(&right.title.to_ascii_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });
    skills
}

fn discover_local_skills(
    project_path: Option<&str>,
    directory_provider: SkillsDirectoryProvider,
) -> Result<Vec<LocalCatalogSkillMeta>, String> {
    let workspace_dir = project_path.map(resolve_project_path).transpose()?;
    let mut skills = Vec::new();
    let mut seen_ids = HashSet::new();

    for source_dir in
        discover_local_skill_source_dirs_with_provider(workspace_dir.as_deref(), directory_provider)
    {
        let entries = match std::fs::read_dir(&source_dir.path) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(format!("读取 skills 目录失败 {}: {err}", source_dir.path.display()));
            }
        };

        for entry in entries.filter_map(Result::ok) {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }

            let skill_id = entry.file_name().to_string_lossy().to_string();
            // 发现顺序代表优先级；同 id 技能只保留最先出现的来源，避免目录中出现重复项。
            if !seen_ids.insert(skill_id.clone()) {
                continue;
            }

            let Some((title, description)) = read_local_catalog_metadata(&entry.path(), &skill_id)
            else {
                continue;
            };

            let local_origin = local_skill_source_key(source_dir.kind);
            let market = read_local_market_metadata(&entry.path());
            let market_source = market
                .source
                .as_deref()
                .map(str::trim)
                .filter(|source| !source.is_empty())
                .map(str::to_string);
            skills.push(LocalCatalogSkillMeta {
                id: skill_id,
                title,
                description,
                resource_count: count_skill_resources(&entry.path()),
                enabled: !is_local_skill_disabled(&entry.path()),
                source: market_source
                    .clone()
                    .unwrap_or_else(|| local_origin.to_string()),
                source_path: Some(entry.path().display().to_string()),
                origin: market_source.map(|_| {
                    market
                        .origin
                        .as_deref()
                        .map(str::trim)
                        .filter(|origin| !origin.is_empty())
                        .unwrap_or(local_origin)
                        .to_string()
                }),
                namespace: normalized_metadata_value(market.namespace),
                package_type: normalized_metadata_value(market.package_type),
                source_pack: normalized_metadata_value(market.source_pack),
            });
        }
    }

    skills.sort_by(|left, right| left.title.cmp(&right.title).then_with(|| left.id.cmp(&right.id)));
    Ok(skills)
}

fn read_local_market_metadata(skill_dir: &Path) -> LocalMarketMetadata {
    let Ok(content) = std::fs::read_to_string(skill_dir.join("_meta.json")) else {
        return LocalMarketMetadata::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

fn normalized_metadata_value(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn read_local_catalog_metadata(skill_dir: &Path, skill_id: &str) -> Option<(String, String)> {
    let toml_path = skill_dir.join("SKILL.toml");
    if toml_path.is_file() {
        let content = std::fs::read_to_string(&toml_path).ok()?;
        let manifest: SkillTomlCatalogManifest = toml::from_str(&content).ok()?;
        let title = manifest.skill.name.trim();
        let description = manifest.skill.description.trim();
        return Some((
            if title.is_empty() { humanize_skill_id(skill_id) } else { title.to_string() },
            if description.is_empty() {
                "本地技能。".to_string()
            } else {
                description.to_string()
            },
        ));
    }

    let md_path = skill_dir.join("SKILL.md");
    let metadata = read_markdown_skill_metadata(&md_path).ok()?;
    Some((
        metadata.display_name.unwrap_or_else(|| humanize_skill_id(skill_id)),
        metadata.description.unwrap_or_else(|| "本地技能。".to_string()),
    ))
}

fn count_skill_resources(skill_dir: &Path) -> usize {
    std::fs::read_dir(skill_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    !name.eq_ignore_ascii_case("SKILL.md")
                        && !name.eq_ignore_ascii_case("SKILL.toml")
                })
                .count()
        })
        .unwrap_or(0)
}

fn local_skill_source_key(kind: LocalSkillSourceKind) -> &'static str {
    match kind {
        LocalSkillSourceKind::Workspace => "workspace",
        LocalSkillSourceKind::Ancestor => "ancestor",
        LocalSkillSourceKind::Global => "global",
    }
}

fn parse_skill_frontmatter(markdown: &str) -> Option<SkillFrontmatter> {
    let mut lines = markdown.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }

    let mut yaml = String::new();
    for line in lines {
        if line.trim() == "---" {
            return serde_yaml::from_str(&yaml).ok();
        }
        yaml.push_str(line);
        yaml.push('\n');
    }

    None
}

fn find_built_in_skill(skill_id: &str) -> Option<&'static BuiltInSkillMeta> {
    BUILT_IN_SKILLS.iter().find(|skill| skill.id == skill_id)
}

fn find_bundled_skill_file<'a>(
    skill_dir: &'a Dir<'static>,
    file_name: &str,
) -> Option<&'a include_dir::File<'static>> {
    skill_dir
        .files()
        .find(|file| file.path().file_name().and_then(|name| name.to_str()) == Some(file_name))
}

fn resolve_skill_detail(
    project_path: Option<&str>,
    skill_id: &str,
    directory_provider: SkillsDirectoryProvider,
) -> Result<DesktopSkillDetailDto, String> {
    if let Some(local_skill) = discover_local_skills(project_path, directory_provider)?
        .into_iter()
        .find(|skill| skill.id == skill_id)
    {
        let skill_dir = local_skill
            .source_path
            .as_deref()
            .map(PathBuf::from)
            .ok_or_else(|| format!("技能缺少本地路径: {skill_id}"))?;
        let (document_name, document_content) = read_local_skill_document(&skill_dir)?;
        let kind = match find_built_in_skill(skill_id).map(|skill| skill.group) {
            Some(BuiltInSkillGroup::Recommended) => "recommended".to_string(),
            Some(BuiltInSkillGroup::System) => "system".to_string(),
            None => "personal".to_string(),
        };

        return Ok(DesktopSkillDetailDto {
            id: local_skill.id,
            title: local_skill.title,
            description: local_skill.description,
            kind,
            installed: true,
            enabled: local_skill.enabled,
            source: local_skill.source,
            source_path: local_skill.source_path,
            document_name,
            document_content,
            can_install: false,
            can_toggle: true,
            can_delete: true,
        });
    }

    let built_in_skill =
        find_built_in_skill(skill_id).ok_or_else(|| format!("未找到技能: {skill_id}"))?;
    let document_content = read_built_in_skill_markdown(skill_id)?;

    Ok(DesktopSkillDetailDto {
        id: built_in_skill.id.clone(),
        title: built_in_skill.title.clone(),
        description: built_in_skill.description.clone(),
        kind: match built_in_skill.group {
            BuiltInSkillGroup::Recommended => "recommended".to_string(),
            BuiltInSkillGroup::System => "system".to_string(),
        },
        installed: false,
        enabled: false,
        source: "bundled".to_string(),
        source_path: None,
        document_name: "SKILL.md".to_string(),
        document_content,
        can_install: project_path.is_some(),
        can_toggle: false,
        can_delete: false,
    })
}

fn read_built_in_skill_markdown(skill_id: &str) -> Result<String, String> {
    let skill_dir = BUILT_IN_SKILLS_DIR
        .get_dir(skill_id)
        .ok_or_else(|| format!("未找到内置技能: {skill_id}"))?;
    let skill_file = find_bundled_skill_file(skill_dir, "SKILL.md")
        .ok_or_else(|| format!("内置技能缺少 SKILL.md: {skill_id}"))?;

    std::str::from_utf8(skill_file.contents())
        .map(|content| content.to_string())
        .map_err(|err| format!("读取内置技能文档失败: {err}"))
}

fn read_local_skill_document(skill_dir: &Path) -> Result<(String, String), String> {
    let md_path = skill_dir.join("SKILL.md");
    if md_path.is_file() {
        return std::fs::read_to_string(&md_path)
            .map(|content| ("SKILL.md".to_string(), content))
            .map_err(|err| format!("读取 SKILL.md 失败: {err}"));
    }

    let toml_path = skill_dir.join("SKILL.toml");
    if toml_path.is_file() {
        return std::fs::read_to_string(&toml_path)
            .map(|content| ("SKILL.toml".to_string(), content))
            .map_err(|err| format!("读取 SKILL.toml 失败: {err}"));
    }

    Err(format!("技能目录缺少 SKILL.md 或 SKILL.toml: {}", skill_dir.display()))
}

fn resolve_local_skill_dir(
    project_path: Option<&str>,
    skill_id: &str,
    directory_provider: SkillsDirectoryProvider,
) -> Result<PathBuf, String> {
    let skill = discover_local_skills(project_path, directory_provider)?
        .into_iter()
        .find(|item| item.id == skill_id)
        .ok_or_else(|| format!("未找到可管理的本地技能: {skill_id}"))?;

    skill
        .source_path
        .as_deref()
        .map(PathBuf::from)
        .ok_or_else(|| format!("技能缺少本地路径: {skill_id}"))
}

fn set_local_skill_enabled(
    project_path: Option<&str>,
    skill_id: &str,
    enabled: bool,
    directory_provider: SkillsDirectoryProvider,
) -> Result<String, String> {
    let skill_dir = resolve_local_skill_dir(project_path, skill_id, directory_provider)?;
    let marker_path = local_skill_disabled_marker_path(&skill_dir);

    // 技能启用状态用禁用标记表达，避免修改用户维护的 SKILL 文档内容。
    if enabled {
        if marker_path.exists() {
            std::fs::remove_file(&marker_path).map_err(|err| format!("移除禁用标记失败: {err}"))?;
        }
    } else if !marker_path.exists() {
        std::fs::write(&marker_path, b"disabled\n")
            .map_err(|err| format!("写入禁用标记失败: {err}"))?;
    }

    Ok(skill_dir.display().to_string())
}

fn delete_local_skill(
    project_path: Option<&str>,
    skill_id: &str,
    directory_provider: SkillsDirectoryProvider,
) -> Result<String, String> {
    let skill_dir = resolve_local_skill_dir(project_path, skill_id, directory_provider)?;
    std::fs::remove_dir_all(&skill_dir).map_err(|err| format!("删除技能目录失败: {err}"))?;
    Ok(skill_dir.display().to_string())
}

fn humanize_skill_id(skill_id: &str) -> String {
    let words = skill_id
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = first.to_uppercase().to_string();
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>();

    if words.is_empty() { skill_id.to_string() } else { words.join(" ") }
}

fn normalize_optional_project_path(project_path: Option<String>) -> Option<String> {
    project_path.and_then(|path| {
        let trimmed = path.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    })
}

fn resolve_project_path(project_path: &str) -> Result<PathBuf, String> {
    let trimmed = project_path.trim();
    if trimmed.is_empty() {
        return Err("project_path is required".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.is_dir() {
        return Err(format!("项目目录不存在: {trimmed}"));
    }
    Ok(path)
}

fn unique_skill_directory(skills_root: &Path) -> PathBuf {
    let base = skills_root.join("new-skill");
    if !base.exists() {
        return base;
    }

    for index in 2..1000 {
        let candidate = skills_root.join(format!("new-skill-{index}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    skills_root.join(format!("new-skill-{}", uuid_suffix()))
}

fn uuid_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "custom".to_string())
}

fn create_new_skill_scaffold(
    project_path: &Path,
    directory_provider: SkillsDirectoryProvider,
) -> Result<String, String> {
    let skills_root = workspace_skills_dir(project_path, directory_provider);
    std::fs::create_dir_all(&skills_root).map_err(|err| format!("创建 skills 目录失败: {err}"))?;

    let skill_dir = unique_skill_directory(&skills_root);
    std::fs::create_dir_all(&skill_dir).map_err(|err| format!("创建技能目录失败: {err}"))?;

    let skill_name =
        skill_dir.file_name().and_then(|name| name.to_str()).unwrap_or("new-skill").to_string();
    let title = skill_name.replace('-', " ");
    let content = NEW_SKILL_TEMPLATE.replace("{name}", &skill_name).replace("{title}", &title);
    std::fs::write(skill_dir.join("SKILL.md"), content)
        .map_err(|err| format!("写入 SKILL.md 失败: {err}"))?;

    Ok(skill_dir.display().to_string())
}

fn copy_bundled_dir_recursive(source: &Dir<'_>, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|err| format!("创建目录失败: {err}"))?;

    for file in source.files() {
        let Some(file_name) = file.path().file_name() else {
            continue;
        };
        std::fs::write(dest.join(file_name), file.contents())
            .map_err(|err| format!("写入 {:?} 失败: {err}", file_name))?;
    }

    for dir in source.dirs() {
        let Some(dir_name) = dir.path().file_name() else {
            continue;
        };
        copy_bundled_dir_recursive(dir, &dest.join(dir_name))?;
    }

    Ok(())
}

fn install_built_in_skill(
    project_path: &Path,
    skill_id: &str,
    directory_provider: SkillsDirectoryProvider,
) -> Result<String, String> {
    let Some(skill_dir) = BUILT_IN_SKILLS_DIR.get_dir(skill_id) else {
        return Err(format!("未找到内置技能: {skill_id}"));
    };

    let skills_root = workspace_skills_dir(project_path, directory_provider);
    std::fs::create_dir_all(&skills_root).map_err(|err| format!("创建 skills 目录失败: {err}"))?;

    let target_dir = skills_root.join(skill_id);
    if target_dir.exists() {
        return Ok(target_dir.display().to_string());
    }

    copy_bundled_dir_recursive(skill_dir, &target_dir)?;
    Ok(target_dir.display().to_string())
}

#[cfg(test)]
#[path = "desktop_skills_tests.rs"]
mod desktop_skills_tests;
