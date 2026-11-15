//! 备份功能测试模块
//!
//! 本模块提供针对迁移备份功能的单元测试，验证备份创建行为是否符合预期。
//!
//! # 测试覆盖
//!
//! - 备份目录创建与命名规范
//! - 空目录处理的边界条件
//!
//! # 依赖
//!
//! - `tempfile`: 提供临时目录支持，用于隔离测试环境
//! - 父模块的 `backup_target_memory` 函数

use super::super::*;
use tempfile::TempDir;

/// 测试备份功能能够创建带时间戳的目录
///
/// # 测试场景
///
/// 1. 创建临时目录作为测试环境
/// 2. 在 memory 子目录下创建模拟的数据库文件
/// 3. 调用 `backup_target_memory` 执行备份
/// 4. 验证备份目录存在且包含正确的前缀
///
/// # 预期结果
///
/// - 备份操作返回 `Some(PathBuf)`，表示备份成功
/// - 备份目录实际存在于文件系统
/// - 备份目录名称包含 "openclaw-" 前缀
#[test]
fn backup_creates_timestamped_directory() {
    // 创建临时测试目录
    let tmp = TempDir::new().unwrap();

    // 构建 memory 子目录路径
    let mem_dir = tmp.path().join("memory");
    std::fs::create_dir_all(&mem_dir).unwrap();

    // 创建模拟的数据库文件
    let db_path = mem_dir.join("brain.db");
    std::fs::write(&db_path, "fake db content").unwrap();

    // 执行备份操作
    let result = backup_target_memory(tmp.path()).unwrap();
    assert!(result.is_some(), "backup should be created when files exist");

    // 验证备份目录的属性
    let backup_dir = result.unwrap();
    assert!(backup_dir.exists());
    assert!(
        backup_dir.to_string_lossy().contains("openclaw-"),
        "backup dir must contain openclaw- prefix"
    );
}

/// 测试当没有可备份文件时的处理行为
///
/// # 测试场景
///
/// 1. 创建空的临时目录
/// 2. 调用 `backup_target_memory` 尝试备份
///
/// # 预期结果
///
/// - 备份操作返回 `None`，表示没有文件需要备份
/// - 不应创建任何备份目录
#[test]
fn backup_returns_none_when_no_files() {
    // 创建空的临时测试目录
    let tmp = TempDir::new().unwrap();

    // 对空目录执行备份操作
    let result = backup_target_memory(tmp.path()).unwrap();
    assert!(result.is_none(), "backup should return None when no files to backup");
}
