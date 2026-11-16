pub use crate::git_panel::ops::*;
use crate::page::ViewContext;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn git_repo_path_for_app(app: &ViewContext) -> Option<String> {
    app.repo_path.clone()
}
#[allow(dead_code)]
pub fn get_diff_file_metas(app: &ViewContext) -> Vec<DiffFileMeta> {
    super::git_repo_path_for_app(app)
        .map(|path| crate::git_panel::ops::get_diff_file_metas(Some(&path)))
        .unwrap_or_default()
}
