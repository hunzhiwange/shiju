//! 聊天面板工具函数模块
//!
//! 本模块按职责拆分聊天面板辅助函数，并保持原有对外导出不变。

mod app_state;
mod path;
mod text;
mod theme;
mod time;
mod ui;

#[cfg(test)]
#[path = "app_state_tests.rs"]
mod app_state_tests;
#[cfg(test)]
#[path = "path_tests.rs"]
mod path_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "text_tests.rs"]
mod text_tests;
#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
#[cfg(test)]
#[path = "time_tests.rs"]
mod time_tests;
#[cfg(test)]
#[path = "ui_tests.rs"]
mod ui_tests;

pub use app_state::{
    current_branch_label, current_project_path_label, get_session_title, is_recent_copy,
};
pub use path::{
    normalize_file_reference_to_path, normalize_file_url_to_path, relative_to_project_root,
    resolve_path,
};
pub use text::{
    normalize_display_text, single_line_preview, strip_internal_tool_trace, truncate_chars,
    truncate_lines_middle,
};
pub use theme::{
    additions_pill, change_pills, chat_scroll_direction, chat_secondary_muted_text_color,
    chat_secondary_subtle_text_color, chat_secondary_text_color, deletions_pill,
    eye_icon_button_style, eye_icon_svg_style, file_button_style, icon_button_style, icon_svg,
    is_dark_theme, mix_color, muted_icon_color, pill_button_style, simplified_block_style,
    simplified_code_block_style, weak_file_button_style,
};
pub use time::{
    format_chat_time_label, project_last_modified_ms, relative_modified_label,
    relative_time_bucket, relative_time_label, relative_time_label_for_bucket,
};
pub use ui::{
    bold_font, capped_scroll_height, chat_context_menu, chat_context_target_key,
    copy_tooltip_content,
};
