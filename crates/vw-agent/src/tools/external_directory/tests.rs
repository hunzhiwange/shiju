//! # 外部目录访问测试模块
//!
//! 本模块用于测试工具对外部目录访问的安全策略和边界检查功能。
//!
//! ## 主要功能
//!
//! - 测试空目标路径的处理逻辑
//! - 测试绕过模式对目录检查的影响
//! - 测试工作区内相对路径的访问权限
//! - 测试工作区外路径的拒绝逻辑（未在允许列表中）
//! - 测试允许列表中的路径访问权限
//!
//! ## 测试覆盖
//!
//! - 安全策略配置验证
//! - 目录边界检查
//! - 路径规范化处理
//! - 错误消息格式验证

use super::super::*;
use super::{Kind, Options, assert_external_directory, normalize_path, resolve_full_path};
use crate::app::agent::security::AutonomyLevel;
use std::path::PathBuf;

/// 创建用于测试的安全策略配置
///
/// # 参数
///
/// - `workspace`: 工作区目录路径，作为策略的工作区根目录
///
/// # 返回值
///
/// 返回配置好的 `SecurityPolicy` 实例，特性如下：
/// - 自主级别设置为 `Supervised`（受监督模式）
/// - 工作区目录设置为传入的 `workspace` 参数
/// - 其他配置使用默认值
///
/// # 用途
///
/// 该函数为大多数测试用例提供统一的策略配置基础，
/// 确保测试环境的一致性和可重复性。
fn test_policy(workspace: PathBuf) -> SecurityPolicy {
    SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: workspace,
        ..SecurityPolicy::default()
    }
}

#[test]
fn default_options_enforce_file_checks() {
    let options = Options::default();

    assert!(!options.bypass);
    assert_eq!(options.kind, Kind::File);
}

#[test]
fn relative_paths_are_trimmed_joined_and_normalized() {
    let workspace = PathBuf::from(std::path::MAIN_SEPARATOR_STR).join("workspace");
    let policy = test_policy(workspace.clone());

    let resolved = resolve_full_path(&policy, "  nested/../target.txt  ");

    assert_eq!(normalize_path(&resolved), workspace.join("target.txt"));

    let absolute = PathBuf::from(std::path::MAIN_SEPARATOR_STR).join("external").join("data.txt");
    assert_eq!(resolve_full_path(&policy, &format!("  {}  ", absolute.display())), absolute);
}

/// 测试空目标路径是否返回成功
///
/// # 测试场景
///
/// 1. 目标为 `None`（未指定）
/// 2. 目标为空字符串 `""`
/// 3. 目标为纯空白字符串 `"   "`
///
/// # 预期行为
///
/// 所有空或空白路径应该返回 `Ok(())`，表示无需执行目录检查。
/// 这是合理的边界条件处理，因为空路径不涉及实际的文件系统访问。
#[tokio::test]
async fn empty_target_is_ok() {
    let policy = test_policy(std::env::temp_dir());
    assert!(assert_external_directory(&policy, None, None).await.is_ok());
    assert!(assert_external_directory(&policy, Some(""), None).await.is_ok());
    assert!(assert_external_directory(&policy, Some("   "), None).await.is_ok());
}

/// 测试绕过模式跳过目录检查
///
/// # 测试场景
///
/// 使用 `bypass: true` 选项访问任意路径（示例中使用 `/tmp/somewhere`）
///
/// # 预期行为
///
/// 当 `Options.bypass` 设置为 `true` 时，应跳过所有目录边界检查，
/// 即使路径在工作区之外也应返回成功。
///
/// # 用途
///
/// 验证绕过机制的可用性，该机制通常用于特权操作或管理员场景。
#[tokio::test]
async fn bypass_skips_checks() {
    let policy = test_policy(std::env::temp_dir());
    let options = Options { bypass: true, kind: Kind::Directory };
    assert!(
        assert_external_directory(&policy, Some("/tmp/somewhere"), Some(options)).await.is_ok()
    );
}

/// 测试工作区内相对路径的访问权限
///
/// # 测试场景
///
/// 使用临时目录作为工作区，测试访问嵌套的相对路径 `"nested/dir"`
///
/// # 预期行为
///
/// 相对于工作区目录的路径应该被允许访问，返回成功结果。
/// 这是正常工作流程中最常见的使用场景。
///
/// # 实现
///
/// 使用 `tempfile::TempDir` 创建临时工作区以确保测试隔离性。
#[tokio::test]
async fn workspace_relative_path_is_allowed() {
    let root = tempfile::TempDir::new().unwrap();
    let policy = test_policy(root.path().to_path_buf());
    assert!(assert_external_directory(&policy, Some("nested/dir"), None).await.is_ok());
}

