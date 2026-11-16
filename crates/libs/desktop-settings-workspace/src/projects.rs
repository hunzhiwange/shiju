//! Recent project list mutations; persistence and navigation are shell effects.
#[derive(Debug, Clone)]
pub enum Event {
    QueryChanged(String),
    DeleteRequested(usize),
    DeleteCanceled,
    DeleteConfirmed(usize),
}
pub struct State<'a> {
    pub query: &'a mut String,
    pub tools_menu_path: &'a mut Option<String>,
    pub delete_confirm_idx: &'a mut Option<usize>,
    pub paths: &'a mut Vec<String>,
    pub edits: &'a mut Vec<String>,
}
pub struct DeletedProject {
    pub path: String,
    pub was_current: bool,
}
pub fn apply(state: State<'_>, event: Event, current_path: Option<&str>) -> Option<DeletedProject> {
    match event {
        Event::QueryChanged(query) => {
            *state.query = query;
            *state.tools_menu_path = None;
            *state.delete_confirm_idx = None;
        }
        Event::DeleteRequested(index) => {
            *state.tools_menu_path = None;
            *state.delete_confirm_idx = Some(index);
        }
        Event::DeleteCanceled => *state.delete_confirm_idx = None,
        Event::DeleteConfirmed(index) => {
            *state.delete_confirm_idx = None;
            if index < state.paths.len() {
                let path = state.paths.remove(index);
                if index < state.edits.len() {
                    state.edits.remove(index);
                }
                let was_current = current_path == Some(path.as_str());
                return Some(DeletedProject { path, was_current });
            }
        }
    }
    None
}

/// Remove project metadata and rebuild display names from authoritative rows.
pub fn reconcile_deleted(
    paths: &[String],
    edits: &mut Vec<String>,
    metadata: &mut Vec<desktop_workspace::project_message::edit_meta::RecentProjectMeta>,
    path: &str,
) {
    metadata.retain(|meta| meta.path != path);
    *edits = paths
        .iter()
        .map(|path| {
            metadata
                .iter()
                .find(|meta| &meta.path == path)
                .map(|meta| meta.name.clone())
                .unwrap_or_else(|| {
                    std::path::Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(path)
                        .to_string()
                })
        })
        .collect();
}
pub fn set_worktree_enabled(
    settings: &mut std::collections::HashMap<String, bool>,
    path: &str,
    enabled: bool,
) -> Option<serde_json::Value> {
    let changed = if enabled {
        settings.insert(path.to_owned(), true) != Some(true)
    } else {
        settings.remove(path).is_some()
    };
    changed.then(|| {
        serde_json::json!({ "project_worktree_enabled": {
            path: if enabled { serde_json::Value::Bool(true) } else { serde_json::Value::Null }
        }})
    })
}
