//! GrepTool 单元测试模块
//!
//! 本模块提供了对 `GrepTool` 工具的全面测试覆盖，包括：
//! - 工具名称和参数 schema 验证
//! - 基本搜索功能测试
//! - 文件过滤器测试（include 模式）
//! - 错误处理测试（无效正则表达式）
//! - 速率限制测试
//!
//! 所有测试均在临时目录中运行，确保测试隔离性和可重复性。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
    use tempfile::TempDir;

    /// 创建测试用的安全策略配置
    ///
    /// 返回一个具有以下特性的 `SecurityPolicy`：
    /// - 自主级别为 `Supervised`（监督模式）
    /// - 工作区目录设置为指定的 `workspace` 路径
    /// - 其他配置使用默认值
    ///
    /// # 参数
    ///
    /// * `workspace` - 测试工作区的路径，通常是临时目录
    ///
    /// # 返回值
    ///
    /// 返回一个 `Arc<SecurityPolicy>`，可在多线程间共享的安全策略实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let policy = test_security(PathBuf::from("/tmp/test"));
    /// ```
    fn test_security(workspace: PathBuf) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace,
            ..SecurityPolicy::default()
        })
    }

    /// 创建带有速率限制的测试用安全策略配置
    ///
    /// 返回一个具有以下特性的 `SecurityPolicy`：
    /// - 自主级别为 `Supervised`（监督模式）
    /// - 工作区目录设置为指定的 `workspace` 路径
    /// - 每小时最大操作数限制为 `max_actions_per_hour`
    /// - 其他配置使用默认值
    ///
    /// # 参数
    ///
    /// * `workspace` - 测试工作区的路径，通常是临时目录
    /// * `max_actions_per_hour` - 每小时允许的最大操作数，用于测试速率限制功能
    ///
    /// # 返回值
    ///
    /// 返回一个 `Arc<SecurityPolicy>`，可在多线程间共享的安全策略实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 创建不允许任何操作的策略（用于测试速率限制）
    /// let policy = test_security_with_limit(PathBuf::from("/tmp/test"), 0);
    /// ```
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

    /// 测试 GrepTool 的基本属性和参数 schema
    ///
    /// 验证以下内容：
    /// 1. 工具名称是否正确设置为 "grep"
    /// 2. V2 规格是否暴露 Claude 兼容 canonical 名
    /// 3. 参数 schema 中是否包含必需的字段：
    ///    - `pattern`：搜索模式
    ///    - `path`：搜索路径
    ///    - `include`：文件过滤模式
    ///    - `output_mode`：输出模式
    #[test]
    fn grep_name_and_schema() {
        // 创建 GrepTool 实例
        let tool = GrepTool::new(test_security(std::env::temp_dir()));

        // 验证工具名称
        assert_eq!(tool.name(), "grep");
        let spec = tool.spec();
        assert_eq!(spec.id, "grep");
        assert_eq!(spec.display_name, "grep");
        assert!(spec.aliases.is_empty());
        assert!(spec.read_only);
        assert!(!spec.destructive);
        assert!(spec.concurrency_safe);
        assert!(!spec.requires_user_interaction);
        assert!(spec.strict);

        // 获取并验证参数 schema
        let schema = tool.parameters_schema();

        // 检查必需的参数字段是否存在于 schema 中
        assert!(schema["properties"]["pattern"].is_object());
        assert!(schema["properties"]["path"].is_object());
        assert!(schema["properties"]["include"].is_object());
        assert!(schema["properties"]["output_mode"].is_object());
    }

    /// 测试 GrepTool 使用 include 过滤器查找匹配内容
    ///
    /// 测试场景：
    /// 1. 创建一个临时目录，包含两个文件：
    ///    - `a.rs`：Rust 源文件，包含 "hello" 字符串
    ///    - `b.txt`：文本文件，包含 "hello" 字符串
    /// 2. 使用 `include: "*.rs"` 过滤器搜索 "hello"
    /// 3. 验证：
    ///    - 操作成功
    ///    - 只找到 1 处匹配
    ///    - 结果包含 `a.rs`
    ///    - 结果不包含 `b.txt`（被过滤器排除）
    #[tokio::test]
    async fn grep_finds_matches_with_include() {
        // 创建临时目录
        let dir = TempDir::new().unwrap();

        // 创建测试文件：Rust 源文件
        std::fs::write(dir.path().join("a.rs"), "fn hello() {}\nlet x = 1;\n").unwrap();

        // 创建测试文件：普通文本文件
        std::fs::write(dir.path().join("b.txt"), "hello from txt\n").unwrap();

        // 创建 GrepTool 实例，使用临时目录作为工作区
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        // 执行搜索，只搜索 .rs 文件
        let result = tool
            .execute(json!({
                "pattern": "hello",
                "include": "*.rs",
                "output_mode": "content"
            }))
            .await
            .unwrap();

        // 验证搜索成功
        assert!(result.success);

        // 验证找到了 1 处匹配
        assert!(result.output.contains("找到 1 处匹配"));

        // 验证结果包含 a.rs 文件
        assert!(result.output.contains("a.rs"));

        // 验证结果不包含 b.txt 文件（被过滤器排除）
        assert!(!result.output.contains("b.txt"));
    }

    /// 测试 GrepTool 拒绝无效的正则表达式
    ///
    /// 测试场景：
    /// 1. 创建一个临时目录，包含一个文本文件
    /// 2. 使用无效的正则表达式 `(`（未闭合的括号）进行搜索
    /// 3. 验证：
    ///    - 操作失败（success = false）
    ///    - 错误信息包含"正则表达式无效"的提示
    #[tokio::test]
    async fn grep_rejects_invalid_regex() {
        // 创建临时目录
        let dir = TempDir::new().unwrap();

        // 创建测试文件
        std::fs::write(dir.path().join("a.txt"), "hello\n").unwrap();

        // 创建 GrepTool 实例
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        // 使用无效的正则表达式执行搜索
        let result = tool.execute(json!({ "pattern": "(" })).await.unwrap();

        // 验证操作失败
        assert!(!result.success);

        // 验证错误信息包含正则表达式无效的提示
        assert!(result.error.as_ref().unwrap().contains("正则表达式无效"));
    }

    /// 测试 GrepTool 的速率限制功能
    ///
    /// 测试场景：
    /// 1. 创建一个临时目录
    /// 2. 创建一个每小时最大操作数为 0 的安全策略
    /// 3. 执行搜索操作
    /// 4. 验证：
    ///    - 操作失败（success = false）
    ///    - 错误信息包含"Rate limit"的提示
    ///
    /// 这个测试确保速率限制机制能够正确阻止超出限制的操作
    #[tokio::test]
    async fn grep_rate_limited() {
        // 创建临时目录
        let dir = TempDir::new().unwrap();

        // 创建 GrepTool 实例，设置每小时最大操作数为 0（不允许任何操作）
        let tool = GrepTool::new(test_security_with_limit(dir.path().to_path_buf(), 0));

        // 尝试执行搜索操作
        let result = tool.execute(json!({ "pattern": "hello" })).await.unwrap();

        // 验证操作因速率限制而失败
        assert!(!result.success);

        // 验证错误信息包含速率限制的提示
        assert!(result.error.as_ref().unwrap().contains("Rate limit"));
    }

    fn match_row(path: &str, line_num: usize, line_text: &str) -> MatchRow {
        MatchRow {
            path: path.to_string(),
            mod_time_ms: 0,
            line_num,
            line_text: line_text.to_string(),
        }
    }

    #[test]
    fn args_deserialize_defaults_and_glob_alias() {
        let defaults: Args = serde_json::from_value(json!({ "pattern": "needle" })).unwrap();

        assert_eq!(format_output_mode(defaults.output_mode), "files_with_matches");
        assert_eq!(defaults.head_limit, LIMIT);
        assert_eq!(defaults.offset, 0);
        assert!(!defaults.line_numbers);
        assert!(!defaults.case_insensitive);
        assert!(!defaults.multiline);

        let aliases: Args = serde_json::from_value(json!({
            "pattern": "needle",
            "glob": "*.rs",
            "-n": true,
            "-i": true,
            "output_mode": "count"
        }))
        .unwrap();

        assert_eq!(aliases.include.as_deref(), Some("*.rs"));
        assert!(aliases.line_numbers);
        assert!(aliases.case_insensitive);
        assert_eq!(format_output_mode(aliases.output_mode), "count");
    }

    #[test]
    fn validate_capabilities_rejects_each_unsupported_option() {
        for (input, expected) in [
            (json!({ "pattern": "x", "-A": 1 }), "-A/-B/-C"),
            (json!({ "pattern": "x", "-B": 1 }), "-A/-B/-C"),
            (json!({ "pattern": "x", "-C": 1 }), "-A/-B/-C"),
            (json!({ "pattern": "x", "type": "rust" }), "type"),
            (json!({ "pattern": "x", "multiline": true }), "multiline"),
        ] {
            let args: Args = serde_json::from_value(input).unwrap();
            let error = GrepTool::validate_capabilities(&args).unwrap_err();
            assert!(error.to_string().contains(expected));
        }

        let supported: Args = serde_json::from_value(json!({ "pattern": "x" })).unwrap();
        assert!(GrepTool::validate_capabilities(&supported).is_ok());
    }

    #[test]
    fn format_output_mode_names_every_variant() {
        assert_eq!(format_output_mode(OutputMode::Content), "content");
        assert_eq!(format_output_mode(OutputMode::FilesWithMatches), "files_with_matches");
        assert_eq!(format_output_mode(OutputMode::Count), "count");
    }

    #[test]
    fn unique_paths_collapses_adjacent_rows_in_sorted_order() {
        let rows = vec![
            match_row("a.rs", 1, "first"),
            match_row("a.rs", 2, "second"),
            match_row("b.rs", 1, "third"),
            match_row("b.rs", 3, "fourth"),
        ];

        assert_eq!(unique_paths(&rows), vec!["a.rs", "b.rs"]);
        assert!(unique_paths(&[]).is_empty());
    }

    #[test]
    fn counts_by_path_aggregates_non_adjacent_rows_and_preserves_first_seen_order() {
        let rows = vec![
            match_row("a.rs", 1, "first"),
            match_row("b.rs", 1, "second"),
            match_row("a.rs", 2, "third"),
        ];

        assert_eq!(counts_by_path(&rows), vec![("a.rs".to_string(), 2), ("b.rs".to_string(), 1)]);
        assert!(counts_by_path(&[]).is_empty());
    }

    #[test]
    fn expand_braces_handles_multiple_groups_and_literal_patterns() {
        assert_eq!(expand_braces("*.rs"), vec!["*.rs"]);
        assert_eq!(expand_braces("*.{rs}"), vec!["*.{rs}"]);
        assert_eq!(expand_braces("*.{rs,txt"), vec!["*.{rs,txt"]);
        assert_eq!(
            expand_braces("src/{lib,main}.{rs,txt}"),
            vec!["src/lib.rs", "src/lib.txt", "src/main.rs", "src/main.txt"]
        );
    }

    #[test]
    fn is_binary_detects_extensions_and_file_contents() {
        let dir = TempDir::new().unwrap();
        let text = dir.path().join("notes.txt");
        let empty = dir.path().join("empty");
        let nul = dir.path().join("nul");
        let controls = dir.path().join("controls");

        std::fs::write(&text, b"plain text\nwith a tab\tand newline\n").unwrap();
        std::fs::write(&empty, b"").unwrap();
        std::fs::write(&nul, b"text\0tail").unwrap();
        std::fs::write(&controls, [1_u8; 10]).unwrap();

        assert!(is_binary(&dir.path().join("archive.ZIP")));
        assert!(!is_binary(&text));
        assert!(!is_binary(&empty));
        assert!(is_binary(&nul));
        assert!(is_binary(&controls));
        assert!(!is_binary(&dir.path().join("missing.txt")));
    }

    #[tokio::test]
    async fn grep_files_mode_applies_head_limit_and_reports_truncation() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "needle\n").unwrap();
        std::fs::write(dir.path().join("b.txt"), "needle\n").unwrap();
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        let result = tool
            .call(json!({
                "pattern": "needle",
                "path": "  ",
                "output_mode": "files_with_matches",
                "head_limit": 0
            }))
            .await
            .unwrap();

        assert_eq!(result.data["success"], json!(true));
        assert_eq!(result.data["output_mode"], json!("files_with_matches"));
        assert_eq!(result.data["returned_count"], json!(1));
        assert_eq!(result.data["total_count"], json!(2));
        assert_eq!(result.data["truncated"], json!(true));
        assert!(result.model_result.as_str().unwrap().contains("结果已截断"));
    }

    #[tokio::test]
    async fn grep_include_with_directory_separator_matches_relative_path() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.rs"), "needle\n").unwrap();
        std::fs::write(dir.path().join("root.rs"), "needle\n").unwrap();
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        let result = tool
            .call(json!({
                "pattern": "needle",
                "include": "src/*.rs",
                "output_mode": "content"
            }))
            .await
            .unwrap();

        assert_eq!(result.data["returned_count"], json!(1));
        assert!(result.data["matches"][0]["path"].as_str().unwrap().ends_with("/src/a.rs"));
        assert!(!result.model_result.as_str().unwrap().contains("root.rs"));
    }

    #[tokio::test]
    async fn grep_count_mode_aggregates_matches_and_honors_offset() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "needle\nother\nneedle\n").unwrap();
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        let counted =
            tool.call(json!({ "pattern": "needle", "output_mode": "count" })).await.unwrap();

        assert_eq!(counted.data["counts"][0]["count"], json!(2));
        assert_eq!(counted.data["match_count"], json!(2));
        assert_eq!(counted.data["total_count"], json!(1));
        assert!(counted.model_result.as_str().unwrap().ends_with(": 2"));

        let offset = GrepTool::new(test_security(dir.path().to_path_buf()))
            .call(json!({ "pattern": "needle", "output_mode": "count", "offset": 1 }))
            .await
            .unwrap();
        assert_eq!(offset.data["returned_count"], json!(0));
        assert_eq!(offset.model_result, json!("未找到匹配"));
    }

    #[tokio::test]
    async fn grep_content_mode_supports_case_insensitive_lines_and_truncates_display_only() {
        let dir = TempDir::new().unwrap();
        let long_line = "A".repeat(MAX_LINE_LENGTH + 1);
        std::fs::write(dir.path().join("a.txt"), format!("Needle\n{long_line}\n")).unwrap();
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        let result = tool
            .call(json!({
                "pattern": "needle|a+",
                "output_mode": "content",
                "-i": true,
                "-n": true
            }))
            .await
            .unwrap();

        assert_eq!(result.data["returned_count"], json!(2));
        assert_eq!(result.data["matches"][0]["line_number"], json!(1));
        assert_eq!(
            result.data["matches"][1]["line_text"].as_str().unwrap().len(),
            MAX_LINE_LENGTH + 1
        );
        let output = result.model_result.as_str().unwrap();
        assert!(output.contains("  1: Needle"));
        assert!(output.contains(&format!("  2: {}...", "A".repeat(MAX_LINE_LENGTH))));
    }

    #[tokio::test]
    async fn grep_skips_binary_files_and_returns_empty_structured_result() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("binary.dat"), b"needle\0tail").unwrap();
        std::fs::write(dir.path().join("plain.txt"), "no match\n").unwrap();
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        let result = tool.call(json!({ "pattern": "needle" })).await.unwrap();

        assert_eq!(result.model_result, json!("未找到匹配"));
        assert_eq!(result.data["files"], json!([]));
        assert_eq!(result.data["matches"], json!([]));
        assert_eq!(result.data["counts"], json!([]));
        assert_eq!(result.data["total_count"], json!(0));
        assert_eq!(result.data["has_errors"], json!(false));
    }

    #[tokio::test]
    async fn grep_rejects_blank_pattern_invalid_glob_missing_path_and_parent_traversal() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("file.txt"), "text\n").unwrap();

        for (input, expected) in [
            (json!({ "pattern": "   " }), "Missing pattern"),
            (json!({ "pattern": "x", "include": "[" }), "glob 无效"),
            (json!({ "pattern": "x", "path": "missing" }), "does not exist"),
            (json!({ "pattern": "x", "path": "file.txt" }), "not a directory"),
            (json!({ "pattern": "x", "path": "../" }), "not allowed"),
            (json!({ "pattern": "x", "multiline": true }), "multiline"),
        ] {
            let tool = GrepTool::new(test_security(dir.path().to_path_buf()));
            let result = tool.execute(input).await.unwrap();
            assert!(!result.success);
            assert!(result.error.as_deref().unwrap().contains(expected));
        }

        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));
        let error = tool.execute(json!({})).await.unwrap_err();
        assert!(error.to_string().contains("Missing or invalid parameters"));
    }

    #[tokio::test]
    async fn grep_call_success_populates_content_rendering_and_telemetry() {
        let dir = TempDir::new().unwrap();
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        let result = tool.call(json!({ "pattern": "absent" })).await.unwrap();

        assert_eq!(result.data["success"], json!(true));
        assert_eq!(result.model_result, json!("未找到匹配"));
        assert!(matches!(
            result.content_blocks.first(),
            Some(ToolResultContentDto::Text { text }) if text == "未找到匹配"
        ));
        assert!(matches!(
            result.content_blocks.get(1),
            Some(ToolResultContentDto::Json { value }) if value["success"] == json!(true)
        ));
        let render_hint = result.render_hint.as_ref().unwrap();
        assert_eq!(render_hint.kind.as_deref(), Some("grep"));
        assert_eq!(render_hint.summary.as_deref(), Some("未找到匹配"));
        assert_eq!(render_hint.metadata["output_mode"], json!("files_with_matches"));
        assert_eq!(result.telemetry.as_ref().map(|telemetry| telemetry.success), Some(true));
    }

    #[tokio::test]
    async fn grep_call_failure_returns_structured_error_and_failed_telemetry() {
        let dir = TempDir::new().unwrap();
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        let result = tool.call(json!({ "pattern": "x", "-A": 1 })).await.unwrap();

        assert_eq!(result.data["success"], json!(false));
        assert!(result.data["error"].as_str().unwrap().contains("-A/-B/-C"));
        assert!(result.model_result.as_str().unwrap().contains("-A/-B/-C"));
        assert!(matches!(
            result.content_blocks.first(),
            Some(ToolResultContentDto::Text { text }) if text.contains("-A/-B/-C")
        ));
        let render_hint = result.render_hint.as_ref().unwrap();
        assert_eq!(render_hint.summary.as_deref(), Some("搜索失败"));
        assert_eq!(render_hint.metadata["tool_id"], json!("grep"));
        assert_eq!(result.telemetry.as_ref().map(|telemetry| telemetry.success), Some(false));
    }

    #[tokio::test]
    async fn grep_content_mode_without_line_numbers_separates_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "needle\n").unwrap();
        std::fs::write(dir.path().join("b.txt"), "needle\n").unwrap();
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        let result = tool
            .call(json!({
                "pattern": "needle",
                "output_mode": "content",
                "-n": false
            }))
            .await
            .unwrap();

        assert_eq!(result.data["returned_count"], json!(2));
        let output = result.model_result.as_str().unwrap();
        assert!(output.contains("  needle"));
        assert!(!output.contains("  1: needle"));
        assert!(output.contains("\n\n"), "file groups should be separated");
    }

    #[tokio::test]
    async fn grep_count_mode_reports_truncation_across_multiple_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "needle\n").unwrap();
        std::fs::write(dir.path().join("b.txt"), "needle\n").unwrap();
        let tool = GrepTool::new(test_security(dir.path().to_path_buf()));

        let result = tool
            .call(json!({
                "pattern": "needle",
                "output_mode": "count",
                "head_limit": 1
            }))
            .await
            .unwrap();

        assert_eq!(result.data["returned_count"], json!(1));
        assert_eq!(result.data["total_count"], json!(2));
        assert_eq!(result.data["truncated"], json!(true));
        assert!(result.model_result.as_str().unwrap().contains("结果已截断"));
    }
}
