//! # 文件写入工具测试模块
//!
//! 本模块提供 `FileWriteTool` 的单元测试和集成测试，覆盖以下场景：
//!
//! - **基础功能**：文件创建、内容写入、文件覆盖
//! - **目录处理**：自动创建父目录
//! - **安全边界**：路径遍历攻击防护、绝对路径阻止、符号链接转义防护
//! - **权限控制**：只读模式阻止写入、速率限制阻止写入
//! - **参数验证**：缺失参数检测、空内容处理、路径别名支持
//! - **特殊字符**：空字节注入防护
//!
//! ## 测试分类
//!
//! | 分类 | 测试用例 | 覆盖范围 |
//! |------|----------|----------|
//! | 功能验证 | `file_write_creates_file`, `file_write_overwrites_existing` | 正常写入流程 |
//! | 目录处理 | `file_write_creates_parent_dirs` | 嵌套目录创建 |
//! | 安全防护 | `file_write_blocks_path_traversal`, `file_write_blocks_absolute_path` | 路径安全检查 |
//! | 符号链接 | `file_write_blocks_symlink_escape`, `file_write_blocks_symlink_target_file` | TOCTOU 防护 |
//! | 权限控制 | `file_write_blocks_readonly_mode`, `file_write_blocks_when_rate_limited` | 策略执行 |
//! | 参数验证 | `file_write_missing_path_param`, `file_write_missing_content_param` | 输入验证 |

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
    use crate::app::agent::tools::FileSnapshot;
    use crate::app::agent::tools::ToolUseContext;
    use crate::app::agent::tools::context::scope_tool_use_context;
    use vw_api_types::tools::ToolResultContentDto;

    /// 创建用于测试的默认安全策略
    ///
    /// 构建一个带有 `Supervised`（受监督）自治级别的安全策略实例，
    /// 用于大多数常规测试场景。
    ///
    /// # 参数
    ///
    /// - `workspace`: 工作空间目录路径，所有文件操作将限制在此目录内
    ///
    /// # 返回值
    ///
    /// 返回一个 `Arc<SecurityPolicy>` 实例，包含：
    /// - `autonomy`: 设置为 `AutonomyLevel::Supervised`
    /// - `workspace_dir`: 设置为传入的 `workspace` 路径
    /// - 其他字段：使用 `SecurityPolicy::default()` 的默认值
    fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace,
            ..SecurityPolicy::default()
        })
    }

    /// 创建带有自定义参数的测试安全策略
    ///
    /// 构建一个可自定义自治级别和速率限制的安全策略实例，
    /// 用于测试权限控制和速率限制场景。
    ///
    /// # 参数
    ///
    /// - `workspace`: 工作空间目录路径
    /// - `autonomy`: 自治级别（如 `ReadOnly`、`Supervised`、`Autonomous`）
    /// - `max_actions_per_hour`: 每小时允许的最大操作次数
    ///
    /// # 返回值
    ///
    /// 返回一个 `Arc<SecurityPolicy>` 实例，包含指定的自治级别和速率限制
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 创建只读策略
    /// let policy = test_security_with(dir, AutonomyLevel::ReadOnly, 20);
    ///
    /// // 创建零速率限制策略（触发速率限制）
    /// let policy = test_security_with(dir, AutonomyLevel::Supervised, 0);
    /// ```
    fn test_security_with(
        workspace: std::path::PathBuf,
        autonomy: AutonomyLevel,
        max_actions_per_hour: u32,
    ) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy,
            workspace_dir: workspace,
            max_actions_per_hour,
            ..SecurityPolicy::default()
        })
    }

    fn seed_read_state(
        context: &Arc<ToolUseContext>,
        workspace: &std::path::Path,
        path: &str,
        content: &str,
    ) {
        let read_state = context.read_state_handle();
        read_state.lock().unwrap_or_else(|error| error.into_inner()).note_read(
            Some(workspace),
            path,
            content.len(),
            false,
            None,
            None,
            Some(FileSnapshot::from_text(content)),
        );
    }

    /// 测试工具名称返回值
    ///
    /// 验证 `FileWriteTool::name()` 方法返回正确的工具标识符 "file_write"。
    #[test]
    fn file_write_name() {
        let tool = FileWriteTool::new(test_security(std::env::temp_dir()));
        assert_eq!(tool.name(), "file_write");
    }

    /// 测试参数模式包含必需字段
    ///
    /// 验证 `FileWriteTool::parameters_schema()` 返回的 JSON Schema 包含：
    /// - `filePath` 属性：主要文件路径参数
    /// - `path` 属性：路径别名参数（向后兼容）
    /// - `content` 属性：文件内容参数
    /// - `required` 数组：包含 "path" 和 "content"
    #[test]
    fn file_write_schema_has_path_and_content() {
        let tool = FileWriteTool::new(test_security(std::env::temp_dir()));
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["filePath"].is_object());
        assert!(schema["properties"]["path"].is_object());
        assert!(schema["properties"]["content"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("path")));
        assert!(required.contains(&json!("content")));
    }

    #[test]
    fn file_write_description_is_available() {
        let tool = FileWriteTool::new(test_security(std::env::temp_dir()));
        let description = tool.description();
        assert!(description.contains("完整内容"));
        assert!(description.contains("创建新文件"));
    }

    /// 测试文件创建功能
    ///
    /// 验证 `FileWriteTool` 能够：
    /// 1. 在工作空间内创建新文件
    /// 2. 写入指定内容
    /// 3. 返回成功结果，包含 `<changes>` 标签
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时测试目录
    /// 2. 执行写入操作（`filePath: "out.txt"`, `content: "written!"`）
    /// 3. 验证返回结果成功且包含预期输出
    /// 4. 验证文件内容与写入内容一致
    /// 5. 清理测试目录
    #[tokio::test]
    async fn file_write_creates_file() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-write-create",
            Some(dir.to_string_lossy().to_string()),
        ));

        let result = scope_tool_use_context(
            context.clone(),
            tool.call(json!({"filePath": "out.txt", "content": "written!"})),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["kind"], json!("create"));
        assert_eq!(
            result.render_hint.as_ref().and_then(|hint| hint.kind.as_deref()),
            Some("file_write")
        );
        assert!(matches!(
            result.content_blocks.first(),
            Some(ToolResultContentDto::StructuredPatch { hunks }) if !hunks.is_empty()
        ));

        let content = tokio::fs::read_to_string(dir.join("out.txt")).await.unwrap();
        assert_eq!(content, "written!");

        let mut snapshot = context.read_state_snapshot();
        let entry = snapshot.get(Some(dir.as_path()), "out.txt").expect("read state missing");
        assert_eq!(entry.snapshot, Some(FileSnapshot::from_text("written!")));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_write_blocks_overwrite_without_prior_read() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_requires_read");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("out.txt"), "old").await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-write-requires-read",
            Some(dir.to_string_lossy().to_string()),
        ));

        let result = scope_tool_use_context(
            context,
            tool.call(json!({"filePath": "out.txt", "content": "new"})),
        )
        .await
        .unwrap();

        assert!(!result.is_success());
        assert!(
            result
                .error_text()
                .unwrap_or_default()
                .contains("prior file_read in the current tool context")
        );

        let content = tokio::fs::read_to_string(dir.join("out.txt")).await.unwrap();
        assert_eq!(content, "old");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试自动创建父目录功能
    ///
    /// 验证 `FileWriteTool` 在写入嵌套路径文件时，能够自动创建
    /// 所有不存在的父目录。
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时测试目录
    /// 2. 写入嵌套路径文件 `a/b/c/deep.txt`
    /// 3. 验证操作成功
    /// 4. 验证文件内容正确
    /// 5. 清理测试目录
    #[tokio::test]
    async fn file_write_creates_parent_dirs() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_nested");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let result =
            tool.execute(json!({"filePath": "a/b/c/deep.txt", "content": "deep"})).await.unwrap();
        assert!(result.success);

        // 验证嵌套路径文件被正确创建
        let content = tokio::fs::read_to_string(dir.join("a/b/c/deep.txt")).await.unwrap();
        assert_eq!(content, "deep");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试覆盖已存在文件功能
    ///
    /// 验证 `FileWriteTool` 能够：
    /// 1. 检测文件已存在
    /// 2. 正确覆盖文件内容
    /// 3. 返回带有 "M"（Modified）标记的输出
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时测试目录
    /// 2. 预先创建包含 "old" 内容的文件
    /// 3. 执行写入操作覆盖内容为 "new"
    /// 4. 验证操作成功且输出包含修改标记
    /// 5. 验证文件内容已被更新
    /// 6. 清理测试目录
    #[tokio::test]
    async fn file_write_overwrites_existing() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_overwrite");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("exist.txt"), "old").await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-write-overwrite",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "exist.txt", "old");

        let result = scope_tool_use_context(
            context.clone(),
            tool.call(json!({"filePath": "exist.txt", "content": "new"})),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["kind"], json!("update"));
        assert!(matches!(
            result.content_blocks.first(),
            Some(ToolResultContentDto::StructuredPatch { hunks }) if !hunks.is_empty()
        ));

        let content = tokio::fs::read_to_string(dir.join("exist.txt")).await.unwrap();
        assert_eq!(content, "new");

        let mut snapshot = context.read_state_snapshot();
        let entry = snapshot.get(Some(dir.as_path()), "exist.txt").expect("read state missing");
        assert_eq!(entry.snapshot, Some(FileSnapshot::from_text("new")));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_write_reports_unchanged_existing_content() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_unchanged");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("same.txt"), "same").await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-write-unchanged",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "same.txt", "same");

        let result = scope_tool_use_context(
            context,
            tool.call(json!({"filePath": "same.txt", "content": "same"})),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["kind"], json!("update"));
        assert_eq!(result.render_hint.as_ref().unwrap().metadata["changed"], json!(false));
        assert!(matches!(
            result.content_blocks.first(),
            Some(ToolResultContentDto::StructuredPatch { hunks }) if hunks.is_empty()
        ));
        assert_eq!(tokio::fs::read_to_string(dir.join("same.txt")).await.unwrap(), "same");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_write_blocks_stale_overwrite() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_stale_overwrite");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("exist.txt"), "old").await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-write-stale-overwrite",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "exist.txt", "old");
        tokio::fs::write(dir.join("exist.txt"), "newer").await.unwrap();

        let result = scope_tool_use_context(
            context,
            tool.call(json!({"filePath": "exist.txt", "content": "final"})),
        )
        .await
        .unwrap();

        assert!(!result.is_success());
        assert!(
            result.error_text().unwrap_or_default().contains("changed since the last file_read")
        );
        let content = tokio::fs::read_to_string(dir.join("exist.txt")).await.unwrap();
        assert_eq!(content, "newer");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试路径遍历攻击防护
    ///
    /// 验证 `FileWriteTool` 能够阻止试图通过 `../` 跳出工作空间的
    /// 路径遍历攻击。
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时测试目录
    /// 2. 尝试写入路径 `../../etc/evil`（试图访问工作空间外）
    /// 3. 验证操作失败
    /// 4. 验证错误信息包含 "not allowed"
    /// 5. 清理测试目录
    #[tokio::test]
    async fn file_write_blocks_path_traversal() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_traversal");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let result =
            tool.execute(json!({"filePath": "../../etc/evil", "content": "bad"})).await.unwrap();
        // 验证操作被阻止
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试绝对路径阻止
    ///
    /// 验证 `FileWriteTool` 能够阻止绝对路径写入尝试，
    /// 确保所有文件操作都限制在工作空间内。
    ///
    /// # 测试步骤
    ///
    /// 1. 尝试写入绝对路径 `/etc/evil`
    /// 2. 验证操作失败
    /// 3. 验证错误信息包含 "not allowed"
    #[tokio::test]
    async fn file_write_blocks_absolute_path() {
        let tool = FileWriteTool::new(test_security(std::env::temp_dir()));
        let result =
            tool.execute(json!({"filePath": "/etc/evil", "content": "bad"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn file_write_accepts_workspace_absolute_path() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_workspace_absolute");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let target = dir.join("absolute.txt");

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let result =
            tool.execute(json!({"filePath": target, "content": "absolute"})).await.unwrap();

        assert!(result.success, "error: {:?}", result.error);
        assert_eq!(tokio::fs::read_to_string(dir.join("absolute.txt")).await.unwrap(), "absolute");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_write_rejects_directory_target() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_directory_target");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(dir.join("nested")).await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let result = tool
            .execute(json!({"filePath": "nested", "content": "not a directory"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or_default().contains("Path is a directory"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试缺失路径参数检测
    ///
    /// 验证 `FileWriteTool` 在缺少 `filePath` 或 `path` 参数时，
    /// 返回错误而非执行操作。
    #[tokio::test]
    async fn file_write_missing_path_param() {
        let tool = FileWriteTool::new(test_security(std::env::temp_dir()));
        let result = tool.execute(json!({"content": "data"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn file_write_rejects_blank_path_value() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_blank_path");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"filePath": "   ", "content": "data"})).await.unwrap();

        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("Missing filePath"));
        assert!(tokio::fs::read_dir(&dir).await.unwrap().next_entry().await.unwrap().is_none());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试缺失内容参数检测
    ///
    /// 验证 `FileWriteTool` 在缺少 `content` 参数时，
    /// 返回错误而非执行操作。
    #[tokio::test]
    async fn file_write_missing_content_param() {
        let tool = FileWriteTool::new(test_security(std::env::temp_dir()));
        let result = tool.execute(json!({"path": "file.txt"})).await;
        assert!(result.is_err());
    }

    /// 测试空内容写入功能
    ///
    /// 验证 `FileWriteTool` 能够处理空字符串内容的写入，
    /// 创建空文件并返回带有 "A"（Added）标记的输出。
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时测试目录
    /// 2. 写入空内容到 `empty.txt`
    /// 3. 验证操作成功且输出包含新增标记（A 表示 Added）
    /// 4. 清理测试目录
    #[tokio::test]
    async fn file_write_empty_content() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_empty");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"filePath": "empty.txt", "content": ""})).await.unwrap();
        assert!(result.success);
        let content = tokio::fs::read_to_string(dir.join("empty.txt")).await.unwrap();
        assert_eq!(content, "");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试符号链接目录转义防护（仅 Unix）
    ///
    /// 验证 `FileWriteTool` 能够阻止通过工作空间内的符号链接
    /// 目录访问工作空间外部的文件。
    ///
    /// 这是 TOCTOU（Time-of-Check-Time-of-Use）攻击防护的一部分。
    ///
    /// # 测试场景
    ///
    /// ```
    /// /tmp/vibewindow_test_file_write_symlink_escape/
    /// ├── workspace/          # 工作空间
    /// │   └── escape_dir -> ../outside/  # 指向外部的符号链接
    /// └── outside/            # 工作空间外的目录
    /// ```
    ///
    /// # 测试步骤
    ///
    /// 1. 创建测试目录结构，包含符号链接
    /// 2. 尝试通过符号链接写入文件 `escape_dir/hijack.txt`
    /// 3. 验证操作失败且错误信息提及 "escapes workspace"
    /// 4. 验证外部目录中未创建任何文件
    /// 5. 清理测试目录
    #[cfg(unix)]
    #[tokio::test]
    async fn file_write_blocks_symlink_escape() {
        use std::os::unix::fs::symlink;

        // 创建测试目录结构
        let root = std::env::temp_dir().join("vibewindow_test_file_write_symlink_escape");
        let workspace = root.join("workspace");
        let outside = root.join("outside");

        let _ = tokio::fs::remove_dir_all(&root).await;
        tokio::fs::create_dir_all(&workspace).await.unwrap();
        tokio::fs::create_dir_all(&outside).await.unwrap();

        // 在工作空间内创建指向外部的符号链接
        symlink(&outside, workspace.join("escape_dir")).unwrap();

        let tool = FileWriteTool::new(test_security(workspace.clone()));
        let result = tool
            .execute(json!({"filePath": "escape_dir/hijack.txt", "content": "bad"}))
            .await
            .unwrap();

        // 验证操作被阻止
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").contains("escapes workspace"));
        // 验证外部目录未被污染
        assert!(!outside.join("hijack.txt").exists());

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    /// 测试只读模式阻止写入
    ///
    /// 验证当安全策略设置为 `AutonomyLevel::ReadOnly` 时，
    /// `FileWriteTool` 阻止所有写入操作。
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时测试目录
    /// 2. 配置只读安全策略
    /// 3. 尝试执行写入操作
    /// 4. 验证操作失败且错误信息包含 "read-only"
    /// 5. 验证文件未被创建
    /// 6. 清理测试目录
    #[tokio::test]
    async fn file_write_blocks_readonly_mode() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_readonly");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        // 配置只读安全策略
        let tool = FileWriteTool::new(test_security_with(dir.clone(), AutonomyLevel::ReadOnly, 20));
        let result =
            tool.execute(json!({"filePath": "out.txt", "content": "should-block"})).await.unwrap();

        // 验证操作被只读策略阻止
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").contains("read-only"));
        assert!(!dir.join("out.txt").exists());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试速率限制阻止写入
    ///
    /// 验证当安全策略的 `max_actions_per_hour` 设置为 0 时，
    /// `FileWriteTool` 阻止写入操作并返回速率限制错误。
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时测试目录
    /// 2. 配置零速率限制的安全策略
    /// 3. 尝试执行写入操作
    /// 4. 验证操作失败且错误信息包含 "Rate limit exceeded"
    /// 5. 验证文件未被创建
    /// 6. 清理测试目录
    #[tokio::test]
    async fn file_write_blocks_when_rate_limited() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_rate_limited");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        // 配置零速率限制策略（立即触发速率限制）
        let tool =
            FileWriteTool::new(test_security_with(dir.clone(), AutonomyLevel::Supervised, 0));
        let result =
            tool.execute(json!({"filePath": "out.txt", "content": "should-block"})).await.unwrap();

        // 验证操作被速率限制阻止
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").contains("Rate limit exceeded"));
        assert!(!dir.join("out.txt").exists());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // §5.1 TOCTOU / 符号链接文件写入防护测试
    // ═══════════════════════════════════════════════════════════════════════════

    /// 测试符号链接目标文件写入防护（仅 Unix）
    ///
    /// 验证 `FileWriteTool` 能够阻止写入指向工作空间外部的符号链接文件，
    /// 防止通过符号链接修改外部文件内容。
    ///
    /// 这是 TOCTOU 攻击防护的一部分，确保即使在工作空间内存在恶意符号链接，
    /// 也无法通过工具修改外部文件。
    ///
    /// # 测试场景
    ///
    /// ```
    /// /tmp/vibewindow_test_file_write_symlink_target/
    /// ├── workspace/           # 工作空间
    /// │   └── linked.txt -> ../outside/target.txt  # 指向外部文件的符号链接
    /// └── outside/             # 工作空间外的目录
    ///     └── target.txt       # 原始文件（内容："original"）
    /// ```
    ///
    /// # 测试步骤
    ///
    /// 1. 创建测试目录结构
    /// 2. 在工作空间外创建目标文件 `target.txt`
    /// 3. 在工作空间内创建符号链接 `linked.txt` 指向目标文件
    /// 4. 尝试通过符号链接路径写入内容
    /// 5. 验证操作失败且错误信息提及 "symlink"
    /// 6. 验证原始文件内容未被修改
    /// 7. 清理测试目录
    #[cfg(unix)]
    #[tokio::test]
    async fn file_write_blocks_symlink_target_file() {
        use std::os::unix::fs::symlink;

        // 创建测试目录结构
        let root = std::env::temp_dir().join("vibewindow_test_file_write_symlink_target");
        let workspace = root.join("workspace");
        let outside = root.join("outside");

        let _ = tokio::fs::remove_dir_all(&root).await;
        tokio::fs::create_dir_all(&workspace).await.unwrap();
        tokio::fs::create_dir_all(&outside).await.unwrap();

        // 在工作空间外创建目标文件，并在工作空间内创建指向它的符号链接
        tokio::fs::write(outside.join("target.txt"), "original").await.unwrap();
        symlink(outside.join("target.txt"), workspace.join("linked.txt")).unwrap();

        let tool = FileWriteTool::new(test_security(workspace.clone()));
        let result = tool
            .execute(json!({"filePath": "linked.txt", "content": "overwritten"}))
            .await
            .unwrap();

        // 验证写入通过符号链接被阻止
        assert!(!result.success, "writing through symlink must be blocked");
        assert!(
            result.error.as_deref().unwrap_or("").contains("symlink"),
            "error should mention symlink"
        );

        // 验证原始文件未被修改
        let content = tokio::fs::read_to_string(outside.join("target.txt")).await.unwrap();
        assert_eq!(content, "original", "original file must not be modified");

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    /// 测试空字节注入防护
    ///
    /// 验证 `FileWriteTool` 能够阻止包含空字节（`\0`）的文件路径，
    /// 防止通过空字节截断进行路径注入攻击。
    ///
    /// # 安全背景
    ///
    /// 空字节注入是一种常见的安全漏洞利用技术：
    /// - 在 C 语言中，字符串以空字节结尾
    /// - 攻击者可能利用 `file\u{0000}.txt` 这样的路径
    /// - 如果后端使用 C 库，可能会将路径截断为 `file`
    /// - 这可能导致意外的文件访问或覆盖
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时测试目录
    /// 2. 尝试写入包含空字节的路径 `file\u{0000}.txt`
    /// 3. 验证操作被阻止
    /// 4. 清理测试目录
    #[tokio::test]
    async fn file_write_blocks_null_byte_in_path() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_null");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let result =
            tool.execute(json!({"filePath": "file\u{0000}.txt", "content": "bad"})).await.unwrap();
        // 验证包含空字节的路径被阻止
        assert!(!result.success, "paths with null bytes must be blocked");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试路径别名参数支持
    ///
    /// 验证 `FileWriteTool` 能够接受 `path` 作为 `filePath` 的别名，
    /// 提供向后兼容性。
    ///
    /// # 参数说明
    ///
    /// - `filePath`: 推荐使用的主要路径参数
    /// - `path`: 为了向后兼容提供的别名参数
    ///
    /// 两者功能相同，工具应优先使用 `filePath`，但也要支持 `path`。
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时测试目录
    /// 2. 使用 `path` 别名参数执行写入操作
    /// 3. 验证操作成功
    /// 4. 验证文件内容正确
    /// 5. 清理测试目录
    #[tokio::test]
    async fn file_write_accepts_path_alias() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_path_alias");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        // 使用 path 别名而非 filePath
        let result = tool.execute(json!({"path": "out.txt", "content": "ok"})).await.unwrap();
        assert!(result.success);

        // 验证文件被正确写入
        let content = tokio::fs::read_to_string(dir.join("out.txt")).await.unwrap();
        assert_eq!(content, "ok");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_write_accepts_file_path_alias() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_file_path_alias");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let result = tool.call(json!({"file_path": "out.txt", "content": "ok"})).await.unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["kind"], json!("create"));
        assert_eq!(tokio::fs::read_to_string(dir.join("out.txt")).await.unwrap(), "ok");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_write_call_rejects_invalid_parameters() {
        let tool = FileWriteTool::new(test_security(std::env::temp_dir()));
        let result = tool.call(json!({"filePath": "missing-content.txt"})).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing or invalid parameters"));
    }

    #[tokio::test]
    async fn file_write_rejects_notebook_paths() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_write_notebook_reject");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileWriteTool::new(test_security(dir.clone()));
        let result = tool
            .call(json!({
                "filePath": "demo.ipynb",
                "content": "{}\n"
            }))
            .await
            .unwrap();

        assert!(!result.is_success());
        assert!(result.error_text().unwrap_or_default().contains("use notebook_edit instead"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
