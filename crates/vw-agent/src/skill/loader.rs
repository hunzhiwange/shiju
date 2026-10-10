//! 技能加载器模块
//!
//! 本模块提供技能（Skill）的发现、审计和加载功能，是 VibeWindow 技能系统的核心组件。
//!
//! # 主要功能
//!
//! - **技能发现**：自动扫描工作区中的技能包
//! - **格式支持**：支持 TOML 格式（`SKILL.toml`）和 Markdown 格式（`SKILL.md`）
//! - **安全审计**：在加载前对技能目录和文件进行安全审计
//! - **加载模式**：支持完整加载和仅元数据加载两种模式
//!
//! # 加载策略
//!
//! - 对于工作区技能：优先加载 `SKILL.toml`，如不存在则尝试 `SKILL.md`
//! - 所有技能在加载前必须通过安全审计

use crate::app::agent::skill::audit;
use crate::app::agent::skill::types::{
    Skill, SkillLoadMode, SkillManifest, SkillMetadataManifest, SkillRuntimeConfig,
};
use anyhow::Result;
use std::io::BufRead;
use std::path::Path;
use vw_config_types::skills::SkillSecurityConfig;

/// 从工作区技能目录加载所有技能
///
/// 这是最简单的技能加载入口，使用默认配置加载工作区中的所有技能。
///
/// # 参数
///
/// - `workspace_dir`: 工作区根目录路径
///
/// # 返回值
///
/// 返回加载成功的技能列表。加载失败的技能会被跳过并记录警告日志。
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
/// let skills = load_skills(Path::new("/path/to/workspace"));
/// for skill in skills {
///     println!("Loaded skill: {}", skill.name);
/// }
/// ```
pub fn load_skills(workspace_dir: &Path) -> Vec<Skill> {
    load_workspace_skills(workspace_dir, SkillLoadMode::Full, &SkillSecurityConfig::default())
}

/// 使用运行时配置加载技能（运行时推荐使用此方法）
///
/// 这是运行时加载技能的推荐方法，从配置中读取提示注入模式。
///
/// # 参数
///
/// - `workspace_dir`: 工作区根目录路径
/// - `config`: 技能运行时配置
///
/// # 返回值
///
/// 返回根据配置加载的技能列表。
///
pub fn load_skills_with_config(workspace_dir: &Path, config: &SkillRuntimeConfig) -> Vec<Skill> {
    load_workspace_skills(
        workspace_dir,
        SkillLoadMode::from_prompt_mode(config.skills.prompt_injection_mode),
        &SkillSecurityConfig::default(),
    )
}

/// 使用运行时配置加载完整的技能内容（内部使用）
///
/// 与 `load_skills_with_config` 类似，但强制使用完整加载模式（Full），
/// 忽略配置中的提示注入模式设置。
///
/// # 参数
///
/// - `workspace_dir`: 工作区根目录路径
/// - `config`: 技能运行时配置
///
/// # 返回值
///
/// 返回完整加载的技能列表，包含所有提示和工具定义。
///
/// # 使用场景
///
/// 适用于需要完整技能内容而不仅仅是元数据的内部操作。
pub(crate) fn load_skills_full_with_config(
    workspace_dir: &Path,
    _config: &SkillRuntimeConfig,
) -> Vec<Skill> {
    load_workspace_skills(workspace_dir, SkillLoadMode::Full, &SkillSecurityConfig::default())
}

/// 从工作区加载技能
///
/// 扫描工作区下的 `skills` 目录，加载其中所有通过安全审计的技能包。
///
/// # 参数
///
/// - `workspace_dir`: 工作区根目录路径
/// - `load_mode`: 加载模式
///
/// # 返回值
///
/// 返回工作区中成功加载的技能列表。
fn load_workspace_skills(
    workspace_dir: &Path,
    load_mode: SkillLoadMode,
    security: &SkillSecurityConfig,
) -> Vec<Skill> {
    // 构建技能目录路径：<workspace>/skills/
    let skills_dir = workspace_dir.join("skills");
    load_skills_from_directory(&skills_dir, load_mode, security)
}

/// 从指定目录加载所有技能
///
/// 扫描目录中的子文件夹，每个子文件夹代表一个技能包。
/// 每个技能包必须通过安全审计，然后尝试加载其清单文件。
///
/// # 参数
///
/// - `skills_dir`: 技能目录路径
/// - `load_mode`: 加载模式
///
/// # 返回值
///
/// 返回成功加载的技能列表。不符合安全要求的技能会被跳过并记录警告。
///
/// # 清单文件优先级
///
/// 1. `SKILL.toml`（优先）
/// 2. `SKILL.md`（备用）
fn load_skills_from_directory(
    skills_dir: &Path,
    load_mode: SkillLoadMode,
    security: &SkillSecurityConfig,
) -> Vec<Skill> {
    // 如果技能目录不存在，直接返回空列表
    if !skills_dir.exists() {
        return Vec::new();
    }

    let mut skills = Vec::new();

    // 尝试读取目录内容，失败则返回空列表
    let Ok(entries) = std::fs::read_dir(skills_dir) else {
        return skills;
    };

    // 遍历目录中的每个条目
    for entry in entries.flatten() {
        let path = entry.path();
        // 跳过非目录条目（技能必须是目录形式）
        if !path.is_dir() {
            continue;
        }

        // 对技能目录进行安全审计
        match audit::audit_skill_directory_with_security(&path, security) {
            // 审计通过（干净）则继续加载
            Ok(report) if report.is_clean() => {}
            // 审计发现安全问题，跳过该技能
            Ok(report) => {
                tracing::warn!(
                    "skipping insecure skill directory {}: {}",
                    path.display(),
                    report.summary()
                );
                continue;
            }
            // 审计过程出错，跳过该技能
            Err(err) => {
                tracing::warn!("skipping unauditable skill directory {}: {err}", path.display());
                continue;
            }
        }

        // 优先尝试 SKILL.toml，然后尝试 SKILL.md
        let manifest_path = path.join("SKILL.toml");
        let md_path = path.join("SKILL.md");

        if manifest_path.exists() {
            // 加载 TOML 格式的技能清单
            if let Ok(skill) = load_skill_toml(&manifest_path, load_mode) {
                skills.push(skill);
            }
        } else if md_path.exists() {
            // 加载 Markdown 格式的技能文件
            if let Ok(skill) = load_skill_md(&md_path, &path, load_mode) {
                skills.push(skill);
            }
        }
    }

    skills
}

