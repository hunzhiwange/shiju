//! 工具解析兼容入口。
pub use desktop_chat_tools::tools::tool_parse::{
    ExploreToolKind,
    tool_name_from_raw,
    tool_call_id_from_raw,
    explore_tool_kind,
    is_explore_tool,
    tool_identity_from_raw,
    should_hide_tool_block,
};

#[cfg(test)]
pub fn resolve_output_path(app: &crate::app::App, output_path: &str) -> String {
    desktop_chat_tools::tools::tool_parse::resolve_output_path(app.navigation_tabs.project_path.as_deref(), output_path)
}

#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::AgentToolSessionPayload;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::AgentToolSessionStatus;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::agent_tool_session_payloads_from_content;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::explore_item_dedupe_key;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::is_acp_tool_payload;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_change_file_summaries;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_change_files;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_error_text;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_input;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_input_path;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_output_path;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_output_text;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_render_hint_metadata;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_result_data;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_status;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_status_from_raw;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_structured_diff_text;
#[cfg(test)]
pub use desktop_chat_tools::tools::tool_parse::tool_summary_text;
