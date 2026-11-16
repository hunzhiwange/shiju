use super::view_file_manager;
use crate::app::App;
#[test]
fn file_manager_view_construction_preserves_code_panel_preferences() {
    let mut app = App::new().0;
    app.navigation_tabs.project_path = Some("/tmp/demo".to_string());
    app.file_manager.show_file_manager = false;
    app.navigation_panels.show_diff = false;
    app.layout_drag.split_ratio = 0.37;
    app.chat_panel_fullscreen = true;
    app.git.git_diff_fullscreen = true;
    let before = (
        app.file_manager.show_file_manager,
        app.navigation_panels.show_diff,
        app.layout_drag.split_ratio,
        app.chat_panel_fullscreen,
        app.git.git_diff_fullscreen,
    );

    let _ = view_file_manager(&app);

    assert_eq!(
        (
            app.file_manager.show_file_manager,
            app.navigation_panels.show_diff,
            app.layout_drag.split_ratio,
            app.chat_panel_fullscreen,
            app.git.git_diff_fullscreen,
        ),
        before
    );
}

#[test]
fn clipboard_modes_project_paste_availability() {
    use crate::app::state::{FileTreeClipboard, FileTreeClipboardMode};
    let mut app = App::new().0;
    assert!(!super::adapter::inputs(&app).can_paste);
    for mode in [FileTreeClipboardMode::Copy, FileTreeClipboardMode::Cut] {
        app.file_tree_interaction.file_tree_clipboard =
            Some(FileTreeClipboard { mode, src_paths: vec!["fixture/path".into()] });
        assert!(super::adapter::inputs(&app).can_paste);
    }
}
