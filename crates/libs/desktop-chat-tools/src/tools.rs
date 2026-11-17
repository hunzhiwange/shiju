pub mod diff_utils;
pub mod todo_parse;
pub mod tool_parse;
pub mod types;
pub mod changes;
pub mod files_view;
#[cfg(test)]
mod todo_parse_tests;
#[cfg(test)]
mod tool_parse_tests;
#[cfg(test)]
mod types_tests;
pub mod advanced_view;
pub mod tool_meta;
pub mod tool_permission;
pub use crate::tool_names::canonical_tool_name;
pub use tool_permission::{tool_permission_state, localized_tool_permission_error_text, localized_tool_permission_title};
pub use tool_meta::localized_tool_inline_summary;
#[cfg(test)]
mod advanced_view_tests;
#[cfg(test)]
mod tool_permission_tests;

pub mod apply_patch_preview;
pub mod apply_patch_view;
pub mod bash_view;
pub mod command_summary_view;
pub mod config_view;
pub mod file_link;
mod apply_patch_card;

pub mod files_input;
pub mod explore_input;
pub mod explore_summary_view;
#[cfg(test)]
mod explore_summary_view_tests;
#[cfg(test)]
mod files_view_tests;
#[cfg(test)]
pub(crate) mod view_fixture;

#[cfg(test)]
mod file_link_tests;

mod summary_animation;

pub mod result_input;
pub mod git_diff_view;
#[cfg(test)]
mod git_diff_view_tests;
pub mod lsp_view;
#[cfg(test)]
mod lsp_view_tests;
pub mod mcp_view;
#[cfg(test)]
mod mcp_view_tests;
pub mod question_view;
#[cfg(test)]
mod question_view_tests;
#[cfg(test)]
mod result_fixture;

pub mod read_view;
pub mod skill_view;
pub mod text_view;
#[cfg(test)]
mod read_view_tests;
#[cfg(test)]
mod skill_view_tests;
#[cfg(test)]
mod text_view_tests;
#[cfg(test)]
mod config_card_tests;

pub mod todo_views;
pub mod detail_input;
pub mod tool_detail_dialog;
#[cfg(test)]
mod todo_views_tests;
#[cfg(test)]
mod tool_detail_dialog_tests;
#[cfg(test)]
mod todo_fixture;
#[cfg(test)]
mod detail_fixture;

pub mod web_view;
pub mod workflow_view;
pub mod tool_renderer;
#[cfg(test)]
mod web_view_tests;
#[cfg(test)]
mod tool_renderer_tests;
