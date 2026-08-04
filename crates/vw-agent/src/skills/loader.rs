//! # 技能加载器模块
//!
//! 本模块负责从文件系统加载技能定义，支持多种清单格式与加载模式。
//!
//! ## 主要功能
//!
//! - **目录扫描**：递归扫描技能目录，发现并加载所有有效技能
//! - **格式支持**：支持 TOML 清单（`SKILL.toml`）与 Markdown（`SKILL.md`）两种格式
//! - **安全审计**：加载前自动对技能目录进行安全审计，拒绝不安全的技能
//! - **加载模式**：支持完整加载与仅元数据加载两种模式，按需权衡性能与完整性
//!
//! ## 加载优先级
//!
//! 对于每个技能目录，按以下优先级尝试加载：
//! 1. `SKILL.toml` —— 结构化清单文件（推荐）
//! 2. `SKILL.md` —— Markdown 格式技能定义（兼容模式）
//!
//! ## 安全机制
//!
//! 所有技能目录在加载前必须通过安全审计。未通过审计的技能将被跳过并记录警告日志。

use crate::app::agent::skills::types::{
    Skill, SkillLoadMode, SkillManifest, SkillMetadataManifest,
};
use anyhow::Result;
use serde::Deserialize;
use std::path::Path;
use vw_config_types::skills::SkillSecurityConfig;

#[derive(Debug, Default, Deserialize)]
struct MarkdownSkillFrontmatter {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MarkdownSkillMetadata {
    pub(crate) display_name: Option<String>,
    pub(crate) description: Option<String>,
}

pub(crate) fn read_markdown_skill_metadata(path: &Path) -> Result<MarkdownSkillMetadata> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_markdown_skill_metadata(&content))
}

pub(crate) fn parse_markdown_skill_metadata(content: &str) -> MarkdownSkillMetadata {
    let frontmatter = parse_markdown_skill_frontmatter(content);
    let display_name =
        frontmatter.as_ref().and_then(|meta| sanitize_frontmatter_value(meta.name.as_deref()));
    let description = frontmatter
        .as_ref()
        .and_then(|meta| sanitize_frontmatter_value(meta.description.as_deref()))
        .or_else(|| extract_description_from_body(markdown_body(content)));

    MarkdownSkillMetadata { display_name, description }
}

fn parse_markdown_skill_frontmatter(content: &str) -> Option<MarkdownSkillFrontmatter> {
    let mut lines = content.lines();
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

fn sanitize_frontmatter_value(value: Option<&str>) -> Option<String> {
    value.map(str::trim).filter(|value| !value.is_empty()).map(ToString::to_string)
}

fn markdown_body(content: &str) -> &str {
    let mut parts = content.split_inclusive('\n');
    let Some(first) = parts.next() else {
        return content;
    };
    if first.trim() != "---" {
        return content;
    }

    let mut consumed = first.len();
    for part in parts {
        consumed += part.len();
        if part.trim() == "---" {
            return &content[consumed..];
        }
    }

    content
}

fn extract_description_from_body(content: &str) -> Option<String> {
    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && *line != "---")
        .map(ToString::to_string)
}

