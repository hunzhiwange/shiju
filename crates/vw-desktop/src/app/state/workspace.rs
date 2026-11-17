pub use desktop_git_ui::git_panel::types::GitWorktreeOption;





pub use desktop_workspace::interaction::FileTreeClipboardMode;






/// 使用量步骤原始 LLM 载荷弹窗状态
///
/// 在步骤展开后点击“查看原始记录”时展示完整可复制内容。
pub use desktop_usage::view_types::UsageModelInfo;

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;

#[cfg(test)]
pub use desktop_file_tree::state::FIND_RESULTS_PAGE_SIZE;
#[cfg(test)]
pub use desktop_file_tree::state::FindInFolderMatch;
#[cfg(test)]
pub use desktop_file_tree::state::FindInFolderTab;
#[cfg(test)]
pub use desktop_file_tree::state::find_results_page_bounds;
#[cfg(test)]
pub use desktop_task_board::message::TaskBoardDetailTab;
#[cfg(test)]
pub use desktop_task_board::message::TaskBoardSettingsModalTab;
#[cfg(test)]
pub use desktop_usage::view_types::UsageStepRawModal;
#[cfg(test)]
pub use desktop_workspace::interaction::FileTreeClipboard;
#[cfg(test)]
pub use desktop_workspace::project_message::edit::ProjectEditTab;
#[cfg(test)]
pub use desktop_workspace::workspace_state::ChatTextDiff;
#[cfg(test)]
pub use desktop_workspace::workspace_state::GitDiffFileMenuState;
#[cfg(test)]
pub use desktop_workspace::workspace_state::GitDiffSelectedLine;
#[cfg(test)]
pub(crate) use desktop_git_ui::git_panel::diff_view::GitDiffCommentDraft;
#[cfg(test)]
pub(crate) use desktop_git_ui::git_panel::diff_view::GitDiffContextMenuState;
#[cfg(test)]
pub(crate) use desktop_git_ui::git_panel::diff_view::GitDiffLineRange;
