//! LsTool 单元测试模块
//!
//! 本模块包含 LsTool 的各种单元测试，用于验证文件列表功能：
//! - 工具名称和参数 schema 的正确性
//! - 基本文件列表功能
//! - 忽略模式（ignore patterns）的过滤功能
//!
//! # 测试依赖
//!
//! - `tempfile`: 用于创建临时测试目录
//! - `SecurityPolicy`: 安全策略配置，控制工作空间访问权限
//! - `LsTool`: 被测试的文件列表工具

use super::*;

/// 测试模块
///
/// 包含 LsTool 的所有单元测试用例
#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
    use tempfile::TempDir;

    /// 创建测试用的安全策略
    ///
    /// # 参数
    ///
    /// - `workspace`: 工作空间目录路径，测试将在此目录下执行
    ///
    /// # 返回值
    ///
    /// 返回一个 `Arc<SecurityPolicy>`，配置为：
    /// - 自主级别：Supervised（监督模式）
    /// - 工作空间目录：指定的 workspace 路径
    /// - 其他配置：使用默认值
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let policy = test_security(PathBuf::from("/tmp/test"));
    /// let tool = LsTool::new(policy);
    /// ```
    fn test_security(workspace: PathBuf) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            // 使用监督模式，限制自动执行权限
            autonomy: AutonomyLevel::Supervised,
            // 设置工作空间目录
            workspace_dir: workspace,
            // 其他配置使用默认值
            ..SecurityPolicy::default()
        })
    }

    /// 测试 LsTool 的名称和参数 schema
    ///
    /// 验证：
    /// - 工具名称是否为 "ls"
    /// - 参数 schema 中是否包含 "path" 属性（对象类型）
    /// - 参数 schema 中是否包含 "ignore" 属性（对象类型）
    #[test]
    fn ls_name_and_schema() {
        // 创建 LsTool 实例，使用系统临时目录作为工作空间
        let tool = LsTool::new(test_security(std::env::temp_dir()));

        // 验证工具名称
        assert_eq!(tool.name(), "ls");
        assert!(tool.description().contains("List files and directories at the given path"));

        // 获取参数 schema
        let schema = tool.parameters_schema();

        // 验证 path 参数存在于 schema 中且为对象类型
        assert!(schema["properties"]["path"].is_object());

        // 验证 ignore 参数存在于 schema 中且为对象类型
        assert!(schema["properties"]["ignore"].is_object());
    }

    /// 测试 LsTool 的基本文件列表功能
    ///
    /// 测试场景：
    /// 1. 创建临时目录结构：
    ///    - 根目录下创建 a.txt 文件
    ///    - 创建子目录 sub
    ///    - 在 sub 目录下创建 b.txt 文件
    /// 2. 执行 ls 命令（无参数）
    /// 3. 验证输出包含：
    ///    - a.txt（根目录文件）
    ///    - sub/（子目录）
    ///    - b.txt（子目录中的文件）
    #[tokio::test]
    async fn ls_lists_files() {
        // 创建临时测试目录
        let dir = TempDir::new().unwrap();

        // 创建测试目录结构
        // 创建子目录 sub
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        // 在根目录创建 a.txt 文件
        std::fs::write(dir.path().join("a.txt"), "").unwrap();
        // 在 sub 目录下创建 b.txt 文件
        std::fs::write(dir.path().join("sub/b.txt"), "").unwrap();

        // 创建 LsTool 实例并执行
        let tool = LsTool::new(test_security(dir.path().to_path_buf()));
        let result = tool.execute(json!({})).await.unwrap();

        // 验证执行成功
        assert!(result.success, "error: {:?}", result.error);

        // 验证输出中包含根目录文件 a.txt
        assert!(result.output.contains("a.txt"));

        // 验证输出中包含子目录 sub/（带斜杠后缀表示目录）
        assert!(result.output.contains("sub/"));

        // 验证输出中包含子目录中的文件 b.txt
        assert!(result.output.contains("b.txt"));
    }

    /// 测试 LsTool 的忽略模式（ignore patterns）功能
    ///
    /// 测试场景：
    /// 1. 创建临时目录并添加两个文件：
    ///    - a.rs（Rust 源文件）
    ///    - a.txt（文本文件）
    /// 2. 使用 ignore 参数过滤 *.txt 文件
    /// 3. 验证输出：
    ///    - 应该包含 a.rs
    ///    - 不应该包含 a.txt（被忽略）
    ///
    /// 这测试了 LsTool 的文件过滤能力，允许用户指定要忽略的文件模式
    #[tokio::test]
    async fn ls_supports_ignore_patterns() {
        // 创建临时测试目录
        let dir = TempDir::new().unwrap();

        // 创建不同类型的测试文件
        // 创建 Rust 源文件
        std::fs::write(dir.path().join("a.rs"), "").unwrap();
        // 创建文本文件（将被忽略）
        std::fs::write(dir.path().join("a.txt"), "").unwrap();

        // 创建 LsTool 实例并执行，传入 ignore 参数过滤 *.txt 文件
        let tool = LsTool::new(test_security(dir.path().to_path_buf()));
        let result = tool.execute(json!({ "ignore": ["*.txt"] })).await.unwrap();

        // 验证执行成功
        assert!(result.success, "error: {:?}", result.error);

        // 验证输出中包含 a.rs 文件（未被忽略）
        assert!(result.output.contains("a.rs"));

        // 验证输出中不包含 a.txt 文件（被忽略模式过滤）
        assert!(!result.output.contains("a.txt"));
    }

    #[tokio::test]
    async fn ls_rejects_non_array_ignore_parameter() {
        let tool = LsTool::new(test_security(std::env::temp_dir()));

        let result = tool.execute(json!({ "ignore": "*.txt" })).await;
        let Err(error) = result else {
            panic!("non-array ignore parameter should be rejected");
        };

        let message = error.to_string();
        assert!(message.contains("Missing or invalid parameters"));
        assert!(message.contains("invalid type"));
    }

    #[tokio::test]
    async fn ls_reports_missing_search_directory_without_listing_files() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("missing");
        let tool = LsTool::new(test_security(dir.path().to_path_buf()));

        let result = tool.execute(json!({ "path": missing.to_string_lossy() })).await.unwrap();

        assert!(!result.success);
        assert!(result.output.is_empty());
        assert!(result.error.as_deref().is_some_and(|error| error.contains("does not exist")));
    }

    #[tokio::test]
    async fn ls_lists_relative_subdirectory_after_resolving_workspace_path() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("sub/nested")).unwrap();
        std::fs::write(dir.path().join("sub/nested/inside.txt"), "").unwrap();
        std::fs::write(dir.path().join("root.txt"), "").unwrap();
        let tool = LsTool::new(test_security(dir.path().to_path_buf()));

        let result = tool.execute(json!({ "path": "sub" })).await.unwrap();

        assert!(result.success, "error: {:?}", result.error);
        let resolved_sub = std::fs::canonicalize(dir.path().join("sub")).unwrap();
        assert!(result.output.starts_with(&format!("{}/", resolved_sub.display())));
        assert!(result.output.contains("nested/"));
        assert!(result.output.contains("inside.txt"));
        assert!(!result.output.contains("root.txt"));
    }

    #[tokio::test]
    async fn ls_rejects_relative_path_blocked_by_security_policy() {
        let dir = TempDir::new().unwrap();
        let tool = LsTool::new(test_security(dir.path().to_path_buf()));

        let result = tool.execute(json!({ "path": "../outside" })).await.unwrap();

        assert!(!result.success);
        assert!(result.output.is_empty());
        assert!(
            result
                .error
                .as_deref()
                .is_some_and(|error| error.contains("Path not allowed by security policy"))
        );
    }

    #[tokio::test]
    async fn ls_rejects_existing_file_search_path() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, "").unwrap();
        let tool = LsTool::new(test_security(dir.path().to_path_buf()));

        let result = tool.execute(json!({ "path": file.to_string_lossy() })).await.unwrap();

        assert!(!result.success);
        assert!(result.output.is_empty());
        assert!(
            result
                .error
                .as_deref()
                .is_some_and(|error| error.contains("Search path is not a directory"))
        );
    }

    #[tokio::test]
    async fn ls_reports_invalid_custom_ignore_pattern() {
        let dir = TempDir::new().unwrap();
        let tool = LsTool::new(test_security(dir.path().to_path_buf()));

        let result = tool.execute(json!({ "ignore": ["["] })).await.unwrap();

        assert!(!result.success);
        assert!(result.output.is_empty());
        assert!(
            result
                .error
                .as_deref()
                .is_some_and(|error| error.contains("Invalid ignore pattern '['"))
        );
    }
}
