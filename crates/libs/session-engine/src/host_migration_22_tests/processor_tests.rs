use crate::helpers::should_execute_structured_tool_calls_locally;
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
        let out = crate::tool_output::rewrite_todowrite_completed_when_no_work(&input);

        // 解析输出
        let v = serde_json::from_str::<serde_json::Value>(&out).unwrap();
        let todos = v.get("todos").and_then(|x| x.as_array()).unwrap();

        // 断言：第一个 Todo 的 completed 状态被重写为 pending
        assert_eq!(todos[0].get("status").and_then(|s| s.as_str()), Some("pending"));
        // 断言：第二个 Todo 的 pending 状态保持不变
        assert_eq!(todos[1].get("status").and_then(|s| s.as_str()), Some("pending"));
    }

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
        let out = crate::tool_output::rewrite_todowrite_completed_when_no_work(&input);

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

        let sanitized = crate::tool_output::sanitize_tool_input("file_write", &input);
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

        let sanitized = crate::tool_output::sanitize_tool_input_for_ui("file_write", &input);
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

        let sanitized = crate::tool_output::sanitize_tool_input_for_ui("apply_patch", input);

        assert_eq!(sanitized, input);
    }

#[test]
fn sanitize_tool_input_for_ui_uses_remaining_items_marker() {
        let input = serde_json::json!({
            "messages": (0..25).map(|idx| format!("message-{idx}")).collect::<Vec<_>>()
        })
        .to_string();

        let sanitized = crate::tool_output::sanitize_tool_input_for_ui("shell", &input);

        assert!(sanitized.contains("_remaining_items"));
        assert!(!sanitized.contains("_omitted_items"));
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

        let compacted = crate::tool_output::compact_tool_output_for_ui("read", output);
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
