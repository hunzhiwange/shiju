//! 验证预览视口、标签宽度与滚动条显示的布局计算边界。
//! 文件管理器占宽、差异分栏比例和严格溢出阈值分别参与可用宽度计算。

use super::styles::{
    estimate_preview_tabs_viewport_width, should_show_preview_tabs_scrollbar,
};
use crate::app::components::editor::Editor;
use crate::app::{App, PreviewTab};

fn test_app() -> App {
    App::new().0
}

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "actual={actual}, expected={expected}");
}

fn preview_tab(title: &str) -> PreviewTab {
    PreviewTab {
        path: format!("/tmp/{title}"),
        title: title.to_string(),
        content: String::new(),
        is_dirty: false,
        truncated: false,
        read_only: false,
        auto_save_revision: 0,
        editor: Editor::new("", "txt"),
        scroll_id: iced::widget::Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

#[test]
fn viewport_width_accounts_for_file_manager_and_diff_split() {
    let mut app = test_app();
    app.window_size = (1000.0, 700.0);
    app.show_file_manager = false;
    app.show_diff = false;

    assert_eq!(estimate_preview_tabs_viewport_width(&app), 982.0);

    app.show_file_manager = true;
    app.file_manager_width = 240.0;
    assert_eq!(estimate_preview_tabs_viewport_width(&app), 734.0);

    app.show_diff = true;
    app.split_ratio = 0.75;
    assert_close(estimate_preview_tabs_viewport_width(&app), 168.0);

    app.split_ratio = 0.01;
    assert_close(estimate_preview_tabs_viewport_width(&app), 577.2);
}

#[test]
fn viewport_width_clamps_small_layouts_and_diff_ratio() {
    let mut app = test_app();
    app.show_file_manager = false;
    app.show_diff = false;
    app.window_size = (0.0, 700.0);
    assert_eq!(estimate_preview_tabs_viewport_width(&app), 1.0);

    app.window_size = (1000.0, 700.0);
    app.show_file_manager = true;
    app.file_manager_width = 2000.0;
    assert_eq!(estimate_preview_tabs_viewport_width(&app), 1.0);

    app.show_file_manager = false;
    app.show_diff = true;
    app.split_ratio = 2.0;
    assert_close(estimate_preview_tabs_viewport_width(&app), 180.4);

    app.split_ratio = -1.0;
    assert_close(estimate_preview_tabs_viewport_width(&app), 775.6);
}

#[test]
fn scrollbar_visibility_tracks_estimated_tab_widths() {
    let mut app = test_app();
    app.window_size = (1200.0, 700.0);
    app.preview_tabs = vec![];
    assert!(!should_show_preview_tabs_scrollbar(&app));

    app.preview_tabs = vec![preview_tab("main.rs")];
    assert!(!should_show_preview_tabs_scrollbar(&app));

    app.window_size = (160.0, 700.0);
    app.preview_tabs =
        vec![preview_tab("very-long-file-name-that-will-overflow-the-small-preview-tabs.rs")];
    assert!(should_show_preview_tabs_scrollbar(&app));
}

#[test]
fn scrollbar_visibility_uses_strict_boundary_and_tab_spacing() {
    let mut app = test_app();
    app.show_file_manager = false;
    app.show_diff = false;
    app.preview_tabs = vec![preview_tab("")];

    app.window_size = (116.0, 700.0);
    assert!(!should_show_preview_tabs_scrollbar(&app));
    app.window_size = (115.5, 700.0);
    assert!(should_show_preview_tabs_scrollbar(&app));

    app.preview_tabs = vec![preview_tab(""), preview_tab("")];
    app.window_size = (214.0, 700.0);
    assert!(!should_show_preview_tabs_scrollbar(&app));
    app.window_size = (213.5, 700.0);
    assert!(should_show_preview_tabs_scrollbar(&app));
}
