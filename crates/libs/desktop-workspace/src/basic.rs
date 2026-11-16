//! Platform project pickers; desktop owns opening the selected resource.
pub async fn pick_file() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = rfd::AsyncFileDialog::new().pick_file().await;
        handle.map(|file| file.path().to_string_lossy().to_string())
    }
    #[cfg(target_arch = "wasm32")]
    { None }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn pick_attachments() -> Option<Vec<String>> {
    rfd::AsyncFileDialog::new().pick_files().await.map(|handles| {
        handles.into_iter().map(|file| file.path().to_string_lossy().to_string()).collect()
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn pick_folder() -> Option<String> {
    rfd::FileDialog::new().pick_folder()?.to_str().map(str::to_string)
}

pub fn project_path_to_open(input: &str) -> Option<String> {
    if !input.is_empty() { return Some(input.to_string()); }
    #[cfg(not(target_arch = "wasm32"))]
    { std::env::current_dir().ok()?.to_str().map(str::to_string) }
    #[cfg(target_arch = "wasm32")]
    { None }
}

pub fn file_mention(root: &str, path: &str, line: usize, column: usize) -> Option<String> {
    let relative = std::path::Path::new(path).strip_prefix(root).ok()?;
    let relative = relative.to_string_lossy().replace('\\', "/");
    Some(format!("@{relative}:{line}:{column} "))
}

pub fn display_name_for_path(meta: &[crate::project_message::edit_meta::RecentProjectMeta], path: &str) -> String {
    if let Some(item) = meta.iter().find(|item| item.path == path) {
        return item.name.clone();
    }

    std::path::Path::new(path)
        .file_name()
        .and_then(|segment| segment.to_str())
        .unwrap_or(path)
        .to_string()
}


#[cfg(test)]
#[path = "basic_init_tests.rs"]
mod init_tests;

pub fn search_panel_file_results(files: &[String], query: &str) -> Vec<String> {
    files.iter().filter(|path| path.to_ascii_lowercase().contains(query)).take(8).cloned().collect()
}

/// Reuse the cached projection until its query, project or file index changes.
pub fn refresh_search_panel_file_results(
    files: &[String], query: &str, project: Option<&str>, revision: u64,
    cached_query: &str, cached_project: Option<&str>, cached_revision: u64,
) -> Option<Vec<String>> {
    if cached_query == query && cached_project == project && cached_revision == revision {
        return None;
    }
    Some(search_panel_file_results(files, query))
}

#[derive(Debug, Default)]
pub struct InputState {
    pub project_path_input: String,
    pub file_url_input: String,
}
/// Local project input and attachment-list transitions; the host owns chat rendering.
pub fn handle_input<P, H, U>(state: &mut InputState, files: &mut Vec<String>, message: &crate::project_message::message::ProjectMessage<P, H, U>) -> bool {
    use crate::project_message::message::ProjectMessage as M;
    match message {
        M::ProjectPathChanged(value) => state.project_path_input = value.clone(),
        M::FileUrlChanged(value) => state.file_url_input = value.clone(),
        M::AddFilePressed => {
            if !state.file_url_input.is_empty() {
                files.push(state.file_url_input.clone());
                state.file_url_input.clear();
            }
        }
        M::RemoveAttachedFile(path) => files.retain(|entry| entry != path),
        _ => return false,
    }
    true
}
