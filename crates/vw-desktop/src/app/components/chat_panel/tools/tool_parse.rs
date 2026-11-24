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
