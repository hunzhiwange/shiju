//! Session Processor 单元测试模块
//!
//! 本模块包含 `processor` 模块的单元测试，主要测试以下功能：
//! - TodoWrite 工具的 completed 状态重写规则
//! - 工具调用检测与解析
//!
//! # 测试覆盖范围
//!
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

    #[test]
    fn parse_tool_at_ignores_incomplete_json_blocks() {
        // 准备不完整的 JSON 输入（缺少闭合的 }）
        let lines = vec!["/batch", "{", "\"tool_calls\": []"];
        let allowed = allowed_tool_ids(None);

        // 断言：应返回 None
        assert!(utils::parse_tool_at(&lines, 0, &allowed).is_none());
    }

    #[test]
    fn parse_tool_at_ignores_invalid_json_on_same_line() {
        // 准备无效的同行 JSON 输入（格式不完整）
        let lines = vec!["/batch {", "}"];
        let allowed = allowed_tool_ids(None);

        // 断言：应返回 None
        assert!(utils::parse_tool_at(&lines, 0, &allowed).is_none());
    }

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






}
