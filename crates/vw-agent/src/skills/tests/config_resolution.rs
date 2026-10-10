//! # 技能配置解析测试模块
//!
//! 本模块提供技能配置解析功能的单元测试，主要验证以下内容：
//! - Open Skills 功能开关的解析逻辑（环境变量 > 配置文件 > 默认值）
//! - Open Skills 目录路径的解析逻辑（环境变量 > 配置文件 > 用户主目录）
//! - 从本地配置加载技能的功能（无需网络请求）
//!
//! ## 测试策略
//!
//! 1. **配置优先级测试**：验证多个配置源的正确优先级顺序
//! 2. **边界条件测试**：验证空白值、无效值等边界情况
//! 3. **集成测试**：验证端到端的技能加载流程

use super::super::open_skills::{
    open_skills_enabled_from_sources, resolve_open_skills_dir_from_sources,
};
use super::super::*;
use super::helpers::{EnvVarGuard, open_skills_env_lock};
use std::fs;
use std::path::Path;

/// 测试 Open Skills 功能开关的解析优先级
///
/// 验证 `open_skills_enabled_from_sources` 函数按照以下优先级解析配置：
/// 1. **环境变量**（最高优先级）：`VIBEWINDOW_OPEN_SKILLS_ENABLED`
/// 2. **配置文件**：`config.skills.open_skills_enabled`
/// 3. **默认值**：`false`（最低优先级）
///
/// ## 测试用例
///
/// - `None, None` → 默认值 `false`
/// - `Some(true), None` → 配置文件启用 → `true`
/// - `Some(true), Some("0")` → 环境变量禁用覆盖配置 → `false`
/// - `Some(false), Some("yes")` → 环境变量启用覆盖配置 → `true`
/// - `Some(true), Some("invalid")` → 无效环境值仍解析为 `true`
/// - `Some(false), Some("invalid")` → 无效环境值仍解析为 `false`
///
/// ## 配置优先级规则
///
/// 环境变量始终优先于配置文件，即使环境变量值无效也会被解析。
#[test]
fn open_skills_enabled_resolution_prefers_env_then_config_then_default_false() {
    // 无任何配置源时，应使用默认值 false
    assert!(!open_skills_enabled_from_sources(None, None));

    // 仅配置文件启用，环境变量未设置 → 启用
    assert!(open_skills_enabled_from_sources(Some(true), None));

    // 环境变量禁用（"0"）覆盖配置文件启用 → 禁用
    assert!(!open_skills_enabled_from_sources(Some(true), Some("0")));

    // 环境变量启用（"yes"）覆盖配置文件禁用 → 启用
    assert!(open_skills_enabled_from_sources(Some(false), Some("yes")));

    // 无效环境变量值（"invalid"）按布尔解析规则处理
    assert!(open_skills_enabled_from_sources(Some(true), Some("invalid")));
    assert!(!open_skills_enabled_from_sources(Some(false), Some("invalid")));
}

