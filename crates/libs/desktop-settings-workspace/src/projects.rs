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
