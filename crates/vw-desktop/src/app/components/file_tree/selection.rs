pub use desktop_file_tree::selection::FileTreeSelection;
use crate::app::App;
pub(crate) fn visible_paths(app: &App, root: &str) -> Vec<String> {
    let fallback;
    let tree = match app.current_file_tree_model().filter(|tree| tree.has_entries()) {
        Some(tree) => tree,
        None => { fallback = super::model::build_file_tree_model(root, app.current_file_index()); &fallback }
    };
    desktop_file_tree::selection::visible_paths(tree, root, &app.file_tree_expanded_set)
}
