//! 技能初始化与加载测试模块
//!
//! 本模块包含技能系统初始化和加载功能的单元测试，验证以下核心行为：
//! - 技能目录初始化时创建默认文件结构
//! - 初始化操作的幂等性（多次调用不会破坏现有文件）
//! - Compact 加载模式下仅加载元数据（不加载提示词和工具定义）
//!
//! 这些测试确保技能系统的基础功能正确可靠。

use super::super::*;
use std::fs;

/// 测试技能目录初始化创建默认文件结构
///
/// 验证 `init_skills_dir` 函数在首次调用时创建以下文件：
/// - `skills/README.md` - 技能目录说明文档
/// - `skills/create-task-plans/SKILL.md` - 创建实施计划技能
/// - `skills/do-task-plans/SKILL.md` - 执行实施计划技能
/// - `skills/rustcodegraph/SKILL.md` - Rust 代码图导航技能
/// - `skills/.download-policy.toml` - 下载策略配置文件
///
/// # 测试逻辑
/// 1. 创建临时目录作为工作区
/// 2. 调用初始化函数创建技能目录结构
/// 3. 断言所有预期的文件都已创建
#[test]
fn init_skills_creates_readme() {
    // 创建临时工作区目录
    let dir = tempfile::tempdir().unwrap();

    // 执行技能目录初始化
    init_skills_dir(dir.path()).unwrap();

    // 验证 README.md 文件已创建
    assert!(dir.path().join("skills").join("README.md").exists());
    assert!(dir.path().join("skills").join("create-task-plans").join("SKILL.md").exists());
    assert!(dir.path().join("skills").join("do-task-plans").join("SKILL.md").exists());
    assert!(dir.path().join("skills").join("rustcodegraph").join("SKILL.md").exists());
    // 验证下载策略配置文件已创建
    assert!(dir.path().join("skills").join(".download-policy.toml").exists());
}

/// 测试技能目录初始化的幂等性
///
/// 验证多次调用 `init_skills_dir` 不会破坏或重复创建已存在的文件。
/// 这是重要的健壮性保证，确保用户可以安全地多次运行初始化命令。
///
/// # 测试逻辑
/// 1. 创建临时工作区目录
/// 2. 连续两次调用初始化函数
/// 3. 验证第二次调用后所有文件仍然存在且未损坏
///
/// # 预期行为
/// - 第二次调用不应报错
/// - 所有默认文件应保持存在
#[test]
fn init_skills_idempotent() {
    // 创建临时工作区目录
    let dir = tempfile::tempdir().unwrap();

    // 首次初始化技能目录
    init_skills_dir(dir.path()).unwrap();
    // 再次初始化，验证幂等性
    init_skills_dir(dir.path()).unwrap();

    // 验证 README.md 文件仍然存在
    assert!(dir.path().join("skills").join("README.md").exists());
    assert!(dir.path().join("skills").join("create-task-plans").join("SKILL.md").exists());
    assert!(dir.path().join("skills").join("do-task-plans").join("SKILL.md").exists());
    assert!(dir.path().join("skills").join("rustcodegraph").join("SKILL.md").exists());
}

/// 测试 Compact 模式下仅加载技能元数据
///
/// 验证当配置为 `SkillsPromptInjectionMode::Compact` 时，
/// `load_skills_with_config` 函数仅加载技能的元数据（名称、描述），
/// 而不加载提示词（prompts）和工具定义（tools）。
///
/// # 背景
/// Compact 模式用于减少运行时内存占用和提示词注入体积，
/// 适用于只需要技能元信息的场景（如技能列表展示）。
///
/// # 测试数据
/// 创建两个测试技能：
/// 1. **md-meta**: Markdown 格式技能，包含元数据行和提示词内容
/// 2. **toml-meta**: TOML 格式技能，包含工具定义和提示词
///
/// # 测试逻辑
/// 1. 创建临时技能目录
/// 2. 创建两个包含完整定义的测试技能（含工具和提示词）
/// 3. 配置为 Compact 加载模式
/// 4. 加载技能并按名称排序
/// 5. 验证只加载了元数据，工具和提示词为空
///
/// # 预期结果
/// - 两个技能都被加载
/// - 每个技能的 description 字段正确填充
/// - 每个技能的 prompts 和 tools 字段为空集合
#[test]
fn load_skills_with_config_compact_mode_uses_metadata_only() {
    // 创建临时工作区目录
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");

    // 创建 Markdown 格式的测试技能
    // 第一行 "# Metadata" 被识别为标题，第二行作为描述提取
    let md_skill = skills_dir.join("md-meta");
    fs::create_dir_all(&md_skill).unwrap();
    fs::write(
        md_skill.join("SKILL.md"),
        "# Metadata\nMetadata summary line\nUse this only when needed.\n",
    )
    .unwrap();

    // 创建 TOML 格式的测试技能
    // 包含完整的技能定义：元数据、工具和提示词
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

    // 配置运行时参数，设置为 Compact 模式
    let mut config = crate::app::agent::config::Config::default();
    config.workspace_dir = dir.path().to_path_buf();
    config.skills.prompt_injection_mode =
        crate::app::agent::config::SkillsPromptInjectionMode::Compact;

    // 加载技能（Compact 模式）
    let mut skills = load_skills_with_config(dir.path(), &config);
    // 按名称排序以确保测试断言顺序一致
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    // 本用例只验证工作区内两个技能的 Compact 加载形状。
    assert!(skills.iter().any(|skill| skill.name == "md-meta"));
    assert!(skills.iter().any(|skill| skill.name == "toml-meta"));

    // 验证 Markdown 技能的元数据
    let md = skills.iter().find(|skill| skill.name == "md-meta").unwrap();
    assert_eq!(md.description, "Metadata summary line");
    // Compact 模式下不应加载提示词
    assert!(md.prompts.is_empty());
    // Compact 模式下不应加载工具定义
    assert!(md.tools.is_empty());

    // 验证 TOML 技能的元数据
    let toml = skills.iter().find(|skill| skill.name == "toml-meta").unwrap();
    assert_eq!(toml.description, "Toml metadata description");
    // Compact 模式下不应加载提示词
    assert!(toml.prompts.is_empty());
    // Compact 模式下不应加载工具定义（即使工具定义完整）
    assert!(toml.tools.is_empty());
}