/// 从指定目录加载所有技能
///
/// 扫描给定目录下的所有子目录，对每个子目录进行安全审计后尝试加载技能定义。
/// 支持两种清单格式：`SKILL.toml`（优先）和 `SKILL.md`（备用）。
///
/// # 参数
///
/// - `skills_dir`：技能目录路径，应包含多个技能子目录
/// - `load_mode`：加载模式，决定是完整加载技能内容还是仅加载元数据
///
/// # 返回值
///
/// 返回成功加载的技能列表。以下情况技能将被跳过（不会导致错误）：
/// - 目录不存在
/// - 目录无法读取
/// - 子目录未通过安全审计
/// - 清单文件解析失败
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
/// use crate::app::agent::skills::loader::load_skills_from_directory;
/// use crate::app::agent::skills::types::SkillLoadMode;
///
/// let skills = load_skills_from_directory(
///     Path::new("./skills"),
///     SkillLoadMode::Full
/// );
/// for skill in skills {
///     println!("已加载技能: {}", skill.name);
/// }
/// ```
pub(crate) fn load_skills_from_directory(
    skills_dir: &Path,
    load_mode: SkillLoadMode,
    security: &SkillSecurityConfig,
) -> Vec<Skill> {
    // 目录不存在时返回空列表，不视为错误
    if !skills_dir.exists() {
        return Vec::new();
    }

    let mut skills = Vec::new();

    // 尝试读取目录内容，失败时返回已收集的技能（可能为空）
    let Ok(entries) = std::fs::read_dir(skills_dir) else {
        return skills;
    };

    // 遍历目录中的每个条目
    for entry in entries.flatten() {
        let path = entry.path();

        // 跳过非目录条目（技能必须位于独立目录中）
        if !path.is_dir() {
            continue;
        }

        if crate::app::agent::skills::is_local_skill_disabled(&path) {
            continue;
        }

        // 对技能目录进行安全审计，确保不包含危险文件或符号链接
        match crate::app::agent::skills::audit::audit_skill_directory_with_security(
            &path, security,
        ) {
            // 审计通过，继续加载
            Ok(report) if report.is_clean() => {}
            // 审计发现问题，跳过该技能并记录警告
            Ok(report) => {
                tracing::warn!(
                    "skipping insecure skill directory {}: {}",
                    path.display(),
                    report.summary()
                );
                continue;
            }
            // 审计过程出错，跳过该技能并记录警告
            Err(err) => {
                tracing::warn!("skipping unauditable skill directory {}: {err}", path.display());
                continue;
            }
        }

        // 按优先级尝试加载清单文件：SKILL.toml 优先，SKILL.md 备用
        let manifest_path = path.join("SKILL.toml");
        let md_path = path.join("SKILL.md");

        if manifest_path.exists() {
            // 尝试加载 TOML 清单
            if let Ok(skill) = load_skill_toml(&manifest_path, load_mode) {
                skills.push(skill);
            }
        } else if md_path.exists() {
            // 尝试加载 Markdown 清单
            if let Ok(skill) = load_skill_md(&md_path, &path, load_mode) {
                skills.push(skill);
            }
        }
    }

    skills
}

/// 从 TOML 清单文件加载技能
///
/// 解析 `SKILL.toml` 文件并构建技能对象。根据加载模式决定是否解析工具和提示词。
///
/// # 参数
///
/// - `path`：`SKILL.toml` 文件的完整路径
/// - `load_mode`：加载模式
///   - `Full`：完整解析所有字段（包括工具和提示词）
///   - `MetadataOnly`：仅解析元数据字段，跳过工具和提示词以提升性能
///
/// # 返回值
///
/// - `Ok(Skill)`：成功解析的技能对象
/// - `Err`：文件读取或 TOML 解析失败
///
/// # 清单结构
///
/// TOML 清单应包含 `[skill]` 节定义元数据，可选包含 `[[tools]]` 和 `[[prompts]]` 节。
fn load_skill_toml(path: &Path, load_mode: SkillLoadMode) -> Result<Skill> {
    let content = std::fs::read_to_string(path)?;

    match load_mode {
        // 完整加载模式：解析所有字段
        SkillLoadMode::Full => {
            let manifest: SkillManifest = toml::from_str(&content)?;

            Ok(Skill {
                name: manifest.skill.name,
                description: manifest.skill.description,
                version: manifest.skill.version,
                author: manifest.skill.author,
                tags: manifest.skill.tags,
                tools: manifest.tools,
                prompts: manifest.prompts,
                location: Some(path.to_path_buf()),
            })
        }
        // 仅元数据模式：跳过工具和提示词以提升性能
        SkillLoadMode::MetadataOnly => {
            let manifest: SkillMetadataManifest = toml::from_str(&content)?;

            Ok(Skill {
                name: manifest.skill.name,
                description: manifest.skill.description,
                version: manifest.skill.version,
                author: manifest.skill.author,
                tags: manifest.skill.tags,
                tools: Vec::new(),
                prompts: Vec::new(),
                location: Some(path.to_path_buf()),
            })
        }
    }
}

