//! 工作区解析测试模块
//!
//! 本模块包含用于验证 `resolve_openclaw_workspace` 函数行为的测试用例。
//! 主要测试在不同环境变量配置下，默认工作区路径的解析逻辑：
//!
//! - 当 `HOME` 环境变量可用时，应优先使用 `~/.openclaw/workspace` 路径
//! - 当 `HOME` 环境变量不可用时，应回退到配置范围内的 `.openclaw/workspace` 路径

use super::super::*;
use super::super::{SourceWorkspace, SourceWorkspaceKind};
use super::{env_lock, test_config};
use tempfile::TempDir;

/// 测试当 `HOME` 环境变量可用时，默认工作区应优先使用 `HOME` 环境变量
///
/// # 测试场景
/// 1. 设置一个伪造的 `HOME` 目录路径
/// 2. 清除 `USERPROFILE` 环境变量（Windows 用户目录）
/// 3. 调用 `resolve_openclaw_workspace` 解析工作区
///
/// # 预期结果
/// - 工作区类型应为 `SourceWorkspaceKind::Default`
/// - 工作区路径应为 `$HOME/.openclaw/workspace`
#[test]
fn default_source_prefers_home_env_when_available() {
    // 获取环境变量锁，确保测试期间环境变量不会被其他测试干扰
    let _env_guard = env_lock();

    // 创建临时目录作为测试的配置范围
    let target = TempDir::new().unwrap();

    // 在临时目录下创建伪造的 HOME 目录
    let fake_home = target.path().join("fake-home");
    std::fs::create_dir_all(&fake_home).unwrap();

    // 设置 HOME 环境变量为伪造路径
    let _home_guard = super::EnvGuard::set("HOME", Some(fake_home.to_str().unwrap()));

    // 清除 USERPROFILE 环境变量（避免在非 Windows 系统上产生干扰）
    let _userprofile_guard = super::EnvGuard::set("USERPROFILE", None);

    // 使用临时目录路径创建测试配置
    let config = test_config(target.path());

    // 在未指定显式源路径的情况下解析工作区
    let source = resolve_openclaw_workspace(&config, None);

    // 验证：工作区类型应为 Default
    assert_eq!(source.kind, SourceWorkspaceKind::Default);

    // 验证：工作区路径应基于伪造的 HOME 目录
    assert_eq!(source.path, fake_home.join(".openclaw").join("workspace"));
}

/// 测试当 `HOME` 环境变量不可用时，默认工作区应使用配置范围路径
///
/// # 测试场景
/// 1. 清除 `HOME` 和 `USERPROFILE` 环境变量
/// 2. 调用 `resolve_openclaw_workspace` 解析工作区
///
/// # 预期结果
/// - 工作区类型应为 `SourceWorkspaceKind::Default`
/// - 工作区路径应为配置目录下的 `.openclaw/workspace`
#[test]
fn default_source_is_workspace_scoped_without_home_env() {
    // 获取环境变量锁，确保测试期间环境变量不会被其他测试干扰
    let _env_guard = env_lock();

    // 创建临时目录作为测试的配置范围
    let target = TempDir::new().unwrap();

    // 清除 HOME 环境变量
    let _home_guard = super::EnvGuard::set("HOME", None);

    // 清除 USERPROFILE 环境变量
    let _userprofile_guard = super::EnvGuard::set("USERPROFILE", None);

    // 使用临时目录路径创建测试配置
    let config = test_config(target.path());

    // 在未指定显式源路径的情况下解析工作区
    let source = resolve_openclaw_workspace(&config, None);

    // 验证：工作区类型应为 Default
    assert_eq!(source.kind, SourceWorkspaceKind::Default);

    // 验证：工作区路径应相对于配置范围目录
    assert_eq!(source.path, target.path().join(".openclaw").join("workspace"));
}
