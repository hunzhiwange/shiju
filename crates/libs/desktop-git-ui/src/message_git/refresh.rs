use crate::git_panel::diff_view::GitDiffContextMenuState;
use crate::git_panel::types::GitWorktreeOption;
use crate::git_panel::diff_view::{GitDiffLineRange, GitDiffCommentDraft};
use desktop_workspace::workspace_state::GitDiffSelectedLine;
use std::collections::{HashMap, HashSet};

pub fn select_git_worktree(
    git_worktree_menu_open: &mut bool,
    selected_git_worktree_directory: &mut Option<String>,
    selected_branch: &mut Option<String>,
    git_changed_files: &mut Vec<String>,
    git_changed_files_loading: &mut bool,
    git_changed_files_repo_path: &mut Option<String>,
    git_diff_file_metas: &mut Vec<crate::git_panel::ops::DiffFileMeta>,
    git_diff_file_metas_loading: &mut bool,
    git_diff_file_metas_repo_path: &mut Option<String>,
    git_diff_contents: &mut HashMap<String, (String, String)>,
    git_diff_contents_loading: &mut HashSet<String>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
    git_diff_comment_draft: &mut Option<GitDiffCommentDraft>,
    option: GitWorktreeOption,
) {
    (*git_worktree_menu_open) = false;
    (*selected_git_worktree_directory) = Some(option.directory.clone());
    (*selected_branch) = option.branch.clone();
    (*git_changed_files).clear();
    (*git_changed_files_loading) = false;
    (*git_changed_files_repo_path) = None;
    (*git_diff_file_metas).clear();
    (*git_diff_file_metas_loading) = false;
    (*git_diff_file_metas_repo_path) = None;
    (*git_diff_contents).clear();
    (*git_diff_contents_loading).clear();
    (*git_diff_selected_lines).clear();
    (*git_diff_selected_range) = None;
    (*git_diff_comment_draft) = None;
}

pub fn toggle_git_worktree_menu(git_worktree_menu_open: &mut bool, open: bool) {
    (*git_worktree_menu_open) = open;
}

pub fn same_directory(left: &str, right: &str) -> bool {
    left.replace('\\', "/").trim_end_matches('/') == right.replace('\\', "/").trim_end_matches('/')
}

pub fn prepare_worktrees(
    project_path: &mut Option<String>,
    git_worktree_options: &mut Vec<GitWorktreeOption>,
    selected_git_worktree_directory: &mut Option<String>,
    git_worktree_options_loading: &mut bool,
    git_worktree_options_project_path: &mut Option<String>,
    git_worktree_menu_open: &mut bool,
) -> Option<String> {

            let Some(project_path) = (*project_path).clone() else {
                (*git_worktree_options).clear();
                (*selected_git_worktree_directory) = None;
                (*git_worktree_options_loading) = false;
                (*git_worktree_options_project_path) = None;
                (*git_worktree_menu_open) = false;
                return None;
            };

            #[cfg(not(target_arch = "wasm32"))]
            if git2::Repository::discover(&project_path).is_err() {
                (*git_worktree_options).clear();
                (*selected_git_worktree_directory) = None;
                (*git_worktree_options_loading) = false;
                (*git_worktree_options_project_path) = None;
                (*git_worktree_menu_open) = false;
                return None;
            }

            if (*git_worktree_options_loading)
                && (*git_worktree_options_project_path).as_deref() == Some(project_path.as_str())
            {
                return None;
            }

            (*git_worktree_options_loading) = true;
            (*git_worktree_options_project_path) = Some(project_path.clone());

    Some(project_path)
}

pub fn prepare_changed_files(
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_changed_files_loading: &mut bool,
    git_changed_files: &mut Vec<String>,
    git_changed_files_repo_path: &mut Option<String>,
    file_manager_changes_refreshing: &mut bool,
    repo_path: Option<String>,
) -> Option<String> {

            (*git_diff_context_menu) = None;
            if (*git_changed_files_loading) {
                return None;
            }
            let Some(path) = repo_path.clone() else {
                (*git_changed_files).clear();
                (*git_changed_files_loading) = false;
                (*git_changed_files_repo_path) = None;
                (*file_manager_changes_refreshing) = false;
                return None;
            };

            (*git_changed_files_loading) = true;
            (*git_changed_files_repo_path) = Some(path.clone());
            // 耗时或平台相关操作交给异步任务，避免阻塞界面消息循环。

    Some(path)
}

pub fn prepare_diff_metas(
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_diff_file_metas: &mut Vec<crate::git_panel::ops::DiffFileMeta>,
    git_diff_file_metas_loading: &mut bool,
    git_diff_file_metas_repo_path: &mut Option<String>,
    git_diff_contents: &mut HashMap<String, (String, String)>,
    git_diff_contents_loading: &mut HashSet<String>,
    repo_path: Option<String>,
) -> Option<String> {

            (*git_diff_context_menu) = None;
                        let Some(path) = repo_path.clone() else {
                (*git_diff_file_metas).clear();
                (*git_diff_file_metas_loading) = false;
                (*git_diff_file_metas_repo_path) = None;
                (*git_diff_contents).clear();
                (*git_diff_contents_loading).clear();
                return None;
            };

            if (*git_diff_file_metas_loading)
                && (*git_diff_file_metas_repo_path).as_deref() == Some(path.as_str())
            {
                return None;
            }

            (*git_diff_file_metas_loading) = true;
            (*git_diff_file_metas_repo_path) = Some(path.clone());

    Some(path)
}