/// 从 Markdown 文件加载技能
///
/// 解析 `SKILL.md` 文件并构建技能对象。Markdown 格式技能主要用于向后兼容，
/// 元数据从文件内容和目录名推断。
///
/// # 参数
///
/// - `path`：`SKILL.md` 文件的完整路径
/// - `dir`：技能目录路径，用于提取技能名称
/// - `load_mode`：加载模式
///
/// # 返回值
///
/// - `Ok(Skill)`：成功构建的技能对象
/// - `Err`：文件读取失败
///
/// # 默认值
///
/// Markdown 格式技能使用以下默认值：
/// - `name`：目录名称
/// - `version`：`"0.1.0"`
/// - `author`：`None`
/// - `tags`：空列表
/// - `tools`：空列表
fn load_skill_md(path: &Path, dir: &Path, load_mode: SkillLoadMode) -> Result<Skill> {
    // 使用目录名作为技能名称
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
    let content = std::fs::read_to_string(path)?;
    let metadata = parse_markdown_skill_metadata(&content);
    let description = metadata.description.unwrap_or_else(|| "No description".to_string());
    let prompts = match load_mode {
        SkillLoadMode::Full => vec![content],
        SkillLoadMode::MetadataOnly => Vec::new(),
    };

    Ok(Skill {
        name,
        description,
        version: "0.1.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: Vec::new(),
        prompts,
        location: Some(path.to_path_buf()),
    })
}

/// 从任意 Markdown 文件加载开放技能
///
/// 与 `load_skill_md` 类似，但用于加载不在技能目录中的独立 Markdown 技能文件。
/// 主要用于支持 open-skills 生态的技能加载。
///
/// # 参数
///
/// - `path`：Markdown 技能文件的完整路径
/// - `load_mode`：加载模式
///
/// # 返回值
///
/// - `Ok(Skill)`：成功构建的技能对象
/// - `Err`：文件读取失败
///
/// # 特殊标记
///
/// 开放技能使用以下标记：
/// - `version`：`"open-skills"`（标识来源）
/// - `author`：`"besoeasy/open-skills"`
/// - `tags`：包含 `"open-skills"` 标签
pub(crate) fn load_open_skill_md(path: &Path, load_mode: SkillLoadMode) -> Result<Skill> {
    // 使用文件名（不含扩展名）作为技能名称
    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("open-skill").to_string();
    let content = std::fs::read_to_string(path)?;
    let metadata = parse_markdown_skill_metadata(&content);
    let description = metadata.description.unwrap_or_else(|| "No description".to_string());
    let prompts = match load_mode {
        SkillLoadMode::Full => vec![content],
        SkillLoadMode::MetadataOnly => Vec::new(),
    };

    Ok(Skill {
        name,
        description,
        version: "open-skills".to_string(),
        author: Some("besoeasy/open-skills".to_string()),
        tags: vec!["open-skills".to_string()],
        tools: Vec::new(),
        prompts,
        location: Some(path.to_path_buf()),
    })
}

/// 从 Markdown 内容中提取描述
///
/// 查找第一个非空且非标题的行作为技能描述。
///
/// # 参数
///
/// - `content`：Markdown 文件的完整内容
///
/// # 返回值
///
/// 返回提取到的描述字符串。如果找不到有效描述，返回 `"No description"`。
fn extract_description(content: &str) -> String {
    parse_markdown_skill_metadata(content)
        .description
        .unwrap_or_else(|| "No description".to_string())
}

/// 从 Markdown 文件中流式提取描述
///
/// 与 `extract_description` 功能相同，但使用流式读取避免加载整个文件到内存。
/// 适用于仅需要描述信息的元数据加载场景。
///
/// # 参数
///
/// - `path`：Markdown 文件路径
///
/// # 返回值
///
/// - `Ok(String)`：提取到的描述字符串
/// - `Err`：文件打开或读取失败
///
/// # 性能
///
/// 找到第一个有效描述行后立即返回，不会读取文件的剩余部分。
fn extract_description_from_markdown(path: &Path) -> Result<String> {
    Ok(read_markdown_skill_metadata(path)?
        .description
        .unwrap_or_else(|| "No description".to_string()))
}
#[cfg(test)]
#[path = "loader_tests.rs"]
mod loader_tests;
