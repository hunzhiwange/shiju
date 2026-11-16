// 本文件验证渠道输出清理、内部进度识别与投递提示生成。

use super::*;
use crate::app::agent::tools::{ToolResult, ToolSpec};
use std::collections::HashSet;

struct NamedTool {
    id: &'static str,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.id
    }

    fn description(&self) -> &str {
        "test tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: String::new(), error: None })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.id, self.description(), self.parameters_schema())
    }
}

#[test]
fn split_internal_progress_delta_detects_known_prefix() {
    let delta = format!("{}working", crate::app::agent::agent::loop_::DRAFT_PROGRESS_SENTINEL);
    let (is_progress, content) = split_internal_progress_delta(&delta);

    assert!(is_progress);
    assert_eq!(content, "working");
}

#[test]
fn split_internal_progress_delta_hides_ws_event_payloads_and_preserves_normal_text() {
    let ws_delta = format!(
        "{}{}payload",
        crate::app::agent::agent::loop_::DRAFT_PROGRESS_SENTINEL,
        crate::app::agent::agent::loop_::DRAFT_WS_EVENT_SENTINEL
    );

    assert_eq!(split_internal_progress_delta(&ws_delta), (true, ""));
    assert_eq!(split_internal_progress_delta("plain"), (false, "plain"));
}

#[test]
fn sanitize_channel_response_keeps_only_visible_answer_from_execution_transcript() {
    let tools = Vec::<Box<dyn Tool>>::new();
    let response = concat!(
        "<think>The user asks who are you. Let me inspect the workspace.</think>\n",
        "我先读一下工作区里的身份文件。\n",
        "tool bash\n",
        "{\"status\":\"running\",\"input\":\"ls\",\"output\":\"\"}\n",
        "tool bash\n",
        "{\"status\":\"completed\",\"input\":\"ls\",\"output\":\"AGENTS.md\"}\n",
        "我是 VibeWindow 助手，可以帮你处理项目和日常任务。"
    );

    assert_eq!(
        sanitize_channel_response(response, &tools),
        "我是 VibeWindow 助手，可以帮你处理项目和日常任务。"
    );
}

#[test]
fn sanitize_channel_response_rejects_execution_transcript_without_final_answer() {
    let tools = Vec::<Box<dyn Tool>>::new();
    let response = concat!(
        "准备读取文件。\n",
        "tool file_read\n",
        "{\"status\":\"completed\",\"output\":\"secret internal context\"}\n"
    );

    assert!(sanitize_channel_response(response, &tools).is_empty());
}
