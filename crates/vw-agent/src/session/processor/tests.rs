//! Session Processor 单元测试模块
//!
//! 本模块包含 `processor` 模块的单元测试，主要测试以下功能：
//! - Todo 状态批量更新逻辑
//! - TodoWrite 工具的 completed 状态重写规则
//! - 工具调用检测与解析
//!
//! # 测试覆盖范围
//!
//! 1. **Todo 状态补丁生成**：验证 `build_todo_status_patches` 函数的行为
//! 2. **Completed 状态重写**：验证 `rewrite_todowrite_completed_when_no_work` 函数
//! 3. **工具调用检测**：验证 `query_has_any_tool_calls` 函数
//! 4. **工具解析**：验证 `parse_tool_at` 函数的各种边界情况

use super::*;

/// 处理器模块测试套件
#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::app::agent::session::session::{Role, Session};
    use crate::app::agent::tools::ToolRuntimeContext;

    /// 测试：仅将第一个未完成的 Todo 项状态更新为 in_progress
    ///
    /// # 验证场景
    /// - 列表包含 3 个 Todo 项：pending、in_progress、pending
    /// - 调用 `build_todo_status_patches` 时传入 `only_first = true`
    /// - 期望结果：仅生成 1 个补丁，针对第一个 pending 项（id=1）
    ///
    /// # 业务逻辑
    /// 当 `only_first = true` 时，函数应该跳过已处于 in_progress 的项，
    /// 只将第一个未完成（pending）的项更新为目标状态。
    #[test]
    fn patches_only_first_incomplete_to_in_progress() {
        // 准备测试数据：3 个 Todo 项，中间一个已是 in_progress
        let items = vec![
            todo::Todo {
                content: "a".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
                id: "1".to_string(),
            },
            todo::Todo {
                content: "b".to_string(),
                status: "in_progress".to_string(),
                priority: "high".to_string(),
                id: "2".to_string(),
            },
            todo::Todo {
                content: "c".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
                id: "3".to_string(),
            },
        ];

        // 调用待测函数：仅更新第一个未完成项
        let patches = todos::build_todo_status_patches(&items, "in_progress", true);

        // 断言：应该只生成 1 个补丁
        assert_eq!(patches.len(), 1);
        // 断言：补丁针对第一个 pending 项（id=1）
        assert_eq!(patches[0].get("id").and_then(|v| v.as_str()), Some("1"));
        // 断言：补丁将状态设置为 in_progress
        assert_eq!(patches[0].get("status").and_then(|v| v.as_str()), Some("in_progress"));
    }

    /// 测试：将所有未完成的 Todo 项状态更新为 completed
    ///
    /// # 验证场景
    /// - 列表包含 3 个 Todo 项：completed、in_progress、pending
    /// - 调用 `build_todo_status_patches` 时传入 `only_first = false`
    /// - 期望结果：生成 2 个补丁，分别针对 in_progress（id=2）和 pending（id=3）
    ///
    /// # 业务逻辑
    /// 当 `only_first = false` 时，函数应该将所有未完成（非 completed）的项
    /// 都更新为目标状态。
    #[test]
    fn patches_all_incomplete_to_completed() {
        // 准备测试数据：3 个 Todo 项，第一个已完成
        let items = vec![
            todo::Todo {
                content: "a".to_string(),
                status: "completed".to_string(),
                priority: "high".to_string(),
                id: "1".to_string(),
            },
            todo::Todo {
                content: "b".to_string(),
                status: "in_progress".to_string(),
                priority: "high".to_string(),
                id: "2".to_string(),
            },
            todo::Todo {
                content: "c".to_string(),
                status: "pending".to_string(),
                priority: "high".to_string(),
                id: "3".to_string(),
            },
        ];

        // 调用待测函数：更新所有未完成项
        let patches = todos::build_todo_status_patches(&items, "completed", false);

        // 断言：应该生成 2 个补丁
        assert_eq!(patches.len(), 2);
        // 断言：第一个补丁针对 in_progress 项（id=2）
        assert_eq!(patches[0].get("id").and_then(|v| v.as_str()), Some("2"));
        // 断言：第二个补丁针对 pending 项（id=3）
        assert_eq!(patches[1].get("id").and_then(|v| v.as_str()), Some("3"));
    }

    /// 测试：TodoWrite 无 merge 标志时，completed 状态重写为 pending
    ///
    /// # 验证场景
    /// - 输入 JSON 中包含 2 个 Todo：completed（高优先级）和 pending（低优先级）
    /// - 没有 `merge` 字段或 `merge = false`
    /// - 期望结果：completed 项被重写为 pending，原有的 pending 保持不变
    ///
    /// # 业务逻辑
    /// 当没有工作正在进行时（merge=false），不应保留 completed 状态，
    /// 因为这可能表示任务实际上未完成或需要重新评估。
    #[test]
    fn todowrite_completed_rewritten_to_pending_when_merge_false() {
        // 准备测试输入：包含一个 completed 和一个 pending 的 Todo 列表
        let input = serde_json::json!({
            "todos": [
                { "content": "a", "status": "completed", "priority": "high" },
                { "content": "b", "status": "pending", "priority": "low" }
            ]
        })
        .to_string();

        // 调用待测函数
        let out = utils::rewrite_todowrite_completed_when_no_work(&input);

        // 解析输出
        let v = serde_json::from_str::<serde_json::Value>(&out).unwrap();
        let todos = v.get("todos").and_then(|x| x.as_array()).unwrap();

        // 断言：第一个 Todo 的 completed 状态被重写为 pending
        assert_eq!(todos[0].get("status").and_then(|s| s.as_str()), Some("pending"));
        // 断言：第二个 Todo 的 pending 状态保持不变
        assert_eq!(todos[1].get("status").and_then(|s| s.as_str()), Some("pending"));
    }

    /// 测试：TodoWrite 有 merge=true 时，completed 状态重写为 in_progress
    ///
    /// # 验证场景
    /// - 输入 JSON 中包含 `merge: true` 和 2 个 Todo：completed 和 in_progress
    /// - 期望结果：completed 项被重写为 in_progress，原有的 in_progress 保持不变
    ///
    /// # 业务逻辑
    /// 当 merge=true 时，表示正在合并工作，completed 状态应该被视为 in_progress，
    /// 因为工作可能仍在进行中或需要后续处理。
    #[test]
    fn todowrite_completed_rewritten_to_in_progress_when_merge_true() {
        // 准备测试输入：包含 merge=true 和一个 completed、一个 in_progress 的 Todo 列表
        let input = serde_json::json!({
            "merge": true,
            "todos": [
                { "id": "1", "status": "completed" },
                { "id": "2", "status": "in_progress" }
            ]
        })
        .to_string();

        // 调用待测函数
        let out = utils::rewrite_todowrite_completed_when_no_work(&input);

        // 解析输出
        let v = serde_json::from_str::<serde_json::Value>(&out).unwrap();
        let todos = v.get("todos").and_then(|x| x.as_array()).unwrap();

        // 断言：第一个 Todo 的 completed 状态被重写为 in_progress
        assert_eq!(todos[0].get("status").and_then(|s| s.as_str()), Some("in_progress"));
        // 断言：第二个 Todo 的 in_progress 状态保持不变
        assert_eq!(todos[1].get("status").and_then(|s| s.as_str()), Some("in_progress"));
    }

    #[test]
    fn sanitize_tool_input_redacts_file_write_content_for_logs() {
        let input = serde_json::json!({
            "path": "demo.md",
            "content": "hello\nworld"
        })
        .to_string();

        let sanitized = utils::sanitize_tool_input("file_write", &input);
        let parsed = serde_json::from_str::<serde_json::Value>(&sanitized)
            .expect("log preview should remain valid JSON");

        assert_eq!(parsed.get("path").and_then(|value| value.as_str()), Some("demo.md"));
        assert_eq!(
            parsed.get("content").and_then(|value| value.as_str()),
            Some("<omitted 11 chars>")
        );
    }

    #[test]
    fn sanitize_tool_input_for_ui_keeps_full_file_write_content() {
        let content = "a".repeat(512);
        let input = serde_json::json!({
            "path": "demo.md",
            "content": content
        })
        .to_string();

        let sanitized = utils::sanitize_tool_input_for_ui("file_write", &input);
        let parsed = serde_json::from_str::<serde_json::Value>(&sanitized)
            .expect("ui preview should remain valid JSON");

        assert_eq!(parsed.get("path").and_then(|value| value.as_str()), Some("demo.md"));
        assert_eq!(
            parsed.get("content").and_then(|value| value.as_str()),
            Some("a".repeat(512).as_str())
        );
    }

    #[test]
    fn sanitize_tool_input_for_ui_keeps_raw_apply_patch_input() {
        let input = concat!(
            "*** Begin Patch\n",
            "*** Update File: /tmp/demo.md\n",
            "@@\n",
            "-old\n",
            "+new\n",
            "*** End Patch"
        );

        let sanitized = utils::sanitize_tool_input_for_ui("apply_patch", input);

        assert_eq!(sanitized, input);
    }

    #[test]
    fn sanitize_tool_input_for_ui_uses_remaining_items_marker() {
        let input = serde_json::json!({
            "messages": (0..25).map(|idx| format!("message-{idx}")).collect::<Vec<_>>()
        })
        .to_string();

        let sanitized = utils::sanitize_tool_input_for_ui("shell", &input);

        assert!(sanitized.contains("_remaining_items"));
        assert!(!sanitized.contains("_omitted_items"));
    }

    /// 测试：检测查询中是否包含工具调用
    ///
    /// # 验证场景
    /// - 包含工具调用的文本（如 `/todoread {}`）应返回 true
    /// - 纯文本（如 "just text"）应返回 false
    ///
    /// # 业务逻辑
    /// `query_has_any_tool_calls` 函数用于快速判断用户输入是否包含工具调用语法，
    /// 通常用于决定是否需要解析工具调用。
    #[test]
    fn query_tool_call_detected() {
        // 准备包含工具调用的测试数据
        let q = r#"
            please do this
            /todoread {}
            "#;

        // 断言：包含工具调用时应返回 true
        assert!(utils::query_has_any_tool_calls(q));
        // 断言：纯文本时应返回 false
        assert!(!utils::query_has_any_tool_calls("just text"));
    }

    /// 测试：parse_tool_at 忽略不完整的 JSON 块
    ///
    /// # 验证场景
    /// - 输入多行文本，其中 JSON 块不完整（缺少闭合括号）
    /// - 期望结果：返回 None，表示无法解析
    ///
    /// # 业务逻辑
    /// `parse_tool_at` 函数应该优雅地处理格式错误的输入，
    /// 而不是 panic 或返回错误结果。
    #[test]
    fn parse_tool_at_ignores_incomplete_json_blocks() {
        // 准备不完整的 JSON 输入（缺少闭合的 }）
        let lines = vec!["/batch", "{", "\"tool_calls\": []"];
        let allowed = allowed_tool_ids(None);

        // 断言：应返回 None
        assert!(utils::parse_tool_at(&lines, 0, &allowed).is_none());
    }

    /// 测试：parse_tool_at 忽略同一行中的无效 JSON
    ///
    /// # 验证场景
    /// - 输入多行文本，其中工具名和 JSON 在同一行，但 JSON 格式无效
    /// - 期望结果：返回 None，表示无法解析
    ///
    /// # 业务逻辑
    /// 验证函数对于同一行中的格式错误 JSON 具有鲁棒性。
    #[test]
    fn parse_tool_at_ignores_invalid_json_on_same_line() {
        // 准备无效的同行 JSON 输入（格式不完整）
        let lines = vec!["/batch {", "}"];
        let allowed = allowed_tool_ids(None);

        // 断言：应返回 None
        assert!(utils::parse_tool_at(&lines, 0, &allowed).is_none());
    }

    /// 测试：parse_tool_at 正确解析同一行中的有效 JSON
    ///
    /// # 验证场景
    /// - 输入单行文本，工具名和完整 JSON 在同一行
    /// - 期望结果：成功解析，返回工具名、输入和消耗的行数
    ///
    /// # 业务逻辑
    /// 验证函数能够正确处理紧凑格式的工具调用。
    #[test]
    fn parse_tool_at_accepts_valid_json_on_same_line() {
        // 准备完整的同行 JSON 输入
        let lines = vec!["/batch {\"tool_calls\":[]}"];
        let allowed = allowed_tool_ids(None);

        // 调用待测函数
        let out = utils::parse_tool_at(&lines, 0, &allowed);

        // 断言：应成功解析
        assert!(out.is_some());

        // 解包解析结果
        let (name, input, consumed) = out.unwrap();

        // 断言：工具名为 "batch"
        assert_eq!(name, "batch");
        // 断言：消耗了 1 行
        assert_eq!(consumed, 1);

        // 验证解析出的 JSON 包含 tool_calls 字段
        let v = serde_json::from_str::<serde_json::Value>(&input).unwrap();
        assert!(v.get("tool_calls").is_some());
    }

    /// 测试：parse_tool_at 在格式错误块后恢复并解析 @mention
    ///
    /// # 验证场景
    /// - 输入多行文本，包含格式错误的 JSON 块和后续的 @mention
    /// - 期望结果：能够恢复并正确解析文件路径引用
    ///
    /// # 业务逻辑
    /// 这是一个重要的容错测试，验证函数能够在遇到格式错误后，
    /// 仍然正确解析后续的文件引用（@mention 语法）。
    /// 这对于处理模型输出中的部分错误非常重要。
    #[test]
    fn parse_tool_at_read_recovers_mention_after_malformed_block() {
        // 准备包含格式错误 JSON 和 @mention 的复杂输入
        // 注意：这里有错误消息片段和正确的文件路径引用
        let lines = vec![
            "/read",
            "[\"error\": \"The read tool was called with invalid arguments: missing required field 'fil\"",
            "Please rewrite the input so it satisfies the expected schema.",
            "\"input\":\"n\"",
            "\"status\":\"completed\"",
            "@src/app/agent/tools/todo.rs",
        ];
        let allowed = allowed_tool_ids(None);

        // 调用待测函数
        let out = utils::parse_tool_at(&lines, 0, &allowed);

        // 断言：应成功解析（容错恢复）
        assert!(out.is_some());

        // 解包解析结果
        let (name, input, consumed) = out.unwrap();

        // 断言：工具名为 "read"
        assert_eq!(name, "read");
        // 断言：输入为文件路径（@ 符号已被提取）
        assert_eq!(input, "src/app/agent/tools/todo.rs");
        // 断言：消耗了 6 行（从 /read 到最后的 @mention）
        assert_eq!(consumed, 6);
    }

    #[test]
    fn inline_tool_execution_appends_tool_context_back_to_llm_messages() {
        let workspace = tempfile::tempdir().expect("temp workspace should be created");
        std::fs::write(workspace.path().join("style.css"), "body { color: black; }\n")
            .expect("style.css should be written");

        let mut session = Session::new("inline-tool-session".to_string());
        let ctx = ToolRuntimeContext::new(
            "inline-tool-session",
            Some(workspace.path().to_string_lossy().to_string()),
        );
        let allowed = allowed_tool_ids(None);
        let mut tool_state = ToolSessionState::default();
        let mut ran_tool = false;
        let start_index = session.messages.len();

        let assistant_text = utils::ingest_assistant_answer(
            &mut session,
            "先读取样式文件\n/file_read {\"path\":\"style.css\"}",
            &ctx,
            &allowed,
            &mut |_event| true,
            &mut ran_tool,
            &mut tool_state,
        );

        assert!(ran_tool);

        let mut llm_messages = Vec::new();
        llm_messages::extend_llm_messages_from_session_range(
            &mut llm_messages,
            &session,
            start_index,
        );

        let inline_text = assistant_text.trim();
        assert_eq!(inline_text, "先读取样式文件");
        if !inline_text.is_empty() {
            session.push(Role::Assistant, inline_text.to_string());
            llm_messages.push(utils::assistant_message_with_reasoning(inline_text, ""));
        }

        assert_eq!(llm_messages.len(), 3);
        assert_eq!(llm_messages[0].get("role").and_then(|value| value.as_str()), Some("assistant"));
        assert!(
            llm_messages[0]
                .get("content")
                .and_then(|value| value.as_str())
                .is_some_and(|content| content.contains("/file_read"))
        );
        assert!(
            llm_messages[1]
                .get("content")
                .and_then(|value| value.as_str())
                .is_some_and(|content| content.contains("tool file_read"))
        );
        assert!(
            llm_messages[2]
                .get("content")
                .and_then(|value| value.as_str())
                .is_some_and(|content| content.contains("先读取样式文件"))
        );
    }

    #[test]
    fn compact_tool_output_for_ui_keeps_file_link_and_removes_hidden_hint() {
        let output = concat!(
            "<file_link>\n",
            "path: docs/agents/README.md\n",
            "open: file:///docs/agents/README.md\n",
            "size_bytes: 1528\n",
            "</file_link>\n",
            "内容已隐藏，点击文件名打开"
        );

        let compacted = utils::compact_tool_output_for_ui("read", output);
        assert!(compacted.contains("<file_link>"));
        assert!(compacted.contains("size_bytes: 1528"));
        assert!(!compacted.contains("内容已隐藏，点击文件名打开"));
    }

    #[test]
    fn structured_tool_calls_are_not_executed_locally_for_acp_requests() {
        assert!(!should_execute_structured_tool_calls_locally(&serde_json::json!({
            "acp_test": true
        })));
        assert!(!should_execute_structured_tool_calls_locally(&serde_json::json!({
            "acp_agent": "codex"
        })));
    }

    #[test]
    fn structured_tool_calls_still_execute_locally_for_non_acp_requests() {
        assert!(should_execute_structured_tool_calls_locally(&serde_json::json!({})));
    }
}
