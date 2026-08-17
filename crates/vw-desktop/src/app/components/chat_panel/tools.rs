//! 工具模块 - 聊天面板中的工具视图与解析功能
//!
//! 本模块负责管理和导出聊天面板中使用的各类工具视图与工具解析功能。
//! 工具视图用于在聊天界面中展示和渲染不同类型的工具执行结果，
//! 而工具解析功能则用于识别和处理工具调用相关的元数据。
//!
//! # 模块结构
//!
//! - `apply_patch_view`: 补丁应用工具的视图模块
//! - `bash_view`: Bash 命令执行工具的视图模块
//! - `changes`: 变更追踪工具的相关功能
//! - `diff_utils`: 差异比较工具的辅助函数
//! - `explore_summary_view`: 代码探索摘要工具的视图模块
//! - `files_view`: 文件操作工具的视图模块
//! - `read_view`: 文件读取工具的视图模块
//! - `text_view`: 文本处理工具的视图模块
//! - `todo_views`: 待办事项管理工具的视图模块
//! - `tool_meta`: 工具元数据定义
//! - `tool_parse`: 工具调用解析功能
//! - `types`: 工具相关的类型定义
//!
//! # 主要导出
//!
//! ## 工具视图函数
//!
//! - [`tool_apply_patch_view`]: 补丁应用结果视图
//! - [`tool_bash_view`]: Bash 命令执行结果视图
//! - [`tool_explore_summary_view`]: 代码探索摘要视图
//! - [`tool_files_view`]: 文件操作结果视图
//! - [`tool_read_view`], [`tool_read_compact_view`]: 文件读取结果视图（标准与紧凑两种格式）
//! - [`tool_text_view`]: 文本处理结果视图
//! - [`tool_todos_view`], [`tool_todowrite_compact_view`]: 待办事项视图（标准与紧凑两种格式）
//!
//! ## 工具解析函数
//!
//! - [`is_explore_tool`]: 判断是否为探索类工具
//! - [`should_hide_tool_block`]: 判断是否应隐藏工具块显示
//! - [`tool_identity_from_raw`]: 从原始数据提取工具标识
//! - [`tool_name_from_raw`]: 从原始数据提取工具名称
//!
//! ## 类型定义
//!
//! - [`ChangeFile`]: 表示文件变更的结构体
//! - [`ExploreItem`]: 表示探索项的结构体
//! - [`EXPLORE_GROUP_TOOL_IDX`]: 探索组工具的索引常量

mod advanced_view;
mod apply_patch_preview;
mod apply_patch_view;
mod bash_view;
mod changes;
mod command_summary_view;
mod config_view;
mod diff_utils;
mod explore_summary_view;
mod file_link;
mod files_view;
mod git_diff_view;
mod lsp_view;
mod mcp_view;
mod question_view;
mod read_view;
mod skill_view;
mod text_view;
mod todo_views;
mod tool_detail_dialog;
mod tool_meta;
mod tool_parse;
mod tool_permission;
mod tool_renderer;
mod tool_search_view;
mod types;
mod web_view;
mod workflow_view;

#[cfg(test)]
mod advanced_view_tests;
#[cfg(test)]
mod apply_patch_view_tests;
#[cfg(test)]
mod bash_view_tests;
#[cfg(test)]
mod command_summary_view_tests;
#[cfg(test)]
mod diff_utils_tests;
#[cfg(test)]
mod explore_summary_view_tests;
#[cfg(test)]
mod file_link_tests;
#[cfg(test)]
mod files_view_tests;
#[cfg(test)]
mod git_diff_view_tests;
#[cfg(test)]
mod lsp_view_tests;
#[cfg(test)]
mod mcp_view_tests;
#[cfg(test)]
mod read_view_tests;
#[cfg(test)]
mod skill_view_tests;
#[cfg(test)]
mod text_view_tests;
#[cfg(test)]
mod todo_views_tests;
#[cfg(test)]
mod tool_detail_dialog_tests;
#[cfg(test)]
mod tool_parse_tests;
#[cfg(test)]
mod tool_permission_tests;
#[cfg(test)]
mod tool_renderer_tests;
#[cfg(test)]
mod tool_search_view_tests;
#[cfg(test)]
mod types_tests;
#[cfg(test)]
mod web_view_tests;

// 工具视图函数重导出
pub use crate::app::components::chat_panel::tool_text_support::{
    ToolTextTarget, chat_text_line_height, read_only_text_style, selected_chat_text_for_target,
    tool_inline_text_editor, tool_text_editor, tool_text_key, tool_text_style,
    tool_text_style_with_danger,
};
pub use advanced_view::tool_advanced_view;
pub use apply_patch_view::tool_apply_patch_view;
pub use bash_view::tool_bash_view;
pub(crate) use command_summary_view::{is_command_tool, tool_command_summary_view};
pub use config_view::tool_config_view;
pub(crate) use diff_utils::{extract_diff_block, file_preview};
#[cfg(test)]
pub(crate) use explore_summary_view::explore_summary_expanded;
#[cfg(test)]
pub(crate) use explore_summary_view::explore_summary_is_running;
pub use explore_summary_view::tool_explore_summary_view;
pub use files_view::tool_files_view;
pub use git_diff_view::tool_git_diff_view;
pub use lsp_view::tool_lsp_view;
pub use mcp_view::tool_mcp_view;
#[cfg(test)]
pub(crate) use question_view::question_request_targets_message;
pub use question_view::tool_question_view;
pub use read_view::{tool_read_compact_view, tool_read_view};
pub use skill_view::tool_skill_view;
pub use text_view::tool_text_view;
pub use todo_views::{tool_todos_view, tool_todowrite_compact_view};
pub use tool_detail_dialog::tool_detail_dialog_view;
pub use tool_meta::tool_header_label;
pub(crate) use tool_meta::tool_header_title;
pub(crate) use tool_renderer::render_shared_tool_view;
#[cfg(test)]
pub(crate) use tool_renderer::{SharedToolRenderKind, shared_tool_render_kind};
pub use tool_search_view::tool_search_view;
pub use web_view::tool_web_view;
pub use workflow_view::tool_workflow_view;
pub(crate) use workflow_view::workflow_node_detail_body;

// 工具解析功能重导出
pub use super::tool_names::canonical_tool_name;
pub(crate) use mcp_view::{format_mcp_detail_content, is_mcp_tool_raw};
pub(crate) use tool_meta::tool_inline_summary;
pub(crate) use tool_meta::tool_verb;
pub(crate) use tool_parse::{
    AgentToolSessionPayload, AgentToolSessionStatus, agent_tool_session_payloads_from_content,
    explore_item_dedupe_key, tool_error_text, tool_input, tool_output_text, tool_status,
    tool_status_from_raw, tool_structured_diff_text, tool_summary_text,
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
    tool_permission_target_summary, tool_permission_title,
};

// 类型定义重导出
pub use types::{ChangeFile, ChangeFileSummary, CommandItem, EXPLORE_GROUP_TOOL_IDX, ExploreItem};
