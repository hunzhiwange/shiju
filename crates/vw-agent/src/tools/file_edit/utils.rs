//! 共享文件编辑辅助函数与当前宿主上下文特化。
pub use tool_files::file_edit::utils::*;
use std::path::Path;
use tool_context::read_state::FileReadStateEntry;
pub fn read_state_metadata_for_path(workspace_dir: &Path, full: &Path, action_label: &str) -> Option<serde_json::Value> {
    tool_files::file_edit::utils::read_state_metadata_for_path::<super::super::context::AgentContextHost>(workspace_dir, full, action_label)
}
pub fn require_read_state_for_existing_file(full: &Path, display_path: &str, action_label: &str) -> anyhow::Result<FileReadStateEntry> {
    tool_files::file_edit::utils::require_read_state_for_existing_file::<super::super::context::AgentContextHost>(full, display_path, action_label)
}
