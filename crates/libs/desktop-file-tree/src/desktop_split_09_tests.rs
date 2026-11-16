use crate::{FileTreeView, selection, state};
use crate::{find_results, menu, model, tree_list};
use iced::advanced::renderer::Headless as _;
#[derive(Default)]
pub(super) struct ViewState {
    file_tree_drop_target: Option<String>,
    file_tree_operation_in_progress: bool,
    file_tree_selection: selection::FileTreeSelection,
    file_tree_expanded_set: std::collections::HashSet<String>,
    git_changed_files: Vec<String>,
    git_changed_files_loading: bool,
    git_focused_file: Option<String>,
    file_manager_refresh_frame: usize,
    file_manager_changes_refreshing: bool,
    file_manager_file_tree_refreshing: bool,
    file_manager_show_changes: bool,
    file_index: Vec<String>,
    desktop_locale: config_ui::DesktopLocale,
    project_path: Option<String>,
    find_results_tabs: Vec<state::FindInFolderTab>,
    active_find_results_tab_id: Option<String>,
    find_results_tab_menu_id: Option<String>,
    find_results_tab_menu_pos: Option<iced::Point>,
    file_tree_rename_value: String,
    file_tree_rename_path: Option<String>,
    file_tree_menu_path: Option<String>,
    file_tree_menu_source: Option<String>,
    file_tree_menu_anchor: Option<iced::Point>,
    can_paste: bool,
    file_tree_model: Option<crate::model::FileTreeNode>,
}
impl ViewState {
    fn inputs(&self) -> FileTreeView<'_> {
        FileTreeView {
            file_tree_drop_target: &self.file_tree_drop_target,
            file_tree_operation_in_progress: self.file_tree_operation_in_progress,
            file_tree_selection: &self.file_tree_selection,
            file_tree_expanded_set: &self.file_tree_expanded_set,
            git_changed_files: &self.git_changed_files,
            git_changed_files_loading: self.git_changed_files_loading,
            git_focused_file: &self.git_focused_file,
            file_manager_refresh_frame: self.file_manager_refresh_frame,
            file_manager_changes_refreshing: self.file_manager_changes_refreshing,
            file_manager_file_tree_refreshing: self.file_manager_file_tree_refreshing,
            file_manager_show_changes: self.file_manager_show_changes,
            file_index: &self.file_index,
            desktop_locale: self.desktop_locale,
            project_path: &self.project_path,
            find_results_tabs: &self.find_results_tabs,
            active_find_results_tab_id: &self.active_find_results_tab_id,
            find_results_tab_menu_id: &self.find_results_tab_menu_id,
            find_results_tab_menu_pos: self.find_results_tab_menu_pos,
            file_tree_rename_value: &self.file_tree_rename_value,
            file_tree_rename_path: &self.file_tree_rename_path,
            file_tree_menu_path: &self.file_tree_menu_path,
            file_tree_menu_source: &self.file_tree_menu_source,
            file_tree_menu_anchor: self.file_tree_menu_anchor,
            can_paste: self.can_paste,
            file_tree_model: self.file_tree_model.as_ref(),
        }
    }
}

async fn renderer() -> iced::Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    iced::Renderer::new(iced::Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .expect("tiny-skia renderer")
}

mod find_results_tests;
mod inline_rename_tests;
mod menu_tests;
mod tree_gutter_tests;
mod tree_list_tests;
