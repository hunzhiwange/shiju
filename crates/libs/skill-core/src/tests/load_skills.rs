//! 技能加载功能测试模块
//!
//! 本模块包含技能加载系统的完整测试套件，验证从不同来源（TOML、Markdown）
//! 加载技能的各种场景，包括正常加载、错误处理、优先级规则等。
//!
//! # 测试覆盖范围
//!
//! - 空目录加载
//! - 从 TOML 清单文件加载技能
//! - 从 Markdown 文件加载技能
//! - 紧凑模式下的元数据加载
//! - 不存在目录的错误处理
//! - 无效技能目录的过滤
//! - 多技能批量加载
//! - TOML 与 Markdown 的优先级规则

use super::*;
use std::fs;

/// 测试从空技能目录加载
///
/// 验证当技能目录为空时，加载函数应返回空列表而不会发生错误。
/// 这是技能加载的基础边界情况测试。
#[test]
fn load_empty_skills_dir() {
    // 创建临时空目录作为工作空间
    let dir = tempfile::tempdir().unwrap();
    // 尝试从空目录加载技能
    let skills = load_skills(dir.path());
    // 断言：应返回空的技能列表
    assert!(skills.is_empty());
}

/// 测试从 TOML 清单文件加载技能
///
/// 验证系统能够正确解析 SKILL.toml 文件并提取技能信息，
/// 包括技能元数据（名称、描述、版本、标签）和工具定义。
///
/// # 测试场景
///
/// - 创建包含完整配置的 TOML 文件
/// - 验证技能名称和描述正确加载
/// - 验证工具定义正确解析
#[test]
fn load_skill_from_toml() {
    // 创建临时目录
    let dir = tempfile::tempdir().unwrap();
    // 构建技能目录结构：<workspace>/skills/test-skill/
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("test-skill");
    fs::create_dir_all(&skill_dir).unwrap();

    // 创建有效的 SKILL.toml 配置文件
    // 包含技能元数据和一个 shell 类型工具
    fs::write(
        skill_dir.join("SKILL.toml"),
        r#"
[skill]
name = "test-skill"
description = "A test skill"
version = "1.0.0"
tags = ["test"]

[[tools]]
name = "hello"
description = "Says hello"
kind = "shell"
command = "echo hello"
"#,
    )
    .unwrap();

    // 执行技能加载
    let skills = load_skills(dir.path());

    // 验证技能列表长度
    assert_eq!(skills.len(), 1);
    // 验证技能元数据正确加载
    assert_eq!(skills[0].name, "test-skill");
    // 验证工具列表正确解析
    assert_eq!(skills[0].tools.len(), 1);
    assert_eq!(skills[0].tools[0].name, "hello");
}

/// 测试从 Markdown 文件加载技能
///
/// 验证系统能够正确解析 SKILL.md 文件，从中提取技能名称
/// （使用目录名）和描述（使用文件内容）。
///
/// # Markdown 技能解析规则
///
/// - 技能名称取自目录名，而非 Markdown 标题
/// - 描述从 Markdown 文件内容中提取
#[test]
fn load_skill_from_md() {
    // 创建临时目录和技能目录结构
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("md-skill");
    fs::create_dir_all(&skill_dir).unwrap();

    // 创建 Markdown 格式的技能描述文件
    fs::write(skill_dir.join("SKILL.md"), "# My Skill\nThis skill does cool things.\n").unwrap();

    // 执行技能加载
    let skills = load_skills(dir.path());

    // 验证技能数量
    assert_eq!(skills.len(), 1);
    // 验证技能名称来自目录名（md-skill）
    assert_eq!(skills[0].name, "md-skill");
    // 验证描述从 Markdown 内容中提取
    assert!(skills[0].description.contains("cool things"));
}

/// 测试紧凑模式下的技能加载行为
///
/// 验证当配置为 Compact 模式时，系统仅加载技能的元数据信息，
/// 不预加载提示词（prompts）和工具（tools）定义。
///
/// # 紧凑模式设计目标
///
/// - 减少初始加载时的内存占用
/// - 延迟加载完整技能定义直到实际需要时
/// - 适用于技能数量较多的场景
///
/// # 测试覆盖
///
/// - Markdown 技能的紧凑加载
/// - TOML 技能的紧凑加载
/// - 验证工具和提示词不被预加载
#[test]
fn load_skills_with_config_compact_mode_uses_metadata_only() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");

    // 创建 Markdown 技能（md-meta）
    let md_skill = skills_dir.join("md-meta");
    fs::create_dir_all(&md_skill).unwrap();
    fs::write(
        md_skill.join("SKILL.md"),
        "# Metadata\nMetadata summary line\nUse this only when needed.\n",
    )
    .unwrap();

    // 创建包含工具和提示词的 TOML 技能（toml-meta）
    // 在紧凑模式下，这些工具和提示词不应被预加载
    let toml_skill = skills_dir.join("toml-meta");
    fs::create_dir_all(&toml_skill).unwrap();
    fs::write(
        toml_skill.join("SKILL.toml"),
        r#"
[skill]
name = "toml-meta"
description = "Toml metadata description"
version = "1.2.3"

[[tools]]
name = "dangerous-tool"
description = "Should not preload"
kind = "shell"
command = "echo no"

prompts = ["Do not preload me"]
"#,
    )
    .unwrap();

    // 配置紧凑模式
    let mut config = SkillRuntimeConfig::default();
    config.workspace_dir = dir.path().to_path_buf();
    config.skills.prompt_injection_mode = SkillsPromptInjectionMode::Compact;

    // 执行紧凑模式加载
    let mut skills = load_skills_with_config(dir.path(), &config);
    // 按名称排序以确保测试稳定性
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    // 验证加载了两个技能
    assert_eq!(skills.len(), 2);

    // 验证 Markdown 技能的紧凑加载
    // - 描述仅包含摘要行
    // - 提示词和工具列表为空
    let md = skills.iter().find(|skill| skill.name == "md-meta").unwrap();
    assert_eq!(md.description, "Metadata summary line");
    assert!(md.prompts.is_empty());
    assert!(md.tools.is_empty());

    // 验证 TOML 技能的紧凑加载
    // - 描述来自 TOML 元数据
    // - 提示词和工具列表为空（未预加载）
    let toml = skills.iter().find(|skill| skill.name == "toml-meta").unwrap();
    assert_eq!(toml.description, "Toml metadata description");
    assert!(toml.prompts.is_empty());
    assert!(toml.tools.is_empty());
}

