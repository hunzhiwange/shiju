//! 技能系统初始化功能测试模块
//!
//! 本模块包含对技能目录初始化相关功能的单元测试，主要验证：
//! - 技能目录及其默认文件的创建
//! - 初始化操作的幂等性
//! - 技能目录路径的正确性计算

use super::super::*;
use std::path::Path;

/// 测试初始化技能目录功能是否正确创建所有必需文件
///
/// 验证点：
/// - skills 目录下必须创建 README.md 文件
/// - 必须创建 create-task-plans / do-task-plans / rustcodegraph 子目录及对应的 SKILL.md 文件
/// - 必须创建 .download-policy.toml 配置文件
#[test]
fn init_skills_creates_readme() {
    // 创建临时目录用于测试
    let dir = tempfile::tempdir().unwrap();

    // 执行技能目录初始化
    init_skills_dir(dir.path()).unwrap();

    // 验证所有必需文件和目录是否创建成功
    assert!(dir.path().join("skills").join("README.md").exists());
    assert!(dir.path().join("skills").join("create-task-plans").join("SKILL.md").exists());
    assert!(dir.path().join("skills").join("do-task-plans").join("SKILL.md").exists());
    assert!(dir.path().join("skills").join("rustcodegraph").join("SKILL.md").exists());
    assert!(dir.path().join("skills").join(".download-policy.toml").exists());
}

/// 测试技能目录初始化操作的幂等性
///
/// 验证点：
/// - 多次调用 init_skills_dir 不应导致错误
/// - 重复初始化后，所有必需文件仍然存在且正确
/// - 确保初始化操作可以安全地重复执行
#[test]
fn init_skills_idempotent() {
    // 创建临时目录用于测试
    let dir = tempfile::tempdir().unwrap();

    // 第一次初始化
    init_skills_dir(dir.path()).unwrap();

    // 第二次初始化（测试幂等性）
    init_skills_dir(dir.path()).unwrap();

    // 验证所有必需文件在重复初始化后仍然存在
    assert!(dir.path().join("skills").join("README.md").exists());
    assert!(dir.path().join("skills").join("create-task-plans").join("SKILL.md").exists());
    assert!(dir.path().join("skills").join("do-task-plans").join("SKILL.md").exists());
    assert!(dir.path().join("skills").join("rustcodegraph").join("SKILL.md").exists());
}

/// 测试技能目录路径的计算功能
///
/// 验证点：
/// - 给定基础路径时，skills_dir 函数应正确返回 skills 子目录路径
/// - 路径拼接应符合预期格式：<base_path>/skills
#[test]
fn skills_dir_path() {
    // 定义测试用的基础路径
    let base = std::path::Path::new("/home/user/.vibewindow");

    // 计算技能目录路径
    let dir = skills_dir(base);

    // 验证返回路径是否正确
    assert_eq!(dir, PathBuf::from("/home/user/.vibewindow/skills"));
}
