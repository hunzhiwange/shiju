use crate::app::{App, Message};
use crate::app::message::ProjectMessage;
pub(super) fn inputs(app: &App) -> desktop_file_tree::FileTreeView<'_> {
    desktop_file_tree::FileTreeView {
        desktop_locale: app.desktop_locale,
        project_path: &app.project_path,
        find_results_tabs: &app.find_results_tabs,
        active_find_results_tab_id: &app.active_find_results_tab_id,
        find_results_tab_menu_id: &app.find_results_tab_menu_id,
        find_results_tab_menu_pos: app.find_results_tab_menu_pos,
        file_tree_rename_value: &app.file_tree_rename_value,
        file_tree_rename_path: &app.file_tree_rename_path,
        file_tree_menu_path: &app.file_tree_menu_path,
        file_tree_menu_source: &app.file_tree_menu_source,
        file_tree_menu_anchor: app.file_tree_menu_anchor,
        can_paste: app.file_tree_clipboard.is_some(),
    }
}
pub(super) fn map(message: desktop_file_tree::Message) -> Message {
    use desktop_file_tree::ProjectMessage as Local;
    match message {
        desktop_file_tree::Message::None => Message::None,
        desktop_file_tree::Message::Project(message) => Message::Project(match message {
        Local::FileTreeRightClicked(v0, v1, v2, v3) => ProjectMessage::FileTreeRightClicked(v0, v1, v2, v3),
        Local::FileTreeMenuClose => ProjectMessage::FileTreeMenuClose,
        Local::FileTreeAction(v0) => ProjectMessage::FileTreeAction(v0),
        Local::FileTreeRenameChanged(v0) => ProjectMessage::FileTreeRenameChanged(v0),
        Local::FileTreeRenameSave => ProjectMessage::FileTreeRenameSave,
        Local::FileTreeFindScopeChanged(v0, v1) => ProjectMessage::FileTreeFindScopeChanged(v0, v1),
        Local::FileTreeFindQueryEditorAction(v0, v1) => ProjectMessage::FileTreeFindQueryEditorAction(v0, v1),
        Local::FileTreeFindReplaceEditorAction(v0, v1) => ProjectMessage::FileTreeFindReplaceEditorAction(v0, v1),
        Local::FileTreeFindCaseSensitiveToggled(v0, v1) => ProjectMessage::FileTreeFindCaseSensitiveToggled(v0, v1),
        Local::FileTreeFindWholeWordToggled(v0, v1) => ProjectMessage::FileTreeFindWholeWordToggled(v0, v1),
        Local::FileTreeFindRegexToggled(v0, v1) => ProjectMessage::FileTreeFindRegexToggled(v0, v1),
        Local::FileTreeFindResultsPageChanged(v0, v1) => ProjectMessage::FileTreeFindResultsPageChanged(v0, v1),
        Local::FileTreeFindRun(v0) => ProjectMessage::FileTreeFindRun(v0),
        Local::FileTreeFindInProject => ProjectMessage::FileTreeFindInProject,
        Local::FileTreeFindMatchSelected { path, line, column } => ProjectMessage::FileTreeFindMatchSelected { path, line, column },
        Local::FileTreeFindTabSelected(v0) => ProjectMessage::FileTreeFindTabSelected(v0),
        Local::FileTreeFindTabClosed(v0) => ProjectMessage::FileTreeFindTabClosed(v0),
        Local::FileTreeFindTabRightClicked(v0, v1, v2) => ProjectMessage::FileTreeFindTabRightClicked(v0, v1, v2),
        Local::FileTreeFindTabMenuClose => ProjectMessage::FileTreeFindTabMenuClose,
        Local::FileTreeFindTabMenuCloseLeft(v0) => ProjectMessage::FileTreeFindTabMenuCloseLeft(v0),
        Local::FileTreeFindTabMenuCloseRight(v0) => ProjectMessage::FileTreeFindTabMenuCloseRight(v0),
        Local::FileTreeFindTabMenuCloseAll => ProjectMessage::FileTreeFindTabMenuCloseAll,
        Local::FileTreeDragStart(v0, v1) => ProjectMessage::FileTreeDragStart(v0, v1),
        Local::FileTreeDragEnd => ProjectMessage::FileTreeDragEnd,
        }),
    }
}
