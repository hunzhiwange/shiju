//! 文件解析兼容入口。
pub(crate) use desktop_chat_tools::tools::files_view::parse::{is_git_diff_tool, should_skip_files_view, is_edit_like_tool, is_search_tool, parse_read_range, build_file_list_state};

pub(crate) fn parse_output_files(
    app: &crate::app::App, tool_name: &str, input: &str, output: &str, value: &serde_json::Value,
) -> (std::collections::HashMap<String, super::super::types::ChangeFile>, Vec<(String, String)>) {
    desktop_chat_tools::tools::files_view::parse::parse_output_files(
        app.project_path.as_deref(), tool_name, input, output, value,
    )
}
