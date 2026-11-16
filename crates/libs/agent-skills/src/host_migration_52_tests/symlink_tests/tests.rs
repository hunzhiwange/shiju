//! 技能符号链接测试模块
//!
//! 本模块提供了技能系统符号链接功能的集成测试，主要验证以下方面：
//! - Unix 平台上的符号链接创建与读取
//! - Windows 平台上的符号链接权限处理
//! - 符号链接的边界情况处理
//! - 跨工作空间的符号链接安全性
//!
//! # 测试范围
//!
//! 1. **Unix 平台测试**
//!    - 有效符号链接的创建与验证
//!    - 悬空符号链接（指向不存在的目标）的处理
//!    - 跨工作空间的符号链接支持
//!
//! 2. **Windows 平台测试**
//!    - 需要管理员权限的符号链接创建
//!    - 权限不足时的优雅降级
//!
//! 3. **通用功能测试**
//!    - `skills_dir` 函数的边界情况
//!    - 路径处理的正确性

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::host_migration_52_tests::skills_dir;
    use std::path::Path;
    use tempfile::TempDir;

    /// 测试技能符号链接在 Unix 平台上的边界情况
    ///
    /// # 测试场景
    ///
    /// 本测试覆盖以下用例：
    /// 1. **有效符号链接创建**：验证在 Unix 系统上可以成功创建符号链接
    /// 2. **悬空符号链接**：验证指向不存在目标的符号链接可以创建，但读取时会失败
    /// 3. **Windows 平台降级**：验证 Windows 上符号链接失败时的处理
    /// 4. **`skills_dir` 函数边界**：测试带尾部斜杠的路径处理
    /// 5. **空工作空间目录**：验证空目录的场景
    ///
    /// # 平台差异
    ///
    /// - **Unix**：完整测试符号链接功能
    /// - **Windows**：测试符号链接权限限制及优雅降级
    #[tokio::test]
    async fn test_skills_symlink_unix_edge_cases() {
        // 创建临时目录和工作空间
        let tmp = TempDir::new().unwrap();
        let workspace_dir = tmp.path().join("workspace");
        tokio::fs::create_dir_all(&workspace_dir).await.unwrap();

        // 创建技能目录
        let skills_path = skills_dir(&workspace_dir);
        tokio::fs::create_dir_all(&skills_path).await.unwrap();

        // 测试用例 1：Unix 平台上有效符号链接的创建
        #[cfg(unix)]
        {
            // 准备源技能目录和文件
            let source_dir = tmp.path().join("source_skill");
            tokio::fs::create_dir_all(&source_dir).await.unwrap();
            tokio::fs::write(source_dir.join("SKILL.md"), "# Test Skill\nContent").await.unwrap();

            // 定义符号链接目标路径
            let dest_link = skills_path.join("linked_skill");

            // 创建符号链接并验证创建成功
            let result = std::os::unix::fs::symlink(&source_dir, &dest_link);
            assert!(result.is_ok(), "符号链接创建应该成功");

            // 验证符号链接存在且确实是符号链接
            assert!(dest_link.exists());
            assert!(dest_link.is_symlink());

            // 验证可以通过符号链接读取文件内容
            let content = tokio::fs::read_to_string(dest_link.join("SKILL.md")).await;
            assert!(content.is_ok());
            assert!(content.unwrap().contains("Test Skill"));

            // 测试用例 2：悬空符号链接（指向不存在的目标）应该能够优雅地处理
            let broken_link = skills_path.join("broken_skill");
            let non_existent = tmp.path().join("non_existent");
            let result = std::os::unix::fs::symlink(&non_existent, &broken_link);
            // 符号链接创建应该成功（即使目标不存在）
            assert!(result.is_ok(), "即使目标不存在，符号链接创建也应该成功");

            // 但通过悬空符号链接读取文件应该失败
            let content = tokio::fs::read_to_string(broken_link.join("SKILL.md")).await;
            assert!(content.is_err());
        }

        // 测试用例 3：非 Unix 平台应该优雅地处理符号链接错误
        #[cfg(windows)]
        {
            let source_dir = tmp.path().join("source_skill");
            tokio::fs::create_dir_all(&source_dir).await.unwrap();

            let dest_link = skills_path.join("linked_skill");

            // 在 Windows 上，创建目录符号链接可能需要提升的权限
            let result = std::os::windows::fs::symlink_dir(&source_dir, &dest_link);
            // 如果符号链接创建失败（没有权限），目录应该不存在
            if result.is_err() {
                assert!(!dest_link.exists());
            } else {
                // 如果成功，清理创建的符号链接
                let _ = tokio::fs::remove_dir(&dest_link).await;
            }
        }

        // 测试用例 4：skills_dir 函数的边界情况（带尾部斜杠的路径）
        let workspace_with_trailing_slash = format!("{}/", workspace_dir.display());
        let path_from_str = skills_dir(Path::new(&workspace_with_trailing_slash));
        assert_eq!(path_from_str, skills_path);

        // 测试用例 5：空工作空间目录
        let empty_workspace = tmp.path().join("empty");
        let empty_skills_path = skills_dir(&empty_workspace);
        // 验证技能路径是工作空间下的 skills 子目录
        assert_eq!(empty_skills_path, empty_workspace.join("skills"));
        // 验证目录不存在（因为还没有创建）
        assert!(!empty_skills_path.exists());
    }

    /// 测试技能符号链接的权限和安全性
    ///
    /// # 测试目标
    ///
    /// 验证符号链接系统在以下安全相关场景中的行为：
    /// - 指向工作空间外部目录的符号链接
    /// - 跨目录边界的技能访问
    ///
    /// # 安全策略
    ///
    /// 系统允许用户创建指向工作空间外部的符号链接，这是用户的自主责任。
    /// 此测试验证这种跨边界访问的正确性。
    ///
    /// # 平台限制
    ///
    /// 此测试仅在 Unix 平台上执行。
    #[tokio::test]
    async fn test_skills_symlink_permissions_and_safety() {
        // 创建临时目录和工作空间
        let tmp = TempDir::new().unwrap();
        let workspace_dir = tmp.path().join("workspace");
        tokio::fs::create_dir_all(&workspace_dir).await.unwrap();

        // 创建技能目录
        let skills_path = skills_dir(&workspace_dir);
        tokio::fs::create_dir_all(&skills_path).await.unwrap();

        #[cfg(unix)]
        {
            // 测试用例：指向工作空间外部的符号链接应该被允许（用户责任）
            let outside_dir = tmp.path().join("outside_skill");
            tokio::fs::create_dir_all(&outside_dir).await.unwrap();
            tokio::fs::write(outside_dir.join("SKILL.md"), "# Outside Skill\nContent")
                .await
                .unwrap();

            // 在技能目录中创建指向外部目录的符号链接
            let dest_link = skills_path.join("outside_skill");
            let result = std::os::unix::fs::symlink(&outside_dir, &dest_link);
            // 应该允许创建指向工作空间外部目录的符号链接
            assert!(result.is_ok(), "应该允许创建指向工作空间外部目录的符号链接");

            // 验证仍然可以通过符号链接读取内容
            let content = tokio::fs::read_to_string(dest_link.join("SKILL.md")).await;
            assert!(content.is_ok());
            assert!(content.unwrap().contains("Outside Skill"));
        }
    }
}