/// 测试 Open Skills 目录路径的解析优先级
///
/// 验证 `resolve_open_skills_dir_from_sources` 函数按照以下优先级解析目录路径：
/// 1. **环境变量**（最高优先级）：`VIBEWINDOW_OPEN_SKILLS_DIR`
/// 2. **配置文件**：`config.skills.open_skills_dir`
/// 3. **用户主目录**（最低优先级）：`{home}/open-skills`
///
/// ## 测试用例
///
/// - 环境变量设置有效路径 → 返回环境变量路径
/// - 环境变量为空白（仅空格）→ 回退到配置文件路径
/// - 所有配置源都未设置 → 回退到用户主目录下的默认路径
/// - 无任何配置源且无主目录 → 返回 `None`
///
/// ## 返回值
///
/// - `Some(PathBuf)`：成功解析到的技能目录路径
/// - `None`：无法解析任何有效路径
#[test]
fn resolve_open_skills_dir_resolution_prefers_env_then_config_then_home() {
    // 准备测试用的主目录路径
    let home = Path::new("/tmp/home-dir");

    // 环境变量优先：环境变量、配置文件、主目录都设置时，优先使用环境变量
    assert_eq!(
        resolve_open_skills_dir_from_sources(
            Some("/tmp/env-skills"), // 环境变量路径
            Some("/tmp/config"),     // 配置文件路径
            Some(home)               // 用户主目录
        ),
        Some(PathBuf::from("/tmp/env-skills"))
    );

    // 配置文件优先：环境变量为空白字符串时，回退到配置文件
    assert_eq!(
        resolve_open_skills_dir_from_sources(Some("   "), Some("/tmp/config-skills"), Some(home)),
        Some(PathBuf::from("/tmp/config-skills"))
    );

    // 主目录默认值：环境变量和配置文件都未设置时，使用主目录下的 open-skills 子目录
    assert_eq!(
        resolve_open_skills_dir_from_sources(None, None, Some(home)),
        Some(PathBuf::from("/tmp/home-dir/open-skills"))
    );

    // 完全无配置：所有配置源都缺失时，返回 None
    assert_eq!(resolve_open_skills_dir_from_sources(None, None, None), None);
}

/// 测试从本地配置加载技能的功能（离线模式）
///
/// 验证 `load_skills_with_config` 函数能够：
/// 1. 从本地文件系统读取 Open Skills 配置
/// 2. 无需网络请求即可加载技能
/// 3. 正确解析技能目录结构并过滤非技能文件
///
/// ## 测试场景
///
/// 创建一个临时的 Open Skills 目录结构：
/// ```text
/// open-skills-local/
/// ├── README.md                  # 非技能文件（应被忽略）
/// ├── CONTRIBUTING.md            # 非技能文件（应被忽略）
/// └── skills/
///     └── http_request/
///         └── SKILL.md           # 技能定义文件
/// ```
///
/// ## 验证点
///
/// - 成功加载 1 个技能（`http_request`）
/// - 非技能文件（如 `README.md`、`CONTRIBUTING.md`）被正确过滤
/// - 技能元数据（名称）解析正确
///
/// ## 环境隔离
///
/// - 使用 `open_skills_env_lock` 确保测试间不相互干扰
/// - 清除可能影响测试的环境变量
/// - 使用临时目录，测试后自动清理
#[test]
fn explicit_open_skills_loader_reads_local_dir_without_network() {
    // 获取环境锁，确保此测试独占访问环境变量
    let _env_guard = open_skills_env_lock().lock().unwrap();

    // 清除可能干扰测试的环境变量
    let _enabled_guard = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_ENABLED");
    let _dir_guard = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_DIR");

    // 创建临时目录结构
    let dir = tempfile::tempdir().unwrap();
    let workspace_dir = dir.path().join("workspace");
    fs::create_dir_all(workspace_dir.join("skills")).unwrap();

    // 创建 Open Skills 目录并添加测试技能
    let open_skills_dir = dir.path().join("open-skills-local");
    fs::create_dir_all(open_skills_dir.join("skills/http_request")).unwrap();

    // 添加非技能文件（应被技能加载器忽略）
    fs::write(open_skills_dir.join("README.md"), "# open skills\n").unwrap();
    fs::write(open_skills_dir.join("CONTRIBUTING.md"), "# contribution guide\n").unwrap();

    // 添加技能定义文件
    fs::write(
        open_skills_dir.join("skills/http_request/SKILL.md"),
        "# HTTP request\nFetch API responses.\n",
    )
    .unwrap();

    // 执行技能加载
    let skills = load_skills_with_open_skills_config(
        &workspace_dir,
        Some(true),
        open_skills_dir.to_str(),
        types::SkillLoadMode::Full,
    );

    // 验证：应成功加载 1 个技能
    assert_eq!(skills.len(), 1);
    // 验证：技能名称正确
    assert_eq!(skills[0].name, "http_request");
    // 验证：非技能文件（如 CONTRIBUTING.md）未被误识别为技能
    assert_ne!(skills[0].name, "CONTRIBUTING");
}
