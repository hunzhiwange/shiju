//! `file_edit` 工具测试。
//!
//! 覆盖以下关键行为：
//! - 已有文件必须先 `file_read`
//! - 文件在最近一次读取后发生变化时应拒绝继续编辑
//! - 默认要求 `old_string` 唯一
//! - `replace_all=true` 时允许多处替换
//! - quote normalization + quote style preserve

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
    use crate::app::agent::tools::FileSnapshot;
    use crate::app::agent::tools::ToolUseContext;
    use crate::app::agent::tools::context::scope_tool_use_context;
    use serde_json::json;
    use std::sync::Arc;
    use vw_api_types::tools::ToolResultContentDto;

    fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
        test_security_with(
            workspace,
            AutonomyLevel::Supervised,
            SecurityPolicy::default().max_actions_per_hour,
        )
    }

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

    #[test]
    fn edit_name() {
        let tool = FileEditTool::new(test_security(std::env::temp_dir()));
        assert_eq!(tool.name(), "file_edit");
        assert_eq!(tool.spec().id, "file_edit");
        assert!(tool.spec().aliases.is_empty());
    }

    #[test]
    fn edit_schema_has_required_fields() {
        let tool = FileEditTool::new(test_security(std::env::temp_dir()));
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["file_path"].is_object());
        assert!(schema["properties"]["old_string"].is_object());
        assert!(schema["properties"]["new_string"].is_object());
        assert!(schema["properties"]["replace_all"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("file_path")));
        assert!(required.contains(&json!("old_string")));
        assert!(required.contains(&json!("new_string")));
    }

    #[tokio::test]
    async fn edit_replaces_existing_text_after_read() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_success");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("main.rs"), "let value = \"old\";\n").await.unwrap();

        let tool = FileEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-edit-success",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "main.rs", "let value = \"old\";\n");

        let result = scope_tool_use_context(
            context.clone(),
            tool.call(json!({
                "file_path": "main.rs",
                "old_string": "\"old\"",
                "new_string": "\"new\""
            })),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["kind"], json!("update"));
        assert!(matches!(
            result.content_blocks.first(),
            Some(ToolResultContentDto::StructuredPatch { hunks }) if !hunks.is_empty()
        ));

        let content = tokio::fs::read_to_string(dir.join("main.rs")).await.unwrap();
        assert_eq!(content, "let value = \"new\";\n");

        let mut snapshot = context.read_state_snapshot();
        let entry = snapshot.get(Some(dir.as_path()), "main.rs").expect("read state missing");
        assert_eq!(entry.snapshot, Some(FileSnapshot::from_text("let value = \"new\";\n")));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn edit_blocks_without_prior_read() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_requires_read");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("main.rs"), "fn main() {}\n").await.unwrap();

        let tool = FileEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-edit-requires-read",
            Some(dir.to_string_lossy().to_string()),
        ));

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "file_path": "main.rs",
                "old_string": "main",
                "new_string": "entry"
            })),
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

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn edit_blocks_when_file_changed_since_read() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_stale");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("main.rs"), "before\n").await.unwrap();

        let tool = FileEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-edit-stale",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "main.rs", "before\n");
        tokio::fs::write(dir.join("main.rs"), "external-change\n").await.unwrap();

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "file_path": "main.rs",
                "old_string": "before",
                "new_string": "after"
            })),
        )
        .await
        .unwrap();

        assert!(!result.is_success());
        assert!(
            result.error_text().unwrap_or_default().contains("changed since the last file_read")
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn edit_requires_unique_old_string_unless_replace_all() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_unique");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("dup.txt"), "foo\nfoo\n").await.unwrap();

        let tool = FileEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-edit-unique",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "dup.txt", "foo\nfoo\n");

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "file_path": "dup.txt",
                "old_string": "foo",
                "new_string": "bar"
            })),
        )
        .await
        .unwrap();

        assert!(!result.is_success());
        assert!(result.error_text().unwrap_or_default().contains("matched 2 locations"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn edit_replace_all_replaces_multiple_matches() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_replace_all");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("dup.txt"), "foo\nfoo\n").await.unwrap();

        let tool = FileEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-edit-replace-all",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "dup.txt", "foo\nfoo\n");

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "file_path": "dup.txt",
                "old_string": "foo",
                "new_string": "bar",
                "replace_all": true
            })),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        let content = tokio::fs::read_to_string(dir.join("dup.txt")).await.unwrap();
        assert_eq!(content, "bar\nbar\n");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn edit_preserves_quote_style_for_normalized_match() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_quote_style");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("quote.rs"), "let value = 'old';\n").await.unwrap();

        let tool = FileEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-edit-quote-style",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "quote.rs", "let value = 'old';\n");

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "file_path": "quote.rs",
                "old_string": "let value = \"old\";\n",
                "new_string": "let value = \"new\";\n"
            })),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["quote_normalized_match"], json!(true));
        let content = tokio::fs::read_to_string(dir.join("quote.rs")).await.unwrap();
        assert_eq!(content, "let value = 'new';\n");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn edit_reports_no_op_when_replacement_is_identical() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_no_op");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("same.txt"), "unchanged\n").await.unwrap();

        let tool = FileEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-edit-no-op",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "same.txt", "unchanged\n");

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "file_path": "same.txt",
                "old_string": "unchanged",
                "new_string": "unchanged"
            })),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["replacements"], json!(1));
        assert!(result.model_result.as_str().unwrap_or_default().contains("did not change"));
        assert!(matches!(
            result.content_blocks.first(),
            Some(ToolResultContentDto::StructuredPatch { hunks }) if hunks.is_empty()
        ));
        assert_eq!(tokio::fs::read_to_string(dir.join("same.txt")).await.unwrap(), "unchanged\n");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn edit_rejects_notebook_paths() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_notebook_reject");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("demo.ipynb"), "{}\n").await.unwrap();

        let tool = FileEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "file-edit-notebook-reject",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "demo.ipynb", "{}\n");

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "file_path": "demo.ipynb",
                "old_string": "{}",
                "new_string": "{\"cells\": []}"
            })),
        )
        .await
        .unwrap();

        assert!(!result.is_success());
        assert!(result.error_text().unwrap_or_default().contains("use notebook_edit instead"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn edit_call_rejects_blank_file_path() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_blank_path");
        let tool = FileEditTool::new(test_security(dir));

        let result = tool
            .call(json!({
                "file_path": "  ",
                "old_string": "before",
                "new_string": "after"
            }))
            .await
            .unwrap();

        assert!(!result.is_success());
        assert_eq!(result.error_text().as_deref(), Some("Missing file_path"));
    }

    #[tokio::test]
    async fn edit_execute_rejects_invalid_parameters() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_execute_invalid");
        let tool = FileEditTool::new(test_security(dir));

        let error = tool
            .execute(json!({
                "file_path": "main.rs",
                "old_string": "before"
            }))
            .await
            .expect_err("missing new_string must reject parameters");

        assert!(error.to_string().contains("Missing or invalid parameters"), "{error}");
    }

    #[tokio::test]
    async fn edit_call_rejects_invalid_parameters() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_call_invalid");
        let tool = FileEditTool::new(test_security(dir));

        let error = tool
            .call(json!({
                "file_path": "main.rs",
                "new_string": "after"
            }))
            .await
            .expect_err("missing old_string must reject parameters");

        assert!(error.to_string().contains("Missing or invalid parameters"), "{error}");
    }

    #[tokio::test]
    async fn edit_execute_returns_failure_for_missing_file() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_execute_missing_file");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let tool = FileEditTool::new(test_security(dir.clone()));

        let result = tool
            .execute(json!({
                "file_path": "missing.txt",
                "old_string": "before",
                "new_string": "after"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Failed to read file metadata"), "{}", result.output);
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn edit_blocks_read_only_before_file_access() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_read_only");
        let tool = FileEditTool::new(test_security_with(dir, AutonomyLevel::ReadOnly, 20));

        let result = tool
            .call(json!({
                "file_path": "main.rs",
                "old_string": "before",
                "new_string": "after"
            }))
            .await
            .unwrap();

        assert!(!result.is_success());
        assert_eq!(result.error_text().as_deref(), Some("Action blocked: autonomy is read-only"));
    }

    #[tokio::test]
    async fn edit_blocks_when_rate_limited_before_file_access() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_rate_limited");
        let tool = FileEditTool::new(test_security_with(dir, AutonomyLevel::Supervised, 0));

        let result = tool
            .call(json!({
                "file_path": "main.rs",
                "old_string": "before",
                "new_string": "after"
            }))
            .await
            .unwrap();

        assert!(!result.is_success());
        assert_eq!(
            result.error_text().as_deref(),
            Some("Rate limit exceeded: too many actions in the last hour")
        );
    }

    #[tokio::test]
    async fn edit_rejects_missing_file_metadata() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_missing_file");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let tool = FileEditTool::new(test_security(dir.clone()));

        let result = tool
            .call(json!({
                "file_path": "missing.txt",
                "old_string": "before",
                "new_string": "after"
            }))
            .await
            .unwrap();

        assert!(!result.is_success());
        assert!(result.error_text().unwrap_or_default().contains("Failed to read file metadata"));
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn edit_rejects_directory_target() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_directory");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(dir.join("nested")).await.unwrap();
        let tool = FileEditTool::new(test_security(dir.clone()));

        let result = tool
            .call(json!({
                "file_path": "nested",
                "old_string": "before",
                "new_string": "after"
            }))
            .await
            .unwrap();

        assert!(!result.is_success());
        assert!(result.error_text().unwrap_or_default().contains("Path is a directory"));
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn edit_rejects_symlink_target() {
        use std::os::unix::fs::symlink;

        let dir = std::env::temp_dir().join("vibewindow_test_file_edit_symlink");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("real.txt"), "before\n").await.unwrap();
        symlink(dir.join("real.txt"), dir.join("linked.txt")).unwrap();
        let tool = FileEditTool::new(test_security(dir.clone()));

        let result = tool
            .call(json!({
                "file_path": "linked.txt",
                "old_string": "before",
                "new_string": "after"
            }))
            .await
            .unwrap();

        assert!(!result.is_success());
        assert!(
            result.error_text().unwrap_or_default().contains("Refusing to edit through symlink")
        );
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
