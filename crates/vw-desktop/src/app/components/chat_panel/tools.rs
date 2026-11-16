//! Tool domain exports and desktop cross-domain view ports.
mod advanced_view;
mod card_adapter;
mod command_summary_view;
mod explore_summary_view;
mod git_diff_view;
mod tool_detail_dialog;
mod tool_meta;
mod tool_parse;
mod tool_permission;
mod tool_renderer;
mod types;
mod workflow_view;

// 工具视图函数重导出
pub use crate::app::components::chat_panel::tool_text_support::{
    ToolTextTarget, chat_text_line_height, read_only_text_style, selected_chat_text_for_target,
    tool_inline_text_editor, tool_text_editor, tool_text_key, tool_text_style,
    tool_text_style_with_danger,
};
pub(crate) use command_summary_view::{is_command_tool, tool_command_summary_view};
#[cfg(test)]
pub(crate) use explore_summary_view::explore_summary_expanded;
#[cfg(test)]
pub(crate) use explore_summary_view::explore_summary_is_running;
pub use explore_summary_view::tool_explore_summary_view;
pub use tool_detail_dialog::tool_detail_dialog_view;
pub use tool_meta::tool_header_label;
pub(crate) use tool_renderer::render_shared_tool_view;
pub use workflow_view::tool_workflow_view;
pub(crate) use workflow_view::workflow_node_detail_body;

// 工具解析功能重导出
pub use super::tool_names::canonical_tool_name;
pub(crate) use tool_meta::{
    ToolVisual, ToolVisualState, tool_inline_summary, tool_inline_summary_unbounded, tool_verb,
    tool_visual_state,
};
pub(crate) use tool_parse::{
    AgentToolSessionPayload, AgentToolSessionStatus, agent_tool_session_payloads_from_content,
    explore_item_dedupe_key, is_acp_tool_payload, tool_error_text, tool_input, tool_output_text,
    tool_status, tool_status_from_raw, tool_structured_diff_text, tool_summary_text,
};
pub use tool_parse::{
    ExploreToolKind, explore_tool_kind, is_explore_tool, should_hide_tool_block,
    tool_call_id_from_raw, tool_identity_from_raw, tool_name_from_raw,
};
pub(crate) use tool_permission::{
    ToolPermissionState, pending_permission_badge_label, pending_permission_request_badge_label,
    pending_permission_request_for_message, pending_permission_request_for_tool_call,
    pending_permission_targets_message, pending_permission_targets_tool_call,
    tool_permission_error_text, tool_permission_state, tool_permission_summary,
    tool_permission_target_summary,
};

// 类型定义重导出
pub use types::{ChangeFile, ChangeFileSummary, CommandItem, EXPLORE_GROUP_TOOL_IDX, ExploreItem};

pub(crate) use tool_permission::{
    localized_tool_permission_error_text, localized_tool_permission_title,
};

pub(crate) use tool_meta::{
    localized_tool_inline_summary, localized_tool_inline_summary_unbounded,
};

pub(crate) use desktop_chat_tools::tools::mcp_view::is_mcp_tool_raw;