/// 测试从不存在的目录加载技能
///
/// 验证当指定的技能目录不存在时，加载函数应优雅地返回空列表，
/// 而不是抛出错误或 panic。
///
/// # 错误处理策略
///
/// - 不存在的目录视为有效的空结果场景
/// - 避免因路径问题中断整个加载流程
#[test]
fn load_nonexistent_dir() {
    // 创建临时目录作为参考点
    let dir = tempfile::tempdir().unwrap();
    // 构建不存在的路径
    let fake = dir.path().join("nonexistent");
    // 尝试从不存在的路径加载
    let skills = load_skills(&fake);
    // 断言：应返回空列表而非错误
    assert!(skills.is_empty());
}

/// 测试忽略技能目录中的非清单文件
///
/// 验证加载函数仅处理符合清单文件命名规范的文件（SKILL.toml 或 SKILL.md），
/// 而忽略其他类型的文件。
///
/// # 文件过滤规则
///
/// - 仅识别 SKILL.toml 和 SKILL.md 作为有效的清单文件
/// - 其他文件（如 .txt、.json）被忽略
/// - 确保不会因误读无效文件而失败
#[test]
fn load_ignores_files_in_skills_dir() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();
    // 在技能目录中创建非清单文件
    fs::write(skills_dir.join("not-a-skill.txt"), "hello").unwrap();
    // 执行加载
    let skills = load_skills(dir.path());
    // 断言：非清单文件应被忽略，返回空列表
    assert!(skills.is_empty());
}

/// 测试忽略没有清单文件的目录
///
/// 验证加载函数仅将包含有效清单文件（SKILL.toml 或 SKILL.md）的目录
/// 识别为技能目录，而忽略空目录或缺少清单文件的目录。
///
/// # 技能目录识别规则
///
/// - 目录必须包含 SKILL.toml 或 SKILL.md 文件
/// - 空目录不会被识别为技能
/// - 确保技能目录结构的有效性
#[test]
fn load_ignores_dir_without_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    // 创建没有清单文件的目录（模拟不完整的技能）
    let empty_skill = skills_dir.join("empty-skill");
    fs::create_dir_all(&empty_skill).unwrap();
    // 执行加载
    let skills = load_skills(dir.path());
    // 断言：没有清单的目录应被忽略
    assert!(skills.is_empty());
}

/// 测试批量加载多个技能
///
/// 验证系统能够正确扫描并加载技能目录下的所有技能，
/// 确保多技能加载的完整性和正确性。
///
/// # 测试场景
///
/// - 创建多个独立的技能目录
/// - 每个技能使用 Markdown 格式定义
/// - 验证所有技能均被正确加载
#[test]
fn load_multiple_skills() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");

    // 创建三个不同的技能（alpha、beta、gamma）
    for name in ["alpha", "beta", "gamma"] {
        let skill_dir = skills_dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), format!("# {name}\nSkill {name} description.\n"))
            .unwrap();
    }

    // 执行批量加载
    let skills = load_skills(dir.path());
    // 断言：应加载所有三个技能
    assert_eq!(skills.len(), 3);
}

/// 测试 TOML 清单文件的优先级规则
///
/// 验证当技能目录同时包含 SKILL.toml 和 SKILL.md 文件时，
/// 系统优先加载 TOML 格式的清单文件。
///
/// # 清单文件优先级
///
/// - TOML 格式（SKILL.toml）优先于 Markdown 格式（SKILL.md）
/// - 当两者同时存在时，忽略 Markdown 文件
/// - 这种设计允许用 TOML 提供更精确的配置覆盖
#[test]
fn toml_prefers_over_md() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("dual");
    fs::create_dir_all(&skill_dir).unwrap();

    // 同时创建 TOML 和 Markdown 清单文件
    // TOML 文件应优先被加载
    fs::write(
        skill_dir.join("SKILL.toml"),
        "[skill]\nname = \"from-toml\"\ndescription = \"TOML wins\"\n",
    )
    .unwrap();
    fs::write(skill_dir.join("SKILL.md"), "# From MD\nMD description\n").unwrap();

    // 执行加载
    let skills = load_skills(dir.path());
    // 断言：应只加载一个技能（TOML 版本）
    assert_eq!(skills.len(), 1);
    // 验证名称来自 TOML 文件
    assert_eq!(skills[0].name, "from-toml");
}
