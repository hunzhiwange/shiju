//! 工具解析兼容入口。
pub use desktop_chat_tools::tools::tool_parse::{
    ExploreToolKind,
    AgentToolSessionStatus,
    AgentToolSessionPayload,
    agent_tool_session_payloads_from_content,
    tool_name_from_raw,
    tool_call_id_from_raw,
    tool_status_from_raw,
    explore_item_dedupe_key,
    explore_tool_kind,
    is_explore_tool,
    tool_result_data,
    tool_render_hint_metadata,
    tool_status,
    tool_input,
    tool_output_text,
    tool_error_text,
    is_acp_tool_payload,
    tool_summary_text,
    tool_output_path,
    tool_change_files,
    tool_change_file_summaries,
    tool_structured_diff_text,
    tool_identity_from_raw,
    should_hide_tool_block,
    tool_input_path,
};

pub fn resolve_output_path(app: &crate::app::App, output_path: &str) -> String {
    desktop_chat_tools::tools::tool_parse::resolve_output_path(app.project_path.as_deref(), output_path)
}
