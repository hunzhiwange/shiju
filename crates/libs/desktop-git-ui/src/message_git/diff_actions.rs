//! Git 展开、布局状态与单项丢弃操作。
#[derive(Debug, Clone, Copy)]
pub enum ExpandDirection { Up, Down, All }
pub enum DiscardAction { File(String), Hunk(String, usize), DeleteLine(String, usize), RestoreLine(String, usize, usize) }

pub fn is_diff_file_expanded(expanded_files_set: &std::collections::HashSet<String>, path: &str) -> bool {
    (*expanded_files_set).contains(path)
}

pub fn ensure_diff_file_expanded(
    expanded_files_set: &mut std::collections::HashSet<String>,
    expanded_files: &mut Vec<String>,
    path: String,
) {
    if (*expanded_files_set).insert(path.clone()) {
        (*expanded_files).push(path);
    }
}

pub fn clear_expanded_files(
    expanded_files: &mut Vec<String>,
    expanded_files_set: &mut std::collections::HashSet<String>,
) {
    (*expanded_files).clear();
    (*expanded_files_set).clear();
}

pub fn set_single_expanded_file(
    expanded_files: &mut Vec<String>,
    expanded_files_set: &mut std::collections::HashSet<String>,
    path: String,
) {
    (*expanded_files).clear();
    (*expanded_files_set).clear();
    (*expanded_files_set).insert(path.clone());
    (*expanded_files).push(path);
}

pub fn toggle_expand_file(
    expanded_files: &mut Vec<String>,
    expanded_files_set: &mut std::collections::HashSet<String>,
    name: String,
) -> bool {
    if is_diff_file_expanded(expanded_files_set, &name) {
        clear_expanded_files(expanded_files, expanded_files_set);
        false
    } else {
        set_single_expanded_file(expanded_files, expanded_files_set, name);
        true
    }
}

pub fn expand_context(
    expansions: &mut std::collections::HashMap<(String, usize), (usize, usize)>,
    file: String,
    gap_idx: usize,
    direction: ExpandDirection,
) {
    let entry = expansions.entry((file, gap_idx)).or_insert((0, 0));
    match direction {
        ExpandDirection::Down => entry.0 += 20,
        ExpandDirection::Up => entry.1 += 20,
        ExpandDirection::All => { entry.0 = usize::MAX / 2; entry.1 = usize::MAX / 2; }
    }
}

pub fn scroll_changed(offset: &mut f32, viewport: &mut f32, offset_y: f32, viewport_h: f32) {
    *offset = offset_y.clamp(0.0, 1.0);
    *viewport = viewport_h.max(0.0);
}

pub fn toggle_fullscreen(full: &mut bool, half: &mut bool, half_mode: bool) -> bool {
    if half_mode { *half = !*half; *full = false; *half }
    else { *full = !*full; *half = false; *full }
}

pub fn discard(repo_path: Option<&str>, shell: desktop_git::Shell, action: DiscardAction) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = repo_path {
        // 保留既有行为：单项丢弃失败后仍刷新仓库数据。
        let _ = match action {
            DiscardAction::File(file) => desktop_git::git_discard_file(path, &file),
            DiscardAction::Hunk(file, index) => desktop_git::git_discard_hunk(path, &file, index, shell),
            DiscardAction::DeleteLine(file, line) => desktop_git::git_revert_line_delete(path, &file, line),
            DiscardAction::RestoreLine(file, new_line, old_line) => desktop_git::git_revert_line_restore(path, &file, new_line, old_line),
        };
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (repo_path, shell, action);
}