/// 从 SKILL.toml 清单文件加载技能
///
/// 解析 TOML 格式的技能清单文件，根据加载模式决定是否包含工具和提示内容。
///
/// # 参数
///
/// - `path`: SKILL.toml 文件的路径
/// - `load_mode`: 加载模式
///   - `Full`: 完整加载，包含工具和提示
///   - `MetadataOnly`: 仅加载元数据（名称、描述、版本等）
///
/// # 返回值
///
/// - `Ok(Skill)`: 成功解析的技能对象
/// - `Err`: 文件读取失败或 TOML 解析失败
///
/// # TOML 格式要求
///
/// 必须包含 `[skill]` 段，可选包含 `[[tools]]` 和 `[[prompts]]` 段。
fn load_skill_toml(path: &Path, load_mode: SkillLoadMode) -> Result<Skill> {
    // 读取文件内容
    let content = std::fs::read_to_string(path)?;

    match load_mode {
        // 完整模式：解析完整的技能清单（包含工具和提示）
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
        // 元数据模式：仅解析技能元数据（不包含工具和提示）
        SkillLoadMode::MetadataOnly => {
            let manifest: SkillMetadataManifest = toml::from_str(&content)?;

            Ok(Skill {
                name: manifest.skill.name,
                description: manifest.skill.description,
                version: manifest.skill.version,
                author: manifest.skill.author,
                tags: manifest.skill.tags,
                tools: Vec::new(),   // 不加载工具
                prompts: Vec::new(), // 不加载提示
                location: Some(path.to_path_buf()),
            })
        }
    }
}

/// 从 SKILL.md 文件加载技能（简化格式）
///
/// 解析 Markdown 格式的技能文件，这是一种更简单的技能定义方式。
/// 技能名称从所在目录名推断，版本和作者使用默认值。
///
/// # 参数
///
/// - `path`: SKILL.md 文件的路径
/// - `dir`: 技能目录路径（用于推断技能名称）
/// - `load_mode`: 加载模式
///   - `Full`: 完整加载，读取整个 Markdown 文件内容作为提示
///   - `MetadataOnly`: 仅提取描述信息
///
/// # 返回值
///
/// - `Ok(Skill)`: 成功创建的技能对象
/// - `Err`: 文件读取失败
///
/// # 默认值
///
/// - 版本：`"0.1.0"`
/// - 作者：`None`
/// - 标签：空列表
/// - 工具：空列表
fn load_skill_md(path: &Path, dir: &Path, load_mode: SkillLoadMode) -> Result<Skill> {
    // 从目录名推断技能名称
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

    let (description, prompts) = match load_mode {
        // 完整模式：读取整个文件内容，提取描述并保留全文作为提示
        SkillLoadMode::Full => {
            let content = std::fs::read_to_string(path)?;
            (extract_description(&content), vec![content])
        }
        // 元数据模式：仅提取描述，不加载提示内容
        SkillLoadMode::MetadataOnly => (extract_description_from_markdown(path)?, Vec::new()),
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

/// 从 Markdown 内容中提取描述
///
/// 查找第一个非标题、非空的文本行作为技能描述。
/// 这是对已加载到内存中的文本内容的快速提取方法。
///
/// # 参数
///
/// - `content`: Markdown 文件的文本内容
///
/// # 返回值
///
/// 返回提取到的描述文本。如果找不到合适的行，返回 `"No description"`。
///
/// # 提取规则
///
/// - 跳过以 `#` 开头的标题行
/// - 跳过空行和仅包含空白字符的行
/// - 返回第一个符合条件的行（去除首尾空白）
fn extract_description(content: &str) -> String {
    content
        .lines()
        .find(|line| !line.starts_with('#') && !line.trim().is_empty())
        .unwrap_or("No description")
        .trim()
        .to_string()
}

/// 从 Markdown 文件中提取描述（流式读取）
///
/// 逐行读取 Markdown 文件，查找第一个非标题、非空的文本行作为描述。
/// 这种方式避免将整个文件加载到内存，适用于仅需要元数据的场景。
///
/// # 参数
///
/// - `path`: Markdown 文件路径
///
/// # 返回值
///
/// - `Ok(String)`: 成功提取的描述文本
/// - `Err`: 文件打开或读取失败
///
/// # 提取规则
///
/// 与 `extract_description` 相同，但使用流式读取以节省内存。
fn extract_description_from_markdown(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);

    // 逐行读取文件，直到找到合适的描述行
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        // 跳过空行和标题行
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // 找到第一个有效行，作为描述返回
        return Ok(trimmed.to_string());
    }

    // 未找到有效描述，返回默认值
    Ok("No description".to_string())
}
#[cfg(test)]
#[path = "loader_tests.rs"]
mod loader_tests;
