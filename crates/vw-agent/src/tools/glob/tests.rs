//! glob 工具的单元测试模块
//!
//! 本模块包含对 `GlobTool` 的全面测试，覆盖以下场景：
//! - 工具名称和参数 schema 验证
//! - 基本文件模式匹配
//! - 相对路径参数处理
//! - 未找到文件的返回消息
//! - 路径安全边界验证（拒绝访问工作区外的路径）
//! - 速率限制功能
//!
//! 所有测试均使用临时目录进行隔离，确保测试之间互不干扰。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
    use tempfile::TempDir;

    /// 创建用于测试的默认安全策略
    ///
    /// # 参数
    ///
    /// - `workspace`: 工作区目录路径，所有文件操作将被限制在此目录内
    ///
    /// # 返回值
    ///
    /// 返回一个 `Arc<SecurityPolicy>`，配置如下：
    /// - 自主级别为 `Supervised`（受监督模式）
    /// - 工作区目录设置为传入的 `workspace`
    /// - 其他字段使用默认值
    fn test_security(workspace: PathBuf) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace,
            ..SecurityPolicy::default()
        })
    }

    /// 创建带有速率限制的测试安全策略
    ///
    /// # 参数
    ///
    /// - `workspace`: 工作区目录路径
    /// - `max_actions_per_hour`: 每小时允许的最大操作次数，设为 0 可触发速率限制
    ///
    /// # 返回值
    ///
    /// 返回一个 `Arc<SecurityPolicy>`，除基本配置外还包含自定义的速率限制设置。
    fn test_security_with_limit(
        workspace: PathBuf,
        max_actions_per_hour: u32,
    ) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace,
            max_actions_per_hour,
            ..SecurityPolicy::default()
        })
    }

    /// 测试工具名称和参数 schema 的正确性
    ///
    /// 验证内容：
    /// - 工具名称应为 `"glob"`
    /// - 参数 schema 应包含 `pattern` 和 `path` 两个属性
    #[test]
    fn glob_name_and_schema() {
        let tool = GlobTool::new(test_security(std::env::temp_dir()));
        assert_eq!(tool.name(), "glob");

        let schema = tool.parameters_schema();
        assert!(schema["properties"]["pattern"].is_object());
        assert!(schema["properties"]["path"].is_object());
    }

    /// 测试基本文件模式匹配功能
    ///
    /// 场景：
    /// 1. 创建包含 `a.txt` 和 `b.rs` 两个文件的临时目录
    /// 2. 使用 `*.txt` 模式进行搜索
    ///
    /// 验证内容：
    /// - 操作应成功
    /// - 结果应包含 `a.txt`
    /// - 结果不应包含 `b.rs`（因为扩展名不匹配）
    #[tokio::test]
    async fn glob_finds_matches() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "").unwrap();
        std::fs::write(dir.path().join("b.rs"), "").unwrap();

        let tool = GlobTool::new(test_security(dir.path().to_path_buf()));
        let result = tool.execute(json!({ "pattern": "*.txt" })).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("a.txt"));
        assert!(!result.output.contains("b.rs"));
    }

    /// 测试相对路径参数的使用
    ///
    /// 场景：
    /// 1. 创建包含子目录 `sub` 的临时目录
    /// 2. 在 `sub` 目录下创建 `inside.md` 文件
    /// 3. 指定 `path: "sub"` 和 `pattern: "*.md"` 进行搜索
    ///
    /// 验证内容：
    /// - 操作应成功
    /// - 结果应包含 `inside.md`
    #[tokio::test]
    async fn glob_uses_relative_path_argument() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/inside.md"), "").unwrap();

        let tool = GlobTool::new(test_security(dir.path().to_path_buf()));
        let result = tool.execute(json!({ "pattern": "*.md", "path": "sub" })).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("inside.md"));
    }

    #[tokio::test]
    async fn glob_accepts_cwd_alias() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/inside.md"), "").unwrap();

        let tool = GlobTool::new(test_security(dir.path().to_path_buf()));
        let result = tool.execute(json!({ "pattern": "*.md", "cwd": "sub" })).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("inside.md"));
    }

    /// 测试未找到匹配文件时的返回消息
    ///
    /// 场景：
    /// 1. 创建一个空的临时目录
    /// 2. 使用 `*.missing` 模式搜索（该模式不会匹配任何文件）
    ///
    /// 验证内容：
    /// - 操作应成功（未找到文件不是错误）
    /// - 输出应为中文提示 `"未找到文件"`
    #[tokio::test]
    async fn glob_returns_not_found_message() {
        let dir = TempDir::new().unwrap();
        let tool = GlobTool::new(test_security(dir.path().to_path_buf()));
        let result = tool.execute(json!({ "pattern": "*.missing" })).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output, "未找到文件");
    }

    #[tokio::test]
    async fn glob_rejects_blank_pattern() {
        let dir = TempDir::new().unwrap();
        let tool = GlobTool::new(test_security(dir.path().to_path_buf()));

        let result = tool.execute(json!({ "pattern": "  " })).await.unwrap();

        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("Missing pattern"));
    }

    #[tokio::test]
    async fn glob_rejects_missing_search_path() {
        let dir = TempDir::new().unwrap();
        let tool = GlobTool::new(test_security(dir.path().to_path_buf()));

        let result = tool.execute(json!({ "pattern": "*", "path": "missing" })).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or_default().contains("does not exist"));
    }

    #[tokio::test]
    async fn glob_rejects_file_as_search_path() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("not-a-directory.txt"), "content").unwrap();
        let tool = GlobTool::new(test_security(dir.path().to_path_buf()));

        let result =
            tool.execute(json!({ "pattern": "*", "path": "not-a-directory.txt" })).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or_default().contains("not a directory"));
    }

    /// 测试对非法路径的拒绝（安全边界验证）
    ///
    /// 场景：
    /// 1. 创建临时目录作为受限工作区
    /// 2. 尝试使用 `path: "../../etc"` 访问工作区外的目录
    ///
    /// 验证内容：
    /// - 操作应失败（`success` 为 `false`）
    /// - 错误消息应包含 `"Path not allowed"`，表示路径被安全策略拒绝
    ///
    /// 安全意义：
    /// 此测试确保工具不会允许通过路径遍历攻击访问工作区外的敏感文件。
    #[tokio::test]
    async fn glob_rejects_forbidden_path() {
        let dir = TempDir::new().unwrap();
        let tool = GlobTool::new(test_security(dir.path().to_path_buf()));
        let result = tool.execute(json!({ "pattern": "*", "path": "../../etc" })).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("Path not allowed"));
    }

    /// 测试速率限制功能
    ///
    /// 场景：
    /// 1. 创建临时目录
    /// 2. 创建安全策略，将 `max_actions_per_hour` 设为 0（立即触发限制）
    /// 3. 执行 glob 操作
    ///
    /// 验证内容：
    /// - 操作应失败（`success` 为 `false`）
    /// - 错误消息应包含 `"Rate limit"`，表示触发了速率限制
    ///
    /// 设计意义：
    /// 速率限制可防止工具被滥用导致资源耗尽。
    #[tokio::test]
    async fn glob_rate_limited() {
        let dir = TempDir::new().unwrap();
        let tool = GlobTool::new(test_security_with_limit(dir.path().to_path_buf(), 0));
        let result = tool.execute(json!({ "pattern": "*" })).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("Rate limit"));
    }
}