/// 测试工作区外路径在无允许列表时被拒绝
///
/// # 测试场景
///
/// 1. 创建临时工作区目录
/// 2. 构造一个位于工作区父目录下的外部路径
/// 3. 使用不包含允许列表的策略尝试访问该路径
///
/// # 预期行为
///
/// 访问应该被拒绝，并返回包含 "workspace allowlist" 或 "allowed_roots"
/// 关键字的错误消息。
///
/// # 安全意义
///
/// 验证默认的安全边界：除非明确配置允许列表，否则不允许访问
/// 工作区之外的目录，防止路径遍历攻击。
#[tokio::test]
async fn outside_workspace_is_denied_without_allowlist() {
    // 创建临时工作区
    let root = tempfile::TempDir::new().unwrap();
    let policy = test_policy(root.path().to_path_buf());

    // 构造工作区外的外部路径（位于父目录下）
    let outside = root.path().parent().unwrap().join("external_dir_for_test");

    // 尝试访问外部路径，应返回错误
    let err = assert_external_directory(
        &policy,
        Some(outside.to_string_lossy().as_ref()),
        Some(Options { bypass: false, kind: Kind::Directory }),
    )
    .await
    .unwrap_err();

    // 验证错误消息包含预期的关键词
    assert!(err.contains("workspace allowlist") || err.contains("allowed_roots"));
}

#[tokio::test]
async fn outside_file_denial_reports_parent_directory() {
    let root = tempfile::TempDir::new().unwrap();
    let policy = test_policy(root.path().to_path_buf());
    let outside_parent = root.path().parent().unwrap().join("external_file_parent");
    let outside_file = outside_parent.join("secret.txt");

    let err = assert_external_directory(
        &policy,
        Some(outside_file.to_string_lossy().as_ref()),
        Some(Options { bypass: false, kind: Kind::File }),
    )
    .await
    .unwrap_err();

    assert!(err.contains(outside_parent.to_string_lossy().as_ref()));
    assert!(!err.contains("secret.txt"));
}

#[tokio::test]
async fn outside_file_denial_reports_original_path_when_parent_is_missing() {
    let root = tempfile::TempDir::new().unwrap();
    let policy = test_policy(root.path().to_path_buf());
    let filesystem_root = PathBuf::from(std::path::MAIN_SEPARATOR_STR);

    let err = assert_external_directory(
        &policy,
        Some(filesystem_root.to_string_lossy().as_ref()),
        Some(Options { bypass: false, kind: Kind::File }),
    )
    .await
    .unwrap_err();

    assert!(err.contains(filesystem_root.to_string_lossy().as_ref()));
}

/// 测试允许列表中的路径访问权限
///
/// # 测试场景
///
/// 1. 创建两个独立的临时目录：工作区目录和允许目录
/// 2. 配置策略，将允许目录添加到 `allowed_roots` 列表
/// 3. 尝试访问允许目录中的路径
///
/// # 预期行为
///
/// 虽然允许目录不在工作区内，但由于它被添加到 `allowed_roots` 列表，
/// 访问应该被允许并返回成功。
///
/// # 用途
///
/// 验证 `allowed_roots` 配置项的功能，该配置允许管理员
/// 授权访问特定的外部目录，以满足跨目录协作的需求。
#[tokio::test]
async fn allowed_roots_path_is_allowed() {
    // 创建临时工作区目录
    let root = tempfile::TempDir::new().unwrap();

    // 创建独立的允许目录（不在工作区内）
    let allowed = tempfile::TempDir::new().unwrap();

    // 配置策略：指定工作区和允许列表
    let policy = SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: root.path().to_path_buf(),
        allowed_roots: vec![allowed.path().to_path_buf()],
        ..SecurityPolicy::default()
    };

    // 验证允许列表中的路径可以被访问
    assert!(
        assert_external_directory(
            &policy,
            Some(allowed.path().to_string_lossy().as_ref()),
            Some(Options { bypass: false, kind: Kind::Directory }),
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn tool_output_directory_is_allowed_outside_workspace() {
    let root = tempfile::TempDir::new().unwrap();
    let policy = test_policy(root.path().to_path_buf());
    let tool_output_file =
        crate::app::agent::global::paths().data.join("tool-output").join("result.txt");

    assert!(
        assert_external_directory(
            &policy,
            Some(tool_output_file.to_string_lossy().as_ref()),
            Some(Options { bypass: false, kind: Kind::File }),
        )
        .await
        .is_ok()
    );
}