pub fn apply_worktrees(
    git_worktree_options_project_path: &mut Option<String>,
    git_worktree_options_loading: &mut bool,
    git_worktree_options: &mut Vec<GitWorktreeOption>,
    selected_git_worktree_directory: &mut Option<String>,
    selected_branch: &mut Option<String>,
    git_worktree_menu_open: &mut bool,
    project_path: String,
    result: Result<Vec<GitWorktreeOption>, String>,
    current_directory: Option<String>,
) -> Option<String> {

            if (*git_worktree_options_project_path).as_deref() != Some(project_path.as_str()) {
                return None;
            }
            (*git_worktree_options_loading) = false;
            match result {
                Ok(options) => {
                    (*git_worktree_options) = options;
                    (*selected_git_worktree_directory) = current_directory
                        .as_ref()
                        .and_then(|directory| {
                            (*git_worktree_options)
                                .iter()
                                .find(|option| same_directory(&option.directory, directory))
                        })
                        .or_else(|| (*git_worktree_options).first())
                        .map(|option| option.directory.clone());
                    if let Some(selected) =
                        (*selected_git_worktree_directory).as_ref().and_then(|directory| {
                            (*git_worktree_options)
                                .iter()
                                .find(|option| option.directory.as_str() == directory.as_str())
                        })
                    {
                        (*selected_branch) = selected.branch.clone();
                    }
                }
                Err(error) => {
                    (*git_worktree_options).clear();
                    (*selected_git_worktree_directory) = None;
                    (*git_worktree_menu_open) = false;
                    return Some(error);
                }
            }
            None
}

pub fn apply_changed_files(
    git_changed_files_repo_path: &mut Option<String>,
    git_changed_files: &mut Vec<String>,
    git_changed_files_loading: &mut bool,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
    git_diff_comment_draft: &mut Option<GitDiffCommentDraft>,
    repo_path: Option<String>,
    files: Vec<String>,
) -> bool {

            if (*git_changed_files_repo_path) != repo_path {
                return false;
            }
            (*git_changed_files) = files;
            (*git_changed_files_loading) = false;
            (*git_diff_selected_lines).clear();
            (*git_diff_selected_range) = None;
            (*git_diff_comment_draft) = None;

true
}

pub fn apply_diff_metas(
    git_diff_file_metas_repo_path: &mut Option<String>,
    git_diff_file_metas: &mut Vec<crate::git_panel::ops::DiffFileMeta>,
    git_diff_file_metas_loading: &mut bool,
    git_diff_contents: &mut HashMap<String, (String, String)>,
    git_diff_contents_loading: &mut HashSet<String>,
    repo_path: Option<String>,
    metas: Vec<crate::git_panel::ops::DiffFileMeta>,
) -> bool {

            if (*git_diff_file_metas_repo_path) != repo_path {
                return false;
            }
            (*git_diff_file_metas) = metas;
            (*git_diff_file_metas_loading) = false;
            (*git_diff_contents).clear();
            (*git_diff_contents_loading).clear();
            true
}

pub fn apply_diff_content(
    git_diff_file_metas_repo_path: &mut Option<String>,
    git_diff_contents_loading: &mut HashSet<String>,
    git_diff_file_metas: &mut Vec<crate::git_panel::ops::DiffFileMeta>,
    git_diff_contents: &mut HashMap<String, (String, String)>,
    repo_path: Option<String>,
    file: String,
    old_content: String,
    new_content: String,
) -> bool {

            if (*git_diff_file_metas_repo_path) != repo_path {
                return true;
            }
            (*git_diff_contents_loading).remove(&file);
            if !(*git_diff_file_metas).iter().any(|meta| meta.path == file) {
                return false;
            }
            (*git_diff_contents).insert(file, (old_content, new_content));
            false
}

pub enum DiffContentLoad {
    None,
    RefreshMetas,
    Load { repo_path: String, meta: crate::git_panel::ops::DiffFileMeta },
}
pub fn prepare_diff_content(contents: &HashMap<String, (String, String)>, loading: &mut HashSet<String>,
    metas_repo_path: Option<&str>, metas: &[crate::git_panel::ops::DiffFileMeta],
    repo_path: Option<String>, file: &str) -> DiffContentLoad {
    if contents.contains_key(file) || loading.contains(file) { return DiffContentLoad::None; }
    let Some(repo_path) = repo_path else { return DiffContentLoad::None; };
    if metas_repo_path != Some(repo_path.as_str()) { return DiffContentLoad::RefreshMetas; }
    let Some(meta) = metas.iter().find(|meta| meta.path == file).cloned() else { return DiffContentLoad::None; };
    loading.insert(file.to_owned());
    DiffContentLoad::Load { repo_path, meta }
}
pub fn reset_loading_after_mutation(changed_loading: &mut bool, changed_repo_path: &mut Option<String>,
    metas_loading: &mut bool, metas_repo_path: &mut Option<String>, contents_loading: &mut HashSet<String>) {
    *changed_loading = false;
    *changed_repo_path = None;
    *metas_loading = false;
    *metas_repo_path = None;
    contents_loading.clear();
}

#[cfg(test)]
#[path = "refresh_tests.rs"]
mod tests;
