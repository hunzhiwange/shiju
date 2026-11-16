pub mod model;
pub mod icons;
pub mod inline_rename;
pub mod menu;
#[cfg(target_os = "macos")]
pub mod native_drop;
pub mod row_layout;
pub mod selection;
pub mod tree_gutter;
pub mod find_results;
pub mod widgets;
pub mod state;
mod message;
pub use message::{FileTreeAction, Message, ProjectMessage};
#[derive(Clone, Copy)]
pub struct FileTreeView<'a> {
    pub file_tree_drop_target: &'a Option<String>,
    pub file_tree_operation_in_progress: bool,
    pub file_tree_selection: &'a selection::FileTreeSelection,
    pub file_tree_expanded_set: &'a std::collections::HashSet<String>,
    pub git_changed_files: &'a [String],
    pub git_changed_files_loading: bool,
    pub git_focused_file: &'a Option<String>,
    pub file_manager_refresh_frame: usize,
    pub file_manager_changes_refreshing: bool,
    pub file_manager_file_tree_refreshing: bool,
    pub file_manager_show_changes: bool,
    pub file_index: &'a [String],
    pub file_tree_model: Option<&'a model::FileTreeNode>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub project_path: &'a Option<String>,
    pub find_results_tabs: &'a [state::FindInFolderTab],
    pub active_find_results_tab_id: &'a Option<String>,
    pub find_results_tab_menu_id: &'a Option<String>,
    pub find_results_tab_menu_pos: Option<iced::Point>,
    pub file_tree_rename_value: &'a str,
    pub file_tree_rename_path: &'a Option<String>,
    pub file_tree_menu_path: &'a Option<String>,
    pub file_tree_menu_source: &'a Option<String>,
    pub file_tree_menu_anchor: Option<iced::Point>,
    pub can_paste: bool,
}

#[cfg(test)]
mod model_tests;

#[cfg(test)]
mod icons_tests;

#[cfg(test)]
mod find_results_tests;

#[cfg(test)]
mod widgets_tests;

pub mod tree_list;
mod view;
pub use view::{view, view_file_manager};

use view::{scroll_tree, scroll_surface};

#[cfg(test)]
mod tree_list_tests;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_09_tests;

impl FileTreeView<'_> {
    pub fn visible_paths(&self, root: &str) -> Vec<String> {
        let fallback;
        let tree = match self.file_tree_model.filter(|tree| tree.has_entries()) {
            Some(tree) => tree,
            None => {
                fallback = model::build_file_tree_model(root, self.file_index);
                &fallback
            }
        };
        selection::visible_paths(tree, root, self.file_tree_expanded_set)
    }
}
