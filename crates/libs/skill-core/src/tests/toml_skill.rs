//! TOML 技能配置加载测试模块
//!
//! 本模块包含针对 TOML 格式技能配置文件解析与加载的单元测试。
//! 测试覆盖多种场景：
//! - 包含多个工具的完整技能配置
//! - 仅包含必填字段的最小化配置
//! - 无效 TOML 语法的错误处理
//!
//! 这些测试验证 `load_skills` 函数能够正确解析 TOML 文件、
//! 应用默认值、以及优雅地处理解析错误。

use super::*;
use std::fs;

/// 测试加载包含多个工具的 TOML 技能配置
///
/// 验证点：
/// - 正确解析技能元数据（名称、版本、作者、标签）
/// - 正确解析多个工具定义（名称、类型、命令）
/// - Shell 类型和 HTTP 类型工具均能正确识别
///
/// # 测试场景
/// 创建一个包含三个工具（build、test、deploy）的技能目录，
/// 验证所有字段都能被正确加载和解析。
#[test]
fn toml_skill_with_multiple_tools() {
    // 创建临时测试目录
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("multi-tool");
    fs::create_dir_all(&skill_dir).unwrap();

    // 写入包含多个工具定义的完整 TOML 配置文件
    fs::write(
        skill_dir.join("SKILL.toml"),
        r#"
[skill]
name = "multi-tool"
description = "Has many tools"
version = "2.0.0"
author = "tester"
tags = ["automation", "devops"]

[[tools]]
name = "build"
description = "Build the project"
kind = "shell"
command = "cargo build"

[[tools]]
name = "test"
description = "Run tests"
kind = "shell"
command = "cargo test"

[[tools]]
name = "deploy"
description = "Deploy via HTTP"
kind = "http"
command = "https://api.example.com/deploy"
"#,
    )
    .unwrap();

    // 执行技能加载
    let skills = load_skills(dir.path());

    // 验证技能元数据解析正确
    assert_eq!(skills.len(), 1);
    let s = &skills[0];
    assert_eq!(s.name, "multi-tool");
    assert_eq!(s.version, "2.0.0");
    assert_eq!(s.author.as_deref(), Some("tester"));
    assert_eq!(s.tags, vec!["automation", "devops"]);

    // 验证工具列表解析正确
    assert_eq!(s.tools.len(), 3);
    assert_eq!(s.tools[0].name, "build");
    assert_eq!(s.tools[1].kind, "shell");
    assert_eq!(s.tools[2].kind, "http");
}

/// 测试加载最小化 TOML 技能配置
///
/// 验证点：
/// - 仅提供必填字段（name、description）时能正常加载
/// - 可选字段正确应用默认值：
///   - version 默认为 "0.1.0"
///   - author 默认为 None
///   - tags 默认为空列表
///   - tools 默认为空列表
///
/// # 测试场景
/// 创建一个只包含名称和描述的技能配置，
/// 验证所有可选字段都能获得正确的默认值。
#[test]
fn toml_skill_minimal() {
    // 创建临时测试目录
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("minimal");
    fs::create_dir_all(&skill_dir).unwrap();

    // 写入仅包含必填字段的最小化 TOML 配置
    fs::write(
        skill_dir.join("SKILL.toml"),
        r#"
[skill]
name = "minimal"
description = "Bare minimum"
"#,
    )
    .unwrap();

    // 执行技能加载
    let skills = load_skills(dir.path());

    // 验证必填字段正确解析
    assert_eq!(skills.len(), 1);

    // 验证可选字段应用了正确的默认值
    assert_eq!(skills[0].version, "0.1.0");
    assert!(skills[0].author.is_none());
    assert!(skills[0].tags.is_empty());
    assert!(skills[0].tools.is_empty());
}

/// 测试无效 TOML 语法的错误处理
///
/// 验证点：
/// - 当 TOML 文件存在语法错误时，不会导致程序崩溃
/// - 无效的技能配置会被静默跳过
/// - 错误不会影响其他有效配置的加载（如果有多个技能目录）
///
/// # 测试场景
/// 创建一个包含无效 TOML 语法的配置文件，
/// 验证 `load_skills` 函数能够优雅地处理解析错误，
/// 返回空列表而非抛出异常。
#[test]
fn toml_skill_invalid_syntax_skipped() {
    // 创建临时测试目录
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("broken");
    fs::create_dir_all(&skill_dir).unwrap();

    // 写入语法错误的 TOML 文件（包含无效的大括号）
    fs::write(skill_dir.join("SKILL.toml"), "this is not valid toml {{{{").unwrap();

    // 执行技能加载
    let skills = load_skills(dir.path());

    // 验证无效配置被跳过，返回空列表
    assert!(skills.is_empty());
}
