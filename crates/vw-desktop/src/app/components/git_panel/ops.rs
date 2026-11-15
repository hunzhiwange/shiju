pub use desktop_git_ui::git_panel::ops::*;
use crate::app::App;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn git_repo_path_for_app(app: &App) -> Option<String> {
 let active = app.active_session_id.as_ref().and_then(|id| app.sessions.iter().find(|s| &s.id == id)).map(|s| s.directory.as_str());
 git_repo_path(app.selected_git_worktree_directory.as_deref(), active, app.project_path.as_deref())
}
#[allow(dead_code)]
pub fn get_diff_file_metas(app: &App) -> Vec<DiffFileMeta> {
 super::git_repo_path_for_app(app).map(|path| desktop_git_ui::git_panel::ops::get_diff_file_metas(Some(&path))).unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
pub fn open_terminal(_path: &str) -> Result<(), String> {
    Err("Not supported on Web".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_terminal(path: &str) -> Result<(), std::io::Error> {
    crate::app::terminal::open_local_directory(path)
}
